use std::rc::Rc;

use gpui::{App, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce, Window};
use jiff::{
    ToSpan,
    civil::{Date, ISOWeekDate, Weekday},
};

use super::{
    super::options::{float, surface},
    picker::{CALENDAR_ROWS, Face, dropdown, picker_field, popped},
};
use crate::{primitives::IconName, theme::ControlSize};

type OnWeek = Rc<dyn Fn((i16, i8), &mut Window, &mut App)>;

/// The Monday of an ISO week.
pub(crate) fn monday_of((year, week): (i16, i8)) -> Date {
    ISOWeekDate::new(year, week, Weekday::Monday)
        .unwrap_or_else(|error| panic!("week {week} of {year} does not exist: {error}"))
        .date()
}

/// A calendar that picks whole weeks. The value is an ISO year and week.
#[derive(IntoElement)]
pub struct WeekPicker {
    id: ElementId,
    value: Option<(i16, i8)>,
    today: Option<Date>,
    on_change: Option<OnWeek>,
}

impl WeekPicker {
    pub fn new(id: impl Into<ElementId>, value: Option<(i16, i8)>) -> Self {
        Self {
            id: id.into(),
            value,
            today: None,
            on_change: None,
        }
    }

    /// The date treated as now. Defaults to the system's date.
    pub fn today(mut self, date: Date) -> Self {
        self.today = Some(date);
        self
    }

    pub fn on_change(
        mut self,
        handler: impl Fn((i16, i8), &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for WeekPicker {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let (id, value, fixed, on_change) =
            (self.id.clone(), self.value, self.today, self.on_change);
        let state = dropdown(&id, window, cx);
        picker_field(
            self.id,
            Face {
                icon: IconName::CalendarRange,
                shown: value.map(|(year, week)| format!("Week {week}, {year}").into()),
                placeholder: "Pick a week".into(),
                size: ControlSize::default(),
                disabled: false,
            },
            move |close, anchor, window, cx| {
                let mut calendar = popped(&id, &state, (None, None, fixed), cx).week();
                if let Some(value) = value {
                    let monday = monday_of(value);
                    calendar = calendar.span(monday, monday.saturating_add(6.days()));
                }
                let (escape, out, on_change) = (close.clone(), close.clone(), on_change.clone());
                float(
                    id.clone(),
                    anchor,
                    CALENDAR_ROWS,
                    surface((id.clone(), "popup"), cx)
                        .on_mouse_down_out(move |_, window, cx| out(window, cx))
                        .child(calendar.on_escape(escape).on_pick(move |date, window, cx| {
                            let week = date.iso_week_date();
                            log::info!("week picker: {}-W{:02}", week.year(), week.week());
                            if let Some(on_change) = &on_change {
                                on_change((week.year(), week.week()), window, cx);
                            }
                            close(window, cx);
                        })),
                    window,
                    cx,
                )
            },
            window,
            cx,
        )
    }
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;

    use super::monday_of;

    #[test]
    fn iso_weeks_start_on_their_monday() {
        assert_eq!(monday_of((2026, 39)), date(2026, 9, 21));
        assert_eq!(monday_of((2027, 1)), date(2027, 1, 4));
    }
}
