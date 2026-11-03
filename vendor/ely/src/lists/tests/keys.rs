use std::time::Duration;

use gpui::{
    AppContext, Context, Entity, FocusHandle, InteractiveElement, IntoElement, Modifiers,
    ParentElement, Render, TestAppContext, Window, div,
};

use super::{Picks, enter, picks, row, settle, setup};
use crate::{
    buttons::Button,
    forms::{Enter, Input, TextInput},
    lists::{ListItem, SelectableList},
};

/// A list whose open hands focus to a button beside it, and what each heard.
struct Handoff {
    button: FocusHandle,
    heard: Vec<&'static str>,
}

impl Render for Handoff {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (to, opened, clicked) = (self.button.clone(), cx.entity(), cx.entity());
        div()
            .child(
                SelectableList::new("handoff")
                    .row("a", ListItem::new("handoff-a", "A"))
                    .on_activate(move |_, window, cx| {
                        window.focus(&to, cx);
                        opened.update(cx, |view, _| view.heard.push("open"));
                    }),
            )
            .child(
                Button::new("after", "After")
                    .focus_handle(&self.button)
                    .on_click(move |_, _, cx| {
                        clicked.update(cx, |view, _| view.heard.push("click"))
                    }),
            )
    }
}

/// Enter opens on its release, so an open that moves focus to a button leaves the button unpressed.
#[gpui::test]
fn an_open_that_moves_focus_leaves_the_release_alone(cx: &mut TestAppContext) {
    setup(cx);
    let (view, cx) = cx.add_window_view(|_, cx| Handoff {
        button: cx.focus_handle().tab_stop(true),
        heard: Vec::new(),
    });
    settle(cx);
    cx.update(|window, cx| window.focus_next(cx));
    settle(cx);
    enter(cx);
    settle(cx);
    assert_eq!(view.read_with(cx, |view, _| view.heard.clone()), ["open"]);
}

/// A field whose Enter moves focus on to a list, and what the list opened.
struct Relay {
    field: Entity<TextInput>,
    heard: Vec<&'static str>,
}

impl Render for Relay {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let opened = cx.entity();
        div()
            .capture_action(|_: &Enter, window, cx| window.focus_next(cx))
            .child(Input::new(&self.field))
            .child(
                SelectableList::new("relay")
                    .row("a", ListItem::new("relay-a", "A"))
                    .on_activate(move |_, _, cx| {
                        opened.update(cx, |view, _| view.heard.push("open"))
                    }),
            )
    }
}

/// A release whose press fell elsewhere opens nothing.
#[gpui::test]
fn a_release_that_began_elsewhere_opens_nothing(cx: &mut TestAppContext) {
    setup(cx);
    cx.update(crate::forms::bind_keys);
    let (view, cx) = cx.add_window_view(|window, cx| Relay {
        field: cx.new(|cx| TextInput::new(window, cx)),
        heard: Vec::new(),
    });
    settle(cx);
    cx.update(|window, cx| window.focus_next(cx));
    settle(cx);
    enter(cx);
    settle(cx);
    assert!(view.read_with(cx, |view, _| view.heard.is_empty()));
}

/// Typing a row's first letters moves the cursor there and picks it; after a pause the letters start over.
#[gpui::test]
fn letters_move_to_the_row_that_starts_with_them(cx: &mut TestAppContext) {
    setup(cx);
    let (view, cx) = cx.add_window_view(|_, _| Picks(Vec::new(), Vec::new(), None, false));
    settle(cx);
    cx.simulate_click(row(0), Modifiers::none());
    settle(cx);
    cx.simulate_input("c");
    settle(cx);
    assert_eq!(picks(&view, cx), ["c"]);
    cx.executor().advance_clock(Duration::from_secs(1));
    cx.simulate_input("b");
    settle(cx);
    assert_eq!(
        picks(&view, cx),
        ["b"],
        "the pause started the letters over"
    );
}

/// A letter held with Command is the app's shortcut, not a letter to move by: Command-A still takes all.
#[gpui::test]
fn a_letter_with_command_stays_a_shortcut(cx: &mut TestAppContext) {
    setup(cx);
    let (view, cx) = cx.add_window_view(|_, _| Picks(Vec::new(), Vec::new(), None, false));
    settle(cx);
    cx.simulate_click(row(1), Modifiers::none());
    settle(cx);
    cx.simulate_event(gpui::KeyDownEvent {
        keystroke: gpui::Keystroke {
            modifiers: Modifiers::command(),
            key: "a".into(),
            key_char: Some("a".into()),
        },
        is_held: false,
        prefer_character_input: false,
    });
    settle(cx);
    assert_eq!(picks(&view, cx), ["a", "b", "c", "d"]);
}
