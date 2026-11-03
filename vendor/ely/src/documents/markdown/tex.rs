//! `\(…\)` and `\[…\]` read as math between dollars.

use std::{borrow::Cow, ops::Range};

use markdown::{ParseOptions, mdast, unist::Position};

use super::{span, walk};
use crate::documents::tree::{Style, html};

/// Bytes a literal dollar may stand in as while parsing.
const STAND_INS: std::ops::RangeInclusive<u8> = 0x01..=0x08;

/// One pair's delimiters as written.
pub(super) struct Pair {
    open: Range<usize>,
    close: Range<usize>,
}

/// The swapped text and what to put back after parsing.
pub(super) struct Swapped<'a> {
    pub text: Cow<'a, str>,
    pairs: Vec<Pair>,
    /// Spaces that only part two dollars.
    gaps: Vec<usize>,
    /// The byte every other dollar was written as.
    stand_in: u8,
}

/// `text` with each pair's delimiters as dollars, byte for byte.
pub(super) fn swapped<'a>(text: &'a str, options: &ParseOptions) -> Swapped<'a> {
    let stand_in = STAND_INS
        .clone()
        .find(|byte| !text.as_bytes().contains(byte));
    let none = || Swapped {
        text: Cow::Borrowed(text),
        pairs: Vec::new(),
        gaps: Vec::new(),
        stand_in: 0,
    };
    if !text.contains("\\(") && !text.contains("\\[") {
        return none();
    }
    let Some(stand_in) = stand_in else {
        log::warn!("markdown: \\( reads as text; every stand-in byte is used");
        return none();
    };
    let root = markdown::to_mdast(text, options).expect("markdown without MDX always parses");
    let mut nodes = Vec::new();
    walk(&root, &mut nodes);
    let mut kept = vec![false; text.len()];
    let (mut pairs, mut dollars) = (Vec::new(), Vec::new());
    for node in nodes {
        use mdast::Node::{Heading, Paragraph, TableCell};
        if matches!(node, Paragraph(_) | Heading(_) | TableCell(_)) {
            keep(node, text, &mut kept);
            paired(text, span(node), &kept, &mut pairs, &mut dollars);
        }
    }
    if pairs.is_empty() {
        return none();
    }
    let (out, gaps) = written(text, &pairs, &dollars, stand_in);
    Swapped {
        text: Cow::Owned(String::from_utf8(out).expect("ASCII swaps keep UTF-8")),
        pairs,
        gaps,
        stand_in,
    }
}

/// Marks what in `container` is not prose: code, tags, addresses.
fn keep(container: &mdast::Node, text: &str, kept: &mut [bool]) {
    let mut nodes = Vec::new();
    walk(container, &mut nodes);
    // Open styled tags, and where code text last started.
    let (mut open, mut from) = (Vec::new(), 0);
    for node in &nodes[1..] {
        use mdast::Node::*;
        let whole = span(node);
        match node {
            InlineCode(_) | InlineMath(_) | Image(_) | ImageReference(_) | FootnoteReference(_) => {
                kept[whole].fill(true)
            }
            Link(_) | LinkReference(_) => {
                kept[whole.clone()].fill(true);
                // Bracketed text is prose; its address is not.
                let last = node.children().and_then(|children| children.last());
                if text[whole.clone()].starts_with('[')
                    && let Some(last) = last
                {
                    // A text's span skips a leading escape: start at `[`.
                    kept[whole.start + 1..span(last).end].fill(false);
                }
            }
            Html(_) => {
                kept[whole.clone()].fill(true);
                let Some(tag) = html::tag(&text[whole.clone()]) else {
                    continue;
                };
                if html::inline_style(&tag.name).is_none() {
                    continue;
                }
                let was = coded(&open);
                match tag.closing {
                    true => end(&mut open, &tag.name),
                    false => {
                        if tag.name == "a" {
                            end(&mut open, "a");
                        }
                        open.push(tag.name);
                    }
                }
                match (was, coded(&open)) {
                    (false, true) => from = whole.end,
                    (true, false) => kept[from..whole.start].fill(true),
                    _ => {}
                }
            }
            _ => {}
        }
    }
    // An unclosed `<code>` runs to its block's end.
    if coded(&open) {
        kept[from..span(container).end].fill(true);
    }
}

/// Whether an open tag shows its text as code.
fn coded(open: &[String]) -> bool {
    (open.iter()).any(|name| html::inline_style(name) == Some(Style::Code))
}

/// Ends the innermost `name` as the reader does: inner tags close.
fn end(open: &mut Vec<String>, name: &str) {
    let Some(at) = open.iter().rposition(|opened| opened == name) else {
        return;
    };
    let closed = open.split_off(at);
    open.extend(
        closed
            .into_iter()
            .skip(1)
            .filter(|name| html::reopens(name)),
    );
}

