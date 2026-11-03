use std::{ops::Range, rc::Rc};

use gpui::{AppContext as _, Context, Focusable as _, MouseButton, Pixels, Point, Window};
use markdown::mdast;

use super::{
    DocumentEvent, MarkdownDocument,
    cells::{self, Aim},
    edit::Landing,
    edit::moved,
    parse::Block,
};
use crate::documents::{
    markdown::{file_tree, span, walk},
    render::{CellAt, OnPoint, Place, Pointed},
    tree::{Node, Reader, alert, is_toc, lines_at},
};

/// A top-level block's kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TargetBlock {
    Paragraph,
    Heading(u8),
    Quote,
    Alert,
    List,
    Code,
    Math,
    Table,
    Rule,
    Html,
    Footnote,
    Definition,
    FrontMatter,
    Toc,
}

impl TargetBlock {
    /// `node`'s kind, read from `source`.
    pub(super) fn of(node: &mdast::Node, source: &str) -> Self {
        match node {
            mdast::Node::Paragraph(_) if is_toc(node, source) => Self::Toc,
            mdast::Node::Paragraph(_) => Self::Paragraph,
            mdast::Node::Heading(heading) => Self::Heading(heading.depth),
            mdast::Node::Blockquote(quote) if alert(quote).is_some() => Self::Alert,
            mdast::Node::Blockquote(_) => Self::Quote,
            mdast::Node::List(_) => Self::List,
            mdast::Node::Code(_) => Self::Code,
            mdast::Node::Math(_) => Self::Math,
            mdast::Node::Table(_) => Self::Table,
            mdast::Node::ThematicBreak(_) => Self::Rule,
            mdast::Node::Html(_) => Self::Html,
            mdast::Node::FootnoteDefinition(_) => Self::Footnote,
            mdast::Node::Definition(_) => Self::Definition,
            mdast::Node::Yaml(_) | mdast::Node::Toml(_) => Self::FrontMatter,
            other => unreachable!("{other:?} is no block outside MDX"),
        }
    }
}

/// What a secondary press landed on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocumentTarget {
    /// The pressed source byte; an open block's as unwritten.
    pub at: usize,
    /// None past the last block.
    pub block: Option<TargetBlock>,
    pub link: Option<String>,
    pub image: Option<String>,
    pub in_table: bool,
    /// The code block's text, pressed in one.
    pub code: Option<String>,
}

impl MarkdownDocument {
    /// Presses on block `ix`'s drawing, read as it was drawn.
    pub(super) fn on_point(&self, ix: usize, cx: &mut Context<Self>) -> OnPoint {
        let (document, parsed) = (cx.entity().downgrade(), self.parsed.clone());
        let block = parsed.blocks[ix].range.clone();
        let start = block.start;
        Rc::new(move |pointed, button, window, cx| {
            // The drawing counts from its block's start.
            let at = match &pointed.place {
                Place::Text(inline, drawn) => inline.source_at(&parsed.text[start..], *drawn),
                // Unreadable lines land at their start.
                Place::Lines { text, from, at } => {
                    Some(lines_at(&parsed.text[start..], *from, text, *at).unwrap_or(*from))
                }
                Place::At(at) => Some(*at),
                // A cell's padding lands at its end.
                Place::Cell(cell) => cells::find(&parsed.text, block.clone(), Aim::At(*cell))
                    .map(|found| found.range.end - start),
                Place::Block => None,
            }
            .map(|at| start + at);
            // A press in a table opens only its cell.
            let landing = match (&pointed.place, at) {
                (Place::Cell(cell), _) => Landing::Cell(Aim::At(*cell)),
                (_, Some(at)) if pointed.table => Landing::Cell(Aim::Byte(at)),
                (_, Some(at)) => Landing::At(at),
                (_, None) => Landing::End,
            };
            let ix = Some(ix);
            document
                .update(cx, |document, cx| {
                    // The open cell's padding keeps its typing.
                    let open = match pointed.place {
                        Place::Cell(cell) => document.open_cell(&block) == Some(cell),
                        _ => false,
                    };
                    match (button, open) {
                        (MouseButton::Left, true) => document.cell_end(window, cx),
                        (MouseButton::Right, true) => document.point_open(None, cx),
                        (MouseButton::Left, false) => document.open(ix, landing, window, cx),
                        (MouseButton::Right, false) => document.point(ix, at, pointed, window, cx),
                        _ => {}
                    }
                })
                .expect("a block's drawing lives in its document");
        })
    }

