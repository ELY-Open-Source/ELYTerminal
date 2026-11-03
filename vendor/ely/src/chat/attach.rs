use std::rc::Rc;

use gpui::{
    App, ElementId, FontWeight, IntoElement, ObjectFit, ParentElement, RenderOnce, SharedString,
    Styled, Window, div, relative,
};

use crate::{
    buttons::{ButtonVariant, IconButton},
    data_display::Tag,
    documents::source,
    forms::{Pick, Run},
    primitives::{Icon, IconName, Image, file_icon},
    theme::{ActiveTheme, AvatarSize, ControlSize, IconSize, Radius, TextSize},
    typography::{Ellipsis, format::file_size},
};

/// Something attached to a message: its key, name and size, a picture's preview, and how far its upload has come, 0 to 1, while it goes.
#[derive(Clone, Debug, PartialEq)]
pub struct Attachment {
    pub key: SharedString,
    pub name: SharedString,
    pub bytes: u64,
    pub preview: Option<SharedString>,
    pub progress: Option<f32>,
}

/// An attachment: a picture's thumbnail or the file's icon, its name, its size or how far it has gone up, and a way to take it off.
#[derive(IntoElement)]
pub struct AttachmentChip {
    id: ElementId,
    attachment: Attachment,
    on_remove: Option<Run>,
}

impl AttachmentChip {
    pub fn new(id: impl Into<ElementId>, attachment: Attachment) -> Self {
        assert!(
            attachment
                .progress
                .is_none_or(|share| (0.0..=1.0).contains(&share)),
            "an upload of {:?} lies outside 0 to 1",
            attachment.progress
        );
        Self {
            id: id.into(),
            attachment,
            on_remove: None,
        }
    }

    pub fn on_remove(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_remove = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for AttachmentChip {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let side = theme.avatar_size(AvatarSize::Md);
        let item = self.attachment;
        let face = match &item.preview {
            Some(preview) => div()
                .size(side)
                .rounded(theme.radius(Radius::Sm))
                .bg(colors.sunken)
                .child(
                    Image::new((self.id.clone(), "preview"), source(preview))
                        .fit(ObjectFit::Cover)
                        .rounded(theme.radius(Radius::Sm))
                        .size_full(),
                ),
            None => div()
                .size(side)
                .flex()
                .items_center()
                .justify_center()
                .rounded(theme.radius(Radius::Sm))
                .bg(colors.hover)
                .child(
                    Icon::new(file_icon(&item.name, cx))
                        .size(IconSize::Md)
                        .color(colors.fg_muted),
                ),
        };
        let detail = match item.progress {
            Some(share) => div()
                .w_full()
                .h(theme.progress_thickness())
                .rounded_full()
                .bg(colors.hover)
                .child(
                    div()
                        .h_full()
                        .w(relative(share))
                        .rounded_full()
                        .bg(colors.accent),
                )
                .into_any_element(),
            None => div()
                .text_size(theme.text_size(TextSize::Xs))
                .text_color(colors.fg_subtle)
                .child(file_size(item.bytes, false))
                .into_any_element(),
        };
        div()
            .min_w_0()
            .max_w(theme.label_width() * 1.4)
            .flex()
            .items_center()
            .gap_2()
            .p_1p5()
            .rounded(theme.radius(Radius::Md))
            .border_1()
            .border_color(colors.border)
            .bg(colors.surface)
            .child(face)
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .text_size(theme.text_size(TextSize::Sm))
                            .font_weight(FontWeight::MEDIUM)
                            .child(Ellipsis::new(item.name.clone())),
                    )
                    .child(detail),
            )
            .children(self.on_remove.map(|remove| {
                IconButton::new((self.id.clone(), "remove"), IconName::X)
                    .variant(ButtonVariant::Ghost)
                    .size(ControlSize::Sm)
                    .tooltip("Remove")
                    .on_click(move |_, window, cx| {
                        log::info!("attachment: removed");
                        remove(window, cx)
                    })
            }))
    }
}

/// Context given to a message, such as files, pages and a selection: each a tag with its icon, to take off.
#[derive(IntoElement)]
pub struct ContextChips {
    id: ElementId,
    items: Vec<(SharedString, IconName)>,
    on_remove: Option<Pick>,
}

impl ContextChips {
    pub fn new(
        id: impl Into<ElementId>,
        items: impl IntoIterator<Item = (impl Into<SharedString>, IconName)>,
    ) -> Self {
        Self {
            id: id.into(),
            items: items
                .into_iter()
                .map(|(label, icon)| (label.into(), icon))
                .collect(),
            on_remove: None,
        }
    }

    /// Gets the index of the context to take off.
    pub fn on_remove(mut self, handler: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_remove = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for ContextChips {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div()
            .flex()
            .flex_wrap()
            .gap_1()
            .children(
                self.items
                    .into_iter()
                    .enumerate()
                    .map(|(ix, (label, icon))| {
                        let tag =
                            Tag::new((self.id.clone(), format!("context-{ix}")), label).icon(icon);
                        match self.on_remove.clone() {
                            Some(remove) => tag.on_remove(move |window, cx| {
                                log::info!("context chips: removed {ix}");
                                remove(ix, window, cx)
                            }),
                            None => tag,
                        }
                    }),
            )
    }
}
