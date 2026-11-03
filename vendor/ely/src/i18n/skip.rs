use gpui::{
    App, ElementId, FocusHandle, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    SharedString, StatefulInteractiveElement, Styled, Window, anchored, div, prelude::*,
};

use crate::{
    primitives::{raise, tab_stop},
    theme::{ActiveTheme, Elevation, Radius, TextSize},
};

/// A link that shows only while focused and hands focus to `target`. Put it before what a keyboard would otherwise tab through, such as a long nav; it takes no room at rest and lies over the page while shown.
#[derive(IntoElement)]
pub struct SkipLink {
    id: ElementId,
    label: SharedString,
    target: FocusHandle,
}

impl SkipLink {
    pub fn new(
        id: impl Into<ElementId>,
        label: impl Into<SharedString>,
        target: &FocusHandle,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            target: target.clone(),
        }
    }
}

impl RenderOnce for SkipLink {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let handle = tab_stop(self.id.clone(), true, window, cx);
        let shown = handle.is_focused(window);
        let theme = cx.theme();
        let (target, label) = (self.target, self.label.clone());
        let skip = move |window: &mut Window, cx: &mut App| {
            log::info!("skip link {label:?}: focus to its target");
            window.focus(&target, cx);
        };
        let press = skip.clone();
        let pill = div()
            .id((self.id.clone(), "pill"))
            .occlude()
            .px_3()
            .py_1p5()
            .rounded(theme.radius(Radius::Md))
            .bg(theme.colors.overlay)
            .border_1()
            .border_color(theme.colors.focus)
            .shadow(theme.elevation(Elevation::Floating))
            .text_size(theme.text_size(TextSize::Sm))
            .text_color(theme.colors.fg)
            .cursor_pointer()
            .child(self.label)
            .on_click(move |_, window, cx| press(window, cx));
        div()
            .id(self.id.clone())
            .track_focus(&handle)
            .on_click(move |_, window, cx| skip(window, cx))
            .when(shown, |link| {
                link.child(raise(
                    self.id.clone(),
                    anchored().snap_to_window().child(div().p_2().child(pill)),
                ))
            })
    }
}
