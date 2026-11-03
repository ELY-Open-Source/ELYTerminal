//! Block commands at the caret, as Typora's Paragraph menu (R4.7).

mod check;
mod heading;
mod insert;
mod list;
mod quote;
mod split;

use std::ops::Range;

use markdown::mdast::Node;

use super::format::{Change, EditRefusal};
use crate::documents::markdown::{file_tree, span, walk};

/// A command of Typora's Paragraph menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlockCommand {
    /// Levels 1 to 6.
    Heading(u8),
    Paragraph,
    /// A level up; a paragraph turns level 6.
    Promote,
    /// A level down; level 6 turns a paragraph.
    Demote,
    Quote,
    OrderedList,
    BulletList,
    TaskList,
    Rule,
    CodeBlock,
    MathBlock,
    Toc,
    FrontMatter,
    Footnote,
    LinkDefinition,
    ParagraphBefore,
    ParagraphAfter,
    Delete,
}

/// `command` at `selection` in `text`, or why not.
pub(super) fn run(
    text: &str,
    selection: Range<usize>,
    command: BlockCommand,
) -> Result<Change, EditRefusal> {
    let root = file_tree(text);
    let here = Here::new(text, &root, selection);
    match command {
        BlockCommand::Heading(_)
        | BlockCommand::Paragraph
        | BlockCommand::Promote
        | BlockCommand::Demote => heading::run(&here, command),
        BlockCommand::Quote => quote::run(&here),
        BlockCommand::OrderedList | BlockCommand::BulletList | BlockCommand::TaskList => {
            list::run(&here, command)
        }
        BlockCommand::FrontMatter => insert::front_matter(&here),
        BlockCommand::ParagraphBefore => insert::paragraph(&here, true),
        BlockCommand::ParagraphAfter => insert::paragraph(&here, false),
        BlockCommand::Delete => insert::delete(&here),
        _ => insert::run(&here, command),
    }
}

/// The caret's place in the parsed text.
pub(super) struct Here<'a> {
    pub(super) text: &'a str,
    pub(super) selection: Range<usize>,
    /// Root down to the deepest block at the selection's start.
    pub(super) path: Vec<&'a Node>,
    pub(super) root: &'a Node,
    /// Every node, in document order.
    pub(super) nodes: Vec<&'a Node>,
}

impl<'a> Here<'a> {
    fn new(text: &'a str, root: &'a Node, selection: Range<usize>) -> Self {
        let mut nodes = Vec::new();
        walk(root, &mut nodes);
        Here {
            text,
            path: path(text, root, selection.start),
            selection,
            root,
            nodes,
        }
    }

