use std::{collections::HashMap, rc::Rc};

use gpui::{
    App, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce, SharedString,
    Styled, Window, div,
};
use jiff::{Timestamp, ToSpan, civil::Date, tz::TimeZone};

use super::{
    chip::tint,
    event::{Event, When, hours, unique},
    hours::minutes,
};
use crate::{
    data_display::color_mark,
    forms::OnValue,
    lists::{ListItem, Sections},
    theme::{ActiveTheme, TextSize},
    typography::{format, fresh},
};

/// A day's heading: today and tomorrow say so before the date.
pub(crate) fn heading(day: Date, today: Date) -> SharedString {
    let long = day.strftime("%A, %B %-d").to_string();
    let tomorrow = today.tomorrow().expect("a day follows today");
    match day {
        day if day == today => format!("Today · {long}").into(),
        day if day == tomorrow => format!("Tomorrow · {long}").into(),
        _ => long.into(),
    }
}

/// The events on `day` in order: whole days first, then by their start that day.
pub(crate) fn on_day<'a>(events: &'a [Event], day: Date, zone: &TimeZone) -> Vec<&'a Event> {
    let mut on: Vec<(i64, usize, &Event)> = events
        .iter()
        .enumerate()
        .filter_map(|(ix, event)| match event.when {
            When::AllDay { first, last } => {
                (first <= day && day <= last).then_some((-1, ix, event))
            }
            When::Timed { .. } => minutes(event, day, zone).map(|(from, _)| (from, ix, event)),
        })
        .collect();
    on.sort_by_key(|&(from, ix, _)| (from, ix));
    on.into_iter().map(|(_, _, event)| event).collect()
}

/// The days from `from` with their events under each day's name, whole days first, then by the hour; days without events are left out. A press or an arrow selects an event, Enter or a double press opens it.
#[derive(IntoElement)]
pub struct AgendaView {
    id: ElementId,
    from: Date,
    days: usize,
    events: Vec<Event>,
    zone: Option<TimeZone>,
    today: Option<Date>,
    on_event: Option<OnValue>,
}

impl AgendaView {
    pub fn new(
        id: impl Into<ElementId>,
        from: Date,
        days: usize,
        events: impl IntoIterator<Item = Event>,
    ) -> Self {
        Self {
            id: id.into(),
            from,
            days,
            events: unique(events),
            zone: None,
            today: None,
            on_event: None,
        }
    }

    /// The zone its days and hours read in; the system's otherwise.
    pub fn zone(mut self, zone: TimeZone) -> Self {
        self.zone = Some(zone);
        self
    }

    /// The day named today; the zone's own otherwise.
    pub fn today(mut self, date: Date) -> Self {
        self.today = Some(date);
        self
    }

    /// Gets the key of an event opened.
    pub fn on_event(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_event = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for AgendaView {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        if self.today.is_none() {
            fresh((self.id.clone(), "clock"), window, cx);
        }
        let zone = self
            .zone
            .unwrap_or_else(|| format::system_zone("agenda view"));
        let today = self
            .today
            .unwrap_or_else(|| Timestamp::now().to_zoned(zone.clone()).date());
        let picked =
            window.use_keyed_state((self.id.clone(), "picked"), cx, |_, _| None::<SharedString>);
        let theme = cx.theme();
        let mut owners: HashMap<SharedString, SharedString> = HashMap::new();
        let mut sections = Sections::new(self.id.clone());
        for offset in 0..self.days as i64 {
            let day = self.from.checked_add(offset.days()).expect("a day follows");
            let on = on_day(&self.events, day, &zone);
            if on.is_empty() {
                continue;
            }
            let rows = on
                .into_iter()
                .map(|event| {
                    let key = SharedString::from(format!("{day}/{}", event.key));
                    owners.insert(key.clone(), event.key.clone());
                    let detail = match &event.place {
                        Some(place) => format!("{} · {place}", hours(event, &zone)),
                        None => hours(event, &zone),
                    };
                    let mark = color_mark(tint(event, cx), cx);
                    let row =
                        ListItem::new((self.id.clone(), key.to_string()), event.title.clone())
                            .leading(mark)
                            .description(detail);
                    (key, row)
                })
                .collect();
            sections = sections.section(heading(day, today), rows);
        }
        if owners.is_empty() {
            return div()
                .debug_selector(|| "agenda-none".into())
                .py_6()
                .flex()
                .justify_center()
                .text_size(theme.text_size(TextSize::Sm))
                .text_color(theme.colors.fg_subtle)
                .child("Nothing planned.")
                .into_any_element();
        }
        let owners = Rc::new(owners);
        let pick: OnValue = {
            let picked = picked.clone();
            Rc::new(move |key, _, cx| {
                picked.update(cx, |picked, cx| {
                    *picked = Some(key.clone());
                    cx.notify();
                })
            })
        };
        let open: OnValue = {
            let (id, on_event) = (self.id.clone(), self.on_event);
            Rc::new(move |key, window, cx| {
                let event = owners.get(key).expect("a row names its event");
                log::info!("agenda view {id:?}: open {event}");
                if let Some(on_event) = &on_event {
                    on_event(event, window, cx);
                }
            })
        };
        div()
            .child(
                sections
                    .selected(picked.read(cx).clone())
                    .on_select(pick)
                    .on_activate(open),
            )
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use jiff::{civil::date, tz::TimeZone};

    use super::{heading, on_day};
    use crate::calendar::Event;

    #[test]
    fn today_and_tomorrow_say_so_before_the_date() {
        let today = date(2026, 9, 27);
        assert_eq!(heading(today, today), "Today · Sunday, September 27");
        assert_eq!(
            heading(date(2026, 9, 28), today),
            "Tomorrow · Monday, September 28"
        );
        assert_eq!(heading(date(2026, 9, 30), today), "Wednesday, September 30");
    }

    #[test]
    fn a_day_lists_whole_days_first_then_by_the_hour() {
        let at = |day: i8, hour: i8| {
            date(2026, 9, day)
                .at(hour, 0, 0, 0)
                .to_zoned(TimeZone::UTC)
                .expect("a UTC time")
                .timestamp()
        };
        let events = [
            Event::timed("late", "Late", at(22, 15), at(22, 16), 0),
            Event::timed("night", "Night", at(21, 22), at(22, 2), 0),
            Event::all_day("fair", "Fair", date(2026, 9, 21), date(2026, 9, 23), 0),
            Event::timed("early", "Early", at(22, 9), at(22, 10), 0),
        ];
        let keys: Vec<&str> = on_day(&events, date(2026, 9, 22), &TimeZone::UTC)
            .iter()
            .map(|event| event.key.as_ref())
            .collect();
        assert_eq!(keys, ["fair", "night", "early", "late"]);
    }
}
