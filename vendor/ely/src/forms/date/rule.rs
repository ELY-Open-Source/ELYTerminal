use jiff::{
    ToSpan,
    civil::{Date, DateTime},
};

const MONTHS: [&str; 12] = [
    "JAN", "FEB", "MAR", "APR", "MAY", "JUN", "JUL", "AUG", "SEP", "OCT", "NOV", "DEC",
];
const DAYS: [&str; 7] = ["SUN", "MON", "TUE", "WED", "THU", "FRI", "SAT"];
const DAY_NAMES: [&str; 7] = [
    "Sunday",
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
];
const MONTH_NAMES: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

/// One field of a rule: the values it allows, whether they are all of them, and whether it began with `*`.
#[derive(Clone, Debug, PartialEq)]
struct Field {
    values: Vec<u8>,
    full: bool,
    star: bool,
    every: Option<u8>,
}

impl Field {
    fn has(&self, value: u8) -> bool {
        self.values.binary_search(&value).is_ok()
    }
}

fn number(text: &str, names: &[&str], offset: u8, what: &str) -> Result<u8, String> {
    if let Some(ix) = names
        .iter()
        .position(|name| name.eq_ignore_ascii_case(text))
    {
        return Ok(ix as u8 + offset);
    }
    text.parse::<u8>()
        .map_err(|_| format!("{what} {text:?} is not a number"))
}

/// Parses one field over `min..=max`: `*`, values, ranges, lists and `/` steps.
fn field(
    text: &str,
    min: u8,
    max: u8,
    names: &[&str],
    offset: u8,
    what: &str,
) -> Result<Field, String> {
    if text.is_empty() {
        return Err(format!("the {what} field is empty"));
    }
    let mut values = Vec::new();
    let mut every = None;
    for part in text.split(',') {
        let (span, step) = match part.split_once('/') {
            Some((span, step)) => {
                let step: u8 = step
                    .parse()
                    .ok()
                    .filter(|step| *step > 0)
                    .ok_or_else(|| format!("{what} step {step:?} must be a positive number"))?;
                (span, step)
            }
            None => (part, 1),
        };
        let (low, high) = match span {
            "*" => (min, max),
            _ => match span.split_once('-') {
                Some((low, high)) => (
                    number(low, names, offset, what)?,
                    number(high, names, offset, what)?,
                ),
                None => {
                    let low = number(span, names, offset, what)?;
                    (low, if step > 1 { max } else { low })
                }
            },
        };
        for bound in [low, high] {
            if !(min..=max).contains(&bound) {
                return Err(format!("{what} {bound} is outside {min} to {max}"));
            }
        }
        if low > high {
            return Err(format!("{what} range {low}-{high} runs backwards"));
        }
        if span == "*" && step > 1 && text.split(',').count() == 1 {
            every = Some(step);
        }
        values.extend((low..=high).step_by(step as usize));
    }
    values.sort_unstable();
    values.dedup();
    Ok(Field {
        full: values.len() == usize::from(max - min) + 1,
        values,
        star: text.starts_with('*'),
        every,
    })
}

/// A standard five-field cron rule: minute, hour, day of month, month, day of week.
#[derive(Clone, Debug, PartialEq)]
pub struct CronRule {
    minutes: Field,
    hours: Field,
    days: Field,
    months: Field,
    weekdays: Field,
}

impl CronRule {
    pub fn parse(source: &str) -> Result<Self, String> {
        let parts: Vec<&str> = source.split_whitespace().collect();
        let [minute, hour, day, month, weekday] = parts.as_slice() else {
            return Err(format!("a rule has five fields, not {}", parts.len()));
        };
        let mut weekdays = field(weekday, 0, 7, &DAYS, 0, "weekday")?;
        for value in &mut weekdays.values {
            if *value == 7 {
                *value = 0;
            }
        }
        weekdays.values.sort_unstable();
        weekdays.values.dedup();
        weekdays.full = weekdays.values.len() == 7;
        Ok(Self {
            minutes: field(minute, 0, 59, &[], 0, "minute")?,
            hours: field(hour, 0, 23, &[], 0, "hour")?,
            days: field(day, 1, 31, &[], 0, "day")?,
            months: field(month, 1, 12, &MONTHS, 1, "month")?,
            weekdays,
        })
    }