    /// The caret's block, unless the path ends on a container.
    pub(super) fn leaf(&self) -> Option<&'a Node> {
        let last = *self.path.last()?;
        (!holds(last)).then_some(last)
    }

    /// The containers around `path[..upto]`.
    pub(super) fn containers(&self, upto: usize) -> Vec<&'a Node> {
        self.path[..upto]
            .iter()
            .copied()
            .filter(|node| container(node))
            .collect()
    }

    /// Whether the caret's line holds nothing past its prefix.
    pub(super) fn blank(&self) -> bool {
        let line = line_start(self.text, self.selection.start);
        let containers = self.containers(self.path.len());
        inside(self.text, line, &containers)
            .is_some_and(|at| self.text[at..line_end(self.text, line)].trim().is_empty())
    }

    /// Root down to the deepest block at `at`.
    pub(super) fn path_at(&self, at: usize) -> Vec<&'a Node> {
        path(self.text, self.root, at)
    }

    /// The common holder's index and the blocks the selection touches.
    pub(super) fn blocks(&self) -> Option<(usize, Vec<&'a Node>)> {
        let end = self.path_at(self.selection.end);
        let mut k = 0;
        while k + 1 < self.path.len()
            && k + 1 < end.len()
            && std::ptr::eq(self.path[k + 1], end[k + 1])
        {
            k += 1;
        }
        if !holds(self.path[k]) {
            return Some((k - 1, vec![self.path[k]]));
        }
        let (s, e) = (self.selection.start, self.selection.end);
        let kids: Vec<&Node> = (self.path[k].children().map(Vec::as_slice))
            .unwrap_or_default()
            .iter()
            .filter(|kid| span(kid).start <= e && s <= span(kid).end)
            .collect();
        (!kids.is_empty()).then_some((k, kids))
    }

    /// Whether a paragraph ending on the line before joins `block`.
    pub(super) fn joins_before(&self, block: &Range<usize>, gone: &[&Node]) -> bool {
        let line = line_start(self.text, block.start);
        let Some(previous) = prev_line(self.text, line) else {
            return false;
        };
        let ended = (self.nodes.iter()).any(|node| {
            matches!(node, Node::Paragraph(_)) && line_start(self.text, span(node).end) == previous
        });
        ended && !self.opens(line, gone)
    }

    /// Whether the line after `block` would join it as text.
    pub(super) fn joins_after(&self, block: &Range<usize>) -> bool {
        let Some(next) = next_line(self.text, block.end) else {
            return false;
        };
        let starts = (self.nodes.iter()).any(|node| {
            let at = span(node).start;
            line_start(self.text, at) == next
                && match node {
                    Node::Paragraph(_) => true,
                    Node::ThematicBreak(_) => self.text[at..].starts_with('-'),
                    _ => false,
                }
        });
        starts && !self.opens(next, &[])
    }

    /// Whether `line` opens a container that stays and interrupts.
    fn opens(&self, line: usize, gone: &[&Node]) -> bool {
        let on = |node: &Node| line_start(self.text, span(node).start) == line;
        let left = |node: &Node| !gone.iter().any(|gone| std::ptr::eq(*gone, node));
        (self.nodes.iter()).any(|node| match node {
            Node::List(list) if left(node) => list
                .children
                .iter()
                .any(|item| on(item) && (!list.ordered || list.start == Some(1))),
            Node::Blockquote(_) => on(node) && left(node),
            _ => false,
        })
    }
}

/// Blockquotes, list items and footnote definitions.
pub(super) fn container(node: &Node) -> bool {
    matches!(
        node,
        Node::Blockquote(_) | Node::ListItem(_) | Node::FootnoteDefinition(_)
    )
}

/// Whether `node` is task item `holder`'s first text.
pub(super) fn task_head(holder: &Node, node: &Node) -> bool {
    matches!(holder, Node::ListItem(item)
        if item.checked.is_some()
            && item.children.first().is_some_and(|kid| std::ptr::eq(kid, node)))
}

/// Where a heading-closing `#` run starts in `line`.
pub(super) fn closing(line: &str) -> Option<usize> {
    let line = line.trim_end_matches([' ', '\t']);
    let kept = line.trim_end_matches('#');
    (kept.len() < line.len() && kept.ends_with([' ', '\t'])).then_some(kept.len())
}

/// The byte to escape so `line` reads as prose.
pub(super) fn unblocked(line: &str) -> Result<Option<usize>, EditRefusal> {
    let prose = |line: &str| {
        let root = file_tree(line);
        matches!(
            root.children().and_then(|kids| kids.first()),
            Some(Node::Paragraph(_))
        )
    };
    if prose(line) {
        return Ok(None);
    }
    let mark = line.find(|ch: char| ch.is_ascii_punctuation());
    match mark.filter(|mark| prose(&format!("{}\\{}", &line[..*mark], &line[*mark..]))) {
        Some(mark) => Ok(Some(mark)),
        None => Err(check::misread("a line stays a block")),
    }
}

/// Whether the path goes on below `node`.
fn holds(node: &Node) -> bool {
    container(node) || matches!(node, Node::Root(_) | Node::List(_))
}

