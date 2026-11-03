use std::rc::Rc;

use gpui::{
    AnyElement, App, ElementId, FontWeight, InteractiveElement, IntoElement, ParentElement,
    RenderOnce, SharedString, Styled, Window, div, prelude::*,
};
use smallvec::SmallVec;

use crate::{
    buttons::{Button, ButtonVariant},
    forms::Run,
    theme::{ActiveTheme, Radius, TextSize},
};

/// Asks before an action runs, in place: what will happen, any detail, then Cancel and a confirming button.
#[derive(IntoElement)]
pub struct ConfirmationCard {
    id: ElementId,
    title: SharedString,
    body: Option<SharedString>,
    detail: SmallVec<[AnyElement; 2]>,
    confirm: SharedString,
    destructive: bool,
    on_confirm: Option<Run>,
    on_cancel: Option<Run>,
}

impl ConfirmationCard {
    pub fn new(id: impl Into<ElementId>, title: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            body: None,
            detail: SmallVec::new(),
            confirm: "Confirm".into(),
            destructive: false,
            on_confirm: None,
            on_cancel: None,
        }
    }

    pub fn body(mut self, text: impl Into<SharedString>) -> Self {
        self.body = Some(text.into());
        self
    }

    /// The confirming button's label.
    pub fn confirm(mut self, label: impl Into<SharedString>) -> Self {
        self.confirm = label.into();
        self
    }

    /// The confirming button turns danger.
    pub fn destructive(mut self) -> Self {
        self.destructive = true;
        self
    }

    pub fn on_confirm(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_confirm = Some(Rc::new(handler));
        self
    }

    pub fn on_cancel(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_cancel = Some(Rc::new(handler));
        self
    }
}

impl ParentElement for ConfirmationCard {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.detail.extend(elements);
    }
}

impl RenderOnce for ConfirmationCard {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = &theme.colors;
        let (id, confirm, cancel) = (self.id.clone(), self.on_confirm, self.on_cancel);
        let variant = if self.destructive {
            ButtonVariant::Danger
        } else {
            ButtonVariant::Primary
        };
        let run = |handler: Option<Run>, what: &'static str| {
            let id = id.clone();
            move |_: &gpui::ClickEvent, window: &mut Window, cx: &mut App| {
                log::info!("confirmation {id:?}: {what}");
                if let Some(handler) = &handler {
                    handler(window, cx);
                }
            }
        };
        div()
            .id(self.id.clone())
            .flex()
            .flex_col()
            .gap_3()
            .p_4()
            .rounded(theme.radius(Radius::Lg))
            .border_1()
            .border_color(colors.border)
            .bg(colors.surface)
            .text_size(theme.text_size(TextSize::Base))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(colors.fg)
                            .child(self.title),
                    )
                    .when_some(self.body, |words, body| {
                        words.child(div().text_color(colors.fg_muted).child(body))
                    }),
            )
            .when(!self.detail.is_empty(), |card| {
                card.child(div().children(self.detail))
            })
            .child(
                div()
                    .flex()
                    .justify_end()
                    .gap_2()
                    .child(
                        Button::new((self.id.clone(), "cancel"), "Cancel")
                            .variant(ButtonVariant::Ghost)
                            .on_click(run(cancel, "cancelled")),
                    )
                    .child(
                        Button::new((self.id.clone(), "confirm"), self.confirm)
                            .variant(variant)
                            .on_click(run(confirm, "confirmed")),
                    ),
            )
    }
}
