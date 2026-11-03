//! Inline formats toggle at the selection, as Typora's (R4.6).

mod clear;

use std::ops::Range;

use markdown::mdast::Node;

use super::TableRefusal;

use crate::documents::{
    markdown::{file_tree, span, walk},
    tree::inside,
};

/// An inline format of Typora's Format menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InlineFormat {
    Strong,
    Emphasis,
    Code,
    Strike,
    Underline,
    Link,
    Image,
    /// Every inline mark in the touched blocks.
    Clear,
}

/// Why a format or block command did not happen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditRefusal {
    /// Preview takes no edits.
    Preview,
    Code,
    Math,
    Html,
    FrontMatter,
    /// No paragraph, heading or cell at the caret.
    Outside,
    /// A table holds the caret.
    Table,
    /// No picture holds the caret.
    NoImage,
    /// Front matter is there already.
    Present,
    /// The selection crosses a block or another mark.
    Across,
    /// Past the largest read: source alone.
    TooLong,
    /// Another edit is still read.
    Busy,
    /// The text changed before the edit was read.
    Changed,
    /// The result would read as other text or blocks.
    Misread,
    /// A table edit refused, and why.
    Cells(TableRefusal),
}

/// The edits and the selection after them.
pub(super) type Change = (Vec<(Range<usize>, String)>, Range<usize>);

/// Removes the picture at the caret, as Typora's Delete Image.
pub(super) fn drop_image(text: &str, at: usize) -> Result<Change, EditRefusal> {
    let root = file_tree(text);
    let mut nodes = Vec::new();
    walk(&root, &mut nodes);
    let picture = (nodes.iter())
        .filter(|node| matches!(node, Node::Image(_) | Node::ImageReference(_)))
        .map(|node| span(node))
        .find(|range| range.contains(&at))
        .ok_or(EditRefusal::NoImage)?;
    Ok((
        vec![(picture.clone(), String::new())],
        picture.start..picture.start,
    ))
}

/// Typora's word stops: punctuation and space.
fn stop(ch: char) -> bool {
    ch.is_whitespace() || "()[]^&_-+*#@!~`\\/{}'\":;<>,.?%".contains(ch)
}

/// The text a block holds: its children, past heading marks.
fn content(node: &Node) -> Range<usize> {
    let whole = span(node);
    match node.children().map(|kids| (kids.first(), kids.last())) {
        Some((Some(first), Some(last))) => span(first).start..span(last).end,
        _ => whole.end..whole.end,
    }
}

/// Room for text: inside a cell's pipes, trailing spaces too.
fn room(node: &Node, text: &str) -> Range<usize> {
    let inner = match node {
        Node::TableCell(_) => {
            let whole = span(node);
            let cell = &text[whole.clone()];
            whole.start + usize::from(cell.starts_with('|'))
                ..whole.end - usize::from(cell.len() > 1 && cell.ends_with('|'))
        }
        _ => content(node),
    };
    let rest = &text[inner.end..];
    let spaces = rest.len() - rest.trim_start_matches([' ', '\t']).len();
    inner.start..inner.end + spaces
}

/// Which of `sorted`, disjoint spans, holds `inner`.
pub(super) fn holder(sorted: &[Range<usize>], inner: &Range<usize>) -> Option<usize> {
    let at = sorted.partition_point(|span| span.start <= inner.start);
    (at > 0 && inner.end <= sorted[at - 1].end).then(|| at - 1)
}

fn block(node: &Node) -> bool {
    matches!(
        node,
        Node::Paragraph(_) | Node::Heading(_) | Node::TableCell(_)
    )
}

/// A styled node's two marks, kept text between.
pub(super) fn marks(node: &Node, text: &str) -> Option<[Range<usize>; 2]> {
    let whole = span(node);
    let inner = match node {
        Node::Strong(_) | Node::Emphasis(_) | Node::Delete(_) => content(node),
        Node::Link(_) | Node::LinkReference(_) => content(node),
        Node::InlineCode(_) => inside(text, whole.clone(), '`'),
        Node::Image(_) | Node::ImageReference(_) => {
            // Typora keeps the alt up to its first `]`.
            let end = whole.start + text[whole.clone()].find(']')?;
            whole.start + 2..end
        }
        _ => return None,
    };
    Some([whole.start..inner.start, inner.end..whole.end])
}

