use std::rc::Rc;

use gpui::{
    App, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce, Styled, Window, div,
};
use jiff::{Timestamp, civil::Date, tz::TimeZone};

use super::hours::moment;
use crate::{
    forms::{Choice, ChoiceChips},
    layout::seeded::use_seeded,
    theme::{ActiveTheme, TextSize},
    typography::format,
};

type OnTime = Rc<dyn Fn(Timestamp, &mut Window, &mut App)>;

/// Slots start on the half hour.
const STEP: i64 = 30;

/// The starts, in minutes of `day`, of slots `length` long within `hours` that meet no busy stretch and begin after `now`.
pub(crate) fn open_slots(
    day: Date,
    hours: (i64, i64),
    length: i64,
    busy: &[(Timestamp, Timestamp)],
    now: Timestamp,
    zone: &TimeZone,
) -> Vec<i64> {
    let mut starts = Vec::new();
    let mut start = hours.0;
    while start + length <= hours.1 {
        let (from, to) = (moment(day, start, zone), moment(day, start + length, zone));
        let free = busy
            .iter()
            .all(|(taken, until)| *until <= from || *taken >= to);
        if free && from > now {
            starts.push(start);
        }
        start += STEP;
    }
    starts
}

/// Open times to meet over some days: each day with its count of open slots, then the slots of the one chosen. Busy stretches, the working hours and the past close the rest. A pick hands on the slot's start; new days from the owner start the choice over.
#[derive(IntoElement)]
pub struct AvailabilityPicker {
    id: ElementId,
    days: Vec<Date>,
    busy: Vec<(Timestamp, Timestamp)>,
    hours: (i64, i64),
    length: i64,
    zone: Option<TimeZone>,
    now: Option<Timestamp>,
    selected: Option<Timestamp>,
    on_pick: Option<OnTime>,
}

impl AvailabilityPicker {
    pub fn new(id: impl Into<ElementId>, days: impl IntoIterator<Item = Date>) -> Self {
        let id = id.into();
        let days: Vec<Date> = days.into_iter().collect();
        assert!(!days.is_empty(), "availability picker {id:?} has no days");
        Self {
            id,
            days,
            busy: Vec::new(),
            hours: (9 * 60, 17 * 60),
            length: 30,
            zone: None,
            now: None,
            selected: None,
            on_pick: None,
        }
    }

    /// Stretches already taken, each from a moment to a later one.
    pub fn busy(mut self, busy: impl IntoIterator<Item = (Timestamp, Timestamp)>) -> Self {
        self.busy = busy.into_iter().collect();
        for (from, to) in &self.busy {
            assert!(
                to > from,
                "a busy stretch from {from} ends before it starts"
            );
        }
        self
    }

    /// The hours a slot may fall in, from the first to the last; nine to five otherwise.
    pub fn hours(mut self, first: i8, last: i8) -> Self {
        assert!(
            first < last && last <= 24,
            "working hours from {first} to {last}"
        );
        self.hours = (i64::from(first) * 60, i64::from(last) * 60);
        self
    }

    /// How long a slot is, in minutes; half an hour otherwise.
    pub fn length(mut self, minutes: u16) -> Self {
        assert!(minutes > 0, "a slot takes some time");
        self.length = i64::from(minutes);
        self
    }

    /// The zone its days and hours read in; the system's otherwise.
    pub fn zone(mut self, zone: TimeZone) -> Self {
        self.zone = Some(zone);
        self
    }

    /// The moment before which nothing is open; the clock's otherwise.
    pub fn now(mut self, now: Timestamp) -> Self {
        self.now = Some(now);
        self
    }

    /// The slot picked, by its start.
    pub fn selected(mut self, start: Timestamp) -> Self {
        self.selected = Some(start);
        self
    }

