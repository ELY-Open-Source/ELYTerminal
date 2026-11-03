use std::rc::Rc;

use gpui::{
    Context, IntoElement, KeyUpEvent, Keystroke, Modifiers, ParentElement, Render, Styled,
    TestAppContext, Window, div, point, px,
};
use jiff::civil::{Date, Time, date, time};

use super::setup;
use crate::forms::{Calendar, DatePicker, DateRangePicker, TimePicker};

const TODAY: Date = date(2026, 9, 25);

struct Day {
    value: Option<Date>,
    disabled: bool,
}

impl Render for Day {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        div().size_full().child(
            DatePicker::new("day", self.value)
                .today(TODAY)
                .disabled(self.disabled)
                .on_change(move |picked, _, cx| {
                    view.update(cx, |view, cx| {
                        view.value = Some(picked);
                        cx.notify();
                    })
                }),
        )
    }
}

#[gpui::test]
fn a_date_picker_opens_and_picks_from_the_keyboard(cx: &mut TestAppContext) {
    setup(cx);
    let (view, cx) = cx.add_window_view(|_, _| Day {
        value: Some(date(2026, 9, 10)),
        disabled: false,
    });
    cx.update(|window, cx| window.focus_next(cx));
    cx.simulate_keystrokes("enter right enter");
    assert_eq!(
        view.read_with(cx, |view, _| view.value),
        Some(date(2026, 9, 11))
    );
}

struct Span {
    value: Option<(Date, Date)>,
}

impl Render for Span {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        div().size_full().child(
            DateRangePicker::new("span", self.value)
                .today(TODAY)
                .on_change(move |picked, _, cx| {
                    view.update(cx, |view, cx| {
                        view.value = Some(picked);
                        cx.notify();
                    })
                }),
        )
    }
}

#[gpui::test]
fn a_range_picker_takes_two_picks_in_either_order(cx: &mut TestAppContext) {
    setup(cx);
    let (view, cx) = cx.add_window_view(|_, _| Span { value: None });
    cx.update(|window, cx| window.focus_next(cx));
    cx.simulate_keystrokes("enter enter left left enter");
    assert_eq!(
        view.read_with(cx, |view, _| view.value),
        Some((date(2026, 9, 23), TODAY))
    );
}

struct Clock {
    value: Option<Time>,
}

impl Render for Clock {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        div()
            .size_full()
            .child(
                TimePicker::new("clock", self.value)
                    .step(15)
                    .on_change(move |picked, _, cx| {
                        view.update(cx, |view, cx| {
                            view.value = Some(picked);
                            cx.notify();
                        })
                    }),
            )
    }
}

#[gpui::test]
fn a_time_picker_steps_with_the_arrows(cx: &mut TestAppContext) {
    setup(cx);
    let (view, cx) = cx.add_window_view(|_, _| Clock {
        value: Some(time(9, 30, 0, 0)),
    });
    cx.update(|window, cx| window.focus_next(cx));
    cx.simulate_keystrokes("enter down down");
    assert_eq!(
        view.read_with(cx, |view, _| view.value),
        Some(time(10, 0, 0, 0))
    );
}

struct Bounded {
    picked: Option<Date>,
    max: Option<Date>,
}

impl Render for Bounded {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        let calendar = Calendar::new("bounded").selected(TODAY).today(TODAY);
        let calendar = match self.max {
            Some(max) => calendar.max(max),
            None => calendar,
        };
        calendar.on_pick(move |picked, _, cx| {
            view.update(cx, |view, cx| {
                view.picked = Some(picked);
                cx.notify();
            })
        })
    }
}

#[gpui::test]
fn a_calendar_refuses_days_past_its_max(cx: &mut TestAppContext) {
    setup(cx);
    let (view, cx) = cx.add_window_view(|_, _| Bounded {
        picked: None,
        max: Some(TODAY),
    });
    cx.update(|window, cx| window.focus_next(cx));
    cx.simulate_keystrokes("right enter");
    assert_eq!(view.read_with(cx, |view, _| view.picked), None);
    cx.simulate_keystrokes("left enter");
    assert_eq!(view.read_with(cx, |view, _| view.picked), Some(TODAY));
}