/// Whether `at` lies strictly inside one of `node`'s marks.
fn in_marks(node: &Node, text: &str, at: usize) -> bool {
    marks(node, text).is_some_and(|holes| holes.iter().any(|hole| hole.start < at && at < hole.end))
}

fn styled(format: InlineFormat, node: &Node) -> bool {
    match format {
        InlineFormat::Strong => matches!(node, Node::Strong(_)),
        InlineFormat::Emphasis => matches!(node, Node::Emphasis(_)),
        InlineFormat::Code => matches!(node, Node::InlineCode(_)),
        InlineFormat::Strike => matches!(node, Node::Delete(_)),
        InlineFormat::Link => matches!(node, Node::Link(_) | Node::LinkReference(_)),
        InlineFormat::Image => matches!(node, Node::Image(_) | Node::ImageReference(_)),
        InlineFormat::Underline | InlineFormat::Clear => false,
    }
}

fn merges(format: InlineFormat) -> bool {
    matches!(
        format,
        InlineFormat::Strong | InlineFormat::Emphasis | InlineFormat::Strike
    )
}

/// `text[range]` without `holes`, sorted and inside it.
fn cut(text: &str, range: Range<usize>, holes: &[Range<usize>]) -> String {
    let mut out = String::new();
    let mut at = range.start;
    for hole in holes {
        out.push_str(&text[at..hole.start]);
        at = hole.end;
    }
    out.push_str(&text[at..range.end]);
    out
}

/// Byte `at` once `holes` are cut; inside one, its start.
fn after(at: usize, holes: &[Range<usize>]) -> usize {
    let gone: usize = holes
        .iter()
        .map(|hole| at.clamp(hole.start, hole.end) - hole.start)
        .sum();
    at - gone
}

/// Refuses an end inside code, math, HTML or front matter.
fn verbatim(
    nodes: &[&Node],
    selection: &Range<usize>,
    format: InlineFormat,
) -> Result<(), EditRefusal> {
    let mut blocks: Vec<Range<usize>> = nodes
        .iter()
        .filter(|node| block(node))
        .map(|node| span(node))
        .collect();
    blocks.sort_by_key(|block| block.start);
    for node in nodes {
        let whole = span(node);
        let touches = |edge: bool| {
            [selection.start, selection.end]
                .iter()
                .any(|&at| match edge {
                    true => whole.start <= at && at <= whole.end,
                    false => whole.start < at && at < whole.end,
                })
        };
        // Clear takes a block's inline code and tags too.
        let held = format != InlineFormat::Clear && touches(false);
        let why = match node {
            Node::Code(_) if touches(true) => EditRefusal::Code,
            Node::Math(_) if touches(true) => EditRefusal::Math,
            Node::Yaml(_) | Node::Toml(_) if touches(true) => EditRefusal::FrontMatter,
            Node::InlineCode(_) if format != InlineFormat::Code && held => EditRefusal::Code,
            Node::InlineMath(_) if held => EditRefusal::Math,
            Node::Html(_) => match holder(&blocks, &whole).is_some() {
                true if held => EditRefusal::Html,
                false if touches(true) => EditRefusal::Html,
                _ => continue,
            },
            _ => continue,
        };
        return Err(why);
    }
    Ok(())
}

/// The nearest `<u>` and `</u>` pair around `s..e` in `within`.
fn underline(
    nodes: &[&Node],
    within: &Range<usize>,
    s: usize,
    e: usize,
) -> Option<[Range<usize>; 2]> {
    let mut tags: Vec<(&str, Range<usize>)> = (nodes.iter())
        .filter_map(|node| match node {
            Node::Html(html) => Some((html.value.trim(), span(node))),
            _ => None,
        })
        .filter(|(_, at)| within.start <= at.start && at.end <= within.end)
        .collect();
    tags.sort_by_key(|(_, at)| at.start);
    let mut opened = Vec::new();
    for (tag, at) in tags {
        if tag.eq_ignore_ascii_case("<u>") {
            opened.push(at);
        } else if tag.eq_ignore_ascii_case("</u>")
            && let Some(open) = opened.pop()
            && open.end <= s
            && e <= at.start
        {
            // Inner pairs close first.
            return Some([open, at]);
        }
    }
    None
}

