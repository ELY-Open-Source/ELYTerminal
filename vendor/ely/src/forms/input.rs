use gpui::{
    AnyElement, App, Entity, InteractiveElement, IntoElement, MouseButton, ParentElement,
    RenderOnce, SharedString, Styled, Window, div, prelude::*,
};

use super::TextInput;
use crate::i18n;
use crate::{
    buttons::IconButton,
    primitives::{Icon, IconName},
    theme::{ActiveTheme, ControlSize, IconSize, Radius, TextSize},
};

/// A text field's frame: border, focus, and room for things before and after.
#[derive(IntoElement)]
pub struct Input {
    state: Entity<TextInput>,
    size: ControlSize,
    prefix: Option<AnyElement>,
    suffix: Option<AnyElement>,
    clearable: bool,
    invalid: bool,
    least: Option<SharedString>,
}

impl Input {
    pub fn new(state: &Entity<TextInput>) -> Self {
        Self {
            state: state.clone(),
            size: ControlSize::default(),
            prefix: None,
            suffix: None,
            clearable: false,
            invalid: false,
            least: None,
        }
    }

    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    pub fn prefix(mut self, prefix: impl IntoElement) -> Self {
        self.prefix = Some(prefix.into_any_element());
        self
    }

    pub fn suffix(mut self, suffix: impl IntoElement) -> Self {
        self.suffix = Some(suffix.into_any_element());
        self
    }

    /// Holds room to show `text` whole.
    pub fn least(mut self, text: impl Into<SharedString>) -> Self {
        self.least = Some(text.into());
        self
    }

    /// Adds a clear button while there is text.
    pub fn clearable(mut self) -> Self {
        self.clearable = true;
        self
    }

    /// Draws the frame in the danger tone.
    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }
}

pub(crate) fn text_size(size: ControlSize) -> TextSize {
    match size {
        ControlSize::Sm => TextSize::Sm,
        ControlSize::Md => TextSize::Base,
        ControlSize::Lg => TextSize::Md,
    }
}

impl RenderOnce for Input {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let input = self.state.read(cx);
        let focus = input.focus().clone();
        let (focused, empty, multi, disabled) = (
            focus.is_focused(window),
            input.is_empty(),
            input.is_multi_line(),
            input.is_disabled(),
        );
        let theme = cx.theme();
        let colors = &theme.colors;
        let border = if self.invalid {
            colors.danger
        } else if focused {
            colors.focus
        } else {
            colors.border_strong
        };
        let (state, clear_id) = (self.state.clone(), self.state.entity_id());
        div()
            .debug_selector(|| "input-root".into())
            .flex()
            .gap_2()
            .px(theme.control_padding(self.size))
            .map(|frame| {
                if multi {
                    frame.items_start().py_1p5()
                } else {
                    frame.items_center().h(theme.control_height(self.size))
                }
            })
            .rounded(theme.radius(Radius::Md))
            .border_1()
            .border_color(border)
            .bg(if disabled {
                colors.sunken
            } else {
                colors.surface
            })
            .text_size(theme.text_size(text_size(self.size)))
            .text_color(if disabled {
                colors.fg_disabled
            } else {
                colors.fg
            })
            .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                window.focus(&focus, cx)
            })
            .children(self.prefix)
            .child(
                div()
                    .debug_selector(|| "input-text".into())
                    .flex_1()
                    .min_w_0()
                    .child(self.state)
                    // A strut keeps the frame's inset to type in.
                    .child(
                        div()
                            .px(theme.control_padding(self.size))
                            .border_x_1()
                            .h_0()
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .children(self.least),
                    ),
            )
            .when(self.clearable && !empty && !disabled, |frame| {
                frame.child(
                    IconButton::new(("input-clear", clear_id), IconName::CircleX)
                        .size(ControlSize::Sm)
                        .tooltip(i18n::text(cx, "input.clear", &[]))
                        .on_click(move |_, window, cx| {
                            state.update(cx, |input, cx| input.set_text("", cx));
                            window.focus(&state.read(cx).focus().clone(), cx);
                        }),
                )
            })
            .children(self.suffix)
    }
}

/// A masked field with a lock before it and an eye that shows the text.
#[derive(IntoElement)]
pub struct PasswordInput {
    state: Entity<TextInput>,
    size: ControlSize,
}

impl PasswordInput {
    /// Build the state with `TextInput::masked`.
    pub fn new(state: &Entity<TextInput>) -> Self {
        Self {
            state: state.clone(),
            size: ControlSize::default(),
        }
    }

    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }
}

impl RenderOnce for PasswordInput {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let masked = self.state.read(cx).is_masked();
        let (toggle, subtle) = (self.state.clone(), cx.theme().colors.fg_subtle);
        Input::new(&self.state)
            .size(self.size)
            .prefix(Icon::new(IconName::Lock).size(IconSize::Sm).color(subtle))
            .suffix(
                IconButton::new(
                    ("password-eye", self.state.entity_id()),
                    if masked {
                        IconName::Eye
                    } else {
                        IconName::EyeOff
                    },
                )
                .size(ControlSize::Sm)
                .tooltip(i18n::text(
                    cx,
                    if masked {
                        "input.show_password"
                    } else {
                        "input.hide_password"
                    },
                    &[],
                ))
                .on_click(move |_, _, cx| {
                    toggle.update(cx, |input, cx| {
                        let masked = input.is_masked();
                        log::info!(
                            "password input: {}",
                            if masked { "shown" } else { "hidden" }
                        );
                        input.set_masked(!masked, cx);
                    })
                }),
            )
    }
}
