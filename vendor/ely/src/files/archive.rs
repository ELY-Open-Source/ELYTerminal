use std::rc::Rc;

use gpui::{
    App, ElementId, FontWeight, IntoElement, ParentElement, RenderOnce, SharedString, Styled,
    Window, div, prelude::*,
};

use super::FileIcon;
use crate::{
    lists::{Folder, Tree, TreeNode},
    primitives::{IconName, IconTheme, icon_theme},
    theme::{ActiveTheme, IconSize, TextSize},
    typography::{MiddleEllipsis, format},
};

type OnArchived = Rc<dyn Fn(&ArchiveEntry, &mut Window, &mut App)>;

/// A file inside an archive: its path there, with `/` between folders, its size and its packed size, in bytes.
#[derive(Clone, Debug, PartialEq)]
pub struct ArchiveEntry {
    pub path: SharedString,
    pub size: u64,
    pub packed: u64,
}

/// How much smaller packing made `size`, a whole percent.
pub(crate) fn saving(size: u64, packed: u64) -> u64 {
    if size == 0 {
        return 0;
    }
    ((size - packed.min(size)) as f64 / size as f64 * 100.0).round() as u64
}

/// Nodes for `folder` under `prefix`, folders first, then files, each by name; and the bytes it holds.
fn nodes(
    folder: &Folder,
    prefix: &str,
    entries: &[ArchiveEntry],
    icons: &IconTheme,
) -> (Vec<TreeNode>, u64) {
    let mut out = Vec::new();
    let mut held = 0;
    let mut folders: Vec<_> = folder.folders.iter().collect();
    folders.sort_by_key(|(name, _)| name.to_lowercase());
    for (name, inner) in folders {
        let path = format!("{prefix}{name}");
        let (children, bytes) = nodes(inner, &format!("{path}/"), entries, icons);
        held += bytes;
        let node = TreeNode::new(path, name.clone())
            .icon(IconName::Folder)
            .note(format::file_size(bytes, false))
            .children(children);
        out.push(node);
    }
    let mut files: Vec<&String> = folder.files.iter().collect();
    files.sort_by_key(|name| name.to_lowercase());
    for name in files {
        let path = format!("{prefix}{name}");
        let entry = entries
            .iter()
            .find(|entry| entry.path.as_ref() == path)
            .expect("a listed entry");
        held += entry.size;
        let note = format!(
            "{} · {}%",
            format::file_size(entry.size, false),
            saving(entry.size, entry.packed)
        );
        out.push(
            TreeNode::new(path, name.clone())
                .icon(icons.file(name))
                .note(note),
        );
    }
    (out, held)
}

/// What an archive holds, as a tree of its folders: each folder with its size, each file with its size and how much packing saved; the archive's name, its count, its size and its saving above. Enter or a double press on a file hands it to the host. Give it a height.
#[derive(IntoElement)]
pub struct ArchiveViewer {
    id: ElementId,
    name: SharedString,
    entries: Vec<ArchiveEntry>,
    on_open: Option<OnArchived>,
}

impl ArchiveViewer {
    pub fn new(
        id: impl Into<ElementId>,
        name: impl Into<SharedString>,
        entries: impl IntoIterator<Item = ArchiveEntry>,
    ) -> Self {
        let entries: Vec<ArchiveEntry> = entries.into_iter().collect();
        for (ix, entry) in entries.iter().enumerate() {
            assert!(
                !entries[..ix].iter().any(|other| other.path == entry.path),
                "{} twice",
                entry.path
            );
        }
        Self {
            id: id.into(),
            name: name.into(),
            entries,
            on_open: None,
        }
    }

    pub fn on_open(
        mut self,
        handler: impl Fn(&ArchiveEntry, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_open = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for ArchiveViewer {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = &theme.colors;
        let folders = Folder::of(self.entries.iter().map(|entry| entry.path.as_ref()));
        let (nodes, size) = nodes(&folders, "", &self.entries, icon_theme(cx));
        let packed: u64 = self.entries.iter().map(|entry| entry.packed).sum();
        let top: Vec<SharedString> = folders
            .folders
            .keys()
            .map(|name| name.clone().into())
            .collect();
        let summary = format!(
            "{} · {} · {}% smaller",
            match self.entries.len() {
                1 => "1 file".to_string(),
                count => format!("{count} files"),
            },
            format::file_size(size, false),
            saving(size, packed)
        );
        let (id, entries, on_open) = (self.id.clone(), Rc::new(self.entries), self.on_open);
        let tree = Tree::new((self.id.clone(), "tree"), nodes)
            .open(top)
            .flex_1()
            .min_h_0()
            .on_activate(move |key, window, cx| {
                let entry = entries
                    .iter()
                    .find(|entry| entry.path == *key)
                    .expect("a leaf is a listed file");
                log::info!("archive viewer {id:?}: open {}", entry.path);
                if let Some(on_open) = &on_open {
                    on_open(entry, window, cx);
                }
            });
        div()
            .debug_selector(|| "archive-viewer".into())
            .size_full()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(FileIcon::file(&self.name).size(IconSize::Xl))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(colors.fg)
                                    .child(MiddleEllipsis::new(self.name)),
                            )
                            .child(
                                div()
                                    .text_size(theme.text_size(TextSize::Sm))
                                    .text_color(colors.fg_muted)
                                    .child(summary),
                            ),
                    ),
            )
            .child(tree)
    }
}
