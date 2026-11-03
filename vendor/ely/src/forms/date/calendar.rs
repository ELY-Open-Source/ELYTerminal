use std::rc::Rc;

use gpui::{
    Animation, AnimationExt, App, ElementId, FocusHandle, FontWeight, HoverListenerMode,
    InteractiveElement, IntoElement, ParentElement, RenderOnce, StatefulInteractiveElement, Styled,
    Window, div, prelude::*, transparent_black,
};
use jiff::{ToSpan, civil::Date};

use super::{super::options::Run, zoned_now};
use crate::{
    buttons::{ButtonVariant, IconButton},
    layout::seeded::use_seeded,
    motion,
    primitives::{IconName, tab_stop},
    theme::{ActiveTheme, ControlSize, Radius, TextSize},
};

const WEEKDAYS: [&str; 7] = ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"];

/// Monday of `date`'s week.
pub(crate) fn week_start(date: Date) -> Date {
    date.saturating_sub(i64::from(date.weekday().to_monday_zero_offset()).days())
}

/// The 42 days shown for `month`: six weeks, Monday first.
pub(crate) fn month_grid(month: Date) -> Vec<Date> {
    let start = week_start(month.first_of_month());
    (0..42)
        .map(|day| start.saturating_add(day.days()))
        .collect()
}

pub(crate) type OnDate = Rc<dyn Fn(Date, &mut Window, &mut App)>;
type OnHover = Rc<dyn Fn(Option<Date>, &mut Window, &mut App)>;
type Turn = Rc<dyn Fn(Date, &mut App)>;

/// A month of days. Arrows move a cursor, Page keys turn the month, Enter picks.
#[derive(IntoElement)]
pub struct Calendar {
    id: ElementId,
    selected: Option<Date>,
    span: Option<(Date, Date)>,
    week: bool,
    min: Option<Date>,
    max: Option<Date>,
    today: Option<Date>,
    anchor: Option<Date>,
    focus: Option<FocusHandle>,
    on_pick: Option<OnDate>,
    on_hover: Option<OnHover>,
    on_escape: Option<Run>,
}

impl Calendar {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            selected: None,
            span: None,
            week: false,
            min: None,
            max: None,
            today: None,
            anchor: None,
            focus: None,
            on_pick: None,
            on_hover: None,
            on_escape: None,
        }
    }

    pub fn selected(mut self, date: Date) -> Self {
        self.selected = Some(date);
        self
    }

    /// Marks a run of days: ink at both ends, a band between.
    pub fn span(mut self, start: Date, end: Date) -> Self {
        assert!(start <= end, "span {start}..{end} runs backwards");
        self.span = Some((start, end));
        self
    }

    /// Shades the whole week under the pointer.
    pub fn week(mut self) -> Self {
        self.week = true;
        self
    }

    pub fn min(mut self, date: Date) -> Self {
        self.min = Some(date);
        self
    }

    pub fn max(mut self, date: Date) -> Self {
        self.max = Some(date);
        self
    }

    /// The day ringed as today. Defaults to the system's date.
    pub fn today(mut self, date: Date) -> Self {
        self.today = Some(date);
        self
    }

    pub fn on_pick(mut self, handler: impl Fn(Date, &mut Window, &mut App) + 'static) -> Self {
        self.on_pick = Some(Rc::new(handler));
        self
    }

    pub(crate) fn on_hover(mut self, handler: OnHover) -> Self {
        self.on_hover = Some(handler);
        self
    }

    /// The day whose month opens first, when the marks move while picking.
    pub(crate) fn anchor(mut self, date: Date) -> Self {
        self.anchor = Some(date);
        self
    }

    /// Focus owned by a picker that opens this calendar.
    pub(crate) fn focus(mut self, handle: FocusHandle) -> Self {
        self.focus = Some(handle);
        self
    }

    pub(crate) fn on_escape(mut self, handler: Run) -> Self {
        self.on_escape = Some(handler);
        self
    }
}

