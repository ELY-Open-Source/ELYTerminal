use gpui::{
    Context, IntoElement, Modifiers, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent,
    ParentElement, Point, Render, TestAppContext, Window, div, point, prelude::*, px,
};

use super::{ColumnShell, ShellColumn, TitleBar, WindowControls};
use crate::theme::{Platform, Theme};

struct Bar {
    closes: usize,
}

impl Render for Bar {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        div().w(px(600.0)).child(
            TitleBar::new("bar")
                .platform(Platform::Linux)
                .on_close(move |_, cx| view.update(cx, |bar, _| bar.closes += 1)),
        )
    }
}

/// Presses at `at`, moves two pixels, releases.
fn wobble(at: Point<gpui::Pixels>, cx: &mut gpui::VisualTestContext) {
    let moved = at + point(px(2.0), px(1.0));
    cx.simulate_event(MouseDownEvent {
        button: MouseButton::Left,
        position: at,
        modifiers: Modifiers::none(),
        click_count: 1,
        first_mouse: false,
    });
    cx.simulate_event(MouseMoveEvent {
        position: moved,
        pressed_button: Some(MouseButton::Left),
        modifiers: Modifiers::none(),
    });
    cx.simulate_event(MouseUpEvent {
        button: MouseButton::Left,
        position: moved,
        modifiers: Modifiers::none(),
        click_count: 1,
    });
    cx.run_until_parked();
}

/// A press that wobbles on Linux's close button still closes; it never moves the window.
#[gpui::test]
fn a_wobbling_press_on_a_linux_button_clicks_it(cx: &mut TestAppContext) {
    cx.update(Theme::init);
    let (view, cx) = cx.add_window_view(|_, _| Bar { closes: 0 });
    cx.run_until_parked();
    let controls = cx
        .debug_bounds("titlebar-controls")
        .expect("drawn controls");
    // Close is the last of the round buttons.
    wobble(point(controls.right() - px(8.0), controls.center().y), cx);
    assert_eq!(view.read_with(cx, |bar, _| bar.closes), 1);
}

/// The bar's empty space still moves the window (the test window panics on a move).
#[gpui::test]
#[should_panic(expected = "not implemented")]
fn a_wobbling_press_on_the_bar_moves_the_window(cx: &mut TestAppContext) {
    cx.update(Theme::init);
    let (_, cx) = cx.add_window_view(|_, _| Bar { closes: 0 });
    cx.run_until_parked();
    let controls = cx
        .debug_bounds("titlebar-controls")
        .expect("drawn controls");
    wobble(point(controls.left() - px(200.0), controls.center().y), cx);
}

/// Counts presses that reach the box under the controls.
struct Under {
    platform: Platform,
    presses: usize,
}

impl Render for Under {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        div()
            .id("under")
            .flex()
            .items_start()
            .w_full()
            .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                view.update(cx, |under, _| under.presses += 1)
            })
            .child(
                div()
                    .debug_selector(|| "controls".into())
                    .child(WindowControls::new("controls").platform(self.platform)),
            )
    }
}

/// Each style's controls hide what lies under them.
#[gpui::test]
fn window_controls_occlude_what_lies_under(cx: &mut TestAppContext) {
    cx.update(Theme::init);
    for platform in [Platform::Windows, Platform::Mac, Platform::Linux] {
        let (view, cx) = cx.add_window_view(move |_, _| Under {
            platform,
            presses: 0,
        });
        cx.run_until_parked();
        let controls = cx.debug_bounds("controls").expect("drawn controls");
        cx.simulate_event(MouseDownEvent {
            button: MouseButton::Left,
            position: controls.center(),
            modifiers: Modifiers::none(),
            click_count: 1,
            first_mouse: false,
        });
        assert_eq!(
            view.read_with(cx, |under, _| under.presses),
            0,
            "{platform:?}"
        );
    }
}

/// Title bar and column header with inert slot items.
struct Slotted;

impl Render for Slotted {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let item = |name: &'static str| div().debug_selector(move || name.into()).size_6();
        let column = ShellColumn::new("main")
            .leading(item("shell-lead"))
            .action(item("shell-act"));
        div()
            .size_full()
            .flex()
            .flex_col()
            .child(
                TitleBar::new("bar")
                    .platform(Platform::Windows)
                    .child(item("bar-lead"))
                    .action(item("bar-act")),
            )
            .child(
                ColumnShell::new("shell")
                    .platform(Platform::Windows)
                    .column(column),
            )
    }
}

/// A press on a slot item never moves the window.
#[gpui::test]
fn a_wobbling_press_on_a_slot_moves_no_window(cx: &mut TestAppContext) {
    cx.update(Theme::init);
    let (_, cx) = cx.add_window_view(|_, _| Slotted);
    cx.run_until_parked();
    for name in ["bar-lead", "bar-act", "shell-lead", "shell-act"] {
        let item = cx.debug_bounds(name).expect("a slot item");
        wobble(item.center(), cx);
    }
}

/// A column's title still moves the window; a move panics.
#[gpui::test]
#[should_panic(expected = "not implemented")]
fn a_wobbling_press_on_a_column_title_moves_the_window(cx: &mut TestAppContext) {
    cx.update(Theme::init);
    let (_, cx) = cx.add_window_view(|_, _| Slotted);
    cx.run_until_parked();
    let items = cx.debug_bounds("column-shell-items-0").expect("the items");
    wobble(items.center(), cx);
}