    /// The cell open in `block`, if any.
    fn open_cell(&self, block: &Range<usize>) -> Option<CellAt> {
        let open = self.open.as_ref()?;
        let inside = block.start <= open.range.start && open.range.end <= block.end;
        open.cell.filter(|_| inside)
    }

    /// Caret to the open cell's end.
    fn cell_end(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(open) = &self.open else {
            return;
        };
        let input = open.input.clone();
        input.update(cx, |input, cx| {
            let end = input.text().len();
            input.select(end..end, cx);
        });
        window.focus(&input.focus_handle(cx), cx);
    }

    /// A block's kind, text and what it draws.
    fn drawn(&self, block: &Block) -> (TargetBlock, String, Option<Vec<Node>>) {
        let text = self.parsed.text[block.range.clone()].to_string();
        (block.kind, text, block.nodes.clone())
    }

    /// Caret to `at`, else block `ix`; the owner hears.
    pub(super) fn point(
        &mut self,
        ix: Option<usize>,
        at: Option<usize>,
        pointed: Pointed,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.pressed += 1;
        let block = match ix.map(|ix| self.parsed.blocks.get(ix)) {
            Some(None) => return log::warn!("markdown document: a press on a block read away"),
            block => block.flatten(),
        };
        let kind = block.map(|block| block.kind);
        let pressed = block.map(|block| self.drawn(block));
        let code = (pointed.code.as_deref().map(str::to_string)).or_else(|| {
            match block.and_then(|block| block.nodes.as_deref()) {
                Some([Node::Code { text, .. }]) => Some(text.clone()),
                _ => None,
            }
        });
        let at = at.or(block.map(|block| block.range.start));
        let written = self.commit(cx);
        if written.is_some() {
            self.reparse(true, cx);
        }
        if !self.read_now(cx) {
            return log::warn!("markdown document: pressed while the text is read again");
        }
        let at = match at {
            None => self.editor.read(cx).text().len(),
            Some(at) => match moved(at, &written) {
                Some(at) => at,
                None => return log::warn!("markdown document: the pressed text was written over"),
            },
        };
        // A write-back may change the pressed block.
        let now = (self.parsed.blocks.iter())
            .find(|block| block.range.start <= at && at <= block.range.end)
            .map(|block| self.drawn(block));
        // A cell beside the written one keeps its place.
        let beside = pointed.table && now.as_ref().is_some_and(|now| now.0 == TargetBlock::Table);
        if written.is_some() && pressed.is_some() && now != pressed && !beside {
            return log::warn!("markdown document: the write-back changed the pressed block");
        }
        self.editor
            .update(cx, |editor, cx| editor.select(std::iter::once(at..at), cx));
        // Its focus opens no empty block.
        self.pointing = !self.focus.is_focused(window);
        window.focus(&self.focus, cx);
        log::info!("markdown document: a secondary press puts the caret at byte {at}");
        cx.emit(DocumentEvent::Pointed(DocumentTarget {
            at,
            block: kind,
            link: pointed.link,
            image: pointed.image,
            in_table: pointed.table || kind == Some(TargetBlock::Table),
            code,
        }));
        cx.notify();
    }

    /// A right press at `position`, else at the field's end.
    pub(super) fn point_open(&mut self, position: Option<Point<Pixels>>, cx: &mut Context<Self>) {
        let Some(open) = &self.open else {
            return log::warn!("markdown document: a press on a block closed since");
        };
        let input = open.input.read(cx);
        let pressed = match position {
            Some(position) => input.offset_for_point(position),
            None => input.text().len(),
        };
        // A cell's typed pipes shift its bytes.
        let pressed = match open.cell {
            Some(_) => {
                let edges = cells::edges(self.editor.read(cx).text(), open.range.clone());
                cells::kept_at(input.text(), pressed, edges)
            }
            None => pressed,
        };
        let taken = self.snapshot(cx);
        let (full, offset, _) = taken.clone();
        self.pressed += 1;
        let press = self.pressed;
        cx.spawn(async move |document, cx| {
            let target = cx
                .background_spawn(async move { target_at(&full, offset + pressed) })
                .await;
            document
                .update(cx, |document, cx| {
                    if document.pressed != press {
                        return log::info!("markdown document: a newer press came first");
                    }
                    if document.open.is_none() || document.snapshot(cx) != taken {
                        return log::info!("markdown document: the pressed block changed first");
                    }
                    log::info!("markdown document: a secondary press in the open block");
                    cx.emit(DocumentEvent::Pointed(target));
                })
                .ok();
        })
        .detach();
    }
}

