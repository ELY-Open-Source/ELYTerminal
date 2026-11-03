//! Clear Format: every inline mark off the touched blocks.

use std::ops::Range;

use markdown::mdast::Node;

use super::{Change, EditRefusal, after, block, cut, holder, marks, room};
use crate::documents::markdown::{math_inside, span};

/// The backslashes of escapes in a text node's span.
fn escapes(text: &str, whole: Range<usize>) -> Vec<Range<usize>> {
    let mut out = Vec::new();
    let mut chars = text[whole.clone()].char_indices().peekable();
    while let Some((at, ch)) = chars.next() {
        if ch == '\\'
            && chars
                .peek()
                .is_some_and(|(_, next)| next.is_ascii_punctuation())
        {
            out.push(whole.start + at..whole.start + at + 1);
            chars.next();
        }
    }
    out
}

/// A node's inline marks, as Typora's `.md-meta`.
fn meta(node: &Node, text: &str, autolinks: &[Range<usize>]) -> Vec<Range<usize>> {
    let whole = span(node);
    match node {
        Node::Html(_) => vec![whole],
        // Autolink text keeps its backslashes.
        Node::Text(_) if holder(autolinks, &whole).is_some() => Vec::new(),
        Node::Text(_) => escapes(text, whole),
        Node::InlineMath(_) => {
            let inner = math_inside(text, whole.clone());
            vec![whole.start..inner.start, inner.end..whole.end]
        }
        Node::FootnoteReference(_) => vec![whole.start..whole.start + 2, whole.end - 1..whole.end],
        _ => marks(node, text).map(Vec::from).unwrap_or_default(),
    }
}

/// Every inline mark in the blocks `selection` touches.
pub(super) fn clear(
    text: &str,
    nodes: &[&Node],
    selection: Range<usize>,
) -> Result<Change, EditRefusal> {
    let mut blocks: Vec<Range<usize>> = (nodes.iter())
        .filter(|node| block(node))
        .map(|node| room(node, text))
        .filter(|range| range.start <= selection.end && selection.start <= range.end)
        .collect();
    if blocks.is_empty() {
        return Err(EditRefusal::Outside);
    }
    blocks.sort_by_key(|block| block.start);
    let mut autolinks: Vec<Range<usize>> = (nodes.iter())
        .filter(|node| matches!(node, Node::Link(_)))
        .map(|node| span(node))
        .filter(|whole| !text[whole.clone()].starts_with('['))
        .collect();
    autolinks.sort_by_key(|link| link.start);
    let mut inside: Vec<Vec<Range<usize>>> = vec![Vec::new(); blocks.len()];
    for hole in (nodes.iter()).flat_map(|node| meta(node, text, &autolinks)) {
        if let Some(at) = holder(&blocks, &hole).filter(|_| !hole.is_empty()) {
            inside[at].push(hole);
        }
    }
    let mut holes = Vec::new();
    let mut edits = Vec::new();
    for (block, mut cuts) in blocks.into_iter().zip(inside) {
        if cuts.is_empty() {
            continue;
        }
        cuts.sort_by_key(|hole| hole.start);
        edits.push((block.clone(), cut(text, block, &cuts)));
        holes.extend(cuts);
    }
    Ok((
        edits,
        after(selection.start, &holes)..after(selection.end, &holes),
    ))
}
