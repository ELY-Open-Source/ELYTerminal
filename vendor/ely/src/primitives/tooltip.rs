use std::{rc::Rc, time::Duration};

use gpui::{
    Animation, AnimationExt, AnyElement, AnyView, App, AppContext, Context, Div, ElementId,
    HoverListenerMode, InteractiveElement, IntoElement, ParentElement, Pixels, Point, Render,
    RenderOnce, SharedString, StatefulInteractiveElement, Styled, Task, Window, anchored, div,
    point, prelude::*,
};

use super::{Icon, IconName, raise};
use crate::{
    i18n, motion,
    theme::{ActiveTheme, Elevation, IconSize, Radius, TextSize},
};

type Content = Rc<dyn Fn(&mut Window, &mut App) -> AnyElement>;

const DELAY: Duration = Duration::from_millis(400);

fn surface(content: AnyElement, cx: &App) -> impl IntoElement + use<> {
    let theme = cx.theme();
    let rise = motion::NUDGE;
    let body = div()
        .max_w(theme.tooltip_max_width())
        .px_2()
        .py_1()
        .rounded(theme.radius(Radius::Md))
        .bg(theme.colors.tooltip_bg)
        .text_color(theme.colors.tooltip_fg)
        // gpui draws a tooltip as a root of its own.
        .font_family(theme.font_family.clone())
        .text_size(theme.text_size(TextSize::Sm))
        .shadow(theme.elevation(Elevation::Floating))
        .child(content);
    body.with_animation(
        "tooltip-in",
        Animation::new(motion::duration(motion::FAST, cx)).with_easing(motion::ease_out_cubic),
        move |body, t| body.opacity(t).mt(rise * (1.0 - t)),
    )
}

/// Tooltip body for gpui's `.tooltip()`.
pub struct Tooltip {
    content: Content,
    /// Locale changes before its words; after another it withdraws.
    turn: u64,
}

impl Tooltip {
    pub fn text(title: impl Into<SharedString>) -> impl Fn(&mut Window, &mut App) -> AnyView {
        let title = title.into();
        Self::rich(move |_, _| title.clone().into_any_element())
    }

    /// Title plus a quiet trailing hint, such as a shortcut.
    pub fn with_meta(
        title: impl Into<SharedString>,
        meta: impl Into<SharedString>,
    ) -> impl Fn(&mut Window, &mut App) -> AnyView {
        let (title, meta) = (title.into(), meta.into());
        Self::rich(move |_, cx| meta_row(title.clone(), meta.clone(), cx).into_any_element())
    }

    pub fn rich(
        content: impl Fn(&mut Window, &mut App) -> AnyElement + 'static,
    ) -> impl Fn(&mut Window, &mut App) -> AnyView {
        let (content, turn): (Content, u64) = (Rc::new(content), i18n::turns());
        move |_, cx| {
            let content = content.clone();
            cx.new(|_| Tooltip { content, turn }).into()
        }
    }
}

impl Render for Tooltip {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if i18n::turns() != self.turn {
            return div();
        }
        let content = (self.content)(window, cx);
        div()
            .pt(cx.theme().cursor_offset())
            .child(surface(content, cx))
    }
}

fn meta_row(title: SharedString, meta: SharedString, cx: &App) -> Div {
    div().flex().items_center().gap_2().child(title).child(
        div()
            .text_color(cx.theme().colors.tooltip_fg.opacity(0.6))
            .child(meta),
    )
}

#[derive(Default)]
struct TriggerState {
    visible: bool,
    mouse: Point<Pixels>,
    _pending: Option<Task<()>>,
}

/// Tooltip with its own delay, and optional cursor tracking.
#[derive(IntoElement)]
pub struct TooltipTrigger {
    id: ElementId,
    child: AnyElement,
    content: Content,
    delay: Duration,
    follow: bool,
}

impl TooltipTrigger {
    pub fn new(
        id: impl Into<ElementId>,
        child: impl IntoElement,
        content: impl Fn(&mut Window, &mut App) -> AnyElement + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            child: child.into_any_element(),
            content: Rc::new(content),
            delay: DELAY,
            follow: false,
        }
    }

    pub fn delay(mut self, delay: Duration) -> Self {
        self.delay = delay;
        self
    }

    pub fn follow_mouse(mut self) -> Self {
        self.follow = true;
        self
    }
}

impl RenderOnce for TooltipTrigger {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = window.use_keyed_state(self.id.clone(), cx, |_, _| TriggerState::default());
        let (visible, mouse) = {
            let read = state.read(cx);
            (read.visible, read.mouse)
        };
        let (hover, track, press) = (state.clone(), state.clone(), state);
        let (delay, follow) = (self.delay, self.follow);
        let content = self.content;

        let id = self.id.clone();
        div()
            .id(self.id)
            .child(self.child)
            .hover_listener_mode(HoverListenerMode::InputModalityIndependent)
            .on_hover(move |hovered, window, cx| {
                if !*hovered {
                    hover.update(cx, hide);
                    return;
                }
                let weak = hover.downgrade();
                let task = window.spawn(cx, async move |cx| {
                    cx.background_executor().timer(delay).await;
                    let shown = cx.update(|window, cx| {
                        weak.update(cx, |trigger, cx| {
                            trigger.visible = true;
                            trigger.mouse = window.mouse_position();
                            trigger._pending = None;
                            cx.notify();
                        })
                    });
                    if let Err(error) = shown.and_then(|inner| inner) {
                        log::error!("tooltip: trigger vanished before showing: {error:#}");
                    }
                });
                hover.update(cx, |trigger, _| trigger._pending = Some(task));
            })
            .on_mouse_move(move |event, _, cx| {
                track.update(cx, |trigger, cx| {
                    trigger.mouse = event.position;
                    if follow && trigger.visible {
                        cx.notify();
                    }
                })
            })
            .on_any_mouse_down(move |_, _, cx| press.update(cx, hide))
            .when(visible, |trigger| {
                let body = content(window, cx);
                let below = cx.theme().cursor_offset().to_pixels(window.rem_size());
                trigger.child(raise(
                    (id, "raised"),
                    anchored()
                        .position(mouse + point(Pixels::ZERO, below))
                        .snap_to_window()
                        .child(surface(body, cx)),
                ))
            })
    }
}

fn hide(trigger: &mut TriggerState, cx: &mut Context<TriggerState>) {
    trigger.visible = false;
    trigger._pending = None;
    cx.notify();
}

/// A help mark, ?, whose tip shows on hover.
#[derive(IntoElement)]
pub struct HelpTooltip {
    id: ElementId,
    text: SharedString,
}

impl HelpTooltip {
    pub fn new(id: impl Into<ElementId>, text: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
        }
    }
}

impl RenderOnce for HelpTooltip {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .id(self.id)
            .flex_none()
            .child(
                Icon::new(IconName::CircleHelp)
                    .size(IconSize::Xs)
                    .color(cx.theme().colors.fg_subtle),
            )
            .tooltip(Tooltip::text(self.text))
    }
}
