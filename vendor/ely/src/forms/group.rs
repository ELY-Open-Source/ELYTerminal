use gpui::{
    AnyElement, App, Entity, InteractiveElement, IntoElement, MouseButton, ParentElement,
    RenderOnce, SharedString, Styled, Window, div, prelude::*,
};

use super::{TextInput, input::text_size};
use crate::theme::{ActiveTheme, ControlSize, Radius};

/// A block joined to a field's side: text such as `https://`, or a control.
#[derive(IntoElement)]
pub struct InputAddon {
    body: AnyElement,
}

impl InputAddon {
    pub fn text(text: impl Into<SharedString>) -> Self {
        Self {
            body: text.into().into_any_element(),
        }
    }

    pub fn new(body: impl IntoElement) -> Self {
        Self {
            body: body.into_any_element(),
        }
    }
}

impl RenderOnce for InputAddon {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        self.body
    }
}

/// A field with addons joined before and after it, in one frame.
#[derive(IntoElement)]
pub struct InputGroup {
    state: Entity<TextInput>,
    before: Vec<InputAddon>,
    after: Vec<InputAddon>,
    size: ControlSize,
}

impl InputGroup {
    pub fn new(state: &Entity<TextInput>) -> Self {
        Self {
            state: state.clone(),
            before: Vec::new(),
            after: Vec::new(),
            size: ControlSize::default(),
        }
    }

    pub fn before(mut self, addon: InputAddon) -> Self {
        self.before.push(addon);
        self
    }

    pub fn after(mut self, addon: InputAddon) -> Self {
        self.after.push(addon);
        self
    }

    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }
}

impl RenderOnce for InputGroup {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let input = self.state.read(cx);
        let focus = input.focus().clone();
        let focused = focus.is_focused(window);
        let theme = cx.theme();
        let colors = &theme.colors;
        let pad = theme.control_padding(self.size);
        let addon = |addon: InputAddon, before: bool| {
            div()
                .flex()
                .flex_none()
                .items_center()
                .h_full()
                .px(pad)
                .bg(colors.sunken)
                .text_color(colors.fg_muted)
                .map(|block| {
                    if before {
                        block.border_r_1()
                    } else {
                        block.border_l_1()
                    }
                })
                .border_color(colors.border_strong)
                .child(addon)
        };
        div()
            .debug_selector(|| "group-root".into())
            .flex()
            .items_center()
            .h(theme.control_height(self.size))
            .overflow_hidden()
            .rounded(theme.radius(Radius::Md))
            .border_1()
            .border_color(if focused {
                colors.focus
            } else {
                colors.border_strong
            })
            .bg(colors.surface)
            .text_size(theme.text_size(text_size(self.size)))
            .text_color(colors.fg)
            .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                window.focus(&focus, cx)
            })
            .children(self.before.into_iter().map(|block| addon(block, true)))
            .child(div().flex_1().min_w_0().px(pad).child(self.state))
            .children(self.after.into_iter().map(|block| addon(block, false)))
    }
}
