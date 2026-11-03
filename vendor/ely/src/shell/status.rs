use std::{rc::Rc, time::Duration};

use gpui::{
    Animation, AnimationExt, AnyElement, App, ElementId, IntoElement, ParentElement, RenderOnce,
    Styled, Task, Window, div, prelude::*,
};
use web_time::Instant;

use crate::{
    buttons::{ButtonVariant, IconButton},
    motion,
    primitives::{Icon, IconName},
    theme::{ActiveTheme, ControlSize, IconSize, TextSize},
    typography::AnimatedNumber,
};

/// Network state as the host sees it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Connectivity {
    Online,
    Offline,
    Reconnecting,
}

/// How long "Back online" stays.
const HOLD: Duration = Duration::from_millis(2400);

struct Seen {
    last: Connectivity,
    back_at: Option<Instant>,
    generation: u64,
    _hide: Option<Task<()>>,
}

/// The state's dot, in its color; it breathes while reconnecting.
pub(crate) fn connectivity_dot(
    id: impl Into<ElementId>,
    state: Connectivity,
    cx: &App,
) -> AnyElement {
    let theme = cx.theme();
    let colors = &theme.colors;
    let tone = match state {
        Connectivity::Online => colors.success,
        Connectivity::Reconnecting => colors.warning,
        Connectivity::Offline => colors.fg_subtle,
    };
    let dot = div().size(theme.status_dot()).rounded_full().bg(tone);
    if state != Connectivity::Reconnecting || theme.reduced_motion {
        return dot.into_any_element();
    }
    dot.with_animation(
        id,
        Animation::new(motion::duration(motion::SLOW, cx) * 3)
            .repeat()
            .with_easing(motion::ease_in_out_cubic),
        |dot, t| dot.opacity(0.35 + 0.65 * (1.0 - (2.0 * t - 1.0).abs())),
    )
    .into_any_element()
}

/// A quiet pill while offline or reconnecting, and briefly once back.
#[derive(IntoElement)]
pub struct OfflineIndicator {
    id: ElementId,
    state: Connectivity,
}

impl OfflineIndicator {
    pub fn new(id: impl Into<ElementId>, state: Connectivity) -> Self {
        Self {
            id: id.into(),
            state,
        }
    }
}

impl RenderOnce for OfflineIndicator {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = self.state;
        let seen = window.use_keyed_state(self.id.clone(), cx, |_, _| Seen {
            last: state,
            back_at: None,
            generation: 0,
            _hide: None,
        });
        if seen.read(cx).last != state {
            let returned = state == Connectivity::Online;
            let hide = returned.then(|| {
                window.spawn(cx, async move |cx| {
                    cx.background_executor().timer(HOLD).await;
                    if let Err(error) = cx.update(|window, _| window.refresh()) {
                        log::error!("offline indicator: window gone before hiding: {error:#}");
                    }
                })
            });
            log::info!("connectivity: {:?} -> {state:?}", seen.read(cx).last);
            seen.update(cx, |seen, _| {
                seen.last = state;
                seen.back_at = returned.then(Instant::now);
                seen.generation += 1;
                seen._hide = hide;
            });
        }
        let (back_at, generation) = (seen.read(cx).back_at, seen.read(cx).generation);
        let theme = cx.theme();
        let colors = &theme.colors;
        let (icon, label) = match state {
            Connectivity::Offline => (IconName::WifiOff, "Offline · changes stay on this device"),
            Connectivity::Reconnecting => (IconName::Wifi, "Reconnecting…"),
            Connectivity::Online => (IconName::Wifi, "Back online"),
        };
        let showing =
            state != Connectivity::Online || back_at.is_some_and(|at| at.elapsed() < HOLD);
        if !showing {
            return div().id(self.id).into_any_element();
        }
        let enter = motion::duration(motion::BASE, cx);
        let still = theme.reduced_motion;
        div()
            .id(self.id)
            .flex()
            .items_center()
            .gap_2()
            .px_3()
            .py_1()
            .rounded_full()
            .bg(colors.surface)
            .border_1()
            .border_color(colors.border)
            .shadow(theme.elevation(crate::theme::Elevation::Raised))
            .text_size(theme.text_size(TextSize::Sm))
            .text_color(colors.fg_muted)
            .child(connectivity_dot("dot", state, cx))
            .child(Icon::new(icon).size(IconSize::Sm).color(colors.fg_muted))
            .child(label)
            .with_animation(
                ("connectivity", generation),
                Animation::new(if state == Connectivity::Online && !still {
                    HOLD
                } else {
                    enter
                }),
                move |pill, t| match state {
                    Connectivity::Online if !still => pill.opacity(if t < 0.85 {
                        (t / 0.1).min(1.0)
                    } else {
                        (1.0 - t) / 0.15
                    }),
                    _ => pill.opacity(t).mt(motion::NUDGE * (1.0 - t)),
                },
            )
            .into_any_element()
    }
}

