use std::rc::Rc;

use gpui::{App, ElementId, IntoElement, ParentElement, RenderOnce, Window, div, prelude::*};
use jiff::{Timestamp, tz::TimeZone};

use super::times::{
    Ask, OnTime, at_hour, distinct, named, next_monday, pick_item, picking, time_dialog,
};
use crate::{
    buttons::{Button, ButtonVariant},
    forms::Run,
    menus::{Menu, MenuItem, SplitButton},
    typography::format,
};

/// Times to send later from `now` in `zone`: tomorrow at 8:00 and at 13:00, and the next Monday at 8:00 unless tomorrow is one, each named with its day and hour.
fn presets(now: Timestamp, zone: &TimeZone) -> Vec<(String, Timestamp)> {
    let today = now.to_zoned(zone.clone()).date();
    let tomorrow = today.tomorrow().expect("a day follows today");
    distinct([
        named("Tomorrow morning", at_hour(tomorrow, 8, zone), zone),
        named("Tomorrow afternoon", at_hour(tomorrow, 13, zone), zone),
        named("Monday morning", at_hour(next_monday(today), 8, zone), zone),
    ])
}

/// Send, and beside it the times to send later: tomorrow morning, tomorrow afternoon and Monday morning in the zone given, or a time picked in a dialog. Each only with its handler; disabled, only Send shows, dimmed.
#[derive(IntoElement)]
pub struct ScheduleSend {
    id: ElementId,
    zone: Option<TimeZone>,
    disabled: bool,
    on_send: Option<Run>,
    on_schedule: Option<OnTime>,
}

impl ScheduleSend {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            zone: None,
            disabled: false,
            on_send: None,
            on_schedule: None,
        }
    }

    /// The zone its times read in; the system's otherwise.
    pub fn zone(mut self, zone: TimeZone) -> Self {
        self.zone = Some(zone);
        self
    }

    /// Nothing to send yet.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn on_send(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_send = Some(Rc::new(handler));
        self
    }

    /// Gets the moment to send at.
    pub fn on_schedule(
        mut self,
        handler: impl Fn(Timestamp, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_schedule = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for ScheduleSend {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let zone = self
            .zone
            .unwrap_or_else(|| format::system_zone("schedule send"));
        let picking = picking(&self.id, window, cx);
        let menu = self
            .on_schedule
            .clone()
            .filter(|_| !self.disabled)
            .map(|schedule| {
                let presets = presets(Timestamp::now(), &zone).into_iter().fold(
                    Menu::new(),
                    |menu, (name, at)| {
                        let schedule = schedule.clone();
                        menu.item(MenuItem::new(name).on_click(move |window, cx| {
                            log::info!("schedule send: at {at}");
                            schedule(at, window, cx)
                        }))
                    },
                );
                presets
                    .separator()
                    .item(pick_item("schedule send", &picking))
            });
        let send = self.on_send.clone();
        let button = match menu {
            Some(menu) => SplitButton::new((self.id.clone(), "send"), "Send", menu)
                .variant(ButtonVariant::Primary)
                .on_click(move |_, window, cx| {
                    log::info!("schedule send: send");
                    if let Some(send) = &send {
                        send(window, cx);
                    }
                })
                .into_any_element(),
            None => Button::new((self.id.clone(), "send"), "Send")
                .variant(ButtonVariant::Primary)
                .disabled(self.disabled)
                .on_click(move |_, window, cx| {
                    log::info!("schedule send: send");
                    if let Some(send) = &send {
                        send(window, cx);
                    }
                })
                .into_any_element(),
        };
        let dialog = time_dialog(
            &self.id,
            &picking,
            Ask {
                title: "Schedule send",
                detail: "Pick the day and the hour it goes.",
                action: "Schedule",
            },
            &zone,
            self.on_schedule.clone(),
            cx,
        );
        div().flex_none().child(button).children(dialog)
    }
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;

    use super::*;

    fn at(day: i8, hour: i8) -> Timestamp {
        at_hour(date(2026, 9, day), hour, &TimeZone::UTC)
    }

    #[test]
    fn presets_fall_tomorrow_and_on_the_next_monday() {
        let [(morning, a), (afternoon, b), (monday, c)] = &presets(at(26, 12), &TimeZone::UTC)[..]
        else {
            panic!("three times from a Saturday");
        };
        assert_eq!(
            (morning.as_str(), *a),
            ("Tomorrow morning · Sun 08:00", at(27, 8))
        );
        assert_eq!(
            (afternoon.as_str(), *b),
            ("Tomorrow afternoon · Sun 13:00", at(27, 13))
        );
        assert_eq!(
            (monday.as_str(), *c),
            ("Monday morning · Mon 08:00", at(28, 8))
        );
        let next = presets(at(28, 9), &TimeZone::UTC).pop().map(|(_, at)| at);
        assert_eq!(
            next,
            Some(at_hour(date(2026, 10, 5), 8, &TimeZone::UTC)),
            "on a Monday, the next one"
        );
        let sunday: Vec<String> = presets(at(27, 12), &TimeZone::UTC)
            .into_iter()
            .map(|(name, _)| name)
            .collect();
        assert_eq!(
            sunday,
            [
                "Tomorrow morning · Mon 08:00",
                "Tomorrow afternoon · Mon 13:00"
            ],
            "on a Sunday, tomorrow morning is Monday morning"
        );
    }
}
