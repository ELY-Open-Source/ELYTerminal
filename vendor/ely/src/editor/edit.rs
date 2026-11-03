use std::ops::Range;

use gpui::{ClipboardItem, Context, ScrollStrategy};

use super::{
    anchors,
    buffer::Buffer,
    cursor::{Motion, Selection, merged, replace_each},
    state::CodeEditor,
};

/// Pairs typing an opener closes at once.
const PAIRS: [(char, char); 5] = [('(', ')'), ('[', ']'), ('{', '}'), ('"', '"'), ('\'', '\'')];

impl CodeEditor {
    /// Replaces each selection with `text`; typing in one burst makes one undo step.
    pub(crate) fn insert(&mut self, text: &str, typing: bool, cx: &mut Context<Self>) {
        let edits = self
            .selections
            .iter()
            .map(|selection| (selection.range(), text.to_string()))
            .collect();
        self.apply(edits, typing, cx);
    }

    /// Replaces ranges all at once and leaves a caret after each insert.
    pub(crate) fn apply(
        &mut self,
        edits: Vec<(Range<usize>, String)>,
        typing: bool,
        cx: &mut Context<Self>,
    ) {
        if !self.editable() {
            log::debug!("code editor: read only, edit dropped");
            return;
        }
        if edits.is_empty() {
            return;
        }
        let before = self.snapshot();
        let primary = self.primary().range().start;
        let edits = joined(edits);
        let spans: Vec<(Range<usize>, usize)> = edits
            .iter()
            .map(|(range, text)| (range.clone(), text.len()))
            .collect();
        let lead = edits
            .iter()
            .rposition(|(range, _)| range.start <= primary)
            .unwrap_or(0);
        let mut carets = replace_each(&mut self.buffer, edits);
        let lead = carets.remove(lead);
        carets.push(lead);
        self.selections = merged(carets);
        self.marked = None;
        anchors::shift(&mut self.marks, &spans);
        self.commit(before, typing, cx);
        self.reveal(true, cx);
    }

    /// Types text at each cursor; an opener brings its closer, and a closer already there is stepped over.
    pub(crate) fn type_text(&mut self, text: &str, cx: &mut Context<Self>) {
        if !self.editable() {
            return;
        }
        let mut chars = text.chars();
        let (Some(ch), None) = (chars.next(), chars.next()) else {
            return self.insert(text, true, cx);
        };
        let next = |caret: usize| self.buffer.text()[caret..].chars().next();
        let all_empty = self.selections.iter().all(Selection::is_empty);
        if all_empty
            && PAIRS.iter().any(|(_, close)| *close == ch)
            && self
                .selections
                .iter()
                .all(|selection| next(selection.head) == Some(ch))
        {
            return self.motion(Motion::Right, false, cx);
        }
        let Some((open, close)) = PAIRS.iter().find(|(open, _)| *open == ch) else {
            return self.insert(text, true, cx);
        };
        let free = |caret: usize| {
            next(caret).is_none_or(|after| after.is_whitespace() || ")]}".contains(after))
        };
        if !all_empty || !self.selections.iter().all(|selection| free(selection.head)) {
            return self.insert(text, true, cx);
        }
        let edits = self
            .selections
            .iter()
            .map(|selection| (selection.range(), format!("{open}{close}")))
            .collect();
        self.apply(edits, true, cx);
        let back: Vec<Selection> = self
            .selections
            .iter()
            .map(|caret| Selection::caret(caret.head - close.len_utf8()))
            .collect();
        self.set_selections(back, cx);
    }

    /// Deletes each selection, or what `reach` finds from each bare caret.
    pub(crate) fn delete_by(
        &mut self,
        reach: fn(&Buffer, usize) -> Range<usize>,
        cx: &mut Context<Self>,
    ) {
        if !self.editable() {
            return;
        }
        let edits = self
            .selections
            .iter()
            .map(|selection| {
                let range = if selection.is_empty() {
                    reach(&self.buffer, selection.head)
                } else {
                    selection.range()
                };
                (range, String::new())
            })
            .collect();
        self.apply(edits, false, cx);
    }

