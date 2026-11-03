use std::{collections::BTreeSet, ops::Range};

use gpui::{HighlightStyle, SharedString};

use super::{buffer::Buffer, state::Marks};

/// Rows an inline chat takes below its line.
pub(crate) const CHAT_ROWS: usize = 3;

/// One row the editor draws, all the same height.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Row {
    Line(usize),
    /// A code lens, by its place among the lenses.
    Lens(usize),
    /// A removed line of a diff hunk: the hunk, then the line within it.
    Removed(usize, usize),
    /// A line of ghost text after its first.
    Ghost(usize),
    /// The inline chat's rows; the first draws it.
    Chat(usize),
    /// Typewriter room past an end.
    Room,
}

/// Which lines folds hide.
pub(crate) fn hidden(
    lines: usize,
    folds: &[(usize, usize)],
    folded: &BTreeSet<usize>,
) -> Vec<bool> {
    let mut out = vec![false; lines];
    for (header, end) in folds.iter().filter(|(header, _)| folded.contains(header)) {
        for hidden in &mut out[header + 1..=*end] {
            *hidden = true;
        }
    }
    out
}

/// The rows in order: lenses and removed lines above their line, ghost lines and the chat below it, nothing a fold hides.
pub(crate) fn rows(buffer: &Buffer, hidden: &[bool], marks: &Marks) -> Vec<Row> {
    let ghost = marks.ghost.as_ref().map(|ghost| {
        let extra = ghost.text.split('\n').count() - 1;
        (buffer.line_of(ghost.offset), extra)
    });
    let mut out = Vec::with_capacity(buffer.lines());
    for line in (0..buffer.lines()).filter(|line| !hidden[*line]) {
        out.extend(
            marks
                .lenses
                .iter()
                .enumerate()
                .filter(|(_, lens)| lens.line == line)
                .map(|(ix, _)| Row::Lens(ix)),
        );
        for (ix, hunk) in marks
            .hunks
            .iter()
            .enumerate()
            .filter(|(_, hunk)| hunk.line == line)
        {
            out.extend((0..hunk.removed.len()).map(|at| Row::Removed(ix, at)));
        }
        out.push(Row::Line(line));
        if let Some((at, extra)) = ghost
            && at == line
        {
            out.extend((1..=extra).map(Row::Ghost));
        }
        if marks.chat == Some(line) {
            out.extend((0..CHAT_ROWS).map(Row::Chat));
        }
    }
    out
}

/// The headers of the blocks around `line`, outermost first, at most `limit` of them.
pub(crate) fn around(folds: &[(usize, usize)], line: usize, limit: usize) -> Vec<usize> {
    let mut headers: Vec<usize> = folds
        .iter()
        .filter(|(header, end)| *header < line && line <= *end)
        .map(|(header, _)| *header)
        .collect();
    headers.sort_unstable();
    headers.truncate(limit);
    headers
}

/// Styles laid over one another in order, later ones on top, as runs that do not overlap.
pub(crate) fn stack(
    spans: Vec<(Range<usize>, HighlightStyle)>,
) -> Vec<(Range<usize>, HighlightStyle)> {
    let mut edges: Vec<usize> = spans
        .iter()
        .flat_map(|(range, _)| [range.start, range.end])
        .collect();
    edges.sort_unstable();
    edges.dedup();
    edges
        .windows(2)
        .filter_map(|pair| {
            let (start, end) = (pair[0], pair[1]);
            spans
                .iter()
                .filter(|(range, _)| range.start <= start && end <= range.end)
                .fold(None, |below: Option<HighlightStyle>, (_, style)| {
                    Some(below.map_or(*style, |below| below.highlight(*style)))
                })
                .map(|style| (start..end, style))
        })
        .collect()
}

/// A line as drawn: its text with notes set in, their styles, and where each note went.
pub(crate) struct Shown {
    pub text: SharedString,
    pub styles: Vec<(Range<usize>, HighlightStyle)>,
    /// Byte in the line, then bytes and characters set in before it.
    inserts: Vec<(usize, usize, usize)>,
}