#[gpui::test]
fn a_picker_disabled_while_open_stops_picking(cx: &mut TestAppContext) {
    setup(cx);
    let (view, cx) = cx.add_window_view(|_, _| Day {
        value: Some(date(2026, 9, 10)),
        disabled: false,
    });
    cx.update(|window, cx| window.focus_next(cx));
    cx.simulate_keystrokes("enter");
    view.update(cx, |view, cx| {
        view.disabled = true;
        cx.notify();
    });
    cx.simulate_keystrokes("right enter");
    assert_eq!(
        view.read_with(cx, |view, _| view.value),
        Some(date(2026, 9, 10))
    );
}

#[gpui::test]
fn the_month_buttons_carry_the_cursor(cx: &mut TestAppContext) {
    setup(cx);
    let (view, cx) = cx.add_window_view(|_, _| Bounded {
        picked: None,
        max: None,
    });
    cx.update(|window, cx| {
        window.focus_next(cx);
        window.focus_next(cx);
        window.focus_next(cx);
    });
    cx.simulate_keystrokes("enter");
    cx.simulate_event(KeyUpEvent {
        keystroke: Keystroke::parse("enter").unwrap(),
    });
    cx.update(|window, cx| {
        window.focus_prev(cx);
        window.focus_prev(cx);
    });
    cx.simulate_keystrokes("enter");
    assert_eq!(
        view.read_with(cx, |view, _| view.picked),
        Some(date(2026, 10, 25))
    );
}

/// A calendar that records every preview it reports.
struct Watched {
    seen: Vec<Option<Date>>,
}

impl Render for Watched {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        Calendar::new("watched")
            .selected(TODAY)
            .today(TODAY)
            .on_hover(Rc::new(move |date, _, cx| {
                view.update(cx, |view, cx| {
                    view.seen.push(date);
                    cx.notify();
                })
            }))
    }
}

#[gpui::test]
fn keys_keep_the_preview_under_a_resting_pointer(cx: &mut TestAppContext) {
    setup(cx);
    let (view, cx) = cx.add_window_view(|_, _| Watched { seen: Vec::new() });
    cx.update(|window, cx| window.focus_next(cx));
    cx.simulate_mouse_move(point(px(120.0), px(200.0)), None, Modifiers::none());
    let pointed = view.read_with(cx, |view, _| view.seen.last().copied().flatten());
    assert!(pointed.is_some(), "the pointer rests on a day");
    cx.simulate_keystrokes("pagedown");
    cx.update(|window, _| window.refresh());
    cx.run_until_parked();
    assert_eq!(
        view.read_with(cx, |view, _| view.seen.last().copied().flatten()),
        Some(date(2026, 10, 25)),
        "the keyboard's day stays the preview"
    );
}

#[gpui::test]
fn leaving_the_grid_after_a_key_clears_the_preview(cx: &mut TestAppContext) {
    setup(cx);
    for key in ["right", "a", "pagedown"] {
        let (view, cx) = cx.add_window_view(|_, _| Watched { seen: Vec::new() });
        cx.update(|window, cx| window.focus_next(cx));
        cx.simulate_mouse_move(point(px(120.0), px(200.0)), None, Modifiers::none());
        cx.simulate_keystrokes(key);
        cx.update(|window, _| window.refresh());
        cx.run_until_parked();
        cx.simulate_mouse_move(point(px(700.0), px(480.0)), None, Modifiers::none());
        cx.update(|window, _| window.refresh());
        cx.run_until_parked();
        assert_eq!(
            view.read_with(cx, |view, _| view.seen.last().copied().flatten()),
            None,
            "after {key}, leaving the grid clears its preview"
        );
    }
}