impl RenderOnce for Calendar {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let today = self.today.unwrap_or_else(|| zoned_now().date());
        let anchor = self
            .anchor
            .or(self.selected)
            .or(self.span.map(|(start, _)| start))
            .unwrap_or(today);
        let month = use_seeded(
            (self.id.clone(), "month"),
            anchor.first_of_month(),
            window,
            cx,
        );
        let cursor = use_seeded((self.id.clone(), "cursor"), anchor, window, cx);
        let turns = window.use_keyed_state((self.id.clone(), "turns"), cx, |_, _| (0usize, true));
        let hover = window.use_keyed_state((self.id.clone(), "hover"), cx, |_, _| None::<Date>);
        let focus = match self.focus {
            Some(handle) => handle,
            None => tab_stop((self.id.clone(), "focus").into(), true, window, cx),
        };
        let focused = focus.is_focused(window);
        let (shown, at, (turned, forward), hovered) = (
            month.read(cx).value,
            cursor.read(cx).value,
            *turns.read(cx),
            *hover.read(cx),
        );
        let turn: Turn = {
            let (month, turns) = (month.clone(), turns.clone());
            Rc::new(move |next, cx| {
                let first = next.first_of_month();
                let current = month.read(cx).value;
                if first == current {
                    return;
                }
                month.update(cx, |month, cx| {
                    month.value = first;
                    cx.notify();
                });
                turns.update(cx, |(count, forward), _| {
                    *count += 1;
                    *forward = first > current;
                });
            })
        };
        let (min, max) = (self.min, self.max);
        let enabled = move |date: Date| {
            min.is_none_or(|min| date >= min) && max.is_none_or(|max| date <= max)
        };
        let pick: OnDate = {
            let (id, on_pick, turn, cursor) =
                (self.id.clone(), self.on_pick, turn.clone(), cursor.clone());
            Rc::new(move |date, window, cx| {
                if !enabled(date) {
                    return;
                }
                log::info!("calendar {id:?}: {date}");
                cursor.update(cx, |cursor, _| cursor.value = date);
                turn(date, cx);
                if let Some(on_pick) = &on_pick {
                    on_pick(date, window, cx);
                }
            })
        };
        let theme = cx.theme();
        let colors = &theme.colors;
        let cell = theme.control_height(ControlSize::Md);
        let header = div()
            .flex()
            .items_center()
            .justify_between()
            .child(
                div()
                    .pl_1()
                    .text_size(theme.text_size(TextSize::Base))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(colors.fg)
                    .child(shown.strftime("%B %Y").to_string()),
            )
            .child(
                div()
                    .flex()
                    .gap_0p5()
                    .child({
                        let (turn, cursor) = (turn.clone(), cursor.clone());
                        IconButton::new((self.id.clone(), "prev"), IconName::ChevronLeft)
                            .variant(ButtonVariant::Ghost)
                            .size(ControlSize::Sm)
                            .tooltip("Previous month")
                            .on_click(move |_, _, cx| {
                                let next = at.saturating_sub(1.month());
                                cursor.update(cx, |cursor, _| cursor.value = next);
                                turn(next, cx)
                            })
                    })
                    .child({
                        let (turn, cursor) = (turn.clone(), cursor.clone());
                        IconButton::new((self.id.clone(), "next"), IconName::ChevronRight)
                            .variant(ButtonVariant::Ghost)
                            .size(ControlSize::Sm)
                            .tooltip("Next month")
                            .on_click(move |_, _, cx| {
                                let next = at.saturating_add(1.month());
                                cursor.update(cx, |cursor, _| cursor.value = next);
                                turn(next, cx)
                            })
                    }),
            );
        let weekdays = div().grid().grid_cols(7).children(WEEKDAYS.map(|day| {
            div()
                .flex()
                .justify_center()
                .text_size(theme.text_size(TextSize::Xs))
                .text_color(colors.fg_subtle)
                .child(day)
        }));
        let days: Vec<_> = month_grid(shown)
            .into_iter()
            .enumerate()
            .map(|(ix, date)| {
                let open = enabled(date);
                let chosen = self.selected == Some(date)
                    || self
                        .span
                        .is_some_and(|(start, end)| date == start || date == end);
                let inside = self
                    .span
                    .is_some_and(|(start, end)| start < date && date < end)
                    || (self.week
                        && hovered.is_some_and(|over| week_start(over) == week_start(date)));
                let fg = if chosen {
                    colors.on_accent
                } else if !open {
                    colors.fg_disabled
                } else if date.month() != shown.month() {
                    colors.fg_subtle
                } else {
                    colors.fg
                };
                let border = if focused && date == at {
                    colors.focus
                } else if date == today && !chosen {
                    colors.border_strong
                } else {
                    transparent_black()
                };
                let (pick, hover, on_hover) = (pick.clone(), hover.clone(), self.on_hover.clone());
                div()
                    .id(("day", ix))
                    .flex()
                    .items_center()
                    .justify_center()
                    .h(cell)
                    .rounded(theme.radius(Radius::Md))
                    .border_1()
                    .border_color(border)
                    .text_size(theme.text_size(TextSize::Sm))
                    .text_color(fg)
                    .when(date == today, |day| day.font_weight(FontWeight::SEMIBOLD))
                    .map(|day| {
                        if chosen {
                            day.bg(colors.accent)
                        } else if inside {
                            day.bg(colors.active)
                        } else {
                            day
                        }
                    })
                    .when(open, |day| {
                        day.cursor_pointer()
                            .when(!chosen && !inside, |day| {
                                day.hover(|style| style.bg(colors.hover))
                            })
                            .on_hover(move |over, window, cx| {
                                if *over {
                                    hover.update(cx, |hover, cx| {
                                        *hover = Some(date);
                                        cx.notify();
                                    });
                                    if let Some(on_hover) = &on_hover {
                                        on_hover(Some(date), window, cx);
                                    }
                                }
                            })
                            .on_click(move |_, window, cx| pick(date, window, cx))
                    })
                    .child(date.day().to_string())
            })
            .collect();
        let (leave, on_leave) = (hover, self.on_hover.clone());
        let grid = div()
            .id((self.id.clone(), "days"))
            .grid()
            .grid_cols(7)
            .gap_0p5()
            .hover_listener_mode(HoverListenerMode::InputModalityIndependent)
            .on_hover(move |over, window, cx| {
                if !*over && !window.last_input_was_keyboard() {
                    leave.update(cx, |hover, cx| {
                        *hover = None;
                        cx.notify();
                    });
                    if let Some(on_leave) = &on_leave {
                        on_leave(None, window, cx);
                    }
                }
            })
            .children(days);
        let grid = if turned == 0 {
            grid.into_any_element()
        } else {
            let travel = if forward {
                motion::NUDGE * 3.0
            } else {
                motion::NUDGE * -3.0
            };
            grid.with_animation(
                ("calendar-turn", turned),
                Animation::new(motion::duration(motion::BASE, cx))
                    .with_easing(motion::ease_out_cubic),
                move |grid, t| grid.opacity(t).ml(travel * (1.0 - t)),
            )
            .into_any_element()
        };
        let (keys, on_escape, on_move) = (pick, self.on_escape, self.on_hover);
        div()
            .id(self.id)
            .track_focus(&focus)
            .flex()
            .flex_col()
            .gap_2()
            .w(cell * 8.0)
            .p_2()
            .rounded(theme.radius(Radius::Lg))
            .on_key_down(move |event, window, cx| {
                let next = match event.keystroke.key.as_str() {
                    "left" => at.saturating_sub(1.day()),
                    "right" => at.saturating_add(1.day()),
                    "up" => at.saturating_sub(1.week()),
                    "down" => at.saturating_add(1.week()),
                    "pageup" => at.saturating_sub(1.month()),
                    "pagedown" => at.saturating_add(1.month()),
                    "home" => week_start(at),
                    "end" => week_start(at).saturating_add(6.days()),
                    "enter" | "space" => {
                        cx.stop_propagation();
                        return keys(at, window, cx);
                    }
                    "escape" => {
                        if let Some(on_escape) = &on_escape {
                            cx.stop_propagation();
                            on_escape(window, cx);
                        }
                        return;
                    }
                    _ => return,
                };
                cx.stop_propagation();
                cursor.update(cx, |cursor, cx| {
                    cursor.value = next;
                    cx.notify();
                });
                turn(next, cx);
                if let Some(on_move) = &on_move {
                    on_move(Some(next), window, cx);
                }
            })
            .child(header)
            .child(weekdays)
            .child(grid)
    }
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;

    use super::{month_grid, week_start};

    #[test]
    fn a_month_shows_six_weeks_from_a_monday() {
        let days = month_grid(date(2026, 9, 17));
        assert_eq!(days.len(), 42);
        assert_eq!(days[0], date(2026, 8, 31));
        assert_eq!(days[1], date(2026, 9, 1));
        assert_eq!(days[41], date(2026, 10, 11));
        assert_eq!(week_start(date(2026, 9, 27)), date(2026, 9, 21));
    }
}
