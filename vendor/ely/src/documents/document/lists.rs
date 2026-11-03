//! Lists go on as typed; lines shift in and out.

use std::ops::Range;

use gpui::{Context, Window};
use markdown::mdast;

use super::{
    MarkdownDocument,
    cells::{Aim, Move, beside},
    edit::Landing,
};
use crate::documents::markdown::{span, tree, walk};

/// Spaces one indent step adds.
const STEP: usize = 4;

/// Enter at `at`: the edit, none outside list items.
pub(super) fn enter(source: &str, at: usize) -> Option<(Range<usize>, String)> {
    let root = tree(source);
    let mut nodes = Vec::new();
    walk(&root, &mut nodes);
    let held: Vec<&mdast::Node> = nodes
        .into_iter()
        .filter(|node| span(node).start <= at && at <= span(node).end)
        .collect();
    let raw = |node: &&&mdast::Node| matches!(node, mdast::Node::Code(_) | mdast::Node::Math(_));
    if held.iter().any(|node| raw(&node)) {
        return None;
    }
    let item = held
        .iter()
        .filter(|node| matches!(node, mdast::Node::ListItem(_)))
        .min_by_key(|node| span(node).len())?;
    // The item may start at its indent; the marker follows.
    let lead = &source[span(item).start..];
    let start = span(item).start + lead.len() - lead.trim_start_matches([' ', '\t']).len();
    let first = source[..start].rfind('\n').map_or(0, |found| found + 1);
    // Quote marks stay; a parent's marker becomes spaces.
    let container: String = source[first..start]
        .chars()
        .map(|ch| {
            if ch == '>' || ch.is_whitespace() {
                ch
            } else {
                ' '
            }
        })
        .collect();
    let rest = &source[start..];
    let (marker, after) = bullet(rest).or_else(|| ordered(rest))?;
    let gap = rest[after..].len() - rest[after..].trim_start_matches([' ', '\t']).len();
    let gap = match &rest[after..after + gap] {
        "" => " ",
        written => written,
    };
    let body = source[start + after..span(item).end].trim_start();
    // A bare box lacks `checked` yet is a task.
    let boxed = ["[ ]", "[x]", "[X]"]
        .iter()
        .any(|task| body.starts_with(task));
    let task = boxed || matches!(item, mdast::Node::ListItem(item) if item.checked.is_some());
    let body = match boxed {
        true => &body[3..],
        false => body,
    };
    let end = source[at..]
        .find('\n')
        .map_or(source.len(), |found| at + found);
    if body.trim().is_empty() {
        // An empty item ends the list.
        return (at == end).then(|| (start..end, String::new()));
    }
    let task = if task { "[ ] " } else { "" };
    Some((at..at, format!("\n{container}{marker}{gap}{task}")))
}

/// A bullet marker and its length.
fn bullet(rest: &str) -> Option<(String, usize)> {
    let mark = rest.chars().next().filter(|ch| "-*+".contains(*ch))?;
    Some((mark.to_string(), 1))
}

/// A number marker, counted on, and its length.
fn ordered(rest: &str) -> Option<(String, usize)> {
    let digits = rest.chars().take_while(char::is_ascii_digit).count();
    if !(1..=9).contains(&digits) {
        return None;
    }
    let close = rest[digits..]
        .chars()
        .next()
        .filter(|ch| ".)".contains(*ch))?;
    let number: u64 = rest[..digits].parse().expect("ascii digits");
    Some((format!("{}{close}", number + 1), digits + 1))
}

/// Leading bytes making one step; tabs stop every four.
fn outdent(line: &str) -> usize {
    let mut columns = 0;
    for (bytes, ch) in line.char_indices() {
        if columns >= STEP {
            return bytes;
        }
        columns = match ch {
            ' ' => columns + 1,
            '\t' => (columns / STEP + 1) * STEP,
            _ => return bytes,
        };
    }
    line.len()
}

/// Lines `range` touches, a step in or out.
pub(super) fn shift(text: &str, range: Range<usize>, deeper: bool) -> (String, Range<usize>) {
    let first = text[..range.start].rfind('\n').map_or(0, |at| at + 1);
    let last = text[range.end..]
        .find('\n')
        .map_or(text.len(), |at| range.end + at);
    // Each change: where, and bytes added or removed.
    let mut changes: Vec<(usize, isize)> = Vec::new();
    let mut at = first;
    loop {
        let line = &text[at..text[at..].find('\n').map_or(text.len(), |end| at + end)];
        match deeper {
            true => changes.push((at, STEP as isize)),
            false => match outdent(line) {
                0 => {}
                bytes => changes.push((at, -(bytes as isize))),
            },
        }
        match text[at..].find('\n') {
            Some(end) if at + end < last => at += end + 1,
            _ => break,
        }
    }
    let mut out = String::with_capacity(text.len() + changes.len() * STEP);
    let mut from = 0;
    for (at, change) in &changes {
        out.push_str(&text[from..*at]);
        from = *at;
        match *change > 0 {
            true => out.push_str(&" ".repeat(STEP)),
            false => from += change.unsigned_abs(),
        }
    }
    out.push_str(&text[from..]);
    let moved = |pos: usize| {
        let mut shifted = pos as isize;
        for (at, change) in &changes {
            match *change > 0 {
                true if *at <= pos => shifted += change,
                false if *at < pos => shifted -= (pos - at).min(change.unsigned_abs()) as isize,
                _ => {}
            }
        }
        shifted as usize
    };
    (out, moved(range.start)..moved(range.end))
}

