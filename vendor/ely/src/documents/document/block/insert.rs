//! Blocks put in and taken out, as Typora's `insertBlock`.

use std::collections::HashSet;

use markdown::mdast::Node;

use super::{
    BlockCommand, Here, container, eol, inside, line_end, line_start, next_line, placed, prefix,
    prev_line, refusal,
    split::{Room, content, split},
    task_head,
};
use crate::documents::{
    document::format::{Change, EditRefusal},
    markdown::span,
};

/// Whether `line` holds text past `containers`' prefix.
fn text_on(text: &str, line: usize, containers: &[&Node]) -> bool {
    let at = content(text, line, containers);
    at < line_end(text, line)
}

/// The selection's text, line by line, prefixes cut.
fn picked(here: &Here, containers: &[&Node]) -> Vec<String> {
    let text = here.text;
    let (s, e) = (here.selection.start, here.selection.end);
    let mut out = Vec::new();
    let mut at = s;
    while at < e {
        out.push(text[at..line_end(text, at).min(e)].to_string());
        match next_line(text, at).filter(|next| *next < e) {
            Some(next) => at = inside(text, next, containers).unwrap_or(next).min(e),
            None => break,
        }
    }
    out
}

/// The smallest footnote number not taken.
fn free_note(here: &Here) -> usize {
    let used: HashSet<&str> = (here.nodes.iter())
        .filter_map(|node| match node {
            Node::FootnoteDefinition(note) => Some(note.identifier.as_str()),
            Node::FootnoteReference(note) => Some(note.identifier.as_str()),
            _ => None,
        })
        .collect();
    (1..)
        .find(|n: &usize| !used.contains(n.to_string().as_str()))
        .expect("a free number")
}

/// `command`'s lines and the caret's line and byte.
fn lines(
    here: &Here,
    command: BlockCommand,
    body: Vec<String>,
) -> (Vec<String>, Option<(usize, usize)>) {
    let fenced = |fence: String, body: Vec<String>| {
        let mut lines = vec![fence.clone()];
        lines.extend(if body.is_empty() {
            vec![String::new()]
        } else {
            body
        });
        lines.push(fence);
        (lines, Some((1, 0)))
    };
    match command {
        BlockCommand::CodeBlock => {
            let longest = (body.iter())
                .flat_map(|line| line.split(|ch| ch != '`'))
                .map(str::len)
                .max()
                .unwrap_or(0);
            fenced("`".repeat(longest.max(2) + 1), body)
        }
        BlockCommand::MathBlock => fenced("$$".to_string(), body),
        BlockCommand::Toc => (vec!["[toc]".to_string()], Some((0, 5))),
        BlockCommand::Footnote => {
            let line = format!("[^{}]: ", free_note(here));
            let end = line.len();
            (vec![line], Some((0, end)))
        }
        BlockCommand::LinkDefinition => {
            let first = body.first().map_or("", |line| line.trim());
            if first.contains(['/', '\\']) {
                let address: String = first.chars().take(160).collect();
                return (vec![format!("[]: {address}")], Some((0, 1)));
            }
            let label: String = (first.split_whitespace().next().unwrap_or_default())
                .chars()
                .take(20)
                .collect();
            let line = format!("[{label}]: ");
            let caret = if label.is_empty() { 1 } else { line.len() };
            (vec![line], Some((0, caret)))
        }
        _ => (vec!["---".to_string()], None),
    }
}

/// `lines` joined inside `lead`, and the caret's byte in them.
fn joined(
    lines: &[String],
    caret: Option<(usize, usize)>,
    nl: &str,
    lead: &str,
) -> (String, Option<usize>) {
    let mut out = String::new();
    let mut at = None;
    for (ix, line) in lines.iter().enumerate() {
        if ix > 0 {
            out.push_str(nl);
            out.push_str(lead);
        }
        if let Some((row, byte)) = caret
            && row == ix
        {
            at = Some(out.len() + byte);
        }
        out.push_str(line);
    }
    (out, at)
}

/// Whether a paragraph or ATX heading splits at the caret.
fn splits(text: &str, node: &Node) -> bool {
    match node {
        Node::Paragraph(_) => true,
        Node::Heading(_) => text[span(node)].trim_start().starts_with('#'),
        _ => false,
    }
}