/// Root down to the deepest block at `at`.
fn path<'a>(text: &str, root: &'a Node, at: usize) -> Vec<&'a Node> {
    let mut path = vec![root];
    while let Some(node) = path.last().copied().filter(|node| holds(node)) {
        let kids = node.children().map(Vec::as_slice).unwrap_or_default();
        let found = (kids.iter())
            .find(|kid| span(kid).contains(&at) || span(kid).end == at)
            .or_else(|| {
                // A caret before a block on its line is inside.
                (kids.iter()).find(|kid| {
                    let start = span(kid).start;
                    at < start && !text[at..start].contains(['\n', '\r'])
                })
            });
        let Some(kid) = found else {
            break;
        };
        path.push(kid);
    }
    path
}

/// Whether `at` falls between a CR and its LF.
fn splits(text: &str, at: usize) -> bool {
    text[..at].ends_with('\r') && text[at..].starts_with('\n')
}

/// `at`, before the CR when it splits a CRLF.
pub(super) fn settled(text: &str, at: usize) -> usize {
    if splits(text, at) { at - 1 } else { at }
}

pub(super) fn line_start(text: &str, at: usize) -> usize {
    debug_assert!(!splits(text, at), "line_start inside a CRLF");
    text[..at].rfind(['\n', '\r']).map_or(0, |at| at + 1)
}

pub(super) fn line_end(text: &str, at: usize) -> usize {
    debug_assert!(!splits(text, at), "line_end inside a CRLF");
    at + text[at..].find(['\n', '\r']).unwrap_or(text.len() - at)
}

/// The start of the line after `at`'s.
pub(super) fn next_line(text: &str, at: usize) -> Option<usize> {
    let end = line_end(text, at);
    let rest = &text[end..];
    let ending = if rest.starts_with("\r\n") { 2 } else { 1 };
    (end < text.len()).then_some(end + ending)
}

/// The start of the line before `at`'s.
pub(super) fn prev_line(text: &str, at: usize) -> Option<usize> {
    let start = line_start(text, at);
    let ending = if text[..start].ends_with("\r\n") {
        2
    } else {
        1
    };
    (start > 0).then(|| line_start(text, start - ending))
}

/// The text's own line ending.
pub(super) fn eol(text: &str) -> &'static str {
    match (text.contains("\r\n"), text.contains('\r')) {
        (true, _) => "\r\n",
        (false, true) => "\r",
        _ => "\n",
    }
}

/// Columns before `at` on its line, tabs stopping at four.
pub(super) fn column(text: &str, at: usize) -> usize {
    advance(0, &text[line_start(text, at)..at])
}

fn advance(col: usize, run: &str) -> usize {
    run.chars().fold(col, |col, ch| match ch {
        '\t' => col / 4 * 4 + 4,
        _ => col + 1,
    })
}

/// A list marker's or footnote label's bytes, then its spaces.
pub(super) fn marker(text: &str, node: &Node) -> (usize, usize) {
    let start = span(node).start;
    let line = &text[start..line_end(text, start)];
    let mark = match node {
        Node::FootnoteDefinition(_) => line.find("]:").map_or(line.len(), |at| at + 2),
        _ if line.starts_with(['-', '*', '+']) => 1,
        _ => line.find(['.', ')']).map_or(line.len(), |at| at + 1),
    };
    let after = &line[mark..];
    let spaces = after.len() - after.trim_start_matches([' ', '\t']).len();
    (mark, spaces)
}

/// The column a container's continuation lines start at.
pub(super) fn indent(text: &str, node: &Node) -> usize {
    let start = span(node).start;
    let base = column(text, start);
    let (mark, spaces) = marker(text, node);
    if matches!(node, Node::FootnoteDefinition(_)) {
        return base + 4;
    }
    let after = advance(base + mark, &text[start + mark..start + mark + spaces]) - base - mark;
    let blank = text[start + mark..line_end(text, start)].trim().is_empty();
    match blank || !(1..=4).contains(&after) {
        true => base + mark + 1,
        false => base + mark + after,
    }
}

