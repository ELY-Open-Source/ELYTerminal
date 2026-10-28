use std::path::{Path, PathBuf};

use ely_gpui_component::{
    i18n::text,
    menus::{Menu, MenuItem},
    primitives::IconName,
};
use gpui::{App, ClipboardItem, Entity};

use crate::workspace::Workspace;

/// The path as text when it is UTF-8; none when a copy would invent a name.
pub(super) fn copy_text(path: &Path) -> Option<&str> {
    path.to_str()
}

/// What a right press on Files' `path` offers; `here` is the folder a session starts in.
pub fn row(workspace: &Entity<Workspace>, path: PathBuf, here: PathBuf, cx: &App) -> Menu {
    let (open, host, copy) = (path.clone(), workspace.clone(), path);
    Menu::new()
        .item(
            MenuItem::new(text(cx, "et.files.open", &[]))
                .icon(IconName::ExternalLink)
                .on_click(move |_, cx| {
                    log::info!("files: opening {}", open.display());
                    cx.open_with_system(&open);
                }),
        )
        .item(
            MenuItem::new(text(cx, "et.files.new_session", &[]))
                .icon(IconName::Terminal)
                .on_click(move |window, cx| {
                    host.update(cx, |this, cx| this.here(here.clone(), window, cx))
                }),
        )
        .separator()
        .item(
            MenuItem::new(text(cx, "et.files.copy_path", &[]))
                .icon(IconName::Copy)
                .on_click(move |_, cx| match copy_text(&copy) {
                    Some(text) => {
                        log::info!("files: copied {text}");
                        cx.write_to_clipboard(ClipboardItem::new_string(text.to_string()));
                    }
                    None => log::error!("files: {} is no UTF-8 to copy", copy.display()),
                }),
        )
}