pub(super) fn run(here: &Here, command: BlockCommand) -> Result<Change, EditRefusal> {
    let text = here.text;
    let leaf = here.leaf();
    let containers = here.containers(here.path.len() - usize::from(leaf.is_some()));
    let lead = prefix(text, &containers);
    let room = Room {
        nl: eol(text),
        blank: lead.trim_end().to_string(),
        lead,
        containers,
    };
    let (lines, caret) = lines(here, command, picked(here, &room.containers));
    let (block, inner) = joined(&lines, caret, room.nl, &room.lead);
    let rule = inner.is_none();
    match leaf {
        Some(node) if splits(text, node) => {
            if here.selection.end > span(node).end {
                return Err(EditRefusal::Across);
            }
            split(here, node, &room, &block, inner)
        }
        Some(node) if !here.selection.is_empty() => Err(refusal(node)),
        Some(node) => {
            let at = line_end(text, span(node).end);
            let next =
                next_line(text, at).is_some_and(|next| text_on(text, next, &room.containers));
            let gap = room.gap();
            let tail = match rule {
                true => gap.clone(),
                false => String::new(),
            };
            let sep = if next {
                format!("{}{}", room.nl, room.blank)
            } else {
                String::new()
            };
            let caret = at + gap.len() + inner.unwrap_or(block.len() + tail.len());
            Ok(placed(
                vec![(at..at, format!("{gap}{block}{tail}{sep}"))],
                caret,
            ))
        }
        None if here.blank() && here.selection.is_empty() => {
            let line = line_start(text, here.selection.start);
            let at = inside(text, line, &room.containers).expect("a blank line holds its prefix");
            let end = line_end(text, line);
            let prev =
                prev_line(text, line).is_some_and(|prev| text_on(text, prev, &room.containers));
            let next =
                next_line(text, line).is_some_and(|next| text_on(text, next, &room.containers));
            let head = if prev {
                format!("{}{}", room.nl, room.lead)
            } else {
                String::new()
            };
            let tail = if rule { room.gap() } else { String::new() };
            let sep = if next {
                format!("{}{}", room.nl, room.blank)
            } else {
                String::new()
            };
            let caret = at + head.len() + inner.unwrap_or(block.len() + tail.len());
            Ok(placed(
                vec![(at..end, format!("{head}{block}{tail}{sep}"))],
                caret,
            ))
        }
        None => Err(EditRefusal::Outside),
    }
}

/// An empty line before or after the caret's block.
pub(super) fn paragraph(here: &Here, before: bool) -> Result<Change, EditRefusal> {
    let text = here.text;
    let leaf = here.leaf().ok_or(EditRefusal::Outside)?;
    let whole = span(leaf);
    let containers = here.containers(here.path.len() - 1);
    let lead = prefix(text, &containers);
    let room = Room {
        nl: eol(text),
        blank: lead.trim_end().to_string(),
        lead,
        containers,
    };
    let gap = room.gap();
    let holder = here.path[here.path.len() - 2];
    if before && task_head(holder, leaf) {
        return Err(EditRefusal::Outside);
    }
    // A holder's first line keeps its opener: text moves down.
    if before
        && container(holder)
        && !matches!(holder, Node::Blockquote(_))
        && line_start(text, span(holder).start) == line_start(text, whole.start)
    {
        let down = format!("{}{}", room.nl, room.lead);
        return Ok(placed(vec![(whole.start..whole.start, down)], whole.start));
    }
    if before {
        let mut edits = Vec::new();
        let mut shift = 0;
        if here.joins_before(&whole, &[]) {
            let line = line_start(text, whole.start);
            let blank = format!("{}{}", room.blank, room.nl);
            shift = blank.len();
            edits.push((line..line, blank));
        }
        edits.push((whole.start..whole.start, gap));
        return Ok(placed(edits, whole.start + shift));
    }
    let at = line_end(text, whole.end);
    let next = next_line(text, at).is_some_and(|next| text_on(text, next, &room.containers));
    let sep = if next {
        format!("{}{}", room.nl, room.blank)
    } else {
        String::new()
    };
    Ok(placed(
        vec![(at..at, format!("{gap}{sep}"))],
        at + gap.len(),
    ))
}

/// Front matter at the top, unless there is some.
pub(super) fn front_matter(here: &Here) -> Result<Change, EditRefusal> {
    let first = here.root.children().and_then(|kids| kids.first());
    if matches!(first, Some(Node::Yaml(_) | Node::Toml(_))) {
        return Err(EditRefusal::Present);
    }
    let nl = eol(here.text);
    let tail = match here.text.is_empty() || here.text.starts_with(['\n', '\r']) {
        true => "",
        false => nl,
    };
    Ok(placed(
        vec![(0..0, format!("---{nl}{nl}---{nl}{tail}"))],
        3 + nl.len(),
    ))
}

/// The caret's block gone; a lone paragraph takes its holder.
pub(super) fn delete(here: &Here) -> Result<Change, EditRefusal> {
    let text = here.text;
    let leaf = here.leaf().ok_or(EditRefusal::Outside)?;
    let mut ix = here.path.len() - 1;
    if matches!(leaf, Node::Paragraph(_))
        && let Some(parent) = here.path.get(ix - 1)
        && container(parent)
        && parent.children().is_some_and(|kids| kids.len() == 1)
    {
        ix -= 1;
    }
    let whole = span(here.path[ix]);
    let line = line_start(text, whole.start);
    if !text[line..whole.start]
        .chars()
        .all(|ch| matches!(ch, ' ' | '\t' | '>'))
    {
        return Ok(placed(vec![(whole.clone(), String::new())], whole.start));
    }
    // Containers wholly on the lines go too.
    let last = line_end(text, whole.end);
    let top = (1..=ix)
        .find(|j| {
            let held = span(here.path[*j]);
            line <= held.start && held.end <= last
        })
        .expect("the block lies on its lines");
    let gone = &here.path[top..=ix];
    let outer = here.containers(top);
    let mut end = next_line(text, whole.end).unwrap_or(text.len());
    let blank_above = prev_line(text, line).is_none_or(|prev| !text_on(text, prev, &outer));
    if blank_above && end < text.len() && !text_on(text, end, &outer) {
        end = next_line(text, end).unwrap_or(text.len());
    }
    // Neighbors that would join keep a blank line between.
    let joins = here.joins_before(&whole, gone) && here.joins_after(&whole);
    let with = match joins {
        true => format!("{}{}", prefix(text, &outer).trim_end(), eol(text)),
        false => String::new(),
    };
    Ok(placed(vec![(line..end, with)], line))
}
