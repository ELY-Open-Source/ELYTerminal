//! Lists on, off and retyped, as Typora's `toggleIndent`.

use std::ops::Range;

use markdown::mdast::{Code, List, Node};

use super::{
    BlockCommand, Here, change, column, eol, indent, inside, line_end, line_start, next_line,
    opener, prefix, refusal, unblocked,
};
use crate::documents::{
    document::format::{Change, EditRefusal},
    markdown::span,
};

fn kind(list: &List) -> BlockCommand {
    match (list.ordered, list.children.first()) {
        (true, _) => BlockCommand::OrderedList,
        (false, Some(Node::ListItem(item))) if item.checked.is_some() => BlockCommand::TaskList,
        _ => BlockCommand::BulletList,
    }
}

/// The marker an item takes: number `n` or the bullet.
fn marker(kind: BlockCommand, n: u32, bullet: char) -> String {
    match kind {
        BlockCommand::OrderedList => format!("{n}. "),
        BlockCommand::TaskList => format!("{bullet} [ ] "),
        _ => format!("{bullet} "),
    }
}

/// A marker's width for continuation lines; boxes add none.
fn width(kind: BlockCommand, n: u32) -> usize {
    match kind {
        BlockCommand::OrderedList => format!("{n}. ").len(),
        _ => 2,
    }
}

pub(super) fn run(here: &Here, command: BlockCommand) -> Result<Change, EditRefusal> {
    let caret = here.selection.clone();
    let path = &here.path;
    let item = match here.leaf() {
        Some(leaf) => path.len().checked_sub(2).filter(|ix| match path[*ix] {
            Node::ListItem(item) => {
                !caret.is_empty()
                    || item
                        .children
                        .first()
                        .is_some_and(|kid| std::ptr::eq(kid, leaf))
            }
            _ => false,
        }),
        None => Some(path.len() - 1).filter(|ix| matches!(path[*ix], Node::ListItem(_))),
    };
    // A selection past the list takes every touched sibling.
    if let Some(ix) = item
        && let Node::List(list) = path[ix - 1]
        && (here.path_at(caret.end).iter()).any(|node| std::ptr::eq(*node, path[ix - 1]))
    {
        let edits = match kind(list) == command {
            true => unwrap(here, ix - 1, list)?,
            false => {
                let start = if list.ordered {
                    list.start.unwrap_or(1)
                } else {
                    1
                };
                let outer = here.containers(ix - 1);
                retype(here.text, path[ix - 1], list, &outer, command, start)?
            }
        };
        return Ok(change(edits, caret));
    }
    if here.leaf().is_none() && caret.is_empty() {
        if !here.blank() {
            return Err(EditRefusal::Outside);
        }
        let line = line_start(here.text, caret.start);
        let at = inside(here.text, line, &here.containers(path.len()))
            .expect("a blank line holds its prefix");
        return Ok(change(vec![(at..at, marker(command, 1, '-'))], caret));
    }
    let (k, blocks) = here.blocks().ok_or(EditRefusal::Outside)?;
    Ok(change(wrap(here, k, &blocks, command)?, caret))
}

/// Indent from `at` to column `want`: its end, columns past.
fn indented(text: &str, at: usize, want: usize) -> Option<(usize, usize)> {
    let end = line_end(text, at);
    let mut cut = at;
    let mut col = column(text, at);
    while col < want {
        let ch = text[cut..end]
            .chars()
            .next()
            .filter(|ch| matches!(ch, ' ' | '\t'))?;
        col = if ch == '\t' { col / 4 * 4 + 4 } else { col + 1 };
        cut += 1;
    }
    Some((cut, col - want))
}

/// Each non-lazy line of `item` after its first, with indent.
fn continued<'a>(
    text: &'a str,
    item: &Node,
    outer: &'a [&Node],
) -> impl Iterator<Item = (usize, usize, usize)> + 'a {
    let whole = span(item);
    let want = indent(text, item);
    std::iter::successors(next_line(text, whole.start), move |line| {
        next_line(text, *line)
    })
    .take_while(move |line| *line < whole.end)
    .filter_map(move |line| {
        let at = inside(text, line, outer)?;
        if text[at..line_end(text, line)].trim().is_empty() {
            return None;
        }
        let (cut, past) = indented(text, at, want)?;
        Some((at, cut, past))
    })
}

/// Every item's marker off, its lines out one level.
fn unwrap(here: &Here, ix: usize, list: &List) -> Result<Vec<(Range<usize>, String)>, EditRefusal> {
    let text = here.text;
    let outer = here.containers(ix);
    let blank = format!("{}{}", prefix(text, &outer).trim_end(), eol(text));
    let whole = span(here.path[ix]);
    let mut edits = Vec::new();
    for (n, item) in list.children.iter().enumerate() {
        let start = span(item).start;
        let line = line_start(text, start);
        let joins = match n {
            0 => here.joins_before(&whole, &[here.path[ix]]),
            _ => prev_text(text, line, &outer),
        };
        if joins {
            edits.push((line..line, blank.clone()));
        }
        let first = item.children().and_then(|kids| kids.first()).map(span);
        let boxed = matches!(item, Node::ListItem(item) if item.checked.is_some());
        match &first {
            Some(kid) if !text[start..kid.start].contains(['\n', '\r']) => {
                edits.push((start..kid.start, String::new()));
            }
            Some(kid) => {
                let next = next_line(text, start).expect("an item's text lies below");
                edits.push((line..next, String::new()));
                if boxed {
                    edits.push((unbox(text, kid.start), String::new()));
                }
            }
            None => edits.push((start..line_end(text, start), String::new())),
        }
        if boxed && let Some(kid) = first {
            edits.extend(escaped(text, kid.start)?);
        }
        for (at, cut, past) in continued(text, item, &outer) {
            edits.push((at..cut, " ".repeat(past)));
        }
    }
    if here.joins_after(&whole) {
        let next = next_line(text, whole.end).expect("a joining line follows");
        edits.push((next..next, blank.clone()));
    }
    Ok(edits)
}

