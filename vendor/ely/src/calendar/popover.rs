use std::rc::Rc;

use gpui::{
    AnyElement, App, ElementId, FontWeight, InteractiveElement, IntoElement, ParentElement,
    RenderOnce, SharedString, Styled, Window, div, prelude::*,
};
use jiff::tz::TimeZone;

use super::{
    chip::{EventChip, tint},
    event::{Event, When, hours},
    people::{Attendee, AttendeeList},
};
use crate::{
    buttons::{Button, ButtonVariant},
    data_display::color_mark,
    forms::{OnValue, Run},
    overlays::Popover,
    primitives::{Icon, IconName},
    theme::{ActiveTheme, ControlSize, IconSize, TextSize},
    typography::format,
};

/// When an event happens, in full: the day's name and its hours, or its span of whole days.
pub(crate) fn when(event: &Event, zone: &TimeZone) -> String {
    let (first, _) = event.days(zone);
    match event.when {
        When::AllDay { .. } if event.spans(zone) => hours(event, zone),
        _ => format!("{} · {}", first.strftime("%A, %B %-d"), hours(event, zone)),
    }
}

/// An event's chip that opens its details: its title in its hue, when and where, who comes, and what can be done with it. A press outside or Escape closes them.
#[derive(IntoElement)]
pub struct EventPopover {
    id: ElementId,
    event: Event,
    zone: Option<TimeZone>,
    attendees: Vec<Attendee>,
    on_edit: Option<OnValue>,
    on_delete: Option<OnValue>,
}

impl EventPopover {
    pub fn new(id: impl Into<ElementId>, event: Event) -> Self {
        Self {
            id: id.into(),
            event,
            zone: None,
            attendees: Vec::new(),
            on_edit: None,
            on_delete: None,
        }
    }

    /// The zone its hours read in; the system's otherwise.
    pub fn zone(mut self, zone: TimeZone) -> Self {
        self.zone = Some(zone);
        self
    }

    /// Who is asked, shown under where.
    pub fn attendees(mut self, attendees: impl IntoIterator<Item = Attendee>) -> Self {
        self.attendees = attendees.into_iter().collect();
        self
    }

    /// Offers Edit; gets the event's key.
    pub fn on_edit(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_edit = Some(Rc::new(handler));
        self
    }

    /// Offers Delete; gets the event's key.
    pub fn on_delete(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_delete = Some(Rc::new(handler));
        self
    }
}

/// A line of the details: an icon, then its words, wrapping.
fn line(icon: IconName, words: String, cx: &App) -> AnyElement {
    let theme = cx.theme();
    div()
        .flex()
        .items_start()
        .gap_2()
        .child(
            Icon::new(icon)
                .size(IconSize::Sm)
                .color(theme.colors.fg_subtle),
        )
        .child(div().flex_1().min_w_0().child(words))
        .into_any_element()
}

impl RenderOnce for EventPopover {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let zone = self
            .zone
            .unwrap_or_else(|| format::system_zone("event popover"));
        let (event, chip_zone) = (self.event.clone(), zone.clone());
        let (id, chip_id) = (self.id.clone(), (self.id.clone(), "chip"));
        let details = move |close: Run, _: &mut Window, cx: &mut App| {
            let theme = cx.theme();
            let key = self.event.key.clone();
            let action = |label: &'static str, variant, handler: Option<OnValue>| {
                handler.map(|handler| {
                    let (key, close) = (key.clone(), close.clone());
                    Button::new((self.id.clone(), label), label)
                        .variant(variant)
                        .size(ControlSize::Sm)
                        .on_click(move |_, window, cx| {
                            log::info!("event popover: {label} {key}");
                            close(window, cx);
                            handler(&key, window, cx)
                        })
                })
            };
            let delete = action("Delete", ButtonVariant::Ghost, self.on_delete.clone());
            let edit = action("Edit", ButtonVariant::Secondary, self.on_edit.clone());
            let key = self.event.key.clone();
            div()
                .debug_selector(move || format!("event-popover {key}"))
                .w(theme.calendar().card)
                .flex()
                .flex_col()
                .gap_3()
                .text_size(theme.text_size(TextSize::Sm))
                .text_color(theme.colors.fg)
                .child(
                    div()
                        .flex()
                        .items_start()
                        .gap_2()
                        .child(color_mark(tint(&self.event, cx), cx))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .text_size(theme.text_size(TextSize::Base))
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(self.event.title.clone()),
                        ),
                )
                .child(line(IconName::Clock, when(&self.event, &zone), cx))
                .children(
                    self.event
                        .place
                        .clone()
                        .map(|place| line(IconName::MapPin, place.to_string(), cx)),
                )
                .when(!self.attendees.is_empty(), |details| {
                    details.child(AttendeeList::new(
                        (self.id.clone(), "guests"),
                        self.attendees.clone(),
                    ))
                })
                .when(delete.is_some() || edit.is_some(), |details| {
                    details.child(
                        div()
                            .flex()
                            .justify_end()
                            .gap_2()
                            .children(delete)
                            .children(edit),
                    )
                })
        };
        Popover::with_opener(
            id,
            move |toggle| {
                EventChip::new(chip_id, event)
                    .zone(chip_zone)
                    .on_click(move |window, cx| toggle(window, cx))
            },
            details,
        )
    }
}
