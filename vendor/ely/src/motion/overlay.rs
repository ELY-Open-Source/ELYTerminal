use std::f32::consts::PI;

use gpui::{
    Animation, AnimationExt, AnyElement, App, Div, ElementId, InteractiveElement, IntoElement,
    Length, ParentElement, RenderOnce, SharedString, StyleRefinement, Styled, Window, div,
    prelude::*, radians,
};
use smallvec::SmallVec;
use web_time::Instant;

use super::{BASE, FAST, NUDGE, Skeleton, Spinner, changes, duration, ease_out_cubic};
use crate::{
    primitives::{Icon, IconName, IntersectionObserver},
    theme::{ActiveTheme, IconSize, TextSize},
};

/// Veils what it holds with a spinner while `loading`; presses stop at the veil. It fades in and out.
#[derive(IntoElement)]
pub struct LoadingOverlay {
    id: ElementId,
    base: Div,
    loading: bool,
    label: Option<SharedString>,
    body: SmallVec<[AnyElement; 2]>,
}

impl LoadingOverlay {
    pub fn new(id: impl Into<ElementId>, loading: bool) -> Self {
        Self {
            id: id.into(),
            base: div(),
            loading,
            label: None,
            body: SmallVec::new(),
        }
    }

    pub fn label(mut self, text: impl Into<SharedString>) -> Self {
        self.label = Some(text.into());
        self
    }
}

impl Styled for LoadingOverlay {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl ParentElement for LoadingOverlay {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.body.extend(elements);
    }
}

impl RenderOnce for LoadingOverlay {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let loading = self.loading;
        let turn = changes((self.id.clone(), "loading"), loading, window, cx);
        let fade = duration(FAST, cx);
        let since = window.use_keyed_state((self.id.clone(), "since"), cx, |_, _| {
            (loading, Instant::now())
        });
        if since.read(cx).0 != loading {
            since.update(cx, |since, _| *since = (loading, Instant::now()));
        }
        let fading = !loading && turn > 0 && since.read(cx).1.elapsed() < fade;
        let radii = self.base.style().corner_radii.clone();
        let theme = cx.theme();
        let veil = div()
            .id((self.id.clone(), "veil"))
            .absolute()
            .inset_0()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_2()
            .bg(theme.colors.bg.opacity(0.72))
            .text_size(theme.text_size(TextSize::Sm))
            .text_color(theme.colors.fg_muted)
            .when(loading, |veil| veil.occlude())
            .child(Spinner::new((self.id.clone(), "spinner")).size(IconSize::Lg))
            .children(self.label)
            .map(|mut veil| {
                veil.style().corner_radii = radii;
                veil
            })
            .with_animation(
                (self.id.clone(), format!("veil-{turn}")),
                Animation::new(fade).with_easing(ease_out_cubic),
                move |veil, t| match (turn, loading) {
                    (0, _) => veil,
                    (_, true) => veil.opacity(t),
                    (_, false) => veil.opacity(1.0 - t),
                },
            );
        self.base
            .id(self.id)
            .relative()
            .children(self.body)
            .when(loading || fading, |host| host.child(veil))
    }
}

type Build = Box<dyn FnOnce(&mut Window, &mut App) -> AnyElement>;

/// Builds its content once it first scrolls into view; until then a skeleton holds its place.
#[derive(IntoElement)]
pub struct LazyLoad {
    id: ElementId,
    height: Length,
    build: Build,
}

impl LazyLoad {
    /// `height` is the placeholder's, before the content is built.
    pub fn new<E: IntoElement>(
        id: impl Into<ElementId>,
        height: impl Into<Length>,
        build: impl FnOnce(&mut Window, &mut App) -> E + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            height: height.into(),
            build: Box::new(move |window, cx| build(window, cx).into_any_element()),
        }
    }
}

