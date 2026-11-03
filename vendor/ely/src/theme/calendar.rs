use gpui::Rems;

use super::{Theme, tokens::px_to_rems};

/// Calendar's measures.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CalendarSizes {
    /// An event's chip in a day, tall.
    pub chip: Rems,
    /// Rows of events a month's day holds under its number.
    pub day_rows: usize,
    /// A day in a year's small months, square.
    pub mini_day: Rems,
    /// An hour in a time grid, tall.
    pub hour: Rems,
    /// A time grid's column of hours, wide.
    pub gutter: Rems,
    /// An event's details in a panel, wide.
    pub card: Rems,
}

impl Theme {
    pub fn calendar(&self) -> CalendarSizes {
        CalendarSizes {
            chip: px_to_rems(20.0),
            day_rows: 3,
            mini_day: px_to_rems(28.0),
            hour: px_to_rems(48.0),
            gutter: px_to_rems(56.0),
            card: px_to_rems(320.0),
        }
    }
}
