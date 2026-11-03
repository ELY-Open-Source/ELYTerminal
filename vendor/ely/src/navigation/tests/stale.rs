use gpui::{Context, IntoElement, Render, TestAppContext, Window};

use super::setup;
use crate::{
    forms::Choice,
    navigation::{BackForwardNavigation, Breadcrumb, Crumb, Steps, Tabs, Wizard},
};

/// Tabs that may lose every tab, or keep only disabled ones under a gone choice.
struct TabHost {
    empty: bool,
    disabled: bool,
    picked: bool,
}

impl Render for TabHost {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let host = cx.entity();
        let tabs = match (self.empty, self.disabled) {
            (true, _) => Vec::new(),
            (false, true) => vec![
                Choice::new("a", "A").disabled(),
                Choice::new("b", "B").disabled(),
            ],
            (false, false) => vec![Choice::new("a", "A")],
        };
        let selected = if self.disabled { "gone" } else { "a" };
        Tabs::new("tabs", tabs, selected)
            .on_change(move |_, _, cx| host.update(cx, |host, _| host.picked = true))
    }
}

#[gpui::test]
fn an_arrow_on_tabs_that_all_closed_does_nothing(cx: &mut TestAppContext) {
    setup(cx);
    let (host, cx) = cx.add_window_view(|_, _| TabHost {
        empty: false,
        disabled: false,
        picked: false,
    });
    cx.update(|window, cx| window.focus_next(cx));
    host.update(cx, |host, cx| {
        host.empty = true;
        cx.notify();
    });
    cx.run_until_parked();
    cx.simulate_keystrokes("right");
    assert!(!host.read_with(cx, |host, _| host.picked));
}

#[gpui::test]
fn an_arrow_never_picks_a_disabled_tab(cx: &mut TestAppContext) {
    setup(cx);
    let (host, cx) = cx.add_window_view(|_, _| TabHost {
        empty: false,
        disabled: true,
        picked: false,
    });
    cx.update(|window, cx| window.focus_next(cx));
    cx.simulate_keystrokes("right");
    assert!(!host.read_with(cx, |host, _| host.picked));
}

#[derive(Clone, Copy)]
enum Kind {
    Steps,
    Wizard,
    History,
    Crumb,
}

/// A list of two that drops its second while the place still names it.
struct Shrinking {
    kind: Kind,
    shortened: bool,
}

impl Render for Shrinking {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let mut rows = vec![Choice::new("a", "A"), Choice::new("b", "B")];
        if self.shortened {
            rows.pop();
        }
        match self.kind {
            Kind::Steps => Steps::new("steps", rows, 1).into_any_element(),
            Kind::Wizard => {
                Wizard::new("wizard", rows, 1, |_, _, _| {}, |_, _| {}).into_any_element()
            }
            Kind::History => BackForwardNavigation::new("history", rows, 1).into_any_element(),
            Kind::Crumb => {
                Breadcrumb::new("crumb", [Crumb::new("b", "B").siblings(rows)]).into_any_element()
            }
        }
    }
}

#[gpui::test]
fn a_place_past_a_shortened_list_marks_none(cx: &mut TestAppContext) {
    setup(cx);
    for kind in [Kind::Steps, Kind::Wizard, Kind::History, Kind::Crumb] {
        let (host, cx) = cx.add_window_view(move |_, _| Shrinking {
            kind,
            shortened: false,
        });
        cx.run_until_parked();
        host.update(cx, |host, cx| {
            host.shortened = true;
            cx.notify();
        });
        cx.run_until_parked();
    }
}
