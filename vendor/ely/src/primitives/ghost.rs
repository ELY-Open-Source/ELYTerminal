use gpui::{
    App, AppContext, Context, Entity, IntoElement, ParentElement, Render, SharedString, Styled,
    Window, div, prelude::*,
};

use super::{Icon, IconName};
use crate::theme::{ActiveTheme, Elevation, IconSize, Radius, TextSize};

/// What follows the pointer while a tab or panel drags.
pub struct DragGhost {
    title: SharedString,
    icon: Option<IconName>,
}

impl DragGhost {
    pub fn new(title: SharedString, icon: Option<IconName>, cx: &mut App) -> Entity<Self> {
        cx.new(|_| Self { title, icon })
    }
}

impl Render for DragGhost {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        div()
            .flex()
            .items_center()
            .gap_1p5()
            .px_2()
            .py_1()
            .rounded(theme.radius(Radius::Md))
            .bg(theme.colors.overlay)
            .border_1()
            .border_color(theme.colors.border)
            .shadow(theme.elevation(Elevation::Floating))
            .text_size(theme.text_size(TextSize::Sm))
            .text_color(theme.colors.fg)
            .when_some(self.icon, |ghost, icon| {
                ghost.child(
                    Icon::new(icon)
                        .size(IconSize::Sm)
                        .color(theme.colors.fg_muted),
                )
            })
            .child(self.title.clone())
    }
}
