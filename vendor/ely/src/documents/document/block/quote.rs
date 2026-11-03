//! Quotes on and off, as Typora's `toggleIndent`.

use std::ops::Range;

use markdown::mdast::Node;

use super::{
    Here, change, check, eol, inside, line_end, line_start, next_line, prefix, split::content,
    task_head,
};
use crate::documents::{
    alert,
    document::format::{Change, EditRefusal},
    markdown::{file_tree, span},
};

pub(super) fn run(here: &Here) -> Result<Change, EditRefusal> {
    let caret = here.selection.clone();
    let ix = match here.leaf() {
        Some(_) => here.path.len() - 2,
        None => here.path.len() - 1,
    };
    if let Node::Blockquote(_) = here.path[ix]
        && whole(here, ix)
    {
        return unwrap(here, ix).map(|edits| change(edits, caret));
    }
    if here.leaf().is_none() && here.selection.is_empty() {
        if !here.blank() {
            return Err(EditRefusal::Outside);
        }
        let line = line_start(here.text, caret.start);
        let at = inside(here.text, line, &here.containers(here.path.len()))
            .expect("a blank line holds its prefix");
        return Ok(change(vec![(at..at, "> ".to_string())], caret));
    }
    let (k, blocks) = here.blocks().ok_or(EditRefusal::Outside)?;
    // GFM reads a task's first line as text.
    if task_head(here.path[k], blocks[0]) {
        return Err(EditRefusal::Outside);
    }
    Ok(change(wrap(here, k, &blocks), caret))
}

/// Whether the selection takes in the whole quote at `ix`.
fn whole(here: &Here, ix: usize) -> bool {
    if here.selection.is_empty() {
        return true;
    }
    let quote = here.path[ix];
    let end = here.path_at(here.selection.end);
    let kids = quote.children().map(Vec::as_slice).unwrap_or_default();
    let first = here.path.get(ix + 1).copied();
    let last = end.get(ix + 1).copied();
    end.get(ix).is_some_and(|node| std::ptr::eq(*node, quote))
        && first.is_some_and(|first| kids.first().is_some_and(|kid| std::ptr::eq(kid, first)))
        && last.is_some_and(|last| kids.last().is_some_and(|kid| std::ptr::eq(kid, last)))
}

/// One `>` off each line of the quote at `ix`.
fn unwrap(here: &Here, ix: usize) -> Result<Vec<(Range<usize>, String)>, EditRefusal> {
    let text = here.text;
    let quote = here.path[ix];
    let whole = span(quote);
    let outer = here.containers(ix);
    let blank = format!("{}{}", prefix(text, &outer).trim_end(), eol(text));
    let mut edits = Vec::new();
    let mut line = line_start(text, whole.start);
    if here.joins_before(&whole, &[quote]) {
        edits.push((line..line, blank.clone()));
    }
    if let Node::Blockquote(inner) = quote
        && alert(inner).is_some()
    {
        if !marker_only(&text[whole.start..line_end(text, line)]) {
            return Err(check::misread("an alert marker shares its line"));
        }
        // A body block keeps its indent; text sheds it.
        let continues = inner
            .children
            .first()
            .is_some_and(|head| span(head).end > line_end(text, line));
        // The marker goes with blank lines; an outer opener stays.
        let (mut cut, mut rest) = (whole.end, whole.end);
        let mut next = next_line(text, line);
        while let Some(at) = next.filter(|at| *at < whole.end) {
            let lead = match mark(text, at, &outer) {
                Some(mark) => mark.end,
                None => content(text, at, &outer),
            };
            let body =
                lead + text[lead..].len() - text[lead..].trim_start_matches([' ', '\t']).len();
            if body < line_end(text, at) {
                cut = if continues { body } else { lead };
                rest = next_line(text, at).unwrap_or(text.len());
                break;
            }
            next = next_line(text, at);
        }
        edits.push((whole.start..cut, String::new()));
        line = rest;
    }
    while line < whole.end {
        if let Some(mark) = mark(text, line, &outer) {
            edits.push((mark, String::new()));
        }
        match next_line(text, line) {
            Some(next) => line = next,
            None => break,
        }
    }
    if here.joins_after(&whole) {
        let next = next_line(text, whole.end).expect("a joining line follows");
        edits.push((next..next, blank.clone()));
    }
    Ok(edits)
}

/// Whether `line`, a quote's first, holds only an alert marker.
fn marker_only(line: &str) -> bool {
    let root = file_tree(&format!("{line}\n> x"));
    let Some(Node::Blockquote(quote)) = root.children().and_then(|kids| kids.first()) else {
        return false;
    };
    matches!(alert(quote), Some((_, body)) if matches!(&body[..],
        [Node::Paragraph(head)] if matches!(&head.children[..],
            [Node::Text(text)] if text.value == "x")))
}

/// `line`'s `>` and one space after, inside `outer`.
fn mark(text: &str, line: usize, outer: &[&Node]) -> Option<Range<usize>> {
    let at = inside(text, line, outer)?;
    let end = line_end(text, line);
    let lead = text[at..end].len() - text[at..end].trim_start_matches(' ').len();
    let mark = at + lead;
    (lead <= 3 && text[mark..end].starts_with('>')).then(|| {
        let space = usize::from(text[mark + 1..end].starts_with([' ', '\t']));
        mark..mark + 1 + space
    })
}

/// `> ` before each line of `blocks`, held by `path[k]`.
fn wrap(here: &Here, k: usize, blocks: &[&Node]) -> Vec<(Range<usize>, String)> {
    let text = here.text;
    let outer = here.containers(k + 1);
    let first = span(blocks[0]).start;
    let last = span(blocks[blocks.len() - 1]).end;
    let mut edits = Vec::new();
    let mut line = line_start(text, first);
    while line <= last {
        let at = match line == line_start(text, first) {
            true => Some(first),
            false => inside(text, line, &outer),
        };
        // A lazy line stays in its paragraph.
        if let Some(at) = at {
            let blank = text[at..line_end(text, line)].trim().is_empty();
            edits.push((at..at, if blank { ">" } else { "> " }.to_string()));
        }
        match next_line(text, line) {
            Some(next) => line = next,
            None => break,
        }
    }
    edits
}
