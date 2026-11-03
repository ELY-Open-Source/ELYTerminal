use gpui::{App, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window, div};

use crate::theme::{ActiveTheme, TextSize};

/// Hairline rule, across or down.
#[derive(IntoElement)]
pub struct Divider {
    vertical: bool,
    label: Option<SharedString>,
}

impl Divider {
    pub fn horizontal() -> Self {
        Self {
            vertical: false,
            label: None,
        }
    }

    pub fn vertical() -> Self {
        Self {
            vertical: true,
            label: None,
        }
    }

    /// Text centered on a horizontal rule.
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }
}

impl RenderOnce for Divider {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let line = || div().border_color(theme.colors.border);
        if self.vertical {
            return line().h_full().border_l_1();
        }
        match self.label {
            None => line().border_t_1(),
            Some(label) => div()
                .flex()
                .items_center()
                .gap_3()
                .child(line().flex_1().border_t_1())
                .child(
                    div()
                        .text_size(theme.text_size(TextSize::Xs))
                        .text_color(theme.colors.fg_subtle)
                        .child(label),
                )
                .child(line().flex_1().border_t_1()),
        }
    }
}