    /// Gets the start of each slot picked.
    pub fn on_pick(mut self, handler: impl Fn(Timestamp, &mut Window, &mut App) + 'static) -> Self {
        self.on_pick = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for AvailabilityPicker {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let zone = self
            .zone
            .unwrap_or_else(|| format::system_zone("availability picker"));
        let now = self.now.unwrap_or_else(Timestamp::now);
        let open: Vec<(Date, Vec<i64>)> = self
            .days
            .iter()
            .map(|day| {
                let slots = open_slots(*day, self.hours, self.length, &self.busy, now, &zone);
                (*day, slots)
            })
            .collect();
        let picked_day = self
            .selected
            .map(|start| start.to_zoned(zone.clone()).date())
            .filter(|day| self.days.contains(day));
        let first_open = open
            .iter()
            .find(|(_, slots)| !slots.is_empty())
            .map(|(day, _)| *day);
        let seed = (self.days.clone(), picked_day.or(first_open));
        let chosen = use_seeded((self.id.clone(), "day"), seed, window, cx);
        let shown = chosen.read(cx).value.1;
        let theme = cx.theme();
        let quiet = |text: &'static str| {
            div()
                .py_2()
                .text_size(theme.text_size(TextSize::Sm))
                .text_color(theme.colors.fg_subtle)
                .child(text)
        };
        let Some(shown) = shown else {
            return div()
                .debug_selector(|| "availability-none".into())
                .child(quiet("No open time in these days."));
        };
        let days = ChoiceChips::new(
            (self.id.clone(), "days"),
            open.iter().map(|(day, slots)| {
                let label = format!("{} · {}", day.strftime("%a %-d"), slots.len());
                let choice = Choice::new(day.to_string(), label);
                match slots.is_empty() {
                    true => choice.disabled(),
                    false => choice,
                }
            }),
        )
        .selected([shown.to_string()])
        .on_change(move |values, _, cx| {
            let day: Date = values
                .first()
                .expect("a day is chosen")
                .parse()
                .expect("a day's value is its date");
            log::info!("availability picker: {day}");
            chosen.update(cx, |chosen, cx| {
                chosen.value.1 = Some(day);
                cx.notify();
            })
        });
        let slots = open
            .iter()
            .find(|(day, _)| *day == shown)
            .map(|(_, slots)| slots.clone())
            .expect("the day shown is one of the days");
        let picked = self
            .selected
            .filter(|_| picked_day == Some(shown))
            .map(|start| {
                let zoned = start.to_zoned(zone.clone());
                (i64::from(zoned.hour()) * 60 + i64::from(zoned.minute())).to_string()
            });
        let times = (!slots.is_empty()).then(|| {
            let (on_pick, zone) = (self.on_pick.clone(), zone.clone());
            ChoiceChips::new(
                (self.id.clone(), format!("slots-{shown}")),
                slots.iter().map(|minute| {
                    let label = format!("{:02}:{:02}", minute / 60, minute % 60);
                    Choice::new(minute.to_string(), label)
                }),
            )
            .selected(picked)
            .on_change(move |values, window, cx| {
                let minute: i64 = values
                    .first()
                    .expect("a slot is chosen")
                    .parse()
                    .expect("a slot's value is its minute");
                let start = moment(shown, minute, &zone);
                log::info!("availability picker: picked {start}");
                if let Some(on_pick) = &on_pick {
                    on_pick(start, window, cx);
                }
            })
        });
        div()
            .debug_selector(|| "availability".into())
            .flex()
            .flex_col()
            .gap_3()
            .child(days)
            .children(times)
            .children(slots.is_empty().then(|| quiet("No open time this day.")))
    }
}

#[cfg(test)]
mod tests {
    use jiff::{Timestamp, civil::date, tz::TimeZone};

    use super::open_slots;

    fn at(day: i8, hour: i8, minute: i8) -> Timestamp {
        date(2026, 9, day)
            .at(hour, minute, 0, 0)
            .to_zoned(TimeZone::UTC)
            .expect("a UTC time")
            .timestamp()
    }

    #[test]
    fn slots_fill_the_hours_around_what_is_busy_and_after_now() {
        let busy = [(at(28, 10, 0), at(28, 11, 15))];
        let starts = open_slots(
            date(2026, 9, 28),
            (9 * 60, 12 * 60),
            30,
            &busy,
            at(28, 9, 10),
            &TimeZone::UTC,
        );
        assert_eq!(
            starts,
            [9 * 60 + 30, 11 * 60 + 30],
            "09:00 has passed, 10:00 to 11:15 is taken, and the last slot ends at noon"
        );
    }
}
