//! The question before a read: who, for what, getting what.

use std::sync::Arc;

use ely_gpui_component::{
    buttons::{Button, ButtonVariant},
    i18n::text,
    overlays::Dialog,
    theme::{ActiveTheme, TextSize},
    typography::Caption,
};
use gpui::{
    App, ClipboardEntry, Context, Div, ObjectFit, ParentElement, Styled, StyledImage, div, img, px,
};

use crate::{sessions::plain, term::view::TerminalView};

/// The lines of text and the files a preview shows.
const SHOWN: usize = 8;

impl TerminalView {
    /// The read asking, over the window; Escape denies it.
    pub(in crate::term) fn clipboard_prompt(&self, cx: &Context<Self>) -> Option<Dialog> {
        let asking = self.clips.asking.as_ref()?;
        let types = asking.read.mimes.join(" ");
        let detail = match &asking.read.pass {
            Some(pass) => text(
                cx,
                "et.clipboard.named",
                &[("name", &plain(&pass.name)), ("types", &types)],
            ),
            None => text(cx, "et.clipboard.unnamed", &[("types", &types)]),
        };
        let view = cx.entity();
        let decide = move |allow, always| {
            let view = view.clone();
            move |cx: &mut App| view.update(cx, |view, cx| view.decide(allow, always, cx))
        };
        let closed = decide(false, false);
        let mut dialog = Dialog::new(
            "clipboard-read",
            text(cx, "et.clipboard.title", &[]),
            move |_, cx| closed(cx),
        )
        .detail(detail)
        .child(preview(asking.item.as_ref(), cx));
        let button = |id: &'static str, key: &str, allow, always, variant| {
            let label = text(cx, key, &[]);
            let pick = decide(allow, always);
            move |close: ely_gpui_component::overlays::Close| {
                Button::new(id, label)
                    .variant(variant)
                    .on_click(move |_, window, cx| {
                        pick(cx);
                        close(window, cx);
                    })
            }
        };
        dialog = dialog.action(button(
            "clipboard-deny",
            "et.clipboard.deny",
            false,
            false,
            ButtonVariant::Secondary,
        ));
        if asking.read.pass.is_some() {
            dialog = dialog.action(button(
                "clipboard-always",
                "et.clipboard.always",
                true,
                true,
                ButtonVariant::Secondary,
            ));
        }
        Some(dialog.action(button(
            "clipboard-allow",
            "et.clipboard.allow",
            true,
            false,
            ButtonVariant::Primary,
        )))
    }
}

/// The clipboard's first lines of text, its image, its files.
fn preview(item: Option<&gpui::ClipboardItem>, cx: &App) -> Div {
    let theme = cx.theme();
    let column = div()
        .flex()
        .flex_col()
        .gap_1()
        .text_size(theme.text_size(TextSize::Sm))
        .text_color(theme.colors.fg);
    let Some(item) = item else {
        return column.child(Caption::new(text(cx, "et.clipboard.empty", &[])));
    };
    item.entries
        .iter()
        .fold(column, |column, entry| match entry {
            ClipboardEntry::String(string) => {
                let lines = string
                    .text()
                    .lines()
                    .take(SHOWN)
                    .map(|line| div().child(plain(line)));
                column.child(div().flex().flex_col().children(lines))
            }
            ClipboardEntry::Image(image) => column.child(
                img(Arc::new(image.clone()))
                    .max_w(px(320.))
                    .max_h(px(200.))
                    .object_fit(ObjectFit::Contain),
            ),
            ClipboardEntry::ExternalPaths(paths) => {
                let paths = paths.paths();
                let names = paths
                    .iter()
                    .take(SHOWN)
                    .map(|path| div().child(plain(&path.display().to_string())));
                let more = paths.len().saturating_sub(SHOWN);
                let rest = (more > 0).then(|| {
                    Caption::new(text(
                        cx,
                        "et.clipboard.more_files",
                        &[("count", &more.to_string())],
                    ))
                });
                column.children(names).children(rest)
            }
        })
}