/// `format` toggled at `selection` in `text`, or why not.
pub(super) fn toggle(
    text: &str,
    selection: Range<usize>,
    format: InlineFormat,
) -> Result<Change, EditRefusal> {
    toggle_tree(text, &file_tree(text), selection, format)
}

/// `toggle` on `root`, the tree `text` parses to.
pub(super) fn toggle_tree(
    text: &str,
    root: &Node,
    selection: Range<usize>,
    format: InlineFormat,
) -> Result<Change, EditRefusal> {
    let mut nodes = Vec::new();
    walk(root, &mut nodes);
    verbatim(&nodes, &selection, format)?;
    if format == InlineFormat::Clear {
        return clear::clear(text, &nodes, selection);
    }
    let picked = &text[selection.clone()];
    let s = selection.start + (picked.len() - picked.trim_start().len());
    let e = (selection.end - (picked.len() - picked.trim_end().len())).max(s);
    let holds = |node: &Node| {
        let whole = span(node);
        let inside = match s == e {
            true => whole.start < s && s < whole.end,
            false => whole.start <= s && e <= whole.end,
        };
        inside && !in_marks(node, text, s) && !in_marks(node, text, e)
    };
    let unwrap = |range: Range<usize>, holes: [Range<usize>; 2]| -> Change {
        let written = cut(text, range.clone(), &holes);
        (vec![(range, written)], after(s, &holes)..after(e, &holes))
    };
    let found = (nodes.iter())
        .filter(|node| styled(format, node) && holds(node))
        .min_by_key(|node| span(node).len());
    if let Some(node) = found {
        crossed(text, &nodes, &span(node), s, e, format)?;
        return Ok(unwrap(
            span(node),
            marks(node, text).expect("a styled node has marks"),
        ));
    }
    let rooms: Vec<Range<usize>> = (nodes.iter())
        .filter(|node| block(node))
        .map(|node| room(node, text))
        .collect();
    let within = (rooms.iter())
        .filter(|range| range.start <= s && e <= range.end)
        .min_by_key(|range| range.len())
        .cloned();
    let Some(within) = within else {
        if s != e {
            let touched = (rooms.iter()).any(|range| range.start <= e && s <= range.end);
            return Err(match touched {
                true => EditRefusal::Across,
                false => EditRefusal::Outside,
            });
        }
        let line = text[..s]
            .rsplit(['\r', '\n'])
            .next()
            .unwrap_or_default()
            .to_string()
            + text[s..].split(['\r', '\n']).next().unwrap_or_default();
        return match line.trim().is_empty() {
            true => Ok(pair(s, format)),
            false => Err(EditRefusal::Outside),
        };
    };
    if format == InlineFormat::Underline
        && let Some(tags) = underline(&nodes, &within, s, e)
    {
        crossed(text, &nodes, &(tags[0].start..tags[1].end), s, e, format)?;
        return Ok(unwrap(tags[0].start..tags[1].end, tags));
    }
    if s != e {
        return wrap(text, &nodes, &within, s, e, format);
    }
    let prev = text[within.start..s].chars().next_back();
    let next = text[s..within.end].chars().next();
    if prev.is_none_or(stop) && next.is_none_or(stop) {
        crossed(text, &nodes, &within, s, s, format)?;
        return Ok(pair(s, format));
    }
    let word = (nodes.iter())
        .filter(|node| matches!(node, Node::Text(_)))
        .map(|node| span(node))
        .find(|run| run.start <= s && s <= run.end)
        .map(|run| {
            let start = (text[run.start..s].char_indices().rev())
                .find(|(_, ch)| stop(*ch))
                .map_or(run.start, |(at, ch)| run.start + at + ch.len_utf8());
            let end = s + text[s..run.end].find(stop).unwrap_or(run.end - s);
            start..end
        })
        .filter(|word| !word.is_empty());
    match word {
        Some(word) => wrap(text, &nodes, &within, word.start, word.end, format),
        None => {
            crossed(text, &nodes, &within, s, s, format)?;
            Ok(pair(s, format))
        }
    }
}