impl Shown {
    /// `line` with `notes` set in, each at a byte of the line and styled; `styles` color the line's own bytes.
    pub fn new(
        line: &str,
        mut notes: Vec<(usize, String, HighlightStyle)>,
        styles: Vec<(Range<usize>, HighlightStyle)>,
    ) -> Self {
        notes.sort_by_key(|(at, ..)| *at);
        let mut text = String::with_capacity(line.len());
        let mut inserts = Vec::with_capacity(notes.len());
        let mut own = Vec::with_capacity(notes.len());
        let mut last = 0;
        for (at, note, style) in notes {
            text.push_str(&line[last..at]);
            own.push((text.len()..text.len() + note.len(), style));
            text.push_str(&note);
            inserts.push((at, note.len(), note.chars().count()));
            last = at;
        }
        text.push_str(&line[last..]);
        let mut shown = Self {
            text: text.into(),
            styles: Vec::new(),
            inserts,
        };
        let mapped: Vec<(Range<usize>, HighlightStyle)> = styles
            .into_iter()
            .map(|(range, style)| {
                (
                    shown.byte(range.start, false)..shown.byte(range.end, true),
                    style,
                )
            })
            .collect();
        shown.styles = stack(mapped.into_iter().chain(own).collect());
        shown
    }

    /// Where a byte of the line lands; a range's end lands before a note set in at it.
    fn byte(&self, at: usize, end: bool) -> usize {
        at + self
            .inserts
            .iter()
            .filter(|(place, ..)| if end { *place < at } else { *place <= at })
            .map(|(_, bytes, _)| bytes)
            .sum::<usize>()
    }

    /// Where a caret at column `column` of `line` draws, in columns; it sits before a note set in at it.
    pub fn column(&self, line: &str, column: usize) -> usize {
        let byte = line
            .char_indices()
            .nth(column)
            .map_or(line.len(), |(at, _)| at);
        column
            + self
                .inserts
                .iter()
                .filter(|(place, ..)| *place < byte)
                .map(|(.., chars)| chars)
                .sum::<usize>()
    }

    /// The column of the line under drawn column `shown`, skipping notes.
    pub fn back(&self, line: &str, shown: usize) -> usize {
        let mut column = shown;
        for (place, _, chars) in &self.inserts {
            let at = line[..*place].chars().count();
            let drawn = self.column(line, at);
            if shown > drawn {
                column = column.saturating_sub((shown - drawn).min(*chars));
            }
        }
        column.min(line.chars().count())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::decor::{CodeLens, DiffHunk, GhostText};

    #[test]
    fn rows_skip_folds_and_add_what_lies_around_lines() {
        let buffer = Buffer::new("a\n  b\n  c\nd\ne");
        let marks = Marks {
            lenses: vec![CodeLens {
                line: 3,
                items: vec!["Run".into()],
            }],
            hunks: vec![DiffHunk {
                line: 4,
                added: 1,
                removed: vec!["old".into()],
            }],
            ghost: Some(GhostText {
                offset: 0,
                text: "x\ny".into(),
            }),
            ..Marks::default()
        };
        let hide = hidden(5, &[(0, 2)], &BTreeSet::from([0]));
        assert_eq!(
            rows(&buffer, &hide, &marks),
            [
                Row::Line(0),
                Row::Ghost(1),
                Row::Lens(0),
                Row::Line(3),
                Row::Removed(0, 0),
                Row::Line(4)
            ]
        );
        assert_eq!(around(&[(0, 4), (1, 2)], 2, 3), [0, 1]);
    }

    #[test]
    fn later_styles_sit_on_top_and_runs_do_not_overlap() {
        let red = gpui::hsla(0.0, 1.0, 0.5, 1.0);
        let blue = gpui::hsla(0.6, 1.0, 0.5, 1.0);
        let runs = stack(vec![(0..6, red.into()), (2..4, blue.into())]);
        let ranges: Vec<_> = runs.iter().map(|(range, _)| range.clone()).collect();
        assert_eq!(ranges, [0..2, 2..4, 4..6]);
        assert_eq!(
            runs[1].1.color,
            Some(blue),
            "the later span wins where both lie"
        );
    }

    #[test]
    fn notes_shift_what_follows_them_but_not_a_caret_at_them() {
        let style = HighlightStyle::default();
        let shown = Shown::new(
            "let x = 1;",
            vec![(5, ": i32".into(), style)],
            vec![(4..5, style)],
        );
        assert_eq!(shown.text.as_ref(), "let x: i32 = 1;");
        assert_eq!(
            shown.column("let x = 1;", 5),
            5,
            "a caret at the note stays before it"
        );
        assert_eq!(shown.column("let x = 1;", 6), 11);
        assert_eq!(shown.back("let x = 1;", 11), 6);
        assert_eq!(
            shown.back("let x = 1;", 8),
            5,
            "inside the note falls back to it"
        );
    }
}
