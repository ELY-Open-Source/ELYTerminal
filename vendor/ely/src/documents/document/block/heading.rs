//! Heading levels and paragraphs, as Typora's `changeBlock`.

use std::ops::Range;

use markdown::mdast::Node;

use super::{
    BlockCommand, Here, change, check, closing, eol, insert, inside, line_end, line_start,
    next_line, prefix, prev_line, refusal, unblocked,
};
use crate::documents::{
    document::format::{Change, EditRefusal},
    markdown::span,
};

/// The level `command` asks at `level`; 0 is a paragraph.
fn target(level: u8, command: BlockCommand) -> u8 {
    match command {
        BlockCommand::Heading(n) => {
            assert!((1..=6).contains(&n), "a heading level is 1 to 6");
            n
        }
        BlockCommand::Promote if level == 0 => 6,
        BlockCommand::Promote => (level - 1).max(1),
        BlockCommand::Demote if level == 0 || level == 6 => 0,
        BlockCommand::Demote => level + 1,
        _ => 0,
    }
}

pub(super) fn run(here: &Here, command: BlockCommand) -> Result<Change, EditRefusal> {
    let text = here.text;
    let caret = here.selection.clone();
    let leaf = here.leaf();
    let level = match leaf {
        Some(Node::Paragraph(_)) => 0,
        Some(Node::Heading(heading)) => heading.depth,
        Some(node @ (Node::Math(_) | Node::Html(_) | Node::Table(_)))
            if !matches!(command, BlockCommand::Promote | BlockCommand::Demote) =>
        {
            // Typora gives these an empty paragraph after.
            log::info!("block command: {:?} takes a paragraph after", refusal(node));
            return insert::paragraph(here, false);
        }
        Some(node) => return Err(refusal(node)),
        None if here.blank() => 0,
        None => return Err(EditRefusal::Outside),
    };
    let goal = target(level, command);
    if goal == level {
        return Ok((Vec::new(), caret));
    }
    let containers = here.containers(here.path.len());
    let hashes = "#".repeat(usize::from(goal)) + " ";
    let Some(node) = leaf else {
        let line = line_start(text, caret.start);
        let at = inside(text, line, &containers).expect("a blank line holds its prefix");
        return Ok(change(vec![(at..at, hashes)], caret));
    };
    let whole = span(node);
    let edits = match (level, goal) {
        (0, _) if task_head(here, &whole) => return Err(EditRefusal::Outside),
        (0, _) => to_heading(here, &containers, &whole, hashes),
        (_, 0) => to_paragraph(here, node, &containers)?,
        _ => relevel(here, node, &containers, goal),
    };
    let change = change(edits, caret);
    // Every shown character stays; the caret's block is `goal`.
    let (written, after) = check::reread(text, &change.0);
    let before = check::Shown::new(text, here.root);
    let now = check::Shown::new(&written, &after);
    let placed = match now.at(change.1.start) {
        Some((level, _)) => level == goal,
        // An empty paragraph has no node.
        None => goal == 0 && before.inline(node).is_empty(),
    };
    let kept = placed && now.all() == before.all();
    match kept {
        true => Ok(change),
        false => Err(check::misread("a heading change would reread otherwise")),
    }
}

/// Whether `node` is an ATX heading, `#` led.
fn atx(text: &str, node: &Node) -> bool {
    text[span(node)].trim_start().starts_with('#')
}

/// Whether the caret is on a task item's first line.
fn task_head(here: &Here, whole: &Range<usize>) -> bool {
    let first = line_start(here.text, whole.start) == line_start(here.text, here.selection.start);
    let leaf = here.path[here.path.len() - 1];
    first && super::task_head(here.path[here.path.len() - 2], leaf)
}

