mod agenda;
mod allday;
mod availability;
mod card;
mod chip;
mod days;
mod editor;
mod event;
mod grid;
mod head;
mod hours;
mod month;
mod now;
mod people;
mod popover;
mod recurrence;
mod reminders;
mod repeat;
mod weeks;
mod year;

#[cfg(all(test, feature = "test-support"))]
mod tests;

pub use agenda::AgendaView;
pub use allday::AllDayRow;
pub use availability::AvailabilityPicker;
pub use card::EventCard;
pub use chip::EventChip;
pub use days::{CalendarDayView, CalendarDays, CalendarWeekView};
pub use editor::{EventDraft, EventEditor};
pub use event::{Event, When};
pub use grid::{TimeGrid, TimezoneOverlay};
pub use month::CalendarMonthView;
pub use now::CurrentTimeIndicator;
pub use people::{Answer, Attendee, AttendeeList};
pub use popover::EventPopover;
pub use recurrence::RecurrenceEditor;
pub use reminders::ReminderPicker;
pub use repeat::{Ends, Frequency, Recurrence};
pub use year::YearView;

/// The days a calendar shows: jiff's range less a year at each end, so the weeks around any of them fit.
pub(crate) fn within(day: jiff::civil::Date) -> bool {
    (jiff::civil::date(-9998, 1, 1)..=jiff::civil::date(9998, 12, 31)).contains(&day)
}

/// `at` moved `by` while the calendar holds it; a step past its ends logs and goes nowhere.
pub(crate) fn stepped(
    at: jiff::civil::Date,
    by: jiff::Span,
    owner: &str,
) -> Option<jiff::civil::Date> {
    let next = at.checked_add(by).ok().filter(|day| within(*day));
    if next.is_none() {
        log::warn!("{owner}: {at} moved {by} leaves the calendar; it stays");
    }
    next
}