/// Where a container's text starts on its first line.
pub(super) fn opener(text: &str, node: &Node) -> usize {
    let start = span(node).start;
    let (mark, spaces) = marker(text, node);
    let width = advance(
        column(text, start + mark),
        &text[start + mark..start + mark + spaces],
    ) - column(text, start + mark);
    match (1..=4).contains(&width) {
        true => start + mark + spaces,
        false => start + mark + spaces.min(1),
    }
}

/// Where `line`'s text starts inside `containers`; none when lazy.
pub(super) fn inside(text: &str, line: usize, containers: &[&Node]) -> Option<usize> {
    let end = line_end(text, line);
    let mut at = line;
    for node in containers {
        if text[at..end].trim().is_empty() {
            return Some(end);
        }
        if matches!(node, Node::Blockquote(_)) {
            let lead = text[at..end].len() - text[at..end].trim_start_matches(' ').len();
            if lead > 3 || !text[at + lead..end].starts_with('>') {
                return None;
            }
            at += lead + 1;
            at += usize::from(text[at..end].starts_with([' ', '\t']));
            continue;
        }
        let start = span(node).start;
        if line <= start && start < end {
            at = opener(text, node);
            continue;
        }
        let want = indent(text, node);
        let mut col = column(text, at);
        while col < want {
            let ch = text[at..end]
                .chars()
                .next()
                .filter(|ch| matches!(ch, ' ' | '\t'))?;
            col = advance(col, &text[at..at + 1]);
            at += ch.len_utf8();
        }
    }
    Some(at)
}

/// The prefix a new line takes inside `containers`.
pub(super) fn prefix(text: &str, containers: &[&Node]) -> String {
    let mut out = String::new();
    for (ix, node) in containers.iter().enumerate() {
        if matches!(node, Node::Blockquote(_)) {
            out.push_str("> ");
            continue;
        }
        let start = span(node).start;
        let outer = inside(text, line_start(text, start), &containers[..ix]).unwrap_or(start);
        let width = indent(text, node).saturating_sub(column(text, outer.min(start)));
        out.push_str(&" ".repeat(width));
    }
    out
}

/// Where byte `at` lies once `edits`, sorted, are written.
pub(super) fn shifted(at: usize, edits: &[(Range<usize>, String)]) -> usize {
    let mut moved = at;
    for (range, with) in edits {
        if range.end <= at {
            moved = moved + with.len() - range.len();
        } else if range.start < at {
            moved -= at - range.start;
        }
    }
    moved
}

/// `edits`, sorted, and the caret mapped through them.
pub(super) fn change(mut edits: Vec<(Range<usize>, String)>, caret: Range<usize>) -> Change {
    edits.sort_by_key(|(range, _)| range.start);
    assert!(
        edits
            .windows(2)
            .all(|pair| pair[0].0.end <= pair[1].0.start),
        "block edits overlap"
    );
    let caret = shifted(caret.start, &edits)..shifted(caret.end, &edits);
    (edits, caret)
}

/// `edits`, sorted, with the caret already where it lands.
pub(super) fn placed(mut edits: Vec<(Range<usize>, String)>, caret: usize) -> Change {
    edits.sort_by_key(|(range, _)| range.start);
    assert!(
        edits
            .windows(2)
            .all(|pair| pair[0].0.end <= pair[1].0.start),
        "block edits overlap"
    );
    (edits, caret..caret)
}

/// Why `node` takes no block command at the caret.
pub(super) fn refusal(node: &Node) -> EditRefusal {
    match node {
        Node::Code(_) => EditRefusal::Code,
        Node::Math(_) => EditRefusal::Math,
        Node::Html(_) => EditRefusal::Html,
        Node::Table(_) => EditRefusal::Table,
        Node::Yaml(_) | Node::Toml(_) => EditRefusal::FrontMatter,
        _ => EditRefusal::Outside,
    }
}