impl MarkdownDocument {
    /// Enter on a list line in source; false leaves it.
    pub(super) fn source_newline(&mut self, cx: &mut Context<Self>) -> bool {
        let editor = self.editor.read(cx);
        let caret = editor.primary().range();
        if editor.selections().len() != 1 || !caret.is_empty() {
            return false;
        }
        if !self.read_now(cx) {
            self.reparse(true, cx);
        }
        if !self.read_now(cx) {
            log::info!("markdown document: Enter while a long text is read");
            return false;
        }
        let editor = self.editor.read(cx);
        let text = editor.text();
        let ix = self.block_at(caret.start);
        let block = self.parsed.blocks.get(ix).map(|block| block.range.clone());
        let Some(block) =
            block.filter(|block| block.start <= caret.start && caret.start <= block.end)
        else {
            return false;
        };
        let at = caret.start - block.start;
        let Some((range, typed)) = enter(&text[block.clone()], at) else {
            return false;
        };
        let range = range.start + block.start..range.end + block.start;
        let after = range.start + typed.len();
        let edit = (range, typed);
        log::info!("markdown document: a list goes on in source");
        self.editor.update(cx, |editor, cx| {
            editor.edit([edit], cx);
            editor.select(std::iter::once(after..after), cx);
        });
        true
    }

    /// Enter on a block list line; false leaves it.
    pub(super) fn block_newline(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(open) = self.open.as_ref().filter(|open| open.cell.is_none()) else {
            return false;
        };
        let input = open.input.clone();
        let (text, caret) = {
            let input = input.read(cx);
            (input.text().to_string(), input.selection())
        };
        if !caret.is_empty() {
            return false;
        }
        let Some((range, typed)) = enter(&text, caret.start) else {
            return false;
        };
        log::info!("markdown document: a list goes on in a block");
        input.update(cx, |input, cx| {
            input.select(range, cx);
            input.insert(&typed, cx);
        });
        true
    }

    /// Tab or Shift-Tab; in a cell, moves between cells.
    pub(super) fn block_shift(
        &mut self,
        deeper: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(open) = &self.open else {
            return;
        };
        if let Some(at) = open.cell {
            let here = self.block_at(open.range.start);
            let block = self.parsed.blocks[here].range.clone();
            let by = if deeper { Move::Next } else { Move::Previous };
            let text = self.editor.read(cx).text();
            match beside(text, block, at, by) {
                Some(to) => self.open(Some(here), Landing::Cell(Aim::At(to)), window, cx),
                None => log::info!("markdown document: no cell {by:?} of {at:?}"),
            }
            return;
        }
        let input = open.input.clone();
        let (text, selection) = {
            let input = input.read(cx);
            (input.text().to_string(), input.selection())
        };
        let (shifted, selection) = shift(&text, selection, deeper);
        input.update(cx, |input, cx| {
            input.set_text(shifted, cx);
            input.select(selection, cx);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::{enter, shift};

    /// Enter at the end of `source`'s line holding `at`.
    fn entered(source: &str, at: &str) -> Option<String> {
        let at = source.find(at).expect("in the text") + at.len();
        let (range, typed) = enter(source, at)?;
        Some(format!(
            "{}{typed}{}",
            &source[..range.start],
            &source[range.end..]
        ))
    }

    #[test]
    fn items_go_on_with_their_marker() {
        assert_eq!(entered("- a", "a").as_deref(), Some("- a\n- "));
        assert_eq!(entered("  * a", "a").as_deref(), Some("  * a\n  * "));
        assert_eq!(entered("9. a", "a").as_deref(), Some("9. a\n10. "));
        assert_eq!(entered("3) a", "a").as_deref(), Some("3) a\n4) "));
        assert_eq!(
            entered("- [x] done", "done").as_deref(),
            Some("- [x] done\n- [ ] ")
        );
        assert_eq!(entered("- a\n- ", "- a\n- ").as_deref(), Some("- a\n"));
        let plain = [
            ("text", "text"),
            ("-a", "-a"),
            ("* * *", "* * *"),
            ("```\n- a\n```", "- a"),
            ("1.a", "1.a"),
        ];
        for (plain, at) in plain {
            assert_eq!(entered(plain, at), None, "{plain:?}");
        }
    }

    #[test]
    fn quotes_tabs_and_nesting_carry_on() {
        assert_eq!(entered("> - a", "a").as_deref(), Some("> - a\n> - "));
        assert_eq!(entered("-\ta", "a").as_deref(), Some("-\ta\n-\t"));
        assert_eq!(
            entered("- a\n\t- b", "b").as_deref(),
            Some("- a\n\t- b\n\t- ")
        );
        assert_eq!(entered("- - a", "a").as_deref(), Some("- - a\n  - "));
    }

    #[test]
    fn lines_shift_in_and_out_with_the_selection() {
        assert_eq!(shift("a\nb", 0..0, true), ("    a\nb".into(), 4..4));
        assert_eq!(shift("a\nb", 1..3, true), ("    a\n    b".into(), 5..11));
        assert_eq!(shift("      a", 7..7, false), ("  a".into(), 3..3));
        assert_eq!(shift("  a\nb", 1..1, false), ("a\nb".into(), 0..0));
        // Spaces and tabs mixed: four columns go.
        assert_eq!(shift("  \t- b", 6..6, false), ("- b".into(), 3..3));
        assert_eq!(shift("\t  - b", 6..6, false), ("  - b".into(), 5..5));
        assert_eq!(shift(" \t\t- b", 6..6, false), ("\t- b".into(), 4..4));
    }
}
