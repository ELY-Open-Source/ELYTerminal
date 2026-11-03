use std::rc::Rc;

use gpui::{App, ElementId, IntoElement, RenderOnce, SharedString, Window};
use jiff::{ToSpan, civil::Date};

use super::{
    super::{Choice, Select},
    show_span, zoned_now,
};
use crate::theme::ControlSize;

/// A span named from today, such as the last seven days.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RelativeRange {
    Today,
    Yesterday,
    Last7Days,
    Last30Days,
    ThisMonth,
    LastMonth,
    ThisQuarter,
    ThisYear,
}

impl RelativeRange {
    pub const ALL: [Self; 8] = [
        Self::Today,
        Self::Yesterday,
        Self::Last7Days,
        Self::Last30Days,
        Self::ThisMonth,
        Self::LastMonth,
        Self::ThisQuarter,
        Self::ThisYear,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Today => "Today",
            Self::Yesterday => "Yesterday",
            Self::Last7Days => "Last 7 days",
            Self::Last30Days => "Last 30 days",
            Self::ThisMonth => "This month",
            Self::LastMonth => "Last month",
            Self::ThisQuarter => "This quarter",
            Self::ThisYear => "This year",
        }
    }

    /// The first and last day it covers, counted from `today`.
    pub fn span(self, today: Date) -> (Date, Date) {
        let first_of =
            |month: i8| Date::new(today.year(), month, 1).expect("the first of a month exists");
        match self {
            Self::Today => (today, today),
            Self::Yesterday => {
                let day = today.saturating_sub(1.day());
                (day, day)
            }
            Self::Last7Days => (today.saturating_sub(6.days()), today),
            Self::Last30Days => (today.saturating_sub(29.days()), today),
            Self::ThisMonth => (today.first_of_month(), today.last_of_month()),
            Self::LastMonth => {
                let last = today.first_of_month().saturating_sub(1.day());
                (last.first_of_month(), last)
            }
            Self::ThisQuarter => {
                let start = first_of((today.month() - 1) / 3 * 3 + 1);
                (start, start.saturating_add(2.months()).last_of_month())
            }
            Self::ThisYear => (first_of(1), first_of(12).last_of_month()),
        }
    }
}

type OnRange = Rc<dyn Fn(RelativeRange, (Date, Date), &mut Window, &mut App)>;

/// A select of spans named from today; each row shows the days it covers.
#[derive(IntoElement)]
pub struct RelativeDatePicker {
    id: ElementId,
    value: Option<RelativeRange>,
    today: Option<Date>,
    size: ControlSize,
    on_change: Option<OnRange>,
}

impl RelativeDatePicker {
    pub fn new(id: impl Into<ElementId>, value: Option<RelativeRange>) -> Self {
        Self {
            id: id.into(),
            value,
            today: None,
            size: ControlSize::default(),
            on_change: None,
        }
    }

    /// The date treated as today. Defaults to the system's date.
    pub fn today(mut self, date: Date) -> Self {
        self.today = Some(date);
        self
    }

    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    pub fn on_change(
        mut self,
        handler: impl Fn(RelativeRange, (Date, Date), &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for RelativeDatePicker {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let today = self.today.unwrap_or_else(|| zoned_now().date());
        let choices = RelativeRange::ALL.map(|range| {
            let (start, end) = range.span(today);
            Choice::new(range.label(), range.label()).note(show_span(start, end))
        });
        let on_change = self.on_change;
        let mut select = Select::new(self.id, choices)
            .placeholder("Any time")
            .size(self.size)
            .on_change(move |label: &SharedString, window, cx| {
                let range = RelativeRange::ALL
                    .into_iter()
                    .find(|range| range.label() == label.as_ref())
                    .expect("a label from the list");
                if let Some(on_change) = &on_change {
                    on_change(range, range.span(today), window, cx);
                }
            });
        if let Some(value) = self.value {
            select = select.selected(value.label());
        }
        select
    }
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;

    use super::RelativeRange;

    #[test]
    fn spans_count_back_from_today() {
        let today = date(2026, 9, 25);
        assert_eq!(
            RelativeRange::Last7Days.span(today),
            (date(2026, 9, 19), today)
        );
        assert_eq!(
            RelativeRange::LastMonth.span(today),
            (date(2026, 8, 1), date(2026, 8, 31))
        );
        assert_eq!(
            RelativeRange::ThisQuarter.span(today),
            (date(2026, 7, 1), date(2026, 9, 30))
        );
        assert_eq!(
            RelativeRange::ThisYear.span(today),
            (date(2026, 1, 1), date(2026, 12, 31))
        );
        assert_eq!(
            RelativeRange::Yesterday.span(date(2026, 3, 1)).0,
            date(2026, 2, 28)
        );
    }
}
