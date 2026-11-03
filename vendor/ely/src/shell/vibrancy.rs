use gpui::{
    AnyElement, App, Div, IntoElement, ParentElement, RenderOnce, StyleRefinement, Styled, Window,
    div,
};

use crate::theme::ActiveTheme;

/// A pane the window's blur shows through: vibrancy on macOS, acrylic on Windows. Its window opens with `WindowBackgroundAppearance::Blurred` and nothing opaque beneath the pane; high contrast fills it solid.
#[derive(IntoElement)]
pub struct Vibrancy {
    base: Div,
}

impl Vibrancy {
    pub fn new() -> Self {
        Self { base: div() }
    }
}

impl Default for Vibrancy {
    fn default() -> Self {
        Self::new()
    }
}

impl Styled for Vibrancy {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl ParentElement for Vibrancy {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.base.extend(elements);
    }
}

impl RenderOnce for Vibrancy {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        self.base.bg(cx.theme().colors.glass)
    }
}
