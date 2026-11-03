use gpui::{AppContext, Context, EntityInputHandler, IntoElement, Render, TestAppContext, Window};

use super::setup;
use crate::forms::{TextInput, UnitInput};

#[gpui::test]
fn a_selection_outside_the_text_or_inside_a_character_is_left(cx: &mut TestAppContext) {
    setup(cx);
    let (_, cx) = cx.add_window_view(|_, _| gpui::Empty);
    let input = cx.update(|window, cx| cx.new(|cx| TextInput::new(window, cx)));
    input.update(cx, |input, cx| {
        input.set_text("héllo", cx);
        input.select(0..1, cx);
        input.select(0..2, cx);
        input.select(3..99, cx);
        assert_eq!(input.selection(), 0..1);
    });
}

/// A unit field whose list may drop the chosen unit.
struct Units {
    shortened: bool,
}

impl Render for Units {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let units = if self.shortened {
            vec!["em"]
        } else {
            vec!["px", "em"]
        };
        UnitInput::new("length", 12.0, units, "px")
    }
}

#[gpui::test]
fn a_unit_gone_from_its_list_keeps_the_field(cx: &mut TestAppContext) {
    setup(cx);
    let (host, cx) = cx.add_window_view(|_, _| Units { shortened: false });
    cx.run_until_parked();
    host.update(cx, |host, cx| {
        host.shortened = true;
        cx.notify();
    });
    cx.run_until_parked();
    assert!(cx.debug_bounds("number-root").is_some());
}

#[gpui::test]
fn an_ime_finds_its_composition_before_the_next_layout(cx: &mut TestAppContext) {
    let (fields, cx) = super::open(1, false, cx);
    let field = fields[0].clone();
    super::focus(&field, cx);
    field.update(cx, |input, cx| input.set_text("abc", cx));
    cx.run_until_parked();
    let laid = field
        .read_with(cx, |input, _| input.bounds_for(3))
        .expect("laid out");
    let asked = cx.update(|window, cx| {
        field.update(cx, |input, cx| {
            input.replace_and_mark_text_in_range(None, "n", None, window, cx);
            let at = gpui::Bounds::new(laid.origin, gpui::size(gpui::px(200.), laid.size.height));
            input.bounds_for_range(3..4, at, window, cx)
        })
    });
    let asked = asked.expect("the candidate box has a place");
    assert_eq!(asked.origin, laid.origin, "at the caret, on its line");
}