/// The marks `format` writes around `inner`.
fn written(format: InlineFormat, inner: &str) -> (String, String) {
    let (open, close) = match format {
        InlineFormat::Strong => ("**", "**"),
        InlineFormat::Emphasis => ("*", "*"),
        InlineFormat::Strike => ("~~", "~~"),
        InlineFormat::Underline => ("<u>", "</u>"),
        InlineFormat::Link => ("[", "]()"),
        InlineFormat::Image => ("![", "]()"),
        InlineFormat::Code => {
            let longest = inner.split(|ch| ch != '`').map(str::len).max().unwrap_or(0);
            let fence = "`".repeat(longest + 1);
            let pad = if inner.starts_with('`') || inner.ends_with('`') {
                " "
            } else {
                ""
            };
            return (format!("{fence}{pad}"), format!("{pad}{fence}"));
        }
        InlineFormat::Clear => unreachable!("clear writes no marks"),
    };
    (open.into(), close.into())
}

/// An empty pair at `at`, the caret inside it.
fn pair(at: usize, format: InlineFormat) -> Change {
    let (open, close) = written(format, "");
    let caret = at + open.len();
    (vec![(at..at, format!("{open}{close}"))], caret..caret)
}

/// Refuses an end in marks, or a mark split off.
fn crossed(
    text: &str,
    nodes: &[&Node],
    within: &Range<usize>,
    s: usize,
    e: usize,
    format: InlineFormat,
) -> Result<(), EditRefusal> {
    for node in nodes {
        let whole = span(node);
        if matches!(node, Node::Text(_)) || whole.start < within.start || whole.end > within.end {
            continue;
        }
        let holes = match node {
            Node::InlineMath(_) | Node::Html(_) | Node::Break(_) | Node::FootnoteReference(_) => {
                vec![whole.clone()]
            }
            _ => match marks(node, text) {
                Some(marks) => marks.to_vec(),
                None => continue,
            },
        };
        let strictly = |at: usize| holes.iter().any(|hole| hole.start < at && at < hole.end);
        let overlaps = holes.iter().any(|hole| s < hole.end && hole.start < e);
        let holds_all = s <= whole.start && whole.end <= e;
        // A same style cut is taken in whole.
        let same = merges(format) && styled(format, node);
        if strictly(s) || strictly(e) || (overlaps && !holds_all && !same) {
            return Err(EditRefusal::Across);
        }
    }
    Ok(())
}

/// `s..e` in `format`, same marks inside taken in.
fn wrap(
    text: &str,
    nodes: &[&Node],
    within: &Range<usize>,
    mut s: usize,
    mut e: usize,
    format: InlineFormat,
) -> Result<Change, EditRefusal> {
    crossed(text, nodes, within, s, e, format)?;
    if merges(format) {
        for node in nodes.iter().filter(|node| styled(format, node)) {
            let whole = span(node);
            if whole.start < e
                && s < whole.end
                && within.start <= whole.start
                && whole.end <= within.end
            {
                (s, e) = (s.min(whole.start), e.max(whole.end));
            }
        }
        crossed(text, nodes, within, s, e, format)?;
    }
    let mut holes: Vec<Range<usize>> = (nodes.iter())
        .filter(|node| styled(format, node) && s <= span(node).start && span(node).end <= e)
        .flat_map(|node| marks(node, text).expect("a styled node has marks"))
        .filter(|hole| !hole.is_empty())
        .collect();
    holes.sort_by_key(|hole| hole.start);
    let inner = cut(text, s..e, &holes);
    let (open, close) = written(format, &inner);
    let at = s + open.len();
    let selection = match format {
        InlineFormat::Link | InlineFormat::Image => at + inner.len() + 2..at + inner.len() + 2,
        _ => at..at + inner.len(),
    };
    Ok((vec![(s..e, format!("{open}{inner}{close}"))], selection))
}
