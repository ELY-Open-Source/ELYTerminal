//! Dropped files and pasted pictures go to the owner.

use std::path::PathBuf;

use gpui::{ClipboardEntry, Context, EventEmitter, Image};

use super::MarkdownDocument;
use crate::documents::MarkdownMode;

/// What was dropped on or pasted into the document.
#[derive(Clone, Debug, PartialEq)]
pub enum Attachment {
    /// Files dropped, as they lie on disk.
    Files(Vec<PathBuf>),
    /// A picture pasted with no text beside it.
    Picture(Image),
}

impl EventEmitter<Attachment> for MarkdownDocument {}

impl MarkdownDocument {
    /// A pasted picture goes to the owner; false leaves it.
    pub(super) fn paste_picture(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(item) = cx.read_from_clipboard() else {
            return false;
        };
        let entries = item.entries();
        if entries
            .iter()
            .any(|entry| matches!(entry, ClipboardEntry::String(_)))
        {
            return false;
        }
        let Some(picture) = entries.iter().find_map(|entry| match entry {
            ClipboardEntry::Image(image) => Some(image.clone()),
            _ => None,
        }) else {
            return false;
        };
        log::info!("markdown document: a picture pasted");
        cx.emit(Attachment::Picture(picture));
        true
    }

    /// `text` at the caret, as typed; false in preview.
    pub fn insert(&mut self, text: &str, cx: &mut Context<Self>) -> bool {
        if self.mode == MarkdownMode::Preview {
            return false;
        }
        if let Some(open) = &self.open {
            open.input.update(cx, |input, cx| input.insert(text, cx));
            return true;
        }
        let caret = self.editor.read(cx).primary().range();
        let after = caret.start + text.len();
        self.editor.update(cx, |editor, cx| {
            editor.edit([(caret, text.to_string())], cx);
            editor.select(std::iter::once(after..after), cx);
        });
        true
    }
}
