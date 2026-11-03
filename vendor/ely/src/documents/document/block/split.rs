//! A paragraph or heading split, as Typora's `splitAt`.

use std::ops::Range;

use markdown::mdast::Node;

use super::{
    Here, check, closing, inside, line_end, line_start, next_line, placed, prev_line, shifted,
    task_head, unblocked,
};
use crate::documents::{
    document::format::{Change, EditRefusal, marks},
    markdown::span,
};

/// Where the new block's text goes, ending the edit.
pub(super) struct Room<'a> {
    pub(super) nl: &'static str,
    pub(super) lead: String,
    pub(super) blank: String,
    pub(super) containers: Vec<&'a Node>,
}

impl Room<'_> {
    /// A blank line, then a new line's prefix.
    pub(super) fn gap(&self) -> String {
        format!("{}{}{}{}", self.nl, self.blank, self.nl, self.lead)
    }
}

/// Where `line`'s text starts inside `containers`; a lazy line's own.
pub(super) fn content(text: &str, line: usize, containers: &[&Node]) -> usize {
    let at = inside(text, line, containers).unwrap_or(line);
    at + text[at..].len() - text[at..].trim_start_matches([' ', '\t']).len()
}

/// Back from `at` over blanks and prefixes, to `floor`.
fn back(text: &str, mut at: usize, floor: usize, containers: &[&Node]) -> usize {
    loop {
        let line = line_start(text, at).max(floor);
        at = line + text[line..at].trim_end_matches([' ', '\t']).len();
        if line == floor || at > content(text, line, containers) {
            return at;
        }
        at = line_end(text, prev_line(text, line).expect("a line above the floor"));
    }
}

/// On from `at` over blanks and prefixes, to `ceil`.
fn forth(text: &str, mut at: usize, ceil: usize, containers: &[&Node]) -> usize {
    loop {
        let end = line_end(text, at).min(ceil);
        at = end - text[at..end].trim_start_matches([' ', '\t']).len();
        if at < end {
            return at;
        }
        match next_line(text, at) {
            Some(next) if next < ceil => at = content(text, next, containers),
            _ => return ceil,
        }
    }
}

/// Inline nodes with no text to cut.
fn sealed(node: &Node) -> bool {
    matches!(
        node,
        Node::InlineMath(_) | Node::Html(_) | Node::FootnoteReference(_)
    )
}

/// `at` off inline edges; marks to close, then reopen.
fn seam(here: &Here, mut at: usize) -> Result<(usize, String, String), EditRefusal> {
    let text = here.text;
    let marked: Vec<(&Node, [Range<usize>; 2])> = (here.nodes.iter())
        .filter_map(|node| Some((*node, marks(node, text)?)))
        .collect();
    while let Some([open, _]) = (marked.iter())
        .map(|(_, holes)| holes)
        .find(|[open, _]| open.start < at && open.end == at)
    {
        at = open.start;
    }
    while let Some([_, close]) = (marked.iter())
        .map(|(_, holes)| holes)
        .find(|[_, close]| close.start == at && at < close.end)
    {
        at = close.end;
    }
    let strictly = |range: Range<usize>| range.start < at && at < range.end;
    let mut cut: Vec<&(&Node, [Range<usize>; 2])> = (marked.iter())
        .filter(|(node, _)| strictly(span(node)))
        .collect();
    let paired =
        |node: &Node| matches!(node, Node::Strong(_) | Node::Emphasis(_) | Node::Delete(_));
    let unsafe_cut = (here.nodes.iter()).any(|node| sealed(node) && strictly(span(node)))
        || (cut.iter()).any(|(node, [open, close])| {
            !paired(node) || strictly(open.clone()) || strictly(close.clone())
        });
    if unsafe_cut {
        return Err(EditRefusal::Across);
    }
    cut.sort_by_key(|(node, _)| span(node).start);
    let close = (cut.iter().rev()).map(|(_, [_, close])| &text[close.clone()]);
    let open = cut.iter().map(|(_, [open, _])| &text[open.clone()]);
    Ok((at, close.collect(), open.collect()))
}

