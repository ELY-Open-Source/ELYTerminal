use std::cell::Cell;

use gpui::{
    AnyElement, InteractiveElement, IntoElement, Modifiers, ParentElement, TestAppContext, div,
};

use super::{press, shown, stage};
use crate::misc::{Calculator, UnitConverter};

thread_local! {
    static ESCAPES: Cell<usize> = const { Cell::new(0) };
}

fn calculator() -> AnyElement {
    div()
        .on_key_down(|event, _, _| {
            if event.keystroke.key == "escape" {
                ESCAPES.with(|escapes| escapes.set(escapes.get() + 1));
            }
        })
        .child(Calculator::new("calculator"))
        .into_any_element()
}

/// Typed keys work the sum in precedence and Enter gives it, but on a focused key Enter presses that key.
#[gpui::test]
fn a_calculator_takes_typed_keys(cx: &mut TestAppContext) {
    let (_, cx) = stage(calculator, cx);
    press("tab", cx);
    for key in [
        "1",
        "2",
        "+",
        "3",
        "*",
        "4",
        "cmd-backspace",
        "ctrl-backspace",
        "5",
    ] {
        press(key, cx);
    }
    assert!(
        shown("calculator-12 + 3 × 45", cx),
        "Command and Control left the delete to the app"
    );
    press("backspace", cx);
    for _ in 0..5 {
        press("tab", cx);
    }
    press("enter", cx);
    assert!(
        shown("calculator-12 + 3 × 47", cx),
        "Enter on the focused 7 typed it"
    );
    for _ in 0..5 {
        press("shift-tab", cx);
    }
    press("enter", cx);
    assert!(
        shown("calculator-153", cx),
        "Enter on the calculator is equals"
    );
    for key in ["/", "9", "="] {
        press(key, cx);
    }
    assert!(shown("calculator-17", cx), "= is equals too");
    press("tab", cx);
    press("enter", cx);
    press("+", cx);
    press("5", cx);
    assert!(
        shown("calculator-5", cx) && !shown("calculator-17 + 5", cx),
        "Enter on the focused AC cleared the result"
    );
}

/// A press on a key hands the keyboard to the calculator, though the key itself takes no focus.
#[gpui::test]
fn a_pressed_key_hands_the_keyboard_to_the_calculator(cx: &mut TestAppContext) {
    let (_, cx) = stage(calculator, cx);
    let seven = cx.debug_bounds("key-7").expect("the 7 key").center();
    cx.simulate_click(seven, Modifiers::none());
    press("+", cx);
    press("1", cx);
    assert!(shown("calculator-7 + 1", cx));
}

/// Escape clears what there is and stops there; on a blank calculator it passes on, as to a dialog.
#[gpui::test]
fn escape_clears_or_passes_on(cx: &mut TestAppContext) {
    let (_, cx) = stage(calculator, cx);
    press("tab", cx);
    press("escape", cx);
    assert_eq!(ESCAPES.with(Cell::get), 1, "blank, it passed Escape on");
    press("5", cx);
    press("escape", cx);
    press("3", cx);
    assert!(shown("calculator-3", cx) && !shown("calculator-53", cx));
    assert_eq!(ESCAPES.with(Cell::get), 1, "clearing kept it");
}

fn converter() -> AnyElement {
    UnitConverter::new("converter").into_any_element()
}

/// A meter reads in feet and, swapped, a foot in meters; another kind starts on its own pair; a typed amount converts, and text that is no number reads as none.
#[gpui::test]
fn a_converter_reads_an_amount_in_another_unit(cx: &mut TestAppContext) {
    let (_, cx) = stage(converter, cx);
    assert!(shown("converted-3.28084 ft", cx));
    press("tab", cx);
    press("tab", cx);
    press("enter", cx);
    assert!(shown("converted-0.3048 m", cx), "swapped");
    press("shift-tab", cx);
    for key in ["down", "down", "down", "down", "down", "enter"] {
        press(key, cx);
    }
    assert!(
        shown("converted-33.8 °F", cx),
        "Temperature starts on Celsius to Fahrenheit"
    );
    press("tab", cx);
    press("tab", cx);
    press("secondary-a", cx);
    cx.simulate_input("-40");
    assert!(shown("converted-−40 °F", cx));
    press("tab", cx);
    press("tab", cx);
    for key in ["down", "down", "enter"] {
        press(key, cx);
    }
    assert!(shown("converted-233.15 K", cx), "picked Kelvin");
    press("shift-tab", cx);
    press("shift-tab", cx);
    press("secondary-a", cx);
    cx.simulate_input("-");
    assert!(shown("converted-—", cx));
}
