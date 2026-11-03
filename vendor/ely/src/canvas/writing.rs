use std::rc::Rc;

use gpui::{
    AnyElement, App, Entity, FocusHandle, InteractiveElement, IntoElement, MouseButton,
    ParentElement, Pixels, SharedString, Styled, Window, div,
};

use super::{gesture::OnPair, view::Frame};
use crate::forms::{Editing, Input};

/// Opens a field on the words of what `key` names. Enter or leaving hands them to the owner; Escape drops them. Enter and Escape hand focus back to `back`.
pub(super) fn begin(
    editing: &Entity<Editing>,
    key: SharedString,
    words: String,
    on_text: Option<OnPair>,
    back: FocusHandle,
    window: &mut Window,
    cx: &mut App,
) {
    log::info!("canvas: writing on {key}");
    editing.update(cx, |editing, _| {
        editing.on_commit = Some(Rc::new(
            move |text: &SharedString, window: &mut Window, cx: &mut App| {
                log::info!("canvas: {key} reads {text:?}");
                if let Some(on_text) = &on_text {
                    on_text(&key, text, window, cx);
                }
            },
        ));
    });
    Editing::begin(editing, words, window, cx);
    editing.update(cx, |editing, _| editing.returning(back));
}

/// The open field over `frame`, in view pixels.
pub(super) fn field(editing: &Entity<Editing>, frame: Frame, cx: &App) -> Option<AnyElement> {
    let field = editing.read(cx).field()?;
    let (closing, keeping) = (editing.clone(), editing.clone());
    Some(
        div()
            .absolute()
            .left(Pixels::from(frame.x))
            .top(Pixels::from(frame.y))
            .w(Pixels::from(frame.w))
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_key_down(move |event, window, cx| {
                if event.keystroke.key == "escape" {
                    cx.stop_propagation();
                    closing.update(cx, |editing, cx| {
                        editing.finish(true, window, cx);
                        editing.give_back(window, cx);
                    });
                }
            })
            .on_key_up(move |event, window, cx| {
                if event.keystroke.key == "enter" {
                    cx.stop_propagation();
                    keeping.update(cx, |editing, cx| {
                        editing.finish(false, window, cx);
                        editing.give_back(window, cx);
                    });
                }
            })
            .child(Input::new(&field))
            .into_any_element(),
    )
}
