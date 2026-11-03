use jiff::{
    ToSpan,
    civil::{Date, Weekday},
};

use crate::forms::{show_date, week_start, weekday_words};

/// How often an event comes back.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Frequency {
    Daily,
    Weekly,
    Monthly,
    Yearly,
}

/// When a run of repeats stops: never, after a day, or after a count of them, the first among them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ends {
    Never,
    On(Date),
    After(u16),
}

/// An event's repeats: how often, every how many, on which weekdays for a weekly one, and when they stop.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Recurrence {
    pub frequency: Frequency,
    pub interval: u16,
    pub weekdays: Vec<Weekday>,
    pub ends: Ends,
}

/// "1st", "2nd", "3rd", "11th", "22nd".
fn ordinal(day: i8) -> String {
    let suffix = match (day % 10, day % 100) {
        (_, 11..=13) => "th",
        (1, _) => "st",
        (2, _) => "nd",
        (3, _) => "rd",
        _ => "th",
    };
    format!("{day}{suffix}")
}

impl Recurrence {
    /// Every one, on the first day's weekday for a weekly one, without end.
    pub fn new(frequency: Frequency) -> Self {
        Self {
            frequency,
            interval: 1,
            weekdays: Vec::new(),
            ends: Ends::Never,
        }
    }

    /// The weekdays a weekly one falls on, Monday first: its own, or the first day's.
    fn on(&self, first: Date) -> Vec<Weekday> {
        let mut days = match self.weekdays.is_empty() {
            true => vec![first.weekday()],
            false => self.weekdays.clone(),
        };
        days.sort_by_key(|day| day.to_monday_zero_offset());
        days.dedup();
        days
    }

    /// In words, for an event that starts on `first`: "Every 2 weeks on Monday and Thursday, until Dec 31, 2026".
    pub fn describe(&self, first: Date) -> String {
        assert!(
            self.interval > 0,
            "a recurrence comes back every one or more"
        );
        let every = |one: &str, many: &str| match self.interval {
            1 => format!("Every {one}"),
            n => format!("Every {n} {many}"),
        };
        let what = match self.frequency {
            Frequency::Daily => every("day", "days"),
            Frequency::Weekly => {
                let days: Vec<u8> = self
                    .on(first)
                    .iter()
                    .map(|day| day.to_sunday_zero_offset() as u8)
                    .collect();
                format!("{} on {}", every("week", "weeks"), weekday_words(&days))
            }
            Frequency::Monthly => format!(
                "{} on the {}",
                every("month", "months"),
                ordinal(first.day())
            ),
            Frequency::Yearly => {
                format!("{} on {}", every("year", "years"), first.strftime("%B %-d"))
            }
        };
        match self.ends {
            Ends::Never => what,
            Ends::On(last) => format!("{what}, until {}", show_date(last)),
            Ends::After(1) => format!("{what}, once"),
            Ends::After(count) => format!("{what}, {count} times"),
        }
    }

    /// The first `limit` days it falls on, `first` among them, before it ends. A month or a year without the first day's date is passed over.
    pub fn days(&self, first: Date, limit: usize) -> Vec<Date> {
        assert!(
            self.interval > 0,
            "a recurrence comes back every one or more"
        );
        let cap = match self.ends {
            Ends::After(count) => limit.min(usize::from(count)),
            _ => limit,
        };
        let within = |day: Date| match self.ends {
            Ends::On(last) => day <= last,
            _ => true,
        };
        let step = i64::from(self.interval);
        let mut days = Vec::with_capacity(cap);
        let mut round: i64 = 0;
        while days.len() < cap {
            let candidates: Option<Vec<Date>> = match self.frequency {
                Frequency::Daily => first
                    .checked_add((round * step).days())
                    .ok()
                    .map(|day| vec![day]),
                Frequency::Weekly => week_start(first)
                    .checked_add((round * step).weeks())
                    .ok()
                    .map(|monday| {
                        self.on(first)
                            .into_iter()
                            .filter_map(|day| {
                                monday
                                    .checked_add(i64::from(day.to_monday_zero_offset()).days())
                                    .ok()
                            })
                            .filter(|day| *day >= first)
                            .collect()
                    }),
                Frequency::Monthly => first
                    .first_of_month()
                    .checked_add((round * step).months())
                    .ok()
                    .map(|month| {
                        Date::new(month.year(), month.month(), first.day())
                            .into_iter()
                            .collect()
                    }),
                Frequency::Yearly => i16::try_from(i64::from(first.year()) + round * step)
                    .ok()
                    .filter(|year| *year <= Date::MAX.year())
                    .map(|year| {
                        Date::new(year, first.month(), first.day())
                            .into_iter()
                            .collect()
                    }),
            };
            let Some(candidates) = candidates else {
                log::warn!(
                    "recurrence: it runs past the last day jiff holds after {} days",
                    days.len()
                );
                return days;
            };
            for day in candidates {
                if !within(day) || days.len() == cap {
                    return days;
                }
                days.push(day);
            }
            round += 1;
        }
        days
    }
}

#[cfg(test)]
mod tests {
    use jiff::civil::{Weekday, date};

