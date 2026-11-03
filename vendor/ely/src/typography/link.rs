use gpui::{
    App, ClickEvent, ElementId, InteractiveElement, IntoElement, MouseButton, ParentElement,
    RenderOnce, SharedString, StatefulInteractiveElement, Styled, Window, div, prelude::*,
    transparent_black,
};

use crate::{
    primitives::{FocusRing, Icon, IconName},
    theme::{ActiveTheme, IconSize, Radius},
};

type ClickHandler = Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

/// Inline link that runs `on_click`; `ExternalLink` opens an address.
#[derive(IntoElement)]
pub struct Link {
    id: ElementId,
    label: SharedString,
    on_click: ClickHandler,
    external: bool,
}

impl Link {
    pub fn new(
        id: impl Into<ElementId>,
        label: impl Into<SharedString>,
        on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            on_click: Box::new(on_click),
            external: false,
        }
    }
}

impl RenderOnce for Link {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let color = theme.colors.link;
        div()
            .id(self.id)
            .flex()
            .items_center()
            .gap_0p5()
            .px_0p5()
            .rounded(theme.radius(Radius::Sm))
            .border_1()
            .border_color(transparent_black())
            .text_color(color)
            .cursor_pointer()
            .tab_index(0)
            .focus_ring(cx)
            .hover(|style| style.underline())
            .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
            .on_click(self.on_click)
            .child(self.label)
            .when(self.external, |link| {
                link.child(
                    Icon::new(IconName::ArrowUpRight)
                        .size(IconSize::Xs)
                        .color(color),
                )
            })
    }
}

/// Link that leaves the app, marked with an arrow.
#[derive(IntoElement)]
pub struct ExternalLink(Link);

impl ExternalLink {
    pub fn new(
        id: impl Into<ElementId>,
        label: impl Into<SharedString>,
        url: impl Into<SharedString>,
    ) -> Self {
        let url: SharedString = url.into();
        let mut link = Link::new(id, label, move |_, _, cx| cx.open_url(&url));
        link.external = true;
        Self(link)
    }
}

impl RenderOnce for ExternalLink {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, rc::Rc};

    use gpui::{
        Context, IntoElement, KeyUpEvent, Keystroke, ParentElement, Render, TestAppContext, Window,
        div,
    };

    use super::{ExternalLink, Link};
    use crate::theme::Theme;

    struct Links(Rc<Cell<u32>>);

    impl Render for Links {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let presses = self.0.clone();
            div()
                .child(Link::new("guide", "guide", move |_, _, _| {
                    presses.set(presses.get() + 1)
                }))
                .child(ExternalLink::new("site", "site", "https://example.com"))
        }
    }

    #[gpui::test]
    fn a_link_runs_its_handler_and_an_external_link_opens_its_address(cx: &mut TestAppContext) {
        cx.update(Theme::init);
        let presses = Rc::new(Cell::new(0));
        let seen = presses.clone();
        let (_, cx) = cx.add_window_view(|_, _| Links(seen));
        for _ in 0..2 {
            cx.update(|window, cx| window.focus_next(cx));
            cx.simulate_keystrokes("enter");
            cx.simulate_event(KeyUpEvent {
                keystroke: Keystroke::parse("enter").expect("a key"),
            });
        }
        assert_eq!(presses.get(), 1);
        assert_eq!(cx.opened_url().as_deref(), Some("https://example.com"));
    }
}