    /// A new line at each cursor, indented like its own; one step deeper after an opener, with the closer on a line of its own.
    pub(crate) fn newline(&mut self, cx: &mut Context<Self>) {
        if !self.editable() {
            return;
        }
        let tab = self.options.tab;
        let edits = self
            .selections
            .iter()
            .map(|selection| {
                let at = selection.range().start;
                let line = self.buffer.line_of(at);
                let indent: String = self
                    .buffer
                    .line(line)
                    .chars()
                    .take(self.buffer.indent(line))
                    .collect();
                let before = self.buffer.text()[..at].trim_end().chars().next_back();
                let after = self.buffer.text()[selection.range().end..].chars().next();
                let opens = before.is_some_and(|ch| "{[(:".contains(ch));
                let text = match (opens, after) {
                    (true, Some(close)) if "}])".contains(close) => {
                        format!("\n{indent}{}\n{indent}", " ".repeat(tab))
                    }
                    (true, _) => format!("\n{indent}{}", " ".repeat(tab)),
                    _ => format!("\n{indent}"),
                };
                (selection.range(), text)
            })
            .collect::<Vec<_>>();
        let closers: Vec<bool> = edits
            .iter()
            .map(|(_, text)| text.matches('\n').count() == 2)
            .collect();
        self.apply(edits, false, cx);
        let placed: Vec<Selection> = self
            .selections
            .iter()
            .zip(closers)
            .map(|(caret, split)| {
                if split {
                    let line = self.buffer.line_of(caret.head);
                    Selection::caret(self.buffer.line_range(line - 1).end)
                } else {
                    *caret
                }
            })
            .collect();
        self.set_selections(placed, cx);
    }

    /// The lines the selections touch, each once.
    fn touched(&self) -> Vec<usize> {
        let mut lines: Vec<usize> = self
            .selections
            .iter()
            .flat_map(|selection| {
                let range = selection.range();
                let last = if range.end > range.start && self.buffer.point(range.end).1 == 0 {
                    range.end - 1
                } else {
                    range.end
                };
                self.buffer.line_of(range.start)..=self.buffer.line_of(last)
            })
            .collect();
        lines.sort_unstable();
        lines.dedup();
        lines
    }

    /// Tab: takes ghost text when there is some, indents touched lines when a selection spans lines, or fills to the next tab stop.
    pub(crate) fn indent(&mut self, cx: &mut Context<Self>) {
        if !self.editable() {
            return;
        }
        if let Some(ghost) = self.marks.ghost.take() {
            log::info!("code editor: ghost text taken");
            self.set_selections(vec![Selection::caret(ghost.offset)], cx);
            return self.insert(&ghost.text, false, cx);
        }
        let tab = self.options.tab;
        let spans_lines = self.selections.iter().any(|selection| {
            self.buffer.line_of(selection.range().start)
                != self.buffer.line_of(selection.range().end)
        });
        if !spans_lines {
            let edits = self
                .selections
                .iter()
                .map(|selection| {
                    let column = self.buffer.point(selection.range().start).1;
                    (selection.range(), " ".repeat(tab - column % tab))
                })
                .collect();
            return self.apply(edits, false, cx);
        }
        let kept = self.selections.clone();
        let edits = self
            .touched()
            .into_iter()
            .filter(|line| !self.buffer.line(*line).trim().is_empty())
            .map(|line| {
                let start = self.buffer.line_range(line).start;
                (start..start, " ".repeat(tab))
            })
            .collect();
        self.shift_lines(edits, kept, cx);
    }

