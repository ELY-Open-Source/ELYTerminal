use std::rc::Rc;

use gpui::{
    AnyElement, App, ElementId, FontWeight, IntoElement, ParentElement, RenderOnce, SharedString,
    Styled, Window, div, prelude::*, relative,
};
use jiff::{Timestamp, ToSpan, civil::Date, tz::TimeZone};

use super::{
    allday::AllDayRow,
    event::{Event, unique},
    grid::{OnResize, OnSpan, TimeGrid, TimezoneOverlay},
    head::{OnDay, Step, head},
};
use crate::{
    forms::{OnValue, show_span, week_start},
    layout::seeded::use_seeded,
    theme::{ActiveTheme, ControlSize, TextSize},
    typography::format,
};

/// A week of days under their names.
pub type CalendarWeekView = CalendarDays<7>;
/// One day under its name.
pub type CalendarDayView = CalendarDays<1>;

/// `DAYS` days under their names: their whole days above, their hours below, where a drag makes an event and a drag on an event's lower edge moves its end. The arrows step by the view's days; Today comes back. It fills its box.
#[derive(IntoElement)]
pub struct CalendarDays<const DAYS: usize> {
    id: ElementId,
    day: Date,
    events: Vec<Event>,
    zone: Option<TimeZone>,
    now: Option<Timestamp>,
    overlay: Option<TimezoneOverlay>,
    on_create: Option<OnSpan>,
    on_resize: Option<OnResize>,
    on_event: Option<OnValue>,
    on_step: Option<OnDay>,
}

impl<const DAYS: usize> CalendarDays<DAYS> {
    /// The days holding `day`: its week, or the day itself.
    pub fn new(
        id: impl Into<ElementId>,
        day: Date,
        events: impl IntoIterator<Item = Event>,
    ) -> Self {
        Self {
            id: id.into(),
            day,
            events: unique(events),
            zone: None,
            now: None,
            overlay: None,
            on_create: None,
            on_resize: None,
            on_event: None,
            on_step: None,
        }
    }

    /// The zone its days and hours read in; the system's otherwise.
    pub fn zone(mut self, zone: TimeZone) -> Self {
        self.zone = Some(zone);
        self
    }

    /// The moment it takes as now, for today and the line at the hour; the clock's otherwise.
    pub fn now(mut self, now: Timestamp) -> Self {
        self.now = Some(now);
        self
    }

    /// Another zone's hours beside its own.
    pub fn overlay(mut self, overlay: TimezoneOverlay) -> Self {
        self.overlay = Some(overlay);
        self
    }