/// Zoom steps, in percent.
const STEPS: [u32; 11] = [50, 67, 75, 80, 90, 100, 110, 125, 150, 175, 200];

/// The step after `current` in `direction`, or `None` at the end.
fn step(current: u32, direction: i8) -> Option<u32> {
    match direction {
        1 => STEPS.iter().copied().find(|step| *step > current),
        _ => STEPS.iter().rev().copied().find(|step| *step < current),
    }
}

/// A press on a `ZoomControl` its owner handles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ZoomStep {
    Out,
    In,
    Reset,
}

type OnZoom = Rc<dyn Fn(ZoomStep, &mut Window, &mut App)>;

/// Minus, percentage, plus. Scales the window's rem size, and all it holds, unless an owner zooms its own content.
#[derive(IntoElement)]
pub struct ZoomControl {
    id: ElementId,
    owned: Option<(u32, OnZoom)>,
}

impl ZoomControl {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            owned: None,
        }
    }

    /// Shows `percent` and hands each press to `on_zoom`.
    pub fn owned(
        mut self,
        percent: u32,
        on_zoom: impl Fn(ZoomStep, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.owned = Some((percent, Rc::new(on_zoom)));
        self
    }
}

fn apply(percent: u32, window: &mut Window, cx: &mut App) {
    let base = cx.theme().base_rem();
    log::info!("zoom: {percent}%");
    window.set_rem_size(base * (percent as f32 / 100.0));
    window.refresh();
}

impl RenderOnce for ZoomControl {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        if let Some((percent, on_zoom)) = self.owned {
            return owned(self.id, percent, on_zoom, cx).into_any_element();
        }
        let percent = (window.rem_size() / theme.base_rem() * 100.0).round() as u32;
        let (down, up) = (step(percent, -1), step(percent, 1));
        div()
            .id(self.id)
            .flex()
            .items_center()
            .gap_0p5()
            .child(
                IconButton::new("zoom-out", IconName::ZoomOut)
                    .size(ControlSize::Sm)
                    .disabled(down.is_none())
                    .when_some(down, |button, next| {
                        button.on_click(move |_, window, cx| apply(next, window, cx))
                    }),
            )
            .child(
                div()
                    .id("zoom-reset")
                    .flex()
                    .items_center()
                    .justify_center()
                    .min_w(theme.control_height(ControlSize::Lg) * 1.5)
                    .h(theme.control_height(ControlSize::Sm))
                    .rounded_full()
                    .cursor_pointer()
                    .hover(|style| style.bg(theme.colors.hover))
                    .on_click(|_, window, cx| apply(100, window, cx))
                    .child(AnimatedNumber::new("zoom-value", percent as f64).size(TextSize::Sm))
                    .child(div().text_size(theme.text_size(TextSize::Sm)).child("%")),
            )
            .child(
                IconButton::new("zoom-in", IconName::ZoomIn)
                    .size(ControlSize::Sm)
                    .variant(ButtonVariant::Ghost)
                    .disabled(up.is_none())
                    .when_some(up, |button, next| {
                        button.on_click(move |_, window, cx| apply(next, window, cx))
                    }),
            )
            .into_any_element()
    }
}

/// The owner's zoom: its percent, its steps.
fn owned(id: ElementId, percent: u32, on_zoom: OnZoom, cx: &App) -> impl IntoElement {
    let theme = cx.theme();
    let press = |step: ZoomStep| {
        let on_zoom = on_zoom.clone();
        move |_: &gpui::ClickEvent, window: &mut Window, cx: &mut App| on_zoom(step, window, cx)
    };
    div()
        .id(id)
        .flex()
        .flex_none()
        .items_center()
        .gap_0p5()
        .child(
            IconButton::new("zoom-out", IconName::ZoomOut)
                .size(ControlSize::Sm)
                .on_click(press(ZoomStep::Out)),
        )
        .child(
            div()
                .id("zoom-reset")
                .flex()
                .items_center()
                .justify_center()
                .min_w(theme.control_height(ControlSize::Lg) * 1.5)
                .h(theme.control_height(ControlSize::Sm))
                .rounded_full()
                .cursor_pointer()
                .hover(|style| style.bg(theme.colors.hover))
                .on_click(press(ZoomStep::Reset))
                .child(AnimatedNumber::new("zoom-value", percent as f64).size(TextSize::Sm))
                .child(div().text_size(theme.text_size(TextSize::Sm)).child("%")),
        )
        .child(
            IconButton::new("zoom-in", IconName::ZoomIn)
                .size(ControlSize::Sm)
                .on_click(press(ZoomStep::In)),
        )
}

#[cfg(test)]
mod tests {
    use super::step;

    #[test]
    fn steps_walk_the_scale_and_stop_at_its_ends() {
        assert_eq!(step(100, 1), Some(110));
        assert_eq!(step(100, -1), Some(90));
        assert_eq!(step(103, -1), Some(100));
        assert_eq!(step(200, 1), None);
        assert_eq!(step(50, -1), None);
    }
}