    /// Shift-Tab: takes one tab stop of indent off each touched line.
    pub(crate) fn outdent(&mut self, cx: &mut Context<Self>) {
        if !self.editable() {
            return;
        }
        let tab = self.options.tab;
        let kept = self.selections.clone();
        let edits = self
            .touched()
            .into_iter()
            .filter_map(|line| {
                let start = self.buffer.line_range(line).start;
                let cut = self.buffer.indent(line).min(tab);
                (cut > 0).then(|| (start..start + cut, String::new()))
            })
            .collect();
        self.shift_lines(edits, kept, cx);
    }

    /// Applies line-start edits and keeps each selection over the same text.
    fn shift_lines(
        &mut self,
        edits: Vec<(Range<usize>, String)>,
        kept: Vec<Selection>,
        cx: &mut Context<Self>,
    ) {
        let moves: Vec<(std::ops::Range<usize>, isize)> = edits
            .iter()
            .map(|(range, text)| (range.clone(), text.len() as isize - range.len() as isize))
            .collect();
        let shift = |offset: usize| -> usize {
            let mut moved = offset as isize;
            for (range, delta) in &moves {
                if range.start < offset {
                    if offset < range.end {
                        moved -= (offset - range.start) as isize;
                    } else {
                        moved += delta;
                    }
                }
            }
            moved.max(0) as usize
        };
        let kept: Vec<Selection> = kept
            .into_iter()
            .map(|selection| Selection {
                anchor: shift(selection.anchor),
                head: shift(selection.head),
                goal: None,
            })
            .collect();
        self.apply(edits, false, cx);
        self.set_selections(kept, cx);
    }

    /// Comments the touched lines with `//`, or takes the comment off when every one has it.
    pub(crate) fn toggle_comment(&mut self, cx: &mut Context<Self>) {
        if !self.editable() {
            return;
        }
        let lines: Vec<usize> = self
            .touched()
            .into_iter()
            .filter(|line| !self.buffer.line(*line).trim().is_empty())
            .collect();
        let commented = |line: usize| self.buffer.line(line).trim_start().starts_with("//");
        let kept = self.selections.clone();
        let edits = if !lines.is_empty() && lines.iter().all(|line| commented(*line)) {
            lines
                .iter()
                .map(|line| {
                    let start = self.buffer.line_range(*line).start + self.buffer.indent(*line);
                    let text = &self.buffer.text()[start..];
                    let cut = if text.starts_with("// ") { 3 } else { 2 };
                    (start..start + cut, String::new())
                })
                .collect()
        } else {
            let column = lines
                .iter()
                .map(|line| self.buffer.indent(*line))
                .min()
                .unwrap_or(0);
            lines
                .iter()
                .map(|line| {
                    let at = self.buffer.offset(*line, column);
                    (at..at, "// ".to_string())
                })
                .collect()
        };
        self.shift_lines(edits, kept, cx);
    }

    pub(crate) fn undo(&mut self, cx: &mut Context<Self>) {
        if !self.editable() {
            return;
        }
        if let Some(previous) = self.history.undo(self.snapshot()) {
            self.restore(previous, cx);
            self.cap_history();
        }
    }

    pub(crate) fn redo(&mut self, cx: &mut Context<Self>) {
        if !self.editable() {
            return;
        }
        if let Some(next) = self.history.redo(self.snapshot()) {
            self.restore(next, cx);
            self.cap_history();
        }
    }

    /// Each selection's text, a line apiece; the whole line for a bare caret.
    fn copied(&self) -> Vec<(Range<usize>, String)> {
        self.in_order()
            .iter()
            .map(|selection| {
                let range = if selection.is_empty() {
                    let line = self.buffer.line_of(selection.head);
                    let range = self.buffer.line_range(line);
                    range.start..self.buffer.next(range.end)
                } else {
                    selection.range()
                };
                (range.clone(), self.buffer.text()[range].to_string())
            })
            .collect()
    }

    pub(crate) fn copy(&mut self, cx: &mut Context<Self>) {
        let text: Vec<String> = self.copied().into_iter().map(|(_, text)| text).collect();
        cx.write_to_clipboard(ClipboardItem::new_string(text.join("\n")));
    }

