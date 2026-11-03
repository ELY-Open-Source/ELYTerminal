use gpui::{App, IntoElement, RenderOnce, SharedString, Window};

use crate::{
    lists::DirEntry,
    primitives::{Icon, IconName, file_icon},
    theme::{ActiveTheme, IconSize},
};

/// A file's name, its icon found under the icon theme at render, or an icon itself.
enum Shown {
    File(SharedString),
    Icon(IconName),
}

/// A file's icon by its name under the owner's icon theme, or a folder's, open or shut.
#[derive(IntoElement)]
pub struct FileIcon {
    shown: Shown,
    size: IconSize,
}

impl FileIcon {
    pub fn file(name: impl AsRef<str>) -> Self {
        Self {
            shown: Shown::File(name.as_ref().to_string().into()),
            size: IconSize::Md,
        }
    }

    pub fn folder(open: bool) -> Self {
        Self {
            shown: Shown::Icon(if open {
                IconName::FolderOpen
            } else {
                IconName::Folder
            }),
            size: IconSize::Md,
        }
    }

    /// The icon for a listing's entry: a shut folder, or the file's by its name.
    pub fn entry(entry: &DirEntry) -> Self {
        match entry.is_folder() {
            true => Self::folder(false),
            false => Self::file(entry.name()),
        }
    }

    pub fn size(mut self, size: IconSize) -> Self {
        self.size = size;
        self
    }
}

impl RenderOnce for FileIcon {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let icon = match self.shown {
            Shown::File(name) => file_icon(&name, cx),
            Shown::Icon(icon) => icon,
        };
        Icon::new(icon)
            .size(self.size)
            .color(cx.theme().colors.fg_muted)
    }
}
