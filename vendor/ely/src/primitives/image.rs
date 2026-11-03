use std::time::Duration;

use gpui::{
    Animation, AnimationExt, App, Div, ElementId, ImageSource, InteractiveElement, IntoElement,
    ObjectFit, ParentElement, RenderOnce, StyleRefinement, Styled, StyledImage, Window, div, img,
    pulsating_between,
};

use crate::{
    primitives::{Icon, IconName},
    theme::{ActiveTheme, IconSize},
};

const PULSE: Duration = Duration::from_millis(1600);

/// A host's width over height, fail-fast when it is not a positive number.
pub(crate) fn checked_ratio(ratio: f32) -> f32 {
    assert!(
        ratio.is_finite() && ratio > 0.0,
        "a picture ratio of {ratio}"
    );
    ratio
}

/// A box in a picture's shape at the width it is given, sunken until the picture draws.
pub(crate) fn framed(ratio: f32, cx: &App) -> Div {
    let mut frame = div()
        .relative()
        .w_full()
        .overflow_hidden()
        .bg(cx.theme().colors.sunken);
    frame.style().aspect_ratio = Some(ratio);
    frame
}

/// Themed `img()` that fills the box it is given.
#[derive(IntoElement)]
pub struct Image {
    id: ElementId,
    base: Div,
    source: Option<ImageSource>,
    fit: ObjectFit,
}

impl Image {
    /// The id keeps gpui's loading state across frames.
    pub fn new(id: impl Into<ElementId>, source: impl Into<ImageSource>) -> Self {
        Self {
            id: id.into(),
            base: div(),
            source: Some(source.into()),
            fit: ObjectFit::Cover,
        }
    }

    /// Nothing to load yet.
    pub fn placeholder(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            base: div(),
            source: None,
            fit: ObjectFit::Cover,
        }
    }

    pub fn fit(mut self, fit: ObjectFit) -> Self {
        self.fit = fit;
        self
    }
}

impl Styled for Image {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for Image {
    fn render(mut self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let (sunken, subtle) = (theme.colors.sunken, theme.colors.fg_subtle);
        let still = theme.reduced_motion;
        let radii = self.base.style().corner_radii.clone();
        let pane = {
            let radii = radii.clone();
            move || {
                let mut pane = div().size_full().bg(sunken);
                pane.style().corner_radii = radii.clone();
                pane
            }
        };
        let loading_pane = pane.clone();
        let mark = move |name| {
            pane()
                .flex()
                .items_center()
                .justify_center()
                .child(Icon::new(name).size(IconSize::Lg).color(subtle))
        };
        let Some(source) = self.source else {
            return self.base.overflow_hidden().child(mark(IconName::Image));
        };
        let mut picture = img(source)
            .id(self.id)
            .absolute()
            .inset_0()
            .size_full()
            .object_fit(self.fit)
            .with_loading(move || {
                if still {
                    return loading_pane().into_any_element();
                }
                loading_pane()
                    .with_animation(
                        "image-loading",
                        Animation::new(PULSE)
                            .repeat()
                            .with_easing(pulsating_between(0.55, 1.0)),
                        |pane, t| pane.opacity(t),
                    )
                    .into_any_element()
            })
            .with_fallback(move || mark(IconName::ImageOff).into_any_element());
        picture.style().corner_radii = radii;
        self.base.relative().overflow_hidden().child(picture)
    }
}
