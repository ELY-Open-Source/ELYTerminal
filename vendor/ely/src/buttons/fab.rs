use gpui::{
    Animation, AnimationExt, App, ClickEvent, ElementId, FontWeight, InteractiveElement,
    IntoElement, MouseButton, ParentElement, RenderOnce, SharedString, StatefulInteractiveElement,
    Styled, Window, div, prelude::*,
};

use super::button::ClickHandler;
use crate::{
    motion,
    primitives::{FocusRing, Icon, IconName, Tooltip},
    theme::{ActiveTheme, Elevation, IconSize, TextSize},
};

/// A raised round button for a screen's main action; a label makes it a pill.
#[derive(IntoElement)]
pub struct FloatingActionButton {
    id: ElementId,
    icon: IconName,
    label: Option<SharedString>,
    tooltip: Option<SharedString>,
    on_click: Option<ClickHandler>,
}

impl FloatingActionButton {
    pub fn new(id: impl Into<ElementId>, icon: IconName) -> Self {
        Self {
            id: id.into(),
            icon,
            label: None,
            tooltip: None,
            on_click: None,
        }
    }

    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn tooltip(mut self, text: impl Into<SharedString>) -> Self {
        self.tooltip = Some(text.into());
        self
    }

    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }
}

impl RenderOnce for FloatingActionButton {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = &theme.colors;
        let side = theme.fab_size();
        let (hover, lifted) = (colors.accent_hover, theme.elevation(Elevation::Modal));
        div()
            .id(self.id)
            .flex()
            .flex_none()
            .items_center()
            .justify_center()
            .gap_2()
            .h(side)
            .map(|fab| match &self.label {
                Some(_) => fab.pl_5().pr_6(),
                None => fab.w(side),
            })
            .rounded_full()
            .bg(colors.accent)
            .text_color(colors.on_accent)
            .text_size(theme.text_size(TextSize::Md))
            .font_weight(FontWeight::MEDIUM)
            .border_1()
            .border_color(gpui::transparent_black())
            .shadow(theme.elevation(Elevation::Floating))
            .cursor_pointer()
            .tab_index(0)
            .focus_ring(cx)
            .hover(move |style| style.bg(hover).shadow(lifted))
            .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
            .when_some(self.on_click, |fab, handler| fab.on_click(handler))
            .when_some(self.tooltip, |fab, text| fab.tooltip(Tooltip::text(text)))
            .child(
                Icon::new(self.icon)
                    .size(IconSize::Lg)
                    .color(colors.on_accent),
            )
            .when_some(self.label, |fab, label| fab.child(label))
            .with_animation(
                "fab-in",
                Animation::new(motion::duration(motion::SLOW, cx))
                    .with_easing(motion::ease_out_cubic),
                |fab, t| fab.opacity(t).mt(motion::NUDGE * 2.0 * (1.0 - t)),
            )
    }
}
