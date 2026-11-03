use std::rc::Rc;

use gpui::{
    App, Div, ElementId, Entity, IntoElement, ParentElement, Refineable, RenderOnce, SharedString,
    StyleRefinement, Styled, Window, div,
};

use crate::{
    buttons::{Button, ButtonVariant, IconButton},
    forms::{Input, TextInput},
    primitives::{Icon, IconName},
    theme::{ActiveTheme, ControlSize, IconSize, Radius, TextSize},
};

type OnAsk = Rc<dyn Fn(SharedString, &mut Window, &mut App)>;
type OnClose = Rc<dyn Fn(&mut Window, &mut App)>;

/// A prompt between lines of code: ask for a change, send it, or close it. A working state shows while an answer comes.
#[derive(IntoElement)]
pub struct InlineChat {
    base: Div,
    id: ElementId,
    field: Entity<TextInput>,
    working: bool,
    on_ask: Option<OnAsk>,
    on_close: Option<OnClose>,
}

impl InlineChat {
    pub fn new(id: impl Into<ElementId>, field: &Entity<TextInput>) -> Self {
        Self {
            base: div(),
            id: id.into(),
            field: field.clone(),
            working: false,
            on_ask: None,
            on_close: None,
        }
    }

    /// An answer is on its way.
    pub fn working(mut self, working: bool) -> Self {
        self.working = working;
        self
    }

    pub fn on_ask(
        mut self,
        handler: impl Fn(SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_ask = Some(Rc::new(handler));
        self
    }

    pub fn on_close(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_close = Some(Rc::new(handler));
        self
    }
}

impl Styled for InlineChat {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for InlineChat {
    fn render(mut self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let asked = self.field.read(cx).text().trim().to_string();
        let (field, ask) = (self.field.clone(), self.on_ask.clone());
        let send = move |window: &mut Window, cx: &mut App| {
            let prompt = field.read(cx).text().trim().to_string();
            if prompt.is_empty() {
                return;
            }
            log::info!("inline chat: asked {prompt:?}");
            if let Some(ask) = &ask {
                ask(prompt.into(), window, cx);
            }
        };
        let mut chat = div();
        chat.style().refine(self.base.style());
        chat.flex()
            .flex_col()
            .justify_center()
            .gap_1p5()
            .px_3()
            .py_2()
            .rounded(theme.radius(Radius::Md))
            .border_1()
            .border_color(colors.border_strong)
            .bg(colors.surface)
            .text_size(theme.text_size(TextSize::Sm))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        Icon::new(IconName::Sparkles)
                            .size(IconSize::Sm)
                            .color(colors.accent),
                    )
                    .child(
                        div()
                            .flex_1()
                            .child(Input::new(&self.field).size(ControlSize::Sm)),
                    )
                    .child(
                        Button::new(
                            (self.id.clone(), "send"),
                            if self.working { "Working" } else { "Send" },
                        )
                        .variant(ButtonVariant::Primary)
                        .size(ControlSize::Sm)
                        .loading(self.working)
                        .disabled(asked.is_empty() || self.working)
                        .on_click(move |_, window, cx| send(window, cx)),
                    )
                    .children(self.on_close.map(|close| {
                        IconButton::new((self.id.clone(), "close"), IconName::X)
                            .variant(ButtonVariant::Ghost)
                            .size(ControlSize::Sm)
                            .tooltip("Close")
                            .on_click(move |_, window, cx| close(window, cx))
                    })),
            )
            .child(
                div()
                    .text_size(theme.text_size(TextSize::Xs))
                    .text_color(colors.fg_subtle)
                    .child("Enter sends · Escape closes"),
            )
    }
}
