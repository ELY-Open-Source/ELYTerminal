use std::rc::Rc;

use gpui::{App, ElementId, IntoElement, ParentElement, RenderOnce, Window, div, prelude::*};
use jiff::{Timestamp, civil::Weekday, tz::TimeZone};

use super::times::{
    Ask, OnTime, at_hour, distinct, named, next_monday, pick_item, picking, time_dialog,
};
use crate::{
    menus::{DropdownMenu, Menu, MenuItem},
    primitives::IconName,
    typography::format,
};

/// Times to snooze until from `now` in `zone`, each named with its day and hour: later today at 18:00 while before 17:00, tomorrow at 8:00, Saturday at 8:00 on a weekday, and the next Monday at 8:00. A moment comes once.
pub(super) fn snoozes(now: Timestamp, zone: &TimeZone) -> Vec<(String, Timestamp)> {
    let here = now.to_zoned(zone.clone());
    let today = here.date();
    let tomorrow = today.tomorrow().expect("a day follows today");
    let workday = !matches!(today.weekday(), Weekday::Saturday | Weekday::Sunday);
    let times = [
        (here.hour() < 17).then(|| named("Later today", at_hour(today, 18, zone), zone)),
        Some(named("Tomorrow", at_hour(tomorrow, 8, zone), zone)),
        workday.then(|| {
            let saturday = today
                .nth_weekday(1, Weekday::Saturday)
                .expect("a Saturday lies ahead");
            named("This weekend", at_hour(saturday, 8, zone), zone)
        }),
        Some(named(
            "Next week",
            at_hour(next_monday(today), 8, zone),
            zone,
        )),
    ];
    distinct(times.into_iter().flatten())
}

/// A menu of times to put a message off until: later today, tomorrow, this weekend, next week, or a day and hour picked in a dialog, in the zone given.
#[derive(IntoElement)]
pub struct SnoozePicker {
    id: ElementId,
    zone: Option<TimeZone>,
    on_snooze: Option<OnTime>,
}

impl SnoozePicker {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            zone: None,
            on_snooze: None,
        }
    }

    /// The zone its times read in; the system's otherwise.
    pub fn zone(mut self, zone: TimeZone) -> Self {
        self.zone = Some(zone);
        self
    }

    /// Gets the moment the message comes back.
    pub fn on_snooze(
        mut self,
        handler: impl Fn(Timestamp, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_snooze = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for SnoozePicker {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let zone = self
            .zone
            .unwrap_or_else(|| format::system_zone("snooze picker"));
        let picking = picking(&self.id, window, cx);
        let menu = snoozes(Timestamp::now(), &zone)
            .into_iter()
            .fold(Menu::new(), |menu, (name, at)| {
                let on_snooze = self.on_snooze.clone();
                menu.item(MenuItem::new(name).on_click(move |window, cx| {
                    log::info!("snooze picker: until {at}");
                    if let Some(on_snooze) = &on_snooze {
                        on_snooze(at, window, cx);
                    }
                }))
            })
            .separator()
            .item(pick_item("snooze picker", &picking));
        let dialog = time_dialog(
            &self.id,
            &picking,
            Ask {
                title: "Snooze until",
                detail: "Pick the day and the hour it comes back.",
                action: "Snooze",
            },
            &zone,
            self.on_snooze.clone(),
            cx,
        );
        div()
            .flex_none()
            .child(DropdownMenu::new(self.id.clone(), "Snooze", menu).icon(IconName::Clock))
            .children(dialog)
    }
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;

    use super::*;

    fn at(day: i8, hour: i8) -> Timestamp {
        at_hour(date(2026, 10, day), hour, &TimeZone::UTC)
    }

    fn names(now: Timestamp) -> Vec<String> {
        snoozes(now, &TimeZone::UTC)
            .into_iter()
            .map(|(name, _)| name)
            .collect()
    }

    #[test]
    fn a_weekday_morning_offers_later_today_tomorrow_the_weekend_and_next_week() {
        let times: Vec<Timestamp> = snoozes(at(7, 9), &TimeZone::UTC)
            .into_iter()
            .map(|(_, at)| at)
            .collect();
        assert_eq!(
            names(at(7, 9)),
            [
                "Later today · Wed 18:00",
                "Tomorrow · Thu 08:00",
                "This weekend · Sat 08:00",
                "Next week · Mon 08:00",
            ]
        );
        assert_eq!(times, [at(7, 18), at(8, 8), at(10, 8), at(12, 8)]);
    }

    #[test]
    fn late_in_the_day_and_at_the_weekend_the_near_times_step_aside() {
        assert_eq!(
            names(at(7, 17)),
            [
                "Tomorrow · Thu 08:00",
                "This weekend · Sat 08:00",
                "Next week · Mon 08:00",
            ],
            "from 17:00, later today is too near"
        );
        assert_eq!(
            names(at(10, 9)),
            [
                "Later today · Sat 18:00",
                "Tomorrow · Sun 08:00",
                "Next week · Mon 08:00",
            ],
            "on a Saturday, the weekend is here"
        );
        assert_eq!(
            names(at(9, 9)),
            [
                "Later today · Fri 18:00",
                "Tomorrow · Sat 08:00",
                "Next week · Mon 08:00",
            ],
            "on a Friday, tomorrow is the weekend"
        );
        assert_eq!(
            names(at(4, 9)),
            ["Later today · Sun 18:00", "Tomorrow · Mon 08:00"],
            "on a Sunday, tomorrow is next week"
        );
        let last = snoozes(at(12, 9), &TimeZone::UTC).pop();
        assert_eq!(
            last.map(|(_, at)| at),
            Some(at(19, 8)),
            "on a Monday, the next one"
        );
    }
}
