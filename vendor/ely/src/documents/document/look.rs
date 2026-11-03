//! A writing view: how text shows, not what it says.

use gpui::{Context, Rems, SharedString};

use super::MarkdownDocument;
use crate::documents::MarkdownMode;

/// How the document shows its text (MARK-09).
#[derive(Clone, Debug, PartialEq)]
pub struct DocumentLook {
    /// All but the current block or line fade.
    pub focus: bool,
    /// The current block or line stays mid-view.
    pub typewriter: bool,
    /// The widest the text column grows.
    pub measure: Option<Rems>,
    /// The prose font; none keeps the theme's.
    pub font: Option<SharedString>,
    /// Text size against the theme's.
    pub scale: f32,
}

impl Default for DocumentLook {
    fn default() -> Self {
        Self {
            focus: false,
            typewriter: false,
            measure: None,
            font: None,
            scale: 1.0,
        }
    }
}

impl MarkdownDocument {
    pub fn look(&self) -> &DocumentLook {
        &self.look
    }

    pub fn set_look(&mut self, look: DocumentLook, cx: &mut Context<Self>) {
        log::info!("markdown document: look {look:?}");
        self.editor.update(cx, |editor, cx| {
            editor.set_scale(look.scale, cx);
            editor.set_focus_line(look.focus, cx);
            editor.set_typewriter(look.typewriter, cx);
        });
        self.look = look;
        self.centered = None;
        cx.notify();
    }

    /// The reader's block: the open one, or the caret's.
    pub(super) fn current_block(&self, cx: &gpui::App) -> usize {
        match &self.open {
            Some(open) if open.append => self.parsed.blocks.len(),
            Some(open) => self.block_at(open.range.start),
            None => self.block_at(self.editor.read(cx).primary().head),
        }
    }

    /// Half a view of room at each end, for typewriter.
    pub(super) fn make_room(&mut self) {
        let room = match self.look.typewriter && self.mode == MarkdownMode::Visual {
            true => self.list.viewport_bounds().size.height / 2.0,
            false => gpui::Pixels::ZERO,
        };
        if room != self.room {
            self.room = room;
            let last = self.parsed.blocks.len();
            self.list.remeasure_items(0..1);
            self.list.remeasure_items(last..last + 1);
        }
    }

    /// The current block mid-view once it or its text moves.
    pub(super) fn center_current(&mut self, cx: &gpui::App) {
        if !self.look.typewriter || self.mode != MarkdownMode::Visual {
            return;
        }
        let ix = self.current_block(cx);
        let Some(mut block) = self.list.bounds_for_item(ix) else {
            return;
        };
        // The ends' room is no block.
        if ix == 0 {
            block.origin.y += self.room;
            block.size.height -= self.room;
        }
        if ix == self.parsed.blocks.len() {
            block.size.height -= self.room;
        }
        let view = self.list.viewport_bounds();
        // New layout, resize or end room moves it.
        let heights = [block.size.height, view.size.height, self.room]
            .map(|height| f32::from(height).round() as u64);
        let moved = (ix, self.edits, self.typed, heights);
        if self.centered == Some(moved) {
            return;
        }
        self.list.scroll_by(block.center().y - view.center().y);
        self.centered = Some(moved);
    }
}