impl RenderOnce for LazyLoad {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let seen = window.use_keyed_state((self.id.clone(), "seen"), cx, |_, _| false);
        if *seen.read(cx) {
            return (self.build)(window, cx);
        }
        let id = self.id.clone();
        IntersectionObserver::new((self.id.clone(), "watch"), move |visible, _, cx| {
            if visible {
                log::debug!("lazy load {id:?}: in view, building");
                seen.update(cx, |seen, cx| {
                    *seen = true;
                    cx.notify();
                });
            }
        })
        .child(
            Skeleton::new((self.id, "placeholder"))
                .w_full()
                .h(self.height),
        )
        .into_any_element()
    }
}

/// Where a pull to refresh stands.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Refresh {
    Idle,
    /// Pulled this far toward the point where it lets go, 0 to 1.
    Pulling(f32),
    Refreshing,
    Done,
}

/// The mark at the top of a list that refreshes: an arrow that turns with the pull, a spinner while it works, a check when done.
#[derive(IntoElement)]
pub struct RefreshIndicator {
    id: ElementId,
    state: Refresh,
}

impl RefreshIndicator {
    pub fn new(id: impl Into<ElementId>, state: Refresh) -> Self {
        if let Refresh::Pulling(pull) = state {
            assert!((0.0..=1.0).contains(&pull), "pull {pull} is not 0..=1");
        }
        Self {
            id: id.into(),
            state,
        }
    }
}

impl RenderOnce for RefreshIndicator {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let phase = std::mem::discriminant(&self.state);
        let turn = changes((self.id.clone(), "phase"), phase, window, cx);
        let theme = cx.theme();
        let muted = theme.colors.fg_muted;
        let mark = match self.state {
            Refresh::Idle => return div().id(self.id).into_any_element(),
            Refresh::Pulling(pull) => Icon::new(IconName::RefreshCw)
                .size(IconSize::Md)
                .color(muted.opacity(0.35 + 0.65 * pull))
                .rotate(radians(PI * 1.5 * pull))
                .into_any_element(),
            Refresh::Refreshing => Spinner::new((self.id.clone(), "spinner")).into_any_element(),
            Refresh::Done => div()
                .flex()
                .items_center()
                .gap_1p5()
                .text_size(theme.text_size(TextSize::Sm))
                .text_color(muted)
                .child(Icon::new(IconName::Check).size(IconSize::Sm).color(muted))
                .child("Up to date")
                .into_any_element(),
        };
        div()
            .id(self.id.clone())
            .flex()
            .justify_center()
            .py_2()
            .child(mark)
            .with_animation(
                (self.id, format!("phase-{turn}")),
                Animation::new(duration(BASE, cx)).with_easing(ease_out_cubic),
                move |row, t| {
                    if turn == 0 {
                        row
                    } else {
                        row.opacity(t).mt(NUDGE * (t - 1.0))
                    }
                },
            )
            .into_any_element()
    }
}

/// Someone is typing: three dots bob in a small bubble, beside who it is when named.
#[derive(IntoElement)]
pub struct TypingIndicator {
    id: ElementId,
    who: Option<SharedString>,
}

impl TypingIndicator {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            who: None,
        }
    }

    /// Names who is typing, as "Mia is typing…".
    pub fn who(mut self, name: impl Into<SharedString>) -> Self {
        self.who = Some(name.into());
        self
    }
}

impl RenderOnce for TypingIndicator {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let muted = theme.colors.fg_muted;
        div()
            .flex()
            .items_center()
            .gap_2()
            .child(
                div()
                    .px_3()
                    .py_2()
                    .rounded_full()
                    .bg(theme.colors.sunken)
                    .child(
                        Spinner::new(self.id)
                            .style(super::SpinnerStyle::Wave)
                            .size(IconSize::Lg)
                            .color(muted),
                    ),
            )
            .when_some(self.who, |row, who| {
                row.child(
                    div()
                        .text_size(theme.text_size(TextSize::Sm))
                        .text_color(muted)
                        .child(format!("{who} is typing…")),
                )
            })
    }
}