/// The caret's line led by `hashes`; lazy lines kept in.
fn to_heading(
    here: &Here,
    containers: &[&Node],
    whole: &Range<usize>,
    hashes: String,
) -> Vec<(Range<usize>, String)> {
    let text = here.text;
    let lead = prefix(text, containers);
    let first = line_start(text, here.selection.start);
    let mut edits = Vec::new();
    let mut line = first;
    loop {
        let at = match line_start(text, whole.start) == line {
            true => Some(whole.start),
            false => inside(text, line, containers),
        };
        let content = |from: usize| {
            from + text[from..].len() - text[from..].trim_start_matches([' ', '\t']).len()
        };
        let (at, lazy) = match at {
            Some(at) => (content(at), ""),
            None => (content(line), lead.as_str()),
        };
        if line == first {
            edits.push((at..at, format!("{lazy}{hashes}")));
            if let Some(run) = closing(&text[at..line_end(text, at)]) {
                edits.push((at + run..at + run, "\\".to_string()));
            }
        } else if !lazy.is_empty() {
            edits.push((at..at, lazy.to_string()));
        }
        match next_line(text, line) {
            Some(next) if next < whole.end => line = next,
            _ => break,
        }
    }
    edits
}

/// A heading's marks off; a blank line where text joins.
fn to_paragraph(
    here: &Here,
    node: &Node,
    containers: &[&Node],
) -> Result<Vec<(Range<usize>, String)>, EditRefusal> {
    let text = here.text;
    let whole = span(node);
    let blank = format!("{}{}", prefix(text, containers).trim_end(), eol(text));
    let mut edits = Vec::new();
    if here.joins_before(&whole, &[]) {
        let line = line_start(text, whole.start);
        edits.push((line..line, blank.clone()));
    }
    let kids = node.children().map(Vec::as_slice).unwrap_or_default();
    match atx(text, node) {
        true => match (kids.first(), kids.last()) {
            (Some(first), Some(last)) => {
                let start = span(first).start;
                edits.push((whole.start..start, String::new()));
                // The body kept: closing hashes go.
                let end = line_end(text, start).min(span(last).end);
                if let Some(mark) = unblocked(&text[start..end])? {
                    edits.push((start + mark..start + mark, "\\".to_string()));
                }
                edits.push((span(last).end..whole.end, String::new()));
            }
            _ => edits.push((whole.clone(), String::new())),
        },
        false => {
            // The underline line goes with the break before it.
            let under = line_start(text, whole.end);
            let ending = text[..under].len() - text[..under].trim_end_matches(['\r', '\n']).len();
            edits.push((under - ending.min(2)..whole.end, String::new()));
        }
    }
    if here.joins_after(&whole) {
        let next = next_line(text, whole.end).expect("a joining line follows");
        edits.push((next..next, blank.clone()));
    }
    Ok(edits)
}

/// A heading moved to level `goal`.
fn relevel(
    here: &Here,
    node: &Node,
    containers: &[&Node],
    goal: u8,
) -> Vec<(Range<usize>, String)> {
    let text = here.text;
    let whole = span(node);
    if atx(text, node) {
        let at = whole.start + text[whole.clone()].find('#').expect("an ATX heading has #");
        let run = text[at..].len() - text[at..].trim_start_matches('#').len();
        return vec![(at..at + run, "#".repeat(usize::from(goal)))];
    }
    let under = line_start(text, whole.end);
    let marks = &text[under..whole.end];
    let at = under + marks.find(['=', '-']).expect("a setext underline");
    let run = marks.trim_end().len() - (at - under);
    if goal <= 2 {
        let mark = if goal == 1 { "=" } else { "-" };
        return vec![(at..at + run, mark.repeat(run))];
    }
    // Levels past 2 are ATX: one line, the underline gone.
    let mut edits = vec![(
        whole.start..whole.start,
        "#".repeat(usize::from(goal)) + " ",
    )];
    let mut line = line_start(text, whole.start);
    while let Some(next) = next_line(text, line).filter(|next| *next < under) {
        let end = line_end(text, line);
        let from = inside(text, next, containers).unwrap_or(next);
        let from = from + text[from..].len() - text[from..].trim_start_matches([' ', '\t']).len();
        edits.push((end..from, " ".to_string()));
        line = next;
    }
    let last = prev_line(text, under).expect("a setext heading has text");
    if let Some(run) = closing(&text[last..line_end(text, last)]) {
        edits.push((last + run..last + run, "\\".to_string()));
    }
    let ending = text[..under].len() - text[..under].trim_end_matches(['\r', '\n']).len();
    edits.push((under - ending.min(2)..whole.end, String::new()));
    edits
}
