use std::rc::Rc;

use gpui::{
    App, ElementId, FontWeight, Hsla, InteractiveElement, IntoElement, MouseButton, ParentElement,
    RenderOnce, StatefulInteractiveElement, Styled, Window, div, prelude::*, relative,
};
use jiff::tz::TimeZone;

use super::event::{Event, When};
use crate::{
    forms::Run,
    primitives::{FocusRing, tab_stop},
    theme::{ActiveTheme, Radius, TextSize},
    typography::{Ellipsis, format},
};

/// Event `event`'s color: the chart color at its hue.
pub(crate) fn tint(event: &Event, cx: &App) -> Hsla {
    cx.theme()
        .colors
        .hue(event.hue, format_args!("event {}", event.key))
}

/// An event in a line: a dot, the hour it starts and its title; or, for whole days, its title on a wash of its color. Pressable with `on_click`.
#[derive(IntoElement)]
pub struct EventChip {
    id: ElementId,
    event: Event,
    zone: Option<TimeZone>,
    before: bool,
    after: bool,
    on_click: Option<Run>,
}

impl EventChip {
    pub fn new(id: impl Into<ElementId>, event: Event) -> Self {
        Self {
            id: id.into(),
            event,
            zone: None,
            before: false,
            after: false,
            on_click: None,
        }
    }

    /// The zone its hour reads in; the system's otherwise.
    pub fn zone(mut self, zone: TimeZone) -> Self {
        self.zone = Some(zone);
        self
    }

    /// Squares the ends where the event runs on past the chip, as a bar cut at a week's edge.
    pub(crate) fn open(mut self, before: bool, after: bool) -> Self {
        self.before = before;
        self.after = after;
        self
    }

    pub fn on_click(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for EventChip {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let zone = self
            .zone
            .unwrap_or_else(|| format::system_zone("event chip"));
        let focus = tab_stop(
            (self.id.clone(), "focus").into(),
            self.on_click.is_some(),
            window,
            cx,
        );
        let theme = cx.theme();
        let colors = &theme.colors;
        let tint = tint(&self.event, cx);
        let round = theme.radius(Radius::Sm);
        let spans = self.event.spans(&zone);
        let hour = match self.event.when {
            When::Timed { start, .. } if !spans => {
                Some(format::datetime(start, &zone, "%H:%M").expect("a fixed pattern formats"))
            }
            _ => None,
        };
        let key = self.event.key.clone();
        div()
            .id(self.id.clone())
            .debug_selector(move || format!("event-chip {key}"))
            .track_focus(&focus)
            .flex()
            .items_center()
            .gap_1()
            .h(theme.calendar().chip)
            .px_1p5()
            .rounded(round)
            .when(self.before, |chip| chip.rounded_l_none())
            .when(self.after, |chip| chip.rounded_r_none())
            .border_1()
            .border_color(gpui::transparent_black())
            .focus_ring(cx)
            .text_size(theme.text_size(TextSize::Xs))
            .line_height(relative(1.0))
            .text_color(colors.fg)
            .map(|chip| match &hour {
                Some(_) => chip.hover(|style| style.bg(colors.hover)),
                None => chip.bg(tint.alpha(0.18)).font_weight(FontWeight::MEDIUM),
            })
            .when_some(self.on_click, |chip, on_click| {
                let key = self.event.key.clone();
                chip.cursor_pointer()
                    .on_mouse_down(MouseButton::Left, |_, window, cx| {
                        window.prevent_default();
                        cx.stop_propagation();
                    })
                    .on_click(move |_, window, cx| {
                        log::info!("event chip: {key}");
                        on_click(window, cx)
                    })
            })
            .when(hour.is_some(), |chip| {
                chip.child(
                    div()
                        .flex_none()
                        .size(theme.status_dot())
                        .rounded_full()
                        .bg(tint),
                )
            })
            .children(hour.map(|hour| {
                div()
                    .min_w_0()
                    .text_color(colors.fg_muted)
                    .child(Ellipsis::new(hour))
            }))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .child(Ellipsis::new(self.event.title.clone())),
            )
    }
}
