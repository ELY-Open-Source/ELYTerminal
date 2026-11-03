use std::{collections::BTreeMap, rc::Rc};

use gpui::{
    AnyElement, App, ElementId, FontWeight, Hsla, InteractiveElement, IntoElement, MouseButton,
    ParentElement, RenderOnce, Styled, Window, div, prelude::*,
};
use jiff::{Timestamp, ToSpan, civil::Date, tz::TimeZone};

use super::{
    chip::tint,
    event::{Event, unique},
    head::{OnDay, Step, head},
};
use crate::{
    forms::month_grid,
    layout::seeded::use_seeded,
    primitives::tab_stop,
    theme::{ActiveTheme, TextSize},
    typography::{format, fresh},
};

const INITIALS: [&str; 7] = ["M", "T", "W", "T", "F", "S", "S"];

type OnYear = Rc<dyn Fn(i16, &mut Window, &mut App)>;

/// The hue of the first event on each day of `year` in `zone`.
fn marks(events: &[Event], year: i16, zone: &TimeZone) -> BTreeMap<Date, usize> {
    let mut marks = BTreeMap::new();
    for (ix, event) in events.iter().enumerate() {
        let (first, last) = event.days(zone);
        let mut day = first;
        loop {
            if day.year() == year {
                marks.entry(day).or_insert(ix);
            }
            if day >= last {
                break;
            }
            day = day.tomorrow().expect("a day before the last has one after");
        }
    }
    marks
}

/// A year in twelve small months; a day with something on it wears a dot in its first event's hue. Arrows move a day, Page keys a month, and Enter or a double press opens the day.
#[derive(IntoElement)]
pub struct YearView {
    id: ElementId,
    year: i16,
    events: Vec<Event>,
    today: Option<Date>,
    zone: Option<TimeZone>,
    on_day: Option<OnDay>,
    on_year: Option<OnYear>,
}

impl YearView {
    pub fn new(
        id: impl Into<ElementId>,
        year: i16,
        events: impl IntoIterator<Item = Event>,
    ) -> Self {
        Self {
            id: id.into(),
            year,
            events: unique(events),
            today: None,
            zone: None,
            on_day: None,
            on_year: None,
        }
    }

    /// The day marked as today; the zone's own otherwise.
    pub fn today(mut self, date: Date) -> Self {
        self.today = Some(date);
        self
    }

    /// The zone its days read in; the system's otherwise.
    pub fn zone(mut self, zone: TimeZone) -> Self {
        self.zone = Some(zone);
        self
    }

    /// Gets a day opened: Enter on it or a double press.
    pub fn on_day(mut self, handler: impl Fn(Date, &mut Window, &mut App) + 'static) -> Self {
        self.on_day = Some(Rc::new(handler));
        self
    }

    /// Gets each year turned to.
    pub fn on_year(mut self, handler: impl Fn(i16, &mut Window, &mut App) + 'static) -> Self {
        self.on_year = Some(Rc::new(handler));
        self
    }
}

/// One small month: its name, the weekdays, and six weeks with only its own days shown.
struct Mini<'a> {
    id: &'a ElementId,
    today: Date,
    cursor: Option<Date>,
    marks: &'a BTreeMap<Date, Hsla>,
    pick: &'a OnDay,
    open: &'a OnDay,
}

impl Mini<'_> {
    fn month(&self, first: Date, cx: &App) -> AnyElement {
        let theme = cx.theme();
        let colors = &theme.colors;
        let cell = theme.calendar().mini_day;
        let initials = INITIALS.map(|initial| {
            div()
                .size(cell)
                .flex()
                .items_center()
                .justify_center()
                .text_color(colors.fg_subtle)
                .child(initial)
        });
        let days = month_grid(first).into_iter().map(|day| {
            if day.first_of_month() != first {
                return div().size(cell).into_any_element();
            }
            let (pick, open) = (self.pick.clone(), self.open.clone());
            let today = day == self.today;
            div()
                .id((self.id.clone(), format!("day-{day}")))
                .debug_selector(move || format!("year-day {day}"))
                .relative()
                .size(cell)
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .rounded_full()
                .border_1()
                .border_color(match self.cursor == Some(day) {
                    true => colors.focus,
                    false => gpui::transparent_black(),
                })
                .when(today, |day| day.bg(colors.accent))
                .text_color(match today {
                    true => colors.on_accent,
                    false => colors.fg,
                })
                .when(today, |day| day.font_weight(FontWeight::SEMIBOLD))
                .cursor_pointer()
                .on_mouse_down(MouseButton::Left, move |event, window, cx| {
                    if event.click_count == 2 {
                        open(day, window, cx);
                    } else {
                        pick(day, window, cx);
                    }
                })
                .child(day.day().to_string())
                .children(self.marks.get(&day).map(|tint| {
                    div()
                        .absolute()
                        .bottom_0p5()
                        .size(theme.status_dot())
                        .rounded_full()
                        .bg(*tint)
                }))
                .into_any_element()
        });
        div()
            .flex_none()
            .w(cell * 7.0)
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .px_1()
                    .text_size(theme.text_size(TextSize::Sm))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(colors.fg)
                    .child(first.strftime("%B").to_string()),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .text_size(theme.text_size(TextSize::Xs))
                    .children(initials)
                    .children(days.collect::<Vec<_>>()),
            )
            .into_any_element()
    }
}

