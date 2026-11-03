use gpui::{Context, Entity, IntoElement, Render, TestAppContext, VisualTestContext, Window};

use super::{NumberInput, parse, settle};
use crate::{forms::bind_keys, theme::Theme};

#[test]
fn parses_grouped_and_typographic_minus_and_refuses_junk() {
    assert_eq!(parse("1,234.5"), Some(1234.5));
    assert_eq!(parse("\u{2212}3"), Some(-3.0));
    assert_eq!(parse("abc"), None);
    assert_eq!(parse("1e999"), None);
}

#[test]
fn settle_clamps_then_rounds() {
    assert_eq!(settle(1.23456, 0.0, 10.0, 2), 1.23);
    assert_eq!(settle(-5.0, 0.0, 10.0, 0), 0.0);
    assert_eq!(settle(12.6, 0.0, 10.0, 0), 10.0);
}

#[test]
fn settle_rounds_before_it_clamps() {
    assert_eq!(settle(0.2 + 0.1, 0.0, 0.25, 1), 0.2);
    assert_eq!(settle(-0.26, -0.25, 1.0, 1), -0.2);
    assert_eq!(settle(0.3, 0.0, 0.29, 2), 0.29);
    assert_eq!(settle(0.29, 0.29, 0.29, 2), 0.29);
    assert_eq!(settle(10000000.01, 0.0, 10000000.005, 2), 10000000.0);
}

#[test]
#[should_panic(expected = "no 1-place number lies in 0.21..=0.29")]
fn settle_refuses_a_range_without_a_number_at_its_precision() {
    settle(0.25, 0.21, 0.29, 1);
}

/// A field whose owner keeps each commit and shows `precision` places.
struct Owned {
    value: f64,
    precision: usize,
    commits: Vec<f64>,
}

impl Render for Owned {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        NumberInput::new("owned", self.value)
            .precision(self.precision)
            .on_commit(move |value, _, cx| {
                view.update(cx, |owned, cx| {
                    owned.commits.push(value);
                    owned.value = value;
                    cx.notify();
                })
            })
    }
}

fn owned(
    value: f64,
    precision: usize,
    cx: &mut TestAppContext,
) -> (Entity<Owned>, &mut VisualTestContext) {
    cx.update(|cx| {
        Theme::init(cx);
        bind_keys(cx);
    });
    let (view, cx) = cx.add_window_view(move |_, _| Owned {
        value,
        precision,
        commits: Vec::new(),
    });
    cx.update(|window, _| window.activate_window());
    cx.run_until_parked();
    (view, cx)
}

fn commits(view: &Entity<Owned>, cx: &mut VisualTestContext) -> Vec<f64> {
    view.read_with(cx, |owned, _| owned.commits.clone())
}

#[gpui::test]
fn a_value_the_owner_sets_while_focused_shows_once_focus_leaves(cx: &mut TestAppContext) {
    let (view, cx) = owned(4.0, 0, cx);
    cx.update(|window, cx| window.focus_next(cx));
    view.update(cx, |owned, cx| {
        owned.value = 42.0;
        cx.notify();
    });
    cx.run_until_parked();
    cx.update(|window, cx| window.blur(cx));
    cx.run_until_parked();
    assert_eq!(commits(&view, cx), [42.0]);
}

#[gpui::test]
fn new_places_reformat_the_same_value(cx: &mut TestAppContext) {
    let (view, cx) = owned(1.25, 0, cx);
    view.update(cx, |owned, cx| {
        owned.precision = 2;
        cx.notify();
    });
    cx.run_until_parked();
    cx.update(|window, cx| window.focus_next(cx));
    cx.simulate_keystrokes("enter");
    assert_eq!(commits(&view, cx), [1.25], "the field shows 1.25, not 1");
}

#[gpui::test]
fn the_owner_echo_of_a_commit_keeps_the_next_draft(cx: &mut TestAppContext) {
    let (view, cx) = owned(4.0, 0, cx);
    cx.update(|window, cx| window.focus_next(cx));
    for typed in ["12", "18"] {
        cx.simulate_keystrokes("secondary-a");
        cx.simulate_input(typed);
        if typed == "12" {
            cx.simulate_keystrokes("enter");
            cx.run_until_parked();
        }
    }
    cx.update(|window, cx| window.blur(cx));
    cx.run_until_parked();
    assert_eq!(commits(&view, cx), [12.0, 18.0]);
}