/// `block` put where the selection splits `node`.
pub(super) fn split(
    here: &Here,
    node: &Node,
    room: &Room,
    block: &str,
    inner: Option<usize>,
) -> Result<Change, EditRefusal> {
    let text = here.text;
    let whole = span(node);
    let kids = node.children().map(Vec::as_slice).unwrap_or_default();
    let (marks, body) = match node {
        Node::Heading(heading) => (
            "#".repeat(usize::from(heading.depth)) + " ",
            kids.first().map_or(whole.end, |kid| span(kid).start)
                ..kids.last().map_or(whole.end, |kid| span(kid).end),
        ),
        _ => (String::new(), whole.clone()),
    };
    // A caret in heading marks takes the text's edge.
    let s = here.selection.start.clamp(body.start, body.end);
    let e = here.selection.end.clamp(body.start, body.end);
    let (s, close, _) = seam(here, s)?;
    let (e, _, open) = seam(here, e)?;
    let containers = &room.containers;
    let mut from = back(text, s, body.start, containers);
    let after = forth(text, e, body.end, containers);
    let before = from > body.start;
    let follows = after < body.end;
    // GFM reads a task's first line as text.
    if !before && task_head(here.path[here.path.len() - 2], node) {
        return Err(EditRefusal::Outside);
    }
    if !before {
        from = whole.start;
    }
    let to = if follows { after } else { whole.end };
    let gap = room.gap();
    let mut written = String::new();
    if before {
        written.push_str(&close);
        written.push_str(&gap);
    }
    let start = written.len();
    written.push_str(block);
    let lead = if follows { marks.len() + open.len() } else { 0 };
    let caret = match inner {
        Some(inner) => start + inner,
        None => {
            written.push_str(&gap);
            written.len() + lead
        }
    };
    if follows {
        if inner.is_some() {
            written.push_str(&gap);
        }
        written.push_str(&marks);
        written.push_str(&open);
    }
    let mut edits = Vec::new();
    let mut shift = 0;
    if !before && here.joins_before(&whole, &[]) {
        let line = line_start(text, whole.start);
        let blank = format!("{}{}", room.blank, room.nl);
        shift = blank.len();
        edits.push((line..line, blank));
    }
    edits.push((from..to, written));
    // A heading's cut end must not close it.
    let line = line_start(text, from);
    if before
        && close.is_empty()
        && matches!(node, Node::Heading(_))
        && let Some(run) = closing(&text[line..from])
    {
        edits.push((line + run..line + run, "\\".to_string()));
        shift += 1;
    }
    // The rest must not open a block.
    if follows
        && marks.is_empty()
        && open.is_empty()
        && let Some(mark) = unblocked(&text[after..line_end(text, after)])?
    {
        edits.push((after + mark..after + mark, "\\".to_string()));
    }
    let change = placed(edits, from + shift + caret);
    let edits = &change.0;
    let (written, after_root) = check::reread(text, edits);
    let now = check::Shown::new(&written, &after_root);
    let level = match node {
        Node::Heading(heading) => heading.depth,
        _ => 0,
    };
    let piece = |at: usize| {
        now.at(shifted(at, edits))
            .and_then(|(got, chars)| (got == level).then_some(chars))
    };
    let head = if before {
        piece(from - 1)
    } else {
        Some(Vec::new())
    };
    let tail = if follows { piece(to) } else { Some(Vec::new()) };
    let want = check::Shown::new(text, here.root).inline(node);
    let kept = head
        .zip(tail)
        .is_some_and(|(head, tail)| match here.selection.is_empty() {
            true => [head, tail].concat() == want,
            false => {
                want.starts_with(&head)
                    && want.ends_with(&tail)
                    && head.len() + tail.len() <= want.len()
            }
        });
    match kept {
        true => Ok(change),
        false => Err(check::misread("a split would reread otherwise")),
    }
}
