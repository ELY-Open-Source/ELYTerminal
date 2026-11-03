use std::rc::Rc;

use gpui::{
    App, ElementId, FontWeight, InteractiveElement, IntoElement, MouseButton, ParentElement,
    RenderOnce, StatefulInteractiveElement, Styled, Window, div, prelude::*, relative,
};
use jiff::tz::TimeZone;

use super::{
    chip::tint,
    event::{Event, hours},
};
use crate::{
    forms::Run,
    primitives::FocusRing,
    theme::{ActiveTheme, Radius, TextSize},
    typography::{Ellipsis, format},
};

/// An event as a block: a rule in its hue, its title, and under it its hours and place. It fills the box it is given, as a time grid sizes it by the hour. Pressable with `on_click`.
#[derive(IntoElement)]
pub struct EventCard {
    id: ElementId,
    event: Event,
    zone: Option<TimeZone>,
    on_click: Option<Run>,
}

impl EventCard {
    pub fn new(id: impl Into<ElementId>, event: Event) -> Self {
        Self {
            id: id.into(),
            event,
            zone: None,
            on_click: None,
        }
    }

    /// The zone its hours read in; the system's otherwise.
    pub fn zone(mut self, zone: TimeZone) -> Self {
        self.zone = Some(zone);
        self
    }

    pub fn on_click(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for EventCard {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let zone = self
            .zone
            .unwrap_or_else(|| format::system_zone("event card"));
        let theme = cx.theme();
        let colors = &theme.colors;
        let tint = tint(&self.event, cx);
        let detail = match &self.event.place {
            Some(place) => format!("{} · {place}", hours(&self.event, &zone)),
            None => hours(&self.event, &zone),
        };
        let key = self.event.key.clone();
        div()
            .id(self.id)
            .debug_selector(move || format!("event-card {key}"))
            .relative()
            .size_full()
            .overflow_hidden()
            .flex()
            .flex_col()
            .gap_0p5()
            .pl_2p5()
            .pr_1p5()
            .py_1()
            .rounded(theme.radius(Radius::Sm))
            .border_1()
            .border_color(gpui::transparent_black())
            .bg(tint.alpha(0.14))
            .line_height(relative(1.25))
            .when_some(self.on_click, |card, on_click| {
                let key = self.event.key.clone();
                card.tab_index(0)
                    .focus_ring(cx)
                    .cursor_pointer()
                    .on_mouse_down(MouseButton::Left, |_, window, cx| {
                        window.prevent_default();
                        cx.stop_propagation();
                    })
                    .on_click(move |_, window, cx| {
                        log::info!("event card: {key}");
                        on_click(window, cx)
                    })
            })
            .child(
                div()
                    .absolute()
                    .top_1()
                    .bottom_1()
                    .left_1()
                    .w_0p5()
                    .rounded_full()
                    .bg(tint),
            )
            .child(
                div()
                    .text_size(theme.text_size(TextSize::Sm))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(colors.fg)
                    .child(Ellipsis::new(self.event.title.clone())),
            )
            .child(
                div()
                    .text_size(theme.text_size(TextSize::Xs))
                    .text_color(colors.fg_muted)
                    .child(Ellipsis::new(detail)),
            )
    }
}
