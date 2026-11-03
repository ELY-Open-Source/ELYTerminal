use std::rc::Rc;

use gpui::{
    App, ElementId, Entity, FontWeight, IntoElement, ParentElement, RenderOnce, SharedString,
    Styled, Window, div,
};

use super::line::{OnText, send_line};
use crate::{
    buttons::{Button, ButtonVariant},
    data_display::UsageBar,
    forms::TextInput,
    primitives::{Icon, IconName},
    theme::{ActiveTheme, ControlSize, IconSize, Radius, TextSize},
    typography::{format::currency, tabular},
};

/// A question an agent waits on: a few answers to pick, a line to write one, and, once answered, the answer.
#[derive(IntoElement)]
pub struct HumanInputRequest {
    id: ElementId,
    question: SharedString,
    field: Entity<TextInput>,
    choices: Vec<SharedString>,
    answered: Option<SharedString>,
    on_answer: OnText,
}

impl HumanInputRequest {
    /// `field` is the owner's line for a written answer.
    pub fn new(
        id: impl Into<ElementId>,
        question: impl Into<SharedString>,
        field: &Entity<TextInput>,
        on_answer: impl Fn(&str, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            question: question.into(),
            field: field.clone(),
            choices: Vec::new(),
            answered: None,
            on_answer: Rc::new(on_answer),
        }
    }

    /// Answers to pick with a press.
    pub fn choices(mut self, choices: impl IntoIterator<Item = impl Into<SharedString>>) -> Self {
        self.choices = choices.into_iter().map(Into::into).collect();
        self
    }

    /// The answer given; the request stops asking.
    pub fn answered(mut self, answer: impl Into<SharedString>) -> Self {
        self.answered = Some(answer.into());
        self
    }
}

impl RenderOnce for HumanInputRequest {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let asking = self.answered.is_none();
        let choices = (asking && !self.choices.is_empty()).then(|| {
            div()
                .flex()
                .flex_wrap()
                .gap_2()
                .children(self.choices.iter().enumerate().map(|(ix, choice)| {
                    let (answer, picked) = (self.on_answer.clone(), choice.clone());
                    Button::new((self.id.clone(), format!("choice-{ix}")), choice.clone())
                        .variant(ButtonVariant::Secondary)
                        .size(ControlSize::Sm)
                        .on_click(move |_, window, cx| {
                            log::info!("human input: picked {ix}");
                            answer(&picked, window, cx)
                        })
                }))
        });
        let line = asking.then(|| {
            send_line(
                &self.field,
                Button::new((self.id.clone(), "reply"), "Reply").variant(ButtonVariant::Primary),
                self.on_answer.clone(),
                "human input",
                cx,
            )
        });
        let theme = cx.theme();
        let colors = theme.colors.clone();
        div()
            .flex()
            .flex_col()
            .gap_3()
            .p_4()
            .rounded(theme.radius(Radius::Lg))
            .border_1()
            .border_color(colors.border)
            .bg(colors.surface)
            .text_size(theme.text_size(TextSize::Sm))
            .child(
                div()
                    .flex()
                    .items_start()
                    .gap_2()
                    .child(
                        div().flex_none().child(
                            Icon::new(IconName::CircleHelp)
                                .size(IconSize::Sm)
                                .color(colors.fg_muted),
                        ),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(colors.fg)
                            .child(self.question),
                    ),
            )
            .children(self.answered.map(|answer| {
                div()
                    .flex()
                    .items_start()
                    .gap_2()
                    .text_color(colors.fg_muted)
                    .child(
                        div().flex_none().child(
                            Icon::new(IconName::Check)
                                .size(IconSize::Sm)
                                .color(colors.success),
                        ),
                    )
                    .child(div().flex_1().min_w_0().child(answer))
            }))
            .children(choices)
            .children(line)
    }
}

/// What a session cost, as a whole and part by part: the total, then a bar of each part's share with its amount.
#[derive(IntoElement)]
pub struct CostBreakdown {
    label: SharedString,
    code: &'static str,
    parts: Vec<(SharedString, f64)>,
}

impl CostBreakdown {
    /// `code` is the currency, such as "USD".
    pub fn new(label: impl Into<SharedString>, code: &'static str) -> Self {
        Self {
            label: label.into(),
            code,
            parts: Vec::new(),
        }
    }

    pub fn part(mut self, name: impl Into<SharedString>, cost: f64) -> Self {
        assert!(cost >= 0.0 && cost.is_finite(), "a cost of {cost}");
        self.parts.push((name.into(), cost));
        self
    }
}

impl RenderOnce for CostBreakdown {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let total: f64 = self.parts.iter().map(|(_, cost)| cost).sum();
        let code = self.code;
        let bar = (total > 0.0).then(|| {
            self.parts.into_iter().fold(
                UsageBar::new(total).amounts(move |amount| currency(amount, code)),
                |bar, (name, cost)| bar.part(name, cost),
            )
        });
        div()
            .flex()
            .flex_col()
            .gap_3()
            .text_size(theme.text_size(TextSize::Sm))
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_baseline()
                    .justify_between()
                    .gap_x_3()
                    .child(div().text_color(colors.fg_muted).child(self.label))
                    .child(
                        tabular(div())
                            .text_size(theme.text_size(TextSize::Lg))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(colors.fg)
                            .child(currency(total, code)),
                    ),
            )
            .children(bar)
    }
}
