use gpui::{App, ElementId, IntoElement, RenderOnce, Window};
use jiff::Timestamp;

use super::FileIcon;
use crate::{
    lists::{DirEntry, ListItem},
    typography::format,
};

/// A file's size and when it changed, or a folder's word and when it changed.
pub(crate) fn detail(entry: &DirEntry, now: Timestamp) -> String {
    let when = format::relative(entry.modified(), now);
    match entry.size() {
        Some(bytes) => format!("{} · {when}", format::file_size(bytes, false)),
        None => format!("Folder · {when}"),
    }
}

/// A file or folder as a row: its icon and name over its size and when it changed. As a `ListItem`, it goes into a `SelectableList`.
#[derive(IntoElement)]
pub struct FileItem {
    id: ElementId,
    entry: DirEntry,
}

impl FileItem {
    pub fn new(id: impl Into<ElementId>, entry: DirEntry) -> Self {
        Self {
            id: id.into(),
            entry,
        }
    }
}

impl From<FileItem> for ListItem {
    fn from(item: FileItem) -> Self {
        let detail = detail(&item.entry, Timestamp::now());
        ListItem::new(item.id, item.entry.name().clone())
            .leading(FileIcon::entry(&item.entry))
            .description(detail)
    }
}

impl RenderOnce for FileItem {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        ListItem::from(self)
    }
}
