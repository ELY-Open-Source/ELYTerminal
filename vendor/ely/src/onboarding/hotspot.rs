use std::rc::Rc;

use gpui::{
    App, ElementId, FontWeight, InteractiveElement, IntoElement, MouseButton, ParentElement,
    RenderOnce, SharedString, StatefulInteractiveElement, Styled, Window, div,
};

use crate::{
    buttons::{Button, ButtonVariant},
    forms::Run,
    motion::Pulse,
    overlays::Popover,
    primitives::FocusRing,
    theme::{ActiveTheme, ControlSize, TextSize},
};

/// The dot: a Tab stop that opens the tip on a press or on Enter.
#[derive(IntoElement)]
struct Dot {
    id: ElementId,
    toggle: Run,
}

impl RenderOnce for Dot {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let side = theme.onboarding().hotspot;
        let toggle = self.toggle;
        div()
            .id(self.id.clone())
            .flex()
            .flex_none()
            .items_center()
            .justify_center()
            .size(side * 2.0)
            .rounded_full()
            .border_1()
            .border_color(gpui::transparent_black())
            .tab_index(0)
            .focus_ring(cx)
            .cursor_pointer()
            .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
            .on_click(move |_, window, cx| toggle(window, cx))
            .child(
                Pulse::new((self.id, "pulse"))
                    .size(side)
                    .rounded_full()
                    .bg(theme.colors.accent),
            )
    }
}

/// A dot that calls out something new, with rings that widen from it; a press or Enter opens a tip beside it with a title, a line and Got it. Under reduced motion no ring shows.
#[derive(IntoElement)]
pub struct Hotspot {
    id: ElementId,
    title: SharedString,
    body: SharedString,
    on_dismiss: Run,
}

impl Hotspot {
    pub fn new(
        id: impl Into<ElementId>,
        title: impl Into<SharedString>,
        body: impl Into<SharedString>,
        on_dismiss: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            body: body.into(),
            on_dismiss: Rc::new(on_dismiss),
        }
    }
}

impl RenderOnce for Hotspot {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let id = self.id;
        let dismiss = self.on_dismiss;
        let (dot, got_it) = ((id.clone(), "dot"), (id.clone(), "got-it"));
        let (title, body) = (self.title, self.body);
        Popover::with_opener(
            id.clone(),
            move |toggle| Dot {
                id: dot.into(),
                toggle,
            },
            move |close: Run, _: &mut Window, cx: &mut App| {
                let theme = cx.theme();
                let prose =
                    |text: SharedString| div().flex().child(div().flex_1().min_w_0().child(text));
                div()
                    .debug_selector(|| "hotspot-tip".into())
                    .w(theme.tooltip_max_width())
                    .flex()
                    .flex_col()
                    .gap_2()
                    .text_size(theme.text_size(TextSize::Sm))
                    .child(prose(title).font_weight(FontWeight::SEMIBOLD))
                    .child(prose(body).text_color(theme.colors.fg_muted))
                    .child(
                        div().flex().justify_end().pt_1().child(
                            Button::new(got_it, "Got it")
                                .variant(ButtonVariant::Primary)
                                .size(ControlSize::Sm)
                                .on_click(move |_, window, cx| {
                                    log::info!("hotspot: dismissed");
                                    close(window, cx);
                                    dismiss(window, cx)
                                }),
                        ),
                    )
            },
        )
    }
}