    use super::{Ends, Frequency, Recurrence, ordinal};

    fn every(frequency: Frequency, interval: u16) -> Recurrence {
        Recurrence {
            interval,
            ..Recurrence::new(frequency)
        }
    }

    #[test]
    fn a_rule_stops_where_the_calendar_ends() {
        let yearly = every(Frequency::Yearly, 99).days(date(9900, 2, 28), 5);
        assert_eq!(yearly, [date(9900, 2, 28), date(9999, 2, 28)]);
        let monthly = every(Frequency::Monthly, 99).days(date(9990, 1, 15), 5);
        assert_eq!(monthly, [date(9990, 1, 15), date(9998, 4, 15)]);
        assert_eq!(
            every(Frequency::Daily, 99).days(date(9999, 12, 1), 5).len(),
            1
        );
        assert_eq!(
            every(Frequency::Weekly, 99)
                .days(date(9999, 12, 1), 5)
                .len(),
            1
        );
    }

    #[test]
    fn ordinals_read_as_spoken() {
        let said: Vec<String> = [1, 2, 3, 4, 11, 12, 13, 21, 22, 23, 31]
            .map(ordinal)
            .to_vec();
        assert_eq!(
            said,
            [
                "1st", "2nd", "3rd", "4th", "11th", "12th", "13th", "21st", "22nd", "23rd", "31st"
            ]
        );
    }

    #[test]
    fn a_recurrence_reads_in_words() {
        let sunday = date(2026, 9, 27);
        assert_eq!(every(Frequency::Daily, 1).describe(sunday), "Every day");
        assert_eq!(every(Frequency::Daily, 3).describe(sunday), "Every 3 days");
        assert_eq!(
            every(Frequency::Weekly, 1).describe(sunday),
            "Every week on Sunday"
        );
        let twice = Recurrence {
            weekdays: vec![Weekday::Thursday, Weekday::Monday],
            ..every(Frequency::Weekly, 2)
        };
        assert_eq!(
            twice.describe(sunday),
            "Every 2 weeks on Monday and Thursday"
        );
        let workdays = Recurrence {
            weekdays: vec![
                Weekday::Monday,
                Weekday::Tuesday,
                Weekday::Wednesday,
                Weekday::Thursday,
                Weekday::Friday,
            ],
            ..every(Frequency::Weekly, 1)
        };
        assert_eq!(
            workdays.describe(sunday),
            "Every week on Monday through Friday"
        );
        assert_eq!(
            every(Frequency::Monthly, 1).describe(sunday),
            "Every month on the 27th"
        );
        assert_eq!(
            every(Frequency::Yearly, 1).describe(sunday),
            "Every year on September 27"
        );
        let until = Recurrence {
            ends: Ends::On(date(2026, 12, 31)),
            ..every(Frequency::Daily, 1)
        };
        assert_eq!(until.describe(sunday), "Every day, until Dec 31, 2026");
        let counted = Recurrence {
            ends: Ends::After(10),
            ..every(Frequency::Monthly, 3)
        };
        assert_eq!(
            counted.describe(sunday),
            "Every 3 months on the 27th, 10 times"
        );
    }

    #[test]
    fn a_weekly_one_falls_on_its_days_from_the_first() {
        let twice = Recurrence {
            weekdays: vec![Weekday::Monday, Weekday::Thursday],
            ..every(Frequency::Weekly, 2)
        };
        assert_eq!(
            twice.days(date(2026, 9, 27), 4),
            [
                date(2026, 10, 5),
                date(2026, 10, 8),
                date(2026, 10, 19),
                date(2026, 10, 22)
            ],
            "weeks count from the start's own, whose Monday and Thursday have passed"
        );
        let weekly = Recurrence {
            interval: 1,
            ..twice
        };
        assert_eq!(
            weekly.days(date(2026, 9, 27), 3),
            [date(2026, 9, 28), date(2026, 10, 1), date(2026, 10, 5)]
        );
    }

    #[test]
    fn a_month_or_year_without_the_date_is_passed_over() {
        assert_eq!(
            every(Frequency::Monthly, 1).days(date(2027, 1, 31), 3),
            [date(2027, 1, 31), date(2027, 3, 31), date(2027, 5, 31)]
        );
        assert_eq!(
            every(Frequency::Yearly, 1).days(date(2028, 2, 29), 2),
            [date(2028, 2, 29), date(2032, 2, 29)]
        );
    }

    #[test]
    fn repeats_stop_after_a_count_or_a_day() {
        let counted = Recurrence {
            ends: Ends::After(3),
            ..every(Frequency::Daily, 2)
        };
        assert_eq!(
            counted.days(date(2026, 9, 27), 10),
            [date(2026, 9, 27), date(2026, 9, 29), date(2026, 10, 1)]
        );
        let until = Recurrence {
            ends: Ends::On(date(2026, 10, 10)),
            ..every(Frequency::Weekly, 1)
        };
        assert_eq!(
            until.days(date(2026, 9, 27), 10),
            [date(2026, 9, 27), date(2026, 10, 4)]
        );
    }
}
