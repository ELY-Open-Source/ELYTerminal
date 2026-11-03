mod calendar;
mod cron;
mod period;
mod picker;
mod relative;
mod rule;
mod time;
mod week;
mod zone;

use jiff::{
    Timestamp, Zoned,
    civil::{Date, Time},
};

use crate::typography::format::system_zone;

pub use calendar::Calendar;
pub(crate) use calendar::{month_grid, week_start};
pub use cron::CronEditor;
pub use period::{MonthPicker, QuarterPicker, YearPicker};
pub use picker::{DatePicker, DateRangePicker};
pub(crate) use picker::{Face, dropdown, picker_field};
pub use relative::{RelativeDatePicker, RelativeRange};
pub use rule::CronRule;
pub(crate) use rule::weekday_words;
pub use time::{DateTimePicker, DurationPicker, TimePicker};
pub use week::WeekPicker;
pub use zone::TimezoneSelect;

/// The system's clock in its own zone, UTC when that zone is unknown.
pub(crate) fn zoned_now() -> Zoned {
    Timestamp::now().to_zoned(system_zone("date pickers"))
}

/// A day as "Sep 25, 2026".
pub(crate) fn show_date(date: Date) -> String {
    date.strftime("%b %-d, %Y").to_string()
}

/// A time as "09:30".
pub(crate) fn show_time(time: Time) -> String {
    time.strftime("%H:%M").to_string()
}

/// A span as "Sep 3 – 17, 2026", naming only what differs.
pub(crate) fn show_span(start: Date, end: Date) -> String {
    let dash = " \u{2013} ";
    if start == end {
        show_date(start)
    } else if start.year() != end.year() {
        format!("{}{dash}{}", show_date(start), show_date(end))
    } else if start.month() != end.month() {
        format!("{}{dash}{}", start.strftime("%b %-d"), show_date(end))
    } else {
        format!(
            "{}{dash}{}",
            start.strftime("%b %-d"),
            end.strftime("%-d, %Y")
        )
    }
}

#[cfg(test)]
mod tests {
    use jiff::civil::{date, time};

    use super::{show_date, show_span, show_time};

    #[test]
    fn dates_times_and_spans_read_short() {
        assert_eq!(show_date(date(2026, 9, 5)), "Sep 5, 2026");
        assert_eq!(show_time(time(9, 5, 0, 0)), "09:05");
        assert_eq!(
            show_span(date(2026, 9, 3), date(2026, 9, 17)),
            "Sep 3 \u{2013} 17, 2026"
        );
        assert_eq!(
            show_span(date(2026, 9, 28), date(2026, 10, 2)),
            "Sep 28 \u{2013} Oct 2, 2026"
        );
        assert_eq!(
            show_span(date(2026, 12, 30), date(2027, 1, 2)),
            "Dec 30, 2026 \u{2013} Jan 2, 2027"
        );
    }
}