/// What byte `at` of `text` holds, read as drawn.
fn target_at(text: &str, at: usize) -> DocumentTarget {
    let root = file_tree(text);
    let mut nodes = Vec::new();
    walk(&root, &mut nodes);
    let top = (root
        .children()
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter())
    .find(|node| span(node).start <= at && at <= span(node).end);
    let block = top.map(|node| TargetBlock::of(node, text));
    let mut drawn = Vec::new();
    if let Some(node) = top.filter(|node| !matches!(node, mdast::Node::Definition(_))) {
        Reader::new(text, &root).top(node, &mut drawn);
    }
    let mut flat = Vec::new();
    flatten(&drawn, &mut flat);
    let around: Vec<&mdast::Node> = (nodes.iter().copied())
        .filter(|node| span(node).contains(&at))
        .collect();
    let defined = |identifier: &str| {
        nodes.iter().find_map(|node| match node {
            mdast::Node::Definition(definition) if definition.identifier == identifier => {
                Some(definition.url.clone())
            }
            _ => None,
        })
    };
    let written = || {
        around.iter().rev().find_map(|node| match node {
            mdast::Node::Link(link) => Some(link.url.clone()),
            mdast::Node::LinkReference(link) => defined(&link.identifier),
            _ => None,
        })
    };
    let picture = around
        .iter()
        .rev()
        .find(|node| matches!(node, mdast::Node::Image(_) | mdast::Node::ImageReference(_)));
    let shown = picture.and_then(|node| {
        flat.iter().find_map(|kid| match kid {
            Node::Image { at, url, link, .. } if *at == span(node).start => {
                Some((url.clone(), link.clone()))
            }
            _ => None,
        })
    });
    let image = match &shown {
        Some((url, _)) => Some(url.clone()),
        None => picture.and_then(|node| match node {
            mdast::Node::Image(picture) => Some(picture.url.clone()),
            mdast::Node::ImageReference(picture) => defined(&picture.identifier),
            _ => None,
        }),
    };
    let link = match shown {
        Some((_, link)) => link,
        None => drawn_link(&flat, text, at).unwrap_or_else(written),
    };
    let code = around.iter().find_map(|node| match node {
        mdast::Node::Code(code) => Some(code.value.clone()),
        _ => None,
    });
    DocumentTarget {
        at,
        block,
        link,
        image,
        in_table: around
            .iter()
            .any(|node| matches!(node, mdast::Node::Table(_))),
        code,
    }
}

/// `nodes` and all they hold, depth first.
fn flatten<'a>(nodes: &'a [Node], out: &mut Vec<&'a Node>) {
    for node in nodes {
        out.push(node);
        match node {
            Node::Quote(body) | Node::Footnote { body, .. } | Node::Alert { body, .. } => {
                flatten(body, out)
            }
            Node::List { items, .. } => items.iter().for_each(|(_, body)| flatten(body, out)),
            Node::Table { head, rows, .. } => {
                (head.iter().chain(rows.iter().flatten())).for_each(|cell| flatten(cell, out))
            }
            _ => {}
        }
    }
}

/// The link drawn at source byte `at`; none if undrawn.
fn drawn_link(nodes: &[&Node], text: &str, at: usize) -> Option<Option<String>> {
    let inline = nodes.iter().find_map(|node| match node {
        Node::Paragraph(inline) | Node::Heading(_, inline)
            if (inline.sources.iter()).any(|(_, span, _)| span.contains(&at)) =>
        {
            Some(inline)
        }
        _ => None,
    })?;
    let drawn = inline.drawn_at(text, at)?;
    let link = (inline.links.iter()).find(|(range, _)| range.contains(&drawn));
    Some(link.map(|(_, url)| url.clone()))
}