    pub(crate) fn cut(&mut self, cx: &mut Context<Self>) {
        if !self.editable() {
            return;
        }
        let copied = self.copied();
        let text: Vec<String> = copied.iter().map(|(_, text)| text.clone()).collect();
        cx.write_to_clipboard(ClipboardItem::new_string(text.join("\n")));
        self.apply(
            copied
                .into_iter()
                .map(|(range, _)| (range, String::new()))
                .collect(),
            false,
            cx,
        );
    }

    /// Pastes one line at each cursor when the counts match, else the whole text at every one.
    pub(crate) fn paste(&mut self, text: &str, cx: &mut Context<Self>) {
        if !self.editable() {
            return;
        }
        let lines: Vec<&str> = text.split('\n').collect();
        let edits = if lines.len() == self.selections.len() && lines.len() > 1 {
            self.in_order()
                .iter()
                .zip(lines)
                .map(|(selection, line)| (selection.range(), line.to_string()))
                .collect()
        } else {
            self.selections
                .iter()
                .map(|selection| (selection.range(), text.to_string()))
                .collect()
        };
        self.apply(edits, false, cx);
    }
}

impl CodeEditor {
    /// The selections from the top of the text down; the primary one is otherwise last.
    fn in_order(&self) -> Vec<Selection> {
        let mut all = self.selections.clone();
        all.sort_by_key(|selection| selection.range().start);
        all
    }
}

/// Edits sorted, with overlapping ranges joined; the first text of a joined run is kept.
fn joined(mut edits: Vec<(Range<usize>, String)>) -> Vec<(Range<usize>, String)> {
    edits.sort_by_key(|(range, _)| range.start);
    let mut out: Vec<(Range<usize>, String)> = Vec::with_capacity(edits.len());
    for (range, text) in edits {
        match out.last_mut() {
            Some((last, _)) if range.start < last.end => last.end = last.end.max(range.end),
            _ => out.push((range, text)),
        }
    }
    out
}

impl CodeEditor {
    /// Selects these ranges, the last one primary.
    pub fn select(
        &mut self,
        ranges: impl IntoIterator<Item = Range<usize>>,
        cx: &mut Context<Self>,
    ) {
        let chosen: Vec<Selection> = ranges
            .into_iter()
            .filter(|range| {
                let whole = self.whole(range);
                if !whole {
                    log::error!(
                        "code editor: no selection {range:?} in {} bytes",
                        self.buffer.len()
                    );
                }
                whole
            })
            .map(|range| Selection {
                anchor: range.start,
                head: range.end,
                goal: None,
            })
            .collect();
        if chosen.is_empty() {
            return;
        }
        if merged(chosen.clone()) == self.selections {
            return;
        }
        self.set_selections(chosen, cx);
        self.scroll
            .scroll_to_item(self.primary_row(), ScrollStrategy::Center);
    }

    /// Replaces ranges of the text at once, as one undo step; read-only text refuses.
    pub fn edit(
        &mut self,
        edits: impl IntoIterator<Item = (Range<usize>, String)>,
        cx: &mut Context<Self>,
    ) {
        let edits: Vec<(Range<usize>, String)> = edits.into_iter().collect();
        let mut starts: Vec<&Range<usize>> = edits.iter().map(|(range, _)| range).collect();
        starts.sort_by_key(|range| range.start);
        let apart = starts.windows(2).all(|pair| pair[0].end <= pair[1].start);
        let forward = edits.iter().all(|(range, _)| range.start <= range.end);
        if !apart || !forward || !edits.iter().all(|(range, _)| self.whole(range)) {
            log::error!(
                "code editor: edits {:?} do not fit {} bytes; none made",
                starts,
                self.buffer.len()
            );
            return;
        }
        self.apply(edits, false, cx);
    }

    /// Whether both ends of `range` fall between whole characters of the text.
    fn whole(&self, range: &Range<usize>) -> bool {
        let text = self.buffer.text();
        text.is_char_boundary(range.start) && text.is_char_boundary(range.end)
    }
}
