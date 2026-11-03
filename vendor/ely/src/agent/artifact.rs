use std::rc::Rc;

use gpui::{
    AnyElement, App, ElementId, FontWeight, InteractiveElement, IntoElement, ParentElement,
    RenderOnce, SharedString, StatefulInteractiveElement, Styled, Window, div, prelude::*,
};

use crate::{
    buttons::{ButtonVariant, IconButton, SegmentedControl},
    chat::BranchNavigator,
    forms::{Pick, Run},
    primitives::{Icon, IconName},
    theme::{ActiveTheme, ControlSize, IconSize, Radius, TextSize},
    typography::Ellipsis,
};

/// What an agent made, in a panel of its own: its title, the version shown among all, copy and download, and the artifact, as preview or source when it has both.
#[derive(IntoElement)]
pub struct ArtifactPanel {
    id: ElementId,
    title: SharedString,
    icon: IconName,
    preview: Option<AnyElement>,
    source: Option<AnyElement>,
    version: Option<(usize, usize, Pick)>,
    on_copy: Option<Run>,
    on_download: Option<Run>,
}

impl ArtifactPanel {
    pub fn new(id: impl Into<ElementId>, title: impl Into<SharedString>, icon: IconName) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            icon,
            preview: None,
            source: None,
            version: None,
            on_copy: None,
            on_download: None,
        }
    }

    /// The artifact as it shows, such as a page or a document.
    pub fn preview(mut self, element: impl IntoElement) -> Self {
        self.preview = Some(element.into_any_element());
        self
    }

    /// The artifact as written, its code or markup.
    pub fn source(mut self, element: impl IntoElement) -> Self {
        self.source = Some(element.into_any_element());
        self
    }

    /// The version shown, of all; `on_version` gets the one asked for.
    pub fn version(
        mut self,
        current: usize,
        total: usize,
        on_version: impl Fn(usize, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.version = Some((current, total, Rc::new(on_version)));
        self
    }

    pub fn on_copy(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_copy = Some(Rc::new(handler));
        self
    }

    pub fn on_download(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_download = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for ArtifactPanel {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        assert!(
            self.preview.is_some() || self.source.is_some(),
            "artifact {:?} shows neither preview nor source",
            self.id
        );
        let reading = window.use_keyed_state((self.id.clone(), "source"), cx, |_, _| false);
        let both = self.preview.is_some() && self.source.is_some();
        let showing_source = self.preview.is_none() || (both && *reading.read(cx));
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let action = |key: &'static str, icon: IconName, tip: &'static str, run: Run| {
            IconButton::new((self.id.clone(), key), icon)
                .variant(ButtonVariant::Ghost)
                .size(ControlSize::Sm)
                .tooltip(tip)
                .on_click(move |_, window, cx| {
                    log::info!("artifact: {tip}");
                    run(window, cx)
                })
        };
        let view = both.then(|| {
            SegmentedControl::new(
                (self.id.clone(), "view"),
                if showing_source { "source" } else { "preview" },
            )
            .size(ControlSize::Sm)
            .segment("preview", "Preview", Some(IconName::Eye))
            .segment("source", "Source", Some(IconName::Code))
            .on_change(move |key, _, cx| {
                let source = key.as_ref() == "source";
                log::info!("artifact: shows {key}");
                reading.update(cx, |reading, cx| {
                    *reading = source;
                    cx.notify();
                })
            })
        });
        let version = self.version.map(|(current, total, pick)| {
            BranchNavigator::new(
                (self.id.clone(), "versions"),
                current,
                total,
                move |to, window, cx| pick(to, window, cx),
            )
            .into_any_element()
        });
        let copy = (self.on_copy)
            .map(|copy| action("copy", IconName::Copy, "Copy", copy).into_any_element());
        let save = (self.on_download).map(|save| {
            action("download", IconName::Download, "Download", save).into_any_element()
        });
        let controls: Vec<AnyElement> =
            [version, view.map(IntoElement::into_any_element), copy, save]
                .into_iter()
                .flatten()
                .collect();
        let body = if showing_source {
            self.source
        } else {
            self.preview
        };
        div()
            .size_full()
            .flex()
            .flex_col()
            .rounded(theme.radius(Radius::Lg))
            .border_1()
            .border_color(colors.border)
            .bg(colors.surface)
            .child(
                div()
                    .flex_none()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap_x_2()
                    .gap_y_1()
                    .px_3()
                    .py_2()
                    .border_b_1()
                    .border_color(colors.border)
                    .text_size(theme.text_size(TextSize::Sm))
                    .child(
                        div()
                            .flex_1()
                            .min_w(theme.label_width())
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                Icon::new(self.icon)
                                    .size(IconSize::Sm)
                                    .color(colors.fg_muted),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(Ellipsis::new(self.title)),
                            ),
                    )
                    .when(!controls.is_empty(), |header| {
                        header.child(
                            div()
                                .flex()
                                .flex_wrap()
                                .items_center()
                                .gap_1()
                                .children(controls),
                        )
                    }),
            )
            .child(
                div()
                    .id((self.id.clone(), "body"))
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .children(body),
            )
    }
}
