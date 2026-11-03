mod blocks;
mod contents;
mod counts;
mod diagram;
mod document;
mod epub;
mod find;
mod format;
mod history;
mod knowledge;
mod link;
mod markdown;
mod modes;
mod pages;
mod picture;
mod print;
mod reading;
mod render;
mod rich;
mod suggest;
mod templates;
#[cfg(all(test, feature = "test-support"))]
mod tests;
mod thumbs;
mod todo;
mod toolbar;
mod tree;
mod viewer;

use gpui::App;

pub(crate) use blocks::source;
pub use blocks::{Align, BlockData, BlockEditor, BlockEvent, BlockKind, Media};
pub use contents::outline;
pub use counts::{ReadingTime, WordCount};
pub use diagram::{Diagram, diagram_svg};
pub(crate) use diagram::{flatten, options as svg_options};
pub use document::{
    Attachment, BlockCommand, DocumentEvent, DocumentHeading, DocumentLook, DocumentTarget,
    EditRefusal, InlineFormat, LARGEST_READ, MarkdownDocument, TableEdit, TableRefusal,
    TargetBlock,
};
pub use epub::{Chapter, EpubReader};
pub use format::{Format, format};
pub(crate) use history::Change;
pub use history::{PageHistoryDiff, Version, VersionHistory};
pub use knowledge::{Backlink, Backlinks, Favorite, Favorites, TrashBin, Trashed};
pub use link::LinkEditor;
pub use markdown::file_tree as markdown_tree;
pub use markdown::markdown_highlights;
pub use modes::{MarkdownEditor, MarkdownMode};
pub use pages::{PageCover, PageIcon};
pub use print::{Margins, Paper, PrintPreview, PrintSettings, sheet};
pub use reading::{Footnote, Glossary, ReadingProgress, TooltipTerm, ZenMode};
pub use render::MarkdownRenderer;
pub use rich::RichTextEditor;
pub use templates::{Template, TemplatePicker};
pub use thumbs::PageThumbnailList;
pub use todo::{Checklist, TodoItem};
pub use toolbar::FixedFormatToolbar;
pub use tree::{
    AlertKind, Inline, Node as MarkdownBlock, Reader as MarkdownReader, Style, alert, is_toc,
};
pub(crate) use viewer::step as zoom_step;
pub use viewer::{DocPage, DocumentViewer, PageHit, PageNote};

/// Binds the documents keys. `init` calls it.
pub(crate) fn bind_keys(cx: &mut App) {
    rich::bind_keys(cx);
    document::bind_keys(cx);
}