    /// As in Cronie: a day field that begins with `*` makes both day fields apply; otherwise either one does.
    fn runs_on(&self, date: Date) -> bool {
        let day = self.days.has(date.day() as u8);
        let weekday = self
            .weekdays
            .has(date.weekday().to_sunday_zero_offset() as u8);
        let dated = if self.days.star || self.weekdays.star {
            day && weekday
        } else {
            day || weekday
        };
        dated && self.months.has(date.month() as u8)
    }

    /// The next `count` times the rule fires after `after`, looking up to five years ahead.
    pub fn next(&self, after: DateTime, count: usize) -> Vec<DateTime> {
        let mut found = Vec::new();
        if count == 0 {
            return found;
        }
        let mut date = after.date();
        for _ in 0..366 * 5 {
            if self.runs_on(date) {
                for hour in &self.hours.values {
                    for minute in &self.minutes.values {
                        let at = date.at(*hour as i8, *minute as i8, 0, 0);
                        if at > after {
                            found.push(at);
                            if found.len() == count {
                                return found;
                            }
                        }
                    }
                }
            }
            match date.checked_add(1.day()) {
                Ok(next) => date = next,
                Err(_) => break,
            }
        }
        found
    }

    /// The rule in plain words, such as "At 09:00, Monday through Friday".
    pub fn describe(&self) -> String {
        let (minutes, hours) = (&self.minutes, &self.hours);
        let time = if minutes.full && hours.full {
            "Every minute".to_string()
        } else if let Some(every) = minutes.every {
            if hours.full {
                format!("Every {every} minutes")
            } else {
                format!("Every {every} minutes during hours {}", list(&hours.values))
            }
        } else if minutes.values.len() == 1 && hours.values.len() == 1 {
            format!("At {:02}:{:02}", hours.values[0], minutes.values[0])
        } else if minutes.values.len() == 1 && hours.full {
            format!("At minute {} of every hour", minutes.values[0])
        } else {
            format!(
                "At minute {} past hour {}",
                list(&minutes.values),
                list(&hours.values)
            )
        };
        let weekdays = weekday_words(&self.weekdays.values);
        let days = list(&self.days.values);
        let both = self.days.star || self.weekdays.star;
        let day = match (both, self.days.full, self.weekdays.full) {
            (_, true, true) | (false, true, _) | (false, _, true) => String::new(),
            (true, true, false) => format!(", {weekdays}"),
            (true, false, true) => format!(", on day {days} of the month"),
            (true, false, false) => {
                format!(", on day {days} of the month if it falls on {weekdays}")
            }
            (false, false, false) => format!(", on day {days} or {weekdays}"),
        };
        let month = if self.months.full {
            String::new()
        } else {
            let names: Vec<String> = self
                .months
                .values
                .iter()
                .map(|month| MONTH_NAMES[*month as usize - 1].to_string())
                .collect();
            format!(", in {}", join(&names))
        };
        format!("{time}{day}{month}")
    }
}

/// Sorted values in words, with a run of three or more as "a through b".
fn list(values: &[u8]) -> String {
    let mut words = Vec::new();
    let mut run = values.iter().copied().peekable();
    while let Some(first) = run.next() {
        let mut last = first;
        while run.peek() == Some(&(last + 1)) {
            last = run.next().expect("peeked");
        }
        match last - first {
            0 => words.push(first.to_string()),
            1 => words.extend([first.to_string(), last.to_string()]),
            _ => words.push(format!("{first} through {last}")),
        }
    }
    join(&words)
}

