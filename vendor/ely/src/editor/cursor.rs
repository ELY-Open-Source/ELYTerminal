use std::ops::Range;

use super::buffer::Buffer;

/// A cursor and the text it holds: `head` moves, `anchor` stays; `goal` keeps the column a vertical move aims for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Selection {
    pub anchor: usize,
    pub head: usize,
    pub goal: Option<usize>,
}

impl Selection {
    pub fn caret(at: usize) -> Self {
        Self {
            anchor: at,
            head: at,
            goal: None,
        }
    }

    pub fn range(&self) -> Range<usize> {
        self.anchor.min(self.head)..self.anchor.max(self.head)
    }

    pub fn is_empty(&self) -> bool {
        self.anchor == self.head
    }

    /// Moves the head, keeping the anchor when `extend`, or collapsing onto the head.
    fn to(self, head: usize, extend: bool) -> Self {
        Self {
            anchor: if extend { self.anchor } else { head },
            head,
            goal: None,
        }
    }
}

/// Sorted by where they start, overlapping selections joined into one; the last given, the primary, stays last.
pub(crate) fn merged(all: Vec<Selection>) -> Vec<Selection> {
    assert!(!all.is_empty(), "an editor keeps at least one cursor");
    let primary = all.len() - 1;
    let mut all: Vec<(usize, Selection)> = all.into_iter().enumerate().collect();
    all.sort_by_key(|(_, selection)| selection.range().start);
    let mut out: Vec<Selection> = Vec::with_capacity(all.len());
    let mut lead = 0;
    for (given, next) in all {
        match out.last_mut() {
            Some(last)
                if next.range().start < last.range().end
                    || (next.range() == last.range() && next.is_empty()) =>
            {
                let range = last.range().start..last.range().end.max(next.range().end);
                let forward = last.head >= last.anchor;
                *last = Selection {
                    anchor: if forward { range.start } else { range.end },
                    head: if forward { range.end } else { range.start },
                    goal: None,
                };
            }
            _ => out.push(next),
        }
        if given == primary {
            lead = out.len() - 1;
        }
    }
    let lead = out.remove(lead);
    out.push(lead);
    out
}

/// Where a cursor goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Motion {
    Left,
    Right,
    Up,
    Down,
    WordLeft,
    WordRight,
    LineStart,
    LineEnd,
    Start,
    End,
    /// Down or up a page of this many lines.
    Page(isize),
}

/// A selection after a motion. Left and Right close a selection to its edge first.
pub(crate) fn moved(
    buffer: &Buffer,
    selection: Selection,
    motion: Motion,
    extend: bool,
) -> Selection {
    let head = selection.head;
    if !extend && !selection.is_empty() {
        match motion {
            Motion::Left => return Selection::caret(selection.range().start),
            Motion::Right => return Selection::caret(selection.range().end),
            _ => {}
        }
    }
    let vertical = |lines: isize| {
        let (line, column) = buffer.point(head);
        let goal = selection.goal.unwrap_or(column);
        let target = line as isize + lines;
        let offset = if target < 0 {
            0
        } else if target >= buffer.lines() as isize {
            buffer.len()
        } else {
            buffer.offset(target as usize, goal)
        };
        Selection {
            goal: Some(goal),
            ..selection.to(offset, extend)
        }
    };
    match motion {
        Motion::Left => selection.to(buffer.previous(head), extend),
        Motion::Right => selection.to(buffer.next(head), extend),
        Motion::Up => vertical(-1),
        Motion::Down => vertical(1),
        Motion::Page(lines) => vertical(lines),
        Motion::WordLeft => selection.to(buffer.word_start(head), extend),
        Motion::WordRight => selection.to(buffer.word_end(head), extend),
        Motion::LineStart => {
            let line = buffer.line_of(head);
            let start = buffer.line_range(line).start;
            let text = buffer.line_range(line).start + buffer.indent(line);
            selection.to(if head == text { start } else { text }, extend)
        }
        Motion::LineEnd => selection.to(buffer.line_range(buffer.line_of(head)).end, extend),
        Motion::Start => selection.to(0, extend),
        Motion::End => selection.to(buffer.len(), extend),
    }
}

/// Replaces each selection's range with its text, all at once, and returns a caret after each insert.
pub(crate) fn replace_each(
    buffer: &mut Buffer,
    edits: Vec<(Range<usize>, String)>,
) -> Vec<Selection> {
    let mut order: Vec<usize> = (0..edits.len()).collect();
    order.sort_by_key(|ix| edits[*ix].0.start);
    assert!(
        order
            .windows(2)
            .all(|pair| edits[pair[0]].0.end <= edits[pair[1]].0.start),
        "edits do not overlap"
    );
    let mut carets = vec![Selection::caret(0); edits.len()];
    let mut shift: isize = 0;
    for ix in &order {
        let (range, text) = &edits[*ix];
        let start = (range.start as isize + shift) as usize;
        carets[*ix] = Selection::caret(start + text.len());
        shift += text.len() as isize - range.len() as isize;
    }
    for ix in order.iter().rev() {
        let (range, text) = &edits[*ix];
        buffer.replace(range.clone(), text);
    }
    carets
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_last_cursor_given_leads_even_beside_a_touching_selection() {
        let all = merged(vec![
            Selection {
                anchor: 0,
                head: 1,
                goal: None,
            },
            Selection::caret(1),
        ]);
        assert_eq!(all.last(), Some(&Selection::caret(1)));
    }

    #[test]
    fn overlapping_selections_join_and_carets_on_one_spot_become_one() {
        let joined = merged(vec![
            Selection {
                anchor: 4,
                head: 8,
                goal: None,
            },
            Selection {
                anchor: 6,
                head: 10,
                goal: None,
            },
            Selection::caret(12),
            Selection::caret(12),
        ]);
        assert_eq!(joined.len(), 2);
        assert_eq!(joined[0].range(), 4..10);
        assert_eq!(joined[1], Selection::caret(12));
    }

    #[test]
    fn vertical_moves_keep_their_column_through_short_lines() {
        let buffer = Buffer::new("abcdef\nab\nabcdef");
        let start = Selection::caret(5);
        let down = moved(&buffer, start, Motion::Down, false);
        assert_eq!(
            buffer.point(down.head),
            (1, 2),
            "held to the short line's end"
        );
        let again = moved(&buffer, down, Motion::Down, false);
        assert_eq!(
            buffer.point(again.head),
            (2, 5),
            "back to the column it aimed for"
        );
        let selected = moved(&buffer, start, Motion::End, true);
        assert_eq!(selected.range(), 5..buffer.len());
        assert_eq!(
            moved(&buffer, selected, Motion::Left, false),
            Selection::caret(5)
        );
    }

    #[test]
    fn home_toggles_between_the_text_and_the_line_start() {
        let buffer = Buffer::new("    let x;");
        let first = moved(&buffer, Selection::caret(9), Motion::LineStart, false);
        assert_eq!(first.head, 4);
        assert_eq!(moved(&buffer, first, Motion::LineStart, false).head, 0);
    }

    #[test]
    fn edits_at_many_cursors_land_where_each_one_was() {
        let mut buffer = Buffer::new("a1\nb2\nc3");
        let carets = replace_each(
            &mut buffer,
            vec![(7..7, "!".into()), (1..1, "!".into()), (4..4, "!".into())],
        );
        assert_eq!(buffer.text(), "a!1\nb!2\nc!3");
        assert_eq!(
            carets.iter().map(|caret| caret.head).collect::<Vec<_>>(),
            [10, 2, 6]
        );
    }
}
