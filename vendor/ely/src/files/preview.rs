use gpui::{
    AnyElement, App, ElementId, FontWeight, ImageSource, IntoElement, ParentElement, RenderOnce,
    SharedString, Styled, Window, div, prelude::*, relative,
};
use jiff::Timestamp;

use super::FileIcon;
use crate::{
    lists::DirEntry,
    primitives::{Image, checked_ratio, framed},
    theme::{ActiveTheme, IconSize, Radius, TextSize},
    typography::{Ellipsis, LEADING, MiddleEllipsis, format},
};

/// Lines of a text a preview shows.
const LINES: usize = 12;
/// The shape of the well an icon sits in, wide over high.
const WELL: f32 = 4.0 / 3.0;

/// What a preview shows of a file.
enum Look {
    Picture(ImageSource, f32),
    Text(SharedString),
    Icon,
}

/// A file's kind, from its extension: "PDF", "File" without one, "Folder" for a folder.
pub(crate) fn kind(entry: &DirEntry) -> String {
    if entry.is_folder() {
        return "Folder".into();
    }
    match entry.name().rsplit_once('.') {
        Some((_, extension)) if !extension.is_empty() => extension.to_uppercase(),
        _ => "File".into(),
    }
}

/// A file shown large, as a quick look shows it: its picture in the host's shape, the first lines of its text, or its kind's icon; its name, its kind and size, and when it changed below.
#[derive(IntoElement)]
pub struct FilePreview {
    id: ElementId,
    entry: DirEntry,
    look: Look,
}

impl FilePreview {
    pub fn new(id: impl Into<ElementId>, entry: DirEntry) -> Self {
        Self {
            id: id.into(),
            entry,
            look: Look::Icon,
        }
    }

    /// Its picture, `ratio` wide over high.
    pub fn picture(mut self, source: impl Into<ImageSource>, ratio: f32) -> Self {
        self.look = Look::Picture(source.into(), checked_ratio(ratio));
        self
    }

    /// Its text; the first lines show.
    pub fn text(mut self, text: impl Into<SharedString>) -> Self {
        self.look = Look::Text(text.into());
        self
    }
}

impl RenderOnce for FilePreview {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = &theme.colors;
        let round = theme.radius(Radius::Lg);
        let stage: AnyElement = match self.look {
            Look::Picture(source, ratio) => framed(ratio, cx)
                .debug_selector(|| "file-preview-stage".into())
                .rounded(round)
                .child(
                    Image::new((self.id.clone(), "picture"), source)
                        .size_full()
                        .rounded(round),
                )
                .into_any_element(),
            Look::Text(text) => div()
                .w_full()
                .p_4()
                .rounded(round)
                .bg(colors.sunken)
                .font_family(theme.mono_family.clone())
                .text_size(theme.text_size(TextSize::Sm))
                .line_height(relative(LEADING))
                .text_color(colors.fg)
                .children(text.lines().take(LINES).map(|line| {
                    let shown = if line.is_empty() { " " } else { line };
                    Ellipsis::new(shown.to_string())
                }))
                .into_any_element(),
            Look::Icon => framed(WELL, cx)
                .rounded(round)
                .flex()
                .items_center()
                .justify_center()
                .child(FileIcon::entry(&self.entry).size(IconSize::Xxl))
                .into_any_element(),
        };
        let detail = match self.entry.size() {
            Some(bytes) => format!(
                "{} · {}",
                kind(&self.entry),
                format::file_size(bytes, false)
            ),
            None => kind(&self.entry),
        };
        let changed = format!(
            "Changed {}",
            format::relative(self.entry.modified(), Timestamp::now())
        );
        let muted = |text: String| {
            div()
                .text_size(theme.text_size(TextSize::Sm))
                .text_color(colors.fg_muted)
                .child(text)
        };
        div()
            .debug_selector(|| "file-preview".into())
            .flex()
            .flex_col()
            .gap_3()
            .child(stage)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_0p5()
                    .child(
                        div()
                            .text_size(theme.text_size(TextSize::Md))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(colors.fg)
                            .child(MiddleEllipsis::new(self.entry.name().clone())),
                    )
                    .child(muted(detail))
                    .child(muted(changed)),
            )
    }
}