/// The task box before the text at `kid`.
fn unbox(text: &str, kid: usize) -> Range<usize> {
    let line = line_start(text, kid);
    line + text[line..kid].rfind('[').expect("a task's box")..kid
}

/// A backslash where text at `at`, unboxed, opens a block.
fn escaped(text: &str, at: usize) -> Result<Option<(Range<usize>, String)>, EditRefusal> {
    let mark = unblocked(&text[at..line_end(text, at)])?;
    Ok(mark.map(|mark| (at + mark..at + mark, "\\".to_string())))
}

/// Whether `node`, holding `code`, is fenced in the source.
fn fenced(text: &str, node: &Node, code: &Code) -> bool {
    code.lang.is_some()
        || code.value.is_empty()
        || text[span(node)].lines().count() > code.value.lines().count()
}

/// Whether the line before `line` holds text inside `outer`.
fn prev_text(text: &str, line: usize, outer: &[&Node]) -> bool {
    super::prev_line(text, line).is_some_and(|prev| {
        inside(text, prev, outer).is_none_or(|at| !text[at..line_end(text, prev)].trim().is_empty())
    })
}

/// Each marker as `kind`'s from `start`; lines follow.
fn retype(
    text: &str,
    node: &Node,
    list: &List,
    outer: &[&Node],
    kind: BlockCommand,
    start: u32,
) -> Result<Vec<(Range<usize>, String)>, EditRefusal> {
    let bullet = match list.ordered {
        true => '-',
        false => text[span(node).start..].chars().next().unwrap_or('-'),
    };
    let mut edits = Vec::new();
    let task = kind == BlockCommand::TaskList;
    for (n, item) in list.children.iter().enumerate() {
        let number = start + n as u32;
        let at = span(item).start;
        let (mark, spaces) = super::marker(text, item);
        let first = item.children().and_then(|kids| kids.first());
        let boxed = matches!(item, Node::ListItem(item) if item.checked.is_some());
        let body = at + mark + spaces;
        let blank = body == line_end(text, at);
        if boxed {
            // A box stays when the kind keeps boxes.
            let kind = if task { BlockCommand::BulletList } else { kind };
            let end = if blank { opener(text, item) } else { body };
            edits.push((at..end, marker(kind, number, bullet)));
            if !task {
                let kid = span(first.expect("a boxed item has text")).start;
                edits.push((unbox(text, kid), String::new()));
                edits.extend(escaped(text, kid)?);
            }
        } else if task && let Some(kid) = first.filter(|kid| !matches!(kid, Node::Paragraph(_))) {
            // GFM boxes only a paragraph.
            return Err(refusal(kid));
        } else if blank {
            edits.push((at..opener(text, item), marker(kind, number, bullet)));
        } else {
            // Indented code is written again from its value.
            let pad = match first {
                Some(kid @ Node::Code(code)) if !fenced(text, kid, code) => {
                    let line = code.value.lines().next().unwrap_or_default();
                    4 + line.len() - line.trim_start_matches(' ').len()
                }
                _ => 0,
            };
            edits.push((at..body, marker(kind, number, bullet) + &" ".repeat(pad)));
        }
        let want = column(text, at) + width(kind, number);
        for (from, cut, past) in continued(text, item, outer) {
            let pad = want.saturating_sub(column(text, from)) + past;
            edits.push((from..cut, " ".repeat(pad)));
        }
    }
    Ok(edits)
}

/// Each paragraph line, and each other block, made an item.
fn wrap(
    here: &Here,
    k: usize,
    blocks: &[&Node],
    command: BlockCommand,
) -> Result<Vec<(Range<usize>, String)>, EditRefusal> {
    let text = here.text;
    let outer = here.containers(k + 1);
    let lead = prefix(text, &outer);
    let mut edits = Vec::new();
    let mut n = 1;
    for block in blocks {
        if let Node::List(list) = block {
            if kind(list) != command {
                edits.extend(retype(text, block, list, &outer, command, n)?);
            }
            n += list.children.len() as u32;
            continue;
        }
        if command == BlockCommand::TaskList && !matches!(block, Node::Paragraph(_)) {
            return Err(refusal(block));
        }
        let whole = span(block);
        let mut line = line_start(text, whole.start);
        while line <= whole.end {
            let first = line == line_start(text, whole.start);
            let (at, lazy) = match first {
                true => (Some(whole.start), ""),
                false => match inside(text, line, &outer) {
                    Some(at) => (Some(at), ""),
                    None => (None, lead.as_str()),
                },
            };
            let at = at.unwrap_or(line);
            let content = at + text[at..].len() - text[at..].trim_start_matches([' ', '\t']).len();
            if !text[at..line_end(text, line)].trim().is_empty() {
                let mark = match first || matches!(block, Node::Paragraph(_)) {
                    true => {
                        n += 1;
                        marker(command, n - 1, '-')
                    }
                    false => " ".repeat(width(command, n - 1)),
                };
                let at = if matches!(block, Node::Paragraph(_)) {
                    content
                } else {
                    at
                };
                edits.push((at..at, format!("{lazy}{mark}")));
            }
            match next_line(text, line) {
                Some(next) => line = next,
                None => break,
            }
        }
    }
    Ok(edits)
}