/// Pairs and bare dollars in `within`; pairs cross nothing kept.
fn paired(
    text: &str,
    within: Range<usize>,
    kept: &[bool],
    pairs: &mut Vec<Pair>,
    dollars: &mut Vec<usize>,
) {
    let bytes = text.as_bytes();
    let mut open: Option<(usize, u8)> = None;
    let mut at = within.start;
    while at < within.end {
        if kept[at] {
            open = None;
        } else if bytes[at] == b'$' {
            dollars.push(at);
        } else if bytes[at] == b'\\' && at + 1 < within.end {
            match (bytes[at + 1], open) {
                (b'(', None) => open = Some((at, b')')),
                (b'[', None) => open = Some((at, b']')),
                (close, Some((start, wanted))) if close == wanted && !kept[at + 1] => {
                    if !bytes[start + 2..at].iter().all(u8::is_ascii_whitespace) {
                        pairs.push(Pair {
                            open: start..start + 2,
                            close: at..at + 2,
                        });
                    }
                    open = None;
                }
                _ => {}
            }
            // An escaped byte escapes nothing after it.
            at += 2;
            continue;
        }
        at += 1;
    }
}

/// The swapped bytes; a space parts dollars that would touch.
fn written(text: &str, pairs: &[Pair], dollars: &[usize], stand_in: u8) -> (Vec<u8>, Vec<usize>) {
    let mut out = text.as_bytes().to_vec();
    for at in dollars {
        out[*at] = stand_in;
    }
    let mut gaps = Vec::new();
    for (ix, pair) in pairs.iter().enumerate() {
        for byte in &mut out[pair.open.end..pair.close.start] {
            if *byte == b'$' {
                *byte = stand_in;
            }
        }
        let open = match pair.open.start.checked_sub(1).map(|before| out[before]) {
            Some(b'$') => {
                gaps.push(pair.open.start);
                *b" $"
            }
            _ => *b"$ ",
        };
        out[pair.open.clone()].copy_from_slice(&open);
        let after = pair.close.end;
        let opens_next = pairs
            .get(ix + 1)
            .is_some_and(|next| next.open.start == after);
        let close = match opens_next || out.get(after) == Some(&b'$') {
            true => {
                gaps.push(pair.close.start + 1);
                *b"$ "
            }
            false => *b" $",
        };
        out[pair.close.clone()].copy_from_slice(&close);
    }
    (out, gaps)
}

/// Puts TeX, spans and dollars back; drops parting spaces.
pub(super) fn mend(root: &mut mdast::Node, swapped: &Swapped) {
    if swapped.pairs.is_empty() {
        return;
    }
    if let mdast::Node::InlineMath(math) = root
        && let Some(position) = &mut math.position
        && let Some(pair) =
            (swapped.pairs.iter()).find(|pair| pair.open.contains(&position.start.offset))
    {
        math.value = written_tex(&math.value, pair, swapped, position);
        position.start.column -= position.start.offset - pair.open.start;
        position.start.offset = pair.open.start;
        position.end.column += pair.close.end - position.end.offset;
        position.end.offset = pair.close.end;
    }
    if let Some(children) = root.children_mut() {
        children.iter_mut().for_each(|child| trim(child, swapped));
        children.retain(|child| !matches!(child, mdast::Node::Text(text) if text.value.is_empty()));
        children.iter_mut().for_each(|child| mend(child, swapped));
    }
}

/// The TeX as written, from the value markdown-rs read.
fn written_tex(value: &str, pair: &Pair, swapped: &Swapped, at: &Position) -> String {
    let bytes = swapped.text.as_bytes();
    // A pad: the space a delimiter writes inside its dollar.
    let pads = (
        at.start.offset == pair.open.start,
        at.end.offset == pair.close.end,
    );
    let ends = (
        pads.0 || bytes[pair.open.end] == b' ',
        pads.1 || bytes[pair.close.start - 1] == b' ',
    );
    // With spaces at both ends, markdown-rs trimmed one each.
    let mut tex = match ends {
        (true, true) => format!(" {value} "),
        _ => value.to_string(),
    };
    if pads.1 {
        tex.pop();
    }
    if pads.0 {
        tex.remove(0);
    }
    tex.replace(char::from(swapped.stand_in), "$")
}

/// Restores a text's dollars; drops parting spaces.
fn trim(node: &mut mdast::Node, swapped: &Swapped) {
    let mdast::Node::Text(text) = node else {
        return;
    };
    text.value = text.value.replace(char::from(swapped.stand_in), "$");
    let Some(position) = &mut text.position else {
        return;
    };
    if swapped.gaps.contains(&position.start.offset) && text.value.starts_with(' ') {
        text.value.remove(0);
        position.start.offset += 1;
        position.start.column += 1;
    }
    if swapped.gaps.contains(&(position.end.offset - 1)) && text.value.ends_with(' ') {
        text.value.pop();
        position.end.offset -= 1;
        position.end.column -= 1;
    }
}

#[cfg(test)]
mod tests;
