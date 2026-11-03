use gpui::SharedString;
use jiff::{SignedDuration, Timestamp, civil::Date, tz::TimeZone};

use crate::typography::format;

/// When an event happens: from one moment to a later one, or whole days from the first to the last.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum When {
    Timed { start: Timestamp, end: Timestamp },
    AllDay { first: Date, last: Date },
}

/// Something on a calendar: its key, its title, when it happens, its hue among the theme's chart colors, and where.
#[derive(Clone, Debug, PartialEq)]
pub struct Event {
    pub key: SharedString,
    pub title: SharedString,
    pub when: When,
    pub hue: usize,
    pub place: Option<SharedString>,
}

impl Event {
    /// From `start` to `end`, which comes later.
    pub fn timed(
        key: impl Into<SharedString>,
        title: impl Into<SharedString>,
        start: Timestamp,
        end: Timestamp,
        hue: usize,
    ) -> Self {
        let key = key.into();
        assert!(end > start, "event {key}: it ends before it starts");
        Self {
            key,
            title: title.into(),
            when: When::Timed { start, end },
            hue,
            place: None,
        }
    }

    /// Whole days from `first` to `last`.
    pub fn all_day(
        key: impl Into<SharedString>,
        title: impl Into<SharedString>,
        first: Date,
        last: Date,
        hue: usize,
    ) -> Self {
        let key = key.into();
        assert!(
            last >= first,
            "event {key}: its last day comes before its first"
        );
        Self {
            key,
            title: title.into(),
            when: When::AllDay { first, last },
            hue,
            place: None,
        }
    }

    pub fn place(mut self, place: impl Into<SharedString>) -> Self {
        self.place = Some(place.into());
        self
    }

    /// The first and last days it touches in `zone`. An end at midnight touches no part of that day.
    pub(crate) fn days(&self, zone: &TimeZone) -> (Date, Date) {
        match self.when {
            When::AllDay { first, last } => (first, last),
            When::Timed { start, end } => {
                let first = start.to_zoned(zone.clone()).date();
                if end <= start {
                    return (first, first);
                }
                let before_end = end - SignedDuration::from_nanos(1);
                (first, before_end.to_zoned(zone.clone()).date().max(first))
            }
        }
    }

    /// Whether it shows as a bar across days: all day, or timed past midnight.
    pub(crate) fn spans(&self, zone: &TimeZone) -> bool {
        let (first, last) = self.days(zone);
        matches!(self.when, When::AllDay { .. }) || first != last
    }
}

/// Events with each key asserted once.
pub(crate) fn unique(events: impl IntoIterator<Item = Event>) -> Vec<Event> {
    let events: Vec<Event> = events.into_iter().collect();
    for (ix, event) in events.iter().enumerate() {
        let twin = events[..ix].iter().any(|other| other.key == event.key);
        assert!(!twin, "event {} twice", event.key);
    }
    events
}

/// When `event` happens, read in `zone`: its hours, its hours with their days across midnight, all day, or its span of days.
pub(crate) fn hours(event: &Event, zone: &TimeZone) -> String {
    let dash = " \u{2013} ";
    match event.when {
        When::AllDay { first, last } if first == last => "All day".into(),
        When::AllDay { first, last } if first.month() == last.month() => {
            format!("{}{dash}{}", first.strftime("%b %-d"), last.strftime("%-d"))
        }
        When::AllDay { first, last } => {
            format!(
                "{}{dash}{}",
                first.strftime("%b %-d"),
                last.strftime("%b %-d")
            )
        }
        When::Timed { start, end } => {
            let pattern = match event.spans(zone) {
                true => "%a %H:%M",
                false => "%H:%M",
            };
            let at = |moment| format::datetime(moment, zone, pattern).expect("a fixed pattern");
            format!("{}{dash}{}", at(start), at(end))
        }
    }
}

#[cfg(test)]
mod tests {
    use jiff::{Timestamp, civil::date, tz::TimeZone};

    use super::{Event, hours};

    fn at(day: i8, hour: i8, minute: i8) -> Timestamp {
        date(2026, 9, day)
            .at(hour, minute, 0, 0)
            .to_zoned(TimeZone::UTC)
            .expect("a UTC time")
            .timestamp()
    }

    #[test]
    fn hours_read_by_the_clock_across_midnight_and_by_days() {
        let utc = &TimeZone::UTC;
        let said = |event: Event| hours(&event, utc);
        assert_eq!(
            said(Event::timed("a", "A", at(22, 9, 30), at(22, 10, 15), 0)),
            "09:30 \u{2013} 10:15"
        );
        assert_eq!(
            said(Event::timed("b", "B", at(21, 22, 0), at(22, 2, 0), 0)),
            "Mon 22:00 \u{2013} Tue 02:00"
        );
        let day = |first: i8, last| (date(2026, 9, first), last);
        let (first, last) = day(22, date(2026, 9, 22));
        assert_eq!(said(Event::all_day("c", "C", first, last, 0)), "All day");
        let (first, last) = day(22, date(2026, 9, 24));
        assert_eq!(
            said(Event::all_day("d", "D", first, last, 0)),
            "Sep 22 \u{2013} 24"
        );
        let (first, last) = day(29, date(2026, 10, 2));
        assert_eq!(
            said(Event::all_day("e", "E", first, last, 0)),
            "Sep 29 \u{2013} Oct 2"
        );
    }
}
