use std::rc::Rc;

use gpui::{App, Div, Entity, InteractiveElement, ParentElement, Styled, Window, div};

use crate::{
    buttons::Button,
    forms::{Enter, Input, TextInput},
};

pub(super) type OnText = Rc<dyn Fn(&str, &mut Window, &mut App)>;

/// Hands the field's words to `send` and empties it; blank words are kept out.
fn send(
    field: &Entity<TextInput>,
    on_send: &OnText,
    what: &str,
    window: &mut Window,
    cx: &mut App,
) {
    let text = field.read(cx).text().trim().to_string();
    if text.is_empty() {
        return;
    }
    log::info!("{what}: sent {} characters", text.len());
    on_send(&text, window, cx);
    field.update(cx, |input, cx| input.set_text("", cx));
}

/// The owner's field beside `button`: Enter or a press sends its words, and the button waits for some.
pub(super) fn send_line(
    field: &Entity<TextInput>,
    button: Button,
    on_send: OnText,
    what: &'static str,
    cx: &App,
) -> Div {
    let ready = !field.read(cx).text().trim().is_empty();
    let (typed, pressed, enter) = (field.clone(), field.clone(), on_send.clone());
    div()
        .flex()
        .items_center()
        .gap_2()
        .child(
            div()
                .flex_1()
                .min_w_0()
                .capture_action(move |_: &Enter, window, cx| {
                    cx.stop_propagation();
                    send(&typed, &enter, what, window, cx)
                })
                .child(Input::new(field)),
        )
        .child(
            button
                .disabled(!ready)
                .on_click(move |_, window, cx| send(&pressed, &on_send, what, window, cx)),
        )
}
