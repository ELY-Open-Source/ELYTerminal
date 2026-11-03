use std::rc::Rc;

use gpui::{
    App, Context, ElementId, Entity, FontWeight, IntoElement, ParentElement, RenderOnce,
    SharedString, Styled, Subscription, Window, div, prelude::*,
};

use super::{InputEvent, TextInput};
use crate::theme::{ActiveTheme, ControlSize, Radius, TextSize};

type OnCode = Rc<dyn Fn(&str, &mut Window, &mut App)>;

struct Pin {
    input: Entity<TextInput>,
    length: usize,
    attempt: usize,
    on_complete: Option<OnCode>,
    _events: Subscription,
}

/// Boxes for a one-time code. Typing moves along; a paste fills them all.
#[derive(IntoElement)]
pub struct PinInput {
    id: ElementId,
    length: usize,
    letters: bool,
    masked: bool,
    attempt: usize,
    on_complete: Option<OnCode>,
}

impl PinInput {
    pub fn new(id: impl Into<ElementId>, length: usize) -> Self {
        assert!(length > 0, "a pin needs at least one box");
        Self {
            id: id.into(),
            length,
            letters: false,
            masked: false,
            attempt: 0,
            on_complete: None,
        }
    }

    /// Clears the boxes whenever `attempt` changes, so a code that failed makes way for the next.
    pub fn attempt(mut self, attempt: usize) -> Self {
        self.attempt = attempt;
        self
    }

    /// Accepts letters as well as digits.
    pub fn letters(mut self) -> Self {
        self.letters = true;
        self
    }

    /// Shows bullets in filled boxes.
    pub fn masked(mut self) -> Self {
        self.masked = true;
        self
    }

    /// Runs once every box is filled.
    pub fn on_complete(mut self, handler: impl Fn(&str, &mut Window, &mut App) + 'static) -> Self {
        self.on_complete = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for PinInput {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let (length, letters, attempt) = (self.length, self.letters, self.attempt);
        let state = window.use_keyed_state(self.id.clone(), cx, |window, cx: &mut Context<Pin>| {
            let input = cx.new(|cx| {
                TextInput::new(window, cx)
                    .max_len(length)
                    .filter(move |ch| ch.is_ascii_digit() || (letters && ch.is_ascii_alphabetic()))
            });
            let events = cx.subscribe_in(&input, window, |pin, input, event, window, cx| {
                let code = input.read(cx).text().to_uppercase();
                if *event == InputEvent::Changed && code.chars().count() == pin.length {
                    log::info!("pin input: complete");
                    if let Some(done) = pin.on_complete.clone() {
                        done(&code, window, cx);
                    }
                }
            });
            Pin {
                input,
                length,
                attempt,
                on_complete: None,
                _events: events,
            }
        });
        state.update(cx, |pin, cx| {
            pin.on_complete = self.on_complete.clone();
            if pin.attempt != attempt {
                pin.attempt = attempt;
                log::info!("pin input: cleared for attempt {attempt}");
                pin.input.update(cx, |field, cx| field.set_text("", cx));
            }
        });
        let input = state.read(cx).input.clone();
        let (code, focused, caret_on) = {
            let field = input.read(cx);
            (
                field.text().to_uppercase(),
                field.focus().is_focused(window),
                field.caret_on(),
            )
        };
        let end = code.len();
        let selection = input.read(cx).selection();
        let whole = focused && end > 0 && selection == (0..end);
        if focused && !whole && selection != (end..end) {
            input.update(cx, |field, cx| field.select(end..end, cx));
        }
        let filled: Vec<char> = code.chars().collect();
        let active = filled.len().min(length - 1);
        let theme = cx.theme();
        let colors = &theme.colors;
        let side = theme.control_height(ControlSize::Lg) * 1.25;
        let boxes = (0..length).map(|ix| {
            let current = focused && ix == active;
            let glyph = filled.get(ix).map(|ch| if self.masked { '•' } else { *ch });
            div()
                .relative()
                .flex()
                .items_center()
                .justify_center()
                .size(side)
                .rounded(theme.radius(Radius::Md))
                .border_1()
                .border_color(if current {
                    colors.focus
                } else {
                    colors.border_strong
                })
                .bg(if whole && glyph.is_some() {
                    colors.selection
                } else {
                    colors.surface
                })
                .text_size(theme.text_size(TextSize::Xl))
                .font_weight(FontWeight::MEDIUM)
                .text_color(colors.fg)
                .when_some(glyph, |cell, glyph| {
                    cell.child(SharedString::from(glyph.to_string()))
                })
                .when(current && glyph.is_none() && caret_on, |cell| {
                    cell.child(
                        div()
                            .w(theme.caret_width())
                            .h(theme.text_size(TextSize::Xl))
                            .bg(colors.focus),
                    )
                })
        });
        div()
            .relative()
            .flex()
            .gap_2()
            .children(boxes)
            .child(div().absolute().inset_0().opacity(0.0).child(input))
    }
}
