use std::rc::Rc;

use gpui::{
    App, ElementId, ImageSource, InteractiveElement, IntoElement, MouseButton, ParentElement,
    RenderOnce, SharedString, StatefulInteractiveElement, Styled, Window, div, prelude::*,
    transparent_black,
};

use crate::{
    forms::Run,
    primitives::{FocusRing, Image, checked_ratio, framed},
    theme::{ActiveTheme, Radius, TextSize},
    typography::tabular,
};

/// A picture in its own shape, taken from the host, as wide as its box: a shimmer while it loads, a mark if it fails, and a short label in a corner when given, such as a length or a count. Chosen, an accent ring holds it; with `on_click` it is a Tab stop.
#[derive(IntoElement)]
pub struct ImageThumbnail {
    id: ElementId,
    source: ImageSource,
    ratio: f32,
    label: Option<SharedString>,
    selected: bool,
    on_click: Option<Run>,
}

impl ImageThumbnail {
    /// `ratio` is the picture's width over its height.
    pub fn new(id: impl Into<ElementId>, source: impl Into<ImageSource>, ratio: f32) -> Self {
        Self {
            id: id.into(),
            source: source.into(),
            ratio: checked_ratio(ratio),
            label: None,
            selected: false,
            on_click: None,
        }
    }

    /// A few words in the corner, such as "0:42" or "+3".
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn on_click(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for ImageThumbnail {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = &theme.colors;
        let (outer, inner) = (theme.radius(Radius::Md), theme.radius(Radius::Sm));
        let label = self.label.map(|label| {
            tabular(div())
                .absolute()
                .right_1()
                .bottom_1()
                .px_1p5()
                .rounded(inner)
                .bg(colors.media_backdrop.alpha(0.7))
                .text_size(theme.text_size(TextSize::Xs))
                .text_color(colors.on_media)
                .child(label)
        });
        let picture = framed(self.ratio, cx)
            .rounded(inner)
            .child(
                Image::new((self.id.clone(), "picture"), self.source)
                    .size_full()
                    .rounded(inner),
            )
            .children(label);
        div()
            .id(self.id)
            .rounded(outer)
            .border_1()
            .border_color(if self.selected {
                colors.accent
            } else {
                transparent_black()
            })
            .p_0p5()
            .when_some(self.on_click, |thumb, click| {
                thumb
                    .tab_index(0)
                    .focus_ring(cx)
                    .cursor_pointer()
                    .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
                    .on_click(move |_, window, cx| {
                        log::info!("image thumbnail: opened");
                        click(window, cx)
                    })
            })
            .child(picture)
    }
}