/// "a", "a and b", "a, b and c".
fn join(words: &[String]) -> String {
    match words {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

/// Weekdays in words, Monday first, with a run of three or more as a span.
pub(crate) fn weekday_words(values: &[u8]) -> String {
    let mut monday_first: Vec<u8> = values.iter().map(|day| (day + 6) % 7).collect();
    monday_first.sort_unstable();
    let name = |monday_zero: u8| DAY_NAMES[((monday_zero + 1) % 7) as usize].to_string();
    let run = monday_first.windows(2).all(|pair| pair[1] == pair[0] + 1);
    if run && monday_first.len() >= 3 {
        return format!(
            "{} through {}",
            name(monday_first[0]),
            name(monday_first[monday_first.len() - 1])
        );
    }
    join(&monday_first.into_iter().map(name).collect::<Vec<_>>())
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;

    use super::CronRule;

    #[test]
    fn rules_parse_lists_ranges_steps_and_names() {
        assert!(CronRule::parse("*/15 9-17 * * MON-FRI").is_ok());
        assert_eq!(
            CronRule::parse("* * *"),
            Err("a rule has five fields, not 3".into())
        );
        assert_eq!(
            CronRule::parse("61 * * * *"),
            Err("minute 61 is outside 0 to 59".into())
        );
        assert_eq!(
            CronRule::parse("0 5-2 * * *"),
            Err("hour range 5-2 runs backwards".into())
        );
        assert_eq!(
            CronRule::parse("*/0 * * * *"),
            Err("minute step \"0\" must be a positive number".into())
        );
    }

    #[test]
    fn next_runs_follow_the_rule() {
        let rule = CronRule::parse("30 9 * * 1-5").unwrap();
        let friday_noon = date(2026, 9, 25).at(12, 0, 0, 0);
        assert_eq!(
            rule.next(friday_noon, 2),
            [
                date(2026, 9, 28).at(9, 30, 0, 0),
                date(2026, 9, 29).at(9, 30, 0, 0)
            ]
        );
        let either = CronRule::parse("0 0 1 * 0").unwrap();
        assert_eq!(
            either.next(date(2026, 9, 25).at(0, 0, 0, 0), 2),
            [
                date(2026, 9, 27).at(0, 0, 0, 0),
                date(2026, 10, 1).at(0, 0, 0, 0)
            ]
        );
        let mondays = CronRule::parse("0 0 */1 * MON").unwrap();
        assert_eq!(
            mondays.next(date(2026, 9, 25).at(0, 0, 0, 0), 1),
            [date(2026, 9, 28).at(0, 0, 0, 0)]
        );
        assert!(either.next(date(2026, 9, 25).at(0, 0, 0, 0), 0).is_empty());
        assert!(
            CronRule::parse("0 0 30 2 *")
                .unwrap()
                .next(date(2026, 1, 1).at(0, 0, 0, 0), 1)
                .is_empty()
        );
    }

    #[test]
    fn rules_read_in_plain_words() {
        let words = |rule: &str| CronRule::parse(rule).unwrap().describe();
        assert_eq!(words("0 9 * * 1-5"), "At 09:00, Monday through Friday");
        assert_eq!(words("*/15 * * * *"), "Every 15 minutes");
        assert_eq!(
            words("0 0 1 1,7 *"),
            "At 00:00, on day 1 of the month, in January and July"
        );
        assert_eq!(
            words("5 * * * 0,6"),
            "At minute 5 of every hour, Saturday and Sunday"
        );
        assert_eq!(
            words("*/15 9-17 * * 1-5"),
            "Every 15 minutes during hours 9 through 17, Monday through Friday"
        );
        assert_eq!(words("0 0 1-31 * MON"), "At 00:00");
        assert_eq!(words("0 0 1 * 0"), "At 00:00, on day 1 or Sunday");
        assert_eq!(
            words("0 0 */2 * MON"),
            "At 00:00, on day 1, 3, 5, 7, 9, 11, 13, 15, 17, 19, 21, 23, 25, 27, 29 and 31 of the month if it falls on Monday"
        );
        assert_eq!(
            words("0,30 8-10,14 * * *"),
            "At minute 0 and 30 past hour 8 through 10 and 14"
        );
    }
}