impl RenderOnce for YearView {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        if self.today.is_none() {
            fresh((self.id.clone(), "clock"), window, cx);
        }
        let zone = self
            .zone
            .unwrap_or_else(|| format::system_zone("year view"));
        let today = self
            .today
            .unwrap_or_else(|| Timestamp::now().to_zoned(zone.clone()).date());
        let year = use_seeded((self.id.clone(), "year"), self.year, window, cx);
        let shown = year.read(cx).value;
        let start = match today.year() == shown {
            true => today,
            false => Date::new(shown, 1, 1).expect("a year has a first day"),
        };
        let cursor = window.use_keyed_state((self.id.clone(), "cursor"), cx, |_, _| start);
        if cursor.read(cx).year() != shown {
            cursor.update(cx, |cursor, _| *cursor = start);
        }
        let focus = tab_stop((self.id.clone(), "focus").into(), true, window, cx);
        let at = *cursor.read(cx);
        let pick: OnDay = {
            let (id, year, cursor, on_year) = (
                self.id.clone(),
                year.clone(),
                cursor.clone(),
                self.on_year.clone(),
            );
            Rc::new(move |day, window, cx| {
                cursor.update(cx, |cursor, cx| {
                    *cursor = day;
                    cx.notify();
                });
                if year.read(cx).value == day.year() {
                    return;
                }
                log::info!("year view {id:?}: {}", day.year());
                year.update(cx, |year, cx| {
                    year.value = day.year();
                    cx.notify();
                });
                if let Some(on_year) = &on_year {
                    on_year(day.year(), window, cx);
                }
            })
        };
        let open: OnDay = {
            let (id, on_day) = (self.id.clone(), self.on_day.clone());
            Rc::new(move |day, window, cx| {
                log::info!("year view {id:?}: open {day}");
                if let Some(on_day) = &on_day {
                    on_day(day, window, cx);
                }
            })
        };
        let step: Step = {
            let pick = pick.clone();
            Rc::new(move |by, window, cx| {
                let day = match by {
                    0 => today,
                    by => match super::stepped(at, by.years(), "year view") {
                        Some(day) => day,
                        None => return,
                    },
                };
                pick(day, window, cx)
            })
        };
        let keys = {
            let (focus, pick, open) = (focus.clone(), pick.clone(), open.clone());
            move |event: &gpui::KeyDownEvent, window: &mut Window, cx: &mut App| {
                if !focus.is_focused(window) {
                    return;
                }
                let by = match event.keystroke.key.as_str() {
                    "left" => 1.day().negate(),
                    "right" => 1.day(),
                    "up" => 1.week().negate(),
                    "down" => 1.week(),
                    "pageup" => 1.month().negate(),
                    "pagedown" => 1.month(),
                    "enter" => {
                        cx.stop_propagation();
                        return open(at, window, cx);
                    }
                    _ => return,
                };
                cx.stop_propagation();
                if let Some(day) = super::stepped(at, by, "year view") {
                    pick(day, window, cx)
                }
            }
        };
        let tints = marks(&self.events, shown, &zone)
            .into_iter()
            .map(|(day, ix)| (day, tint(&self.events[ix], cx)))
            .collect();
        let mini = Mini {
            id: &self.id,
            today,
            cursor: focus.is_focused(window).then_some(at),
            marks: &tints,
            pick: &pick,
            open: &open,
        };
        let months: Vec<AnyElement> = (1..=12)
            .map(|month| {
                let first = Date::new(shown, month, 1).expect("a month has a first day");
                mini.month(first, cx)
            })
            .collect();
        div()
            .flex()
            .flex_col()
            .gap_4()
            .child(head(
                &self.id,
                shown.to_string(),
                "Previous year",
                "Next year",
                step,
                cx,
            ))
            .child(
                div()
                    .id((self.id.clone(), "months"))
                    .debug_selector(|| "year-months".into())
                    .track_focus(&focus)
                    .on_key_down(keys)
                    .flex()
                    .flex_wrap()
                    .gap_x_8()
                    .gap_y_6()
                    .children(months),
            )
    }
}

#[cfg(test)]
mod tests {
    use jiff::{civil::date, tz::TimeZone};

    use super::marks;
    use crate::calendar::Event;

    #[test]
    fn a_day_takes_the_first_events_mark_within_the_year() {
        let events = [
            Event::all_day("trip", "Trip", date(2026, 12, 30), date(2027, 1, 2), 0),
            Event::all_day(
                "offsite",
                "Offsite",
                date(2026, 12, 31),
                date(2026, 12, 31),
                1,
            ),
        ];
        let marks = marks(&events, 2026, &TimeZone::UTC);
        assert_eq!(
            marks.into_iter().collect::<Vec<_>>(),
            [(date(2026, 12, 30), 0), (date(2026, 12, 31), 0)],
            "the trip marks its days in 2026 only, and first"
        );
    }
}