    /// Gets the start and end of a stretch dragged out on empty time.
    pub fn on_create(
        mut self,
        handler: impl Fn(Timestamp, Timestamp, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_create = Some(Rc::new(handler));
        self
    }

    /// Gets an event's key and its new end as its lower edge is dragged.
    pub fn on_resize(
        mut self,
        handler: impl Fn(&SharedString, Timestamp, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_resize = Some(Rc::new(handler));
        self
    }

    /// Gets the key of an event pressed.
    pub fn on_event(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_event = Some(Rc::new(handler));
        self
    }

    /// Gets the first day shown after each step.
    pub fn on_step(mut self, handler: impl Fn(Date, &mut Window, &mut App) + 'static) -> Self {
        self.on_step = Some(Rc::new(handler));
        self
    }

    /// The first day shown for `day`: its Monday in a week, the day itself alone.
    fn first(day: Date) -> Date {
        match DAYS {
            7 => week_start(day),
            _ => day,
        }
    }
}

/// The days' names at their columns, today's number lit, beside the zones' names over the hours.
fn names(days: &[Date], today: Date, zones: &[String], window: &Window, cx: &App) -> AnyElement {
    let theme = cx.theme();
    let colors = &theme.colors;
    let gutter = theme.calendar().gutter.to_pixels(window.rem_size());
    let gutters = gutter * zones.len() as f32;
    let count = days.len() as f32;
    let labels = zones.iter().enumerate().map(|(ix, zone)| {
        div()
            .absolute()
            .top_0()
            .bottom_0()
            .left(gutter * ix as f32)
            .w(gutter)
            .flex()
            .items_end()
            .justify_end()
            .pb_1()
            .pr_2()
            .text_color(colors.fg_subtle)
            .child(zone.clone())
    });
    let columns = days.iter().enumerate().map(|(ix, day)| {
        let lit = *day == today;
        div()
            .absolute()
            .top_0()
            .bottom_0()
            .left(relative(ix as f32 / count))
            .w(relative(1.0 / count))
            .overflow_hidden()
            .flex()
            .flex_col()
            .items_center()
            .justify_end()
            .pb_1()
            .child(
                div()
                    .text_color(colors.fg_subtle)
                    .child(day.strftime("%a").to_string()),
            )
            .child(
                div()
                    .px_1p5()
                    .rounded_full()
                    .text_size(theme.text_size(TextSize::Sm))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(colors.fg)
                    .when(lit, |number| {
                        number.bg(colors.accent).text_color(colors.on_accent)
                    })
                    .child(day.day().to_string()),
            )
    });
    div()
        .relative()
        .flex_none()
        .w_full()
        .h(theme.control_height(ControlSize::Lg) * 1.25)
        .border_b_1()
        .border_color(colors.border)
        .text_size(theme.text_size(TextSize::Xs))
        .children(labels)
        .child(
            div()
                .absolute()
                .top_0()
                .bottom_0()
                .left(gutters)
                .right_0()
                .children(columns),
        )
        .into_any_element()
}

impl<const DAYS: usize> RenderOnce for CalendarDays<DAYS> {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let zone = self
            .zone
            .clone()
            .unwrap_or_else(|| format::system_zone("calendar days"));
        let now = self.now.unwrap_or_else(Timestamp::now);
        let today = now.to_zoned(zone.clone()).date();
        let first = use_seeded(
            (self.id.clone(), "first"),
            Self::first(self.day),
            window,
            cx,
        );
        let shown = first.read(cx).value;
        let days: Vec<Date> = (0..DAYS as i64)
            .map(|ix| shown.checked_add(ix.days()).expect("a day follows"))
            .collect();
        let step: Step = {
            let (id, on_step) = (self.id.clone(), self.on_step.clone());
            Rc::new(move |by, window, cx| {
                let next = match by {
                    0 => Self::first(today),
                    by => match super::stepped(
                        shown,
                        (i64::from(by) * DAYS as i64).days(),
                        "calendar days",
                    ) {
                        Some(day) => day,
                        None => return,
                    },
                };
                log::info!("calendar days {id:?}: from {next}");
                first.update(cx, |first, cx| {
                    first.value = next;
                    cx.notify();
                });
                if let Some(on_step) = &on_step {
                    on_step(next, window, cx);
                }
            })
        };
        let (title, back, on) = match DAYS {
            7 => (
                show_span(days[0], days[DAYS - 1]),
                "Previous week",
                "Next week",
            ),
            _ => (
                shown.strftime("%A, %B %-d, %Y").to_string(),
                "Previous day",
                "Next day",
            ),
        };
        let mut zones = vec![format::datetime(now, &zone, "%Z").expect("a fixed pattern")];
        if let Some(overlay) = &self.overlay {
            zones.insert(
                0,
                format::datetime(now, &overlay.zone, "%Z").expect("a fixed pattern"),
            );
        }
        let whole = AllDayRow::new(
            (self.id.clone(), "all-day"),
            days.clone(),
            self.events.clone(),
        )
        .zone(zone.clone())
        .gutters(zones.len());
        let whole = match self.on_event.clone() {
            Some(on_event) => whole.on_event(move |key, window, cx| on_event(key, window, cx)),
            None => whole,
        };
        let mut grid =
            TimeGrid::new((self.id.clone(), "grid"), days.clone(), self.events).zone(zone);
        grid.now = self.now;
        grid.overlay = self.overlay;
        grid.on_create = self.on_create;
        grid.on_resize = self.on_resize;
        grid.on_event = self.on_event;
        div()
            .size_full()
            .flex()
            .flex_col()
            .gap_3()
            .child(head(&self.id, title, back, on, step, cx))
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .flex_col()
                    .child(names(&days, today, &zones, window, cx))
                    .child(whole)
                    .child(div().flex_1().min_h_0().child(grid)),
            )
    }
}
