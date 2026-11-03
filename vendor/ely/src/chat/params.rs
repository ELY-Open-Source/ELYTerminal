use std::rc::Rc;

use gpui::{
    App, ElementId, Entity, FontWeight, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    SharedString, StatefulInteractiveElement, Styled, Window, div,
};

use crate::{
    buttons::{Button, ButtonVariant},
    forms::{Input, Run, Slider, TextInput},
    primitives::Tooltip,
    theme::{ActiveTheme, ControlSize, TextSize},
    typography::{
        Ellipsis,
        format::{currency, decimals, group_digits},
        tabular,
    },
};

/// Share of a limit where a count turns amber.
const NEAR: f64 = 0.9;

/// One tuning knob: its key and name, its value, range and step, and a line on what it does.
#[derive(Clone, Debug, PartialEq)]
pub struct Parameter {
    pub key: SharedString,
    pub label: SharedString,
    pub value: f64,
    pub min: f64,
    pub max: f64,
    pub step: f64,
    pub hint: Option<SharedString>,
}

type OnParameter = Rc<dyn Fn(&SharedString, f64, &mut Window, &mut App)>;

/// Places that show the value, both ends and every step exactly.
fn places(parameter: &Parameter) -> usize {
    [parameter.value, parameter.min, parameter.max]
        .into_iter()
        .filter(|number| *number != 0.0)
        .map(|number| decimals(number.abs()))
        .fold(decimals(parameter.step), usize::max)
}

/// How answers are made, knob by knob: each a slider with its value, such as temperature, top p and the longest answer.
#[derive(IntoElement)]
pub struct ParameterPanel {
    id: ElementId,
    parameters: Vec<Parameter>,
    on_change: OnParameter,
}

impl ParameterPanel {
    /// `on_change` gets a parameter's key and its new value.
    pub fn new(
        id: impl Into<ElementId>,
        parameters: impl IntoIterator<Item = Parameter>,
        on_change: impl Fn(&SharedString, f64, &mut Window, &mut App) + 'static,
    ) -> Self {
        let mut parameters: Vec<Parameter> = parameters.into_iter().collect();
        for parameter in &mut parameters {
            assert!(
                parameter.min < parameter.max
                    && parameter.step > 0.0
                    && parameter.value.is_finite(),
                "parameter {} has a bad range",
                parameter.key
            );
            if !(parameter.min..=parameter.max).contains(&parameter.value) {
                log::error!(
                    "parameters: {} at {} lies outside {}..={}; pegged",
                    parameter.key,
                    parameter.value,
                    parameter.min,
                    parameter.max
                );
                parameter.value = parameter.value.clamp(parameter.min, parameter.max);
            }
        }
        Self {
            id: id.into(),
            parameters,
            on_change: Rc::new(on_change),
        }
    }
}

impl RenderOnce for ParameterPanel {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        div()
            .flex()
            .flex_col()
            .gap_4()
            .children(self.parameters.into_iter().map(|parameter| {
                let change = self.on_change.clone();
                let key = parameter.key.clone();
                let places = places(&parameter);
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .flex()
                            .justify_between()
                            .text_size(theme.text_size(TextSize::Sm))
                            .gap_2()
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(Ellipsis::new(parameter.label)),
                            )
                            .child(
                                tabular(div().flex_none().text_color(colors.fg_muted))
                                    .child(format!("{:.places$}", parameter.value)),
                            ),
                    )
                    .child(
                        Slider::new((self.id.clone(), format!("slider-{key}")), parameter.value)
                            .range(parameter.min, parameter.max)
                            .step(parameter.step)
                            .on_change(move |value, window, cx| {
                                log::info!("parameters: {key} {value}");
                                change(&key, value, window, cx)
                            }),
                    )
                    .children(parameter.hint.map(|hint| {
                        div()
                            .text_size(theme.text_size(TextSize::Xs))
                            .text_color(colors.fg_subtle)
                            .child(hint)
                    }))
            }))
    }
}

/// A token count against a limit: quiet while there is room, amber near the end, red past it.
#[derive(IntoElement)]
pub struct TokenCounter {
    tokens: usize,
    limit: Option<usize>,
}

impl TokenCounter {
    /// `tokens` as the model's own tokenizer counts them.
    pub fn new(tokens: usize) -> Self {
        Self {
            tokens,
            limit: None,
        }
    }

    pub fn limit(mut self, limit: usize) -> Self {
        assert!(limit > 0, "a limit of none takes no tokens");
        self.limit = Some(limit);
        self
    }
}

impl RenderOnce for TokenCounter {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let count = group_digits(&self.tokens.to_string(), ',');
        let (text, color) = match self.limit {
            Some(limit) => {
                let share = self.tokens as f64 / limit as f64;
                let color = if share > 1.0 {
                    colors.danger
                } else if share >= NEAR {
                    colors.warning
                } else {
                    colors.fg_subtle
                };
                (
                    format!("{count} / {} tokens", group_digits(&limit.to_string(), ',')),
                    color,
                )
            }
            None => (format!("{count} tokens"), colors.fg_subtle),
        };
        tabular(
            div()
                .text_size(theme.text_size(TextSize::Xs))
                .text_color(color),
        )
        .child(text)
    }
}

/// What tokens cost in dollars, at prices per million tokens in and out.
pub(crate) fn cost(input: usize, output: usize, per_million_in: f64, per_million_out: f64) -> f64 {
    (input as f64 * per_million_in + output as f64 * per_million_out) / 1_000_000.0
}

/// What a message may cost: its tokens in and out at the model's prices per million; the parts in a tooltip.
#[derive(IntoElement)]
pub struct CostEstimator {
    id: ElementId,
    input: usize,
    output: usize,
    prices: (f64, f64),
}

impl CostEstimator {
    /// `prices` are dollars per million tokens, in and out.
    pub fn new(id: impl Into<ElementId>, input: usize, output: usize, prices: (f64, f64)) -> Self {
        assert!(prices.0 >= 0.0 && prices.1 >= 0.0, "a price below zero");
        Self {
            id: id.into(),
            input,
            output,
            prices,
        }
    }
}

impl RenderOnce for CostEstimator {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let dollars = cost(self.input, self.output, self.prices.0, self.prices.1);
        let shown = if dollars > 0.0 && dollars < 0.01 {
            "< $0.01".to_string()
        } else {
            format!("≈ {}", currency(dollars, "USD"))
        };
        let parts = format!(
            "{} in, {} out",
            group_digits(&self.input.to_string(), ','),
            group_digits(&self.output.to_string(), ',')
        );
        tabular(
            div()
                .id(self.id)
                .text_size(theme.text_size(TextSize::Xs))
                .text_color(theme.colors.fg_subtle)
                .tooltip(Tooltip::with_meta(shown.clone(), parts)),
        )
        .child(shown)
    }
}

/// The instructions every answer follows: a titled field, its tokens against the limit, and a way back to the default.
#[derive(IntoElement)]
pub struct SystemPromptEditor {
    id: ElementId,
    field: Entity<TextInput>,
    tokens: usize,
    limit: usize,
    on_reset: Option<Run>,
}

impl SystemPromptEditor {
    /// `tokens` counts the field's text, as the model's tokenizer does.
    pub fn new(
        id: impl Into<ElementId>,
        field: &Entity<TextInput>,
        tokens: usize,
        limit: usize,
    ) -> Self {
        Self {
            id: id.into(),
            field: field.clone(),
            tokens,
            limit,
            on_reset: None,
        }
    }

    /// Offers a way back to the default instructions.
    pub fn on_reset(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_reset = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for SystemPromptEditor {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_size(theme.text_size(TextSize::Sm))
                            .font_weight(FontWeight::MEDIUM)
                            .child("System prompt"),
                    )
                    .children(self.on_reset.map(|reset| {
                        Button::new((self.id.clone(), "reset"), "Reset")
                            .variant(ButtonVariant::Ghost)
                            .size(ControlSize::Sm)
                            .on_click(move |_, window, cx| {
                                log::info!("system prompt: reset");
                                reset(window, cx)
                            })
                    })),
            )
            .child(Input::new(&self.field))
            .child(
                div()
                    .flex()
                    .justify_end()
                    .child(TokenCounter::new(self.tokens).limit(self.limit)),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::{Parameter, ParameterPanel, cost, places};

    #[test]
    fn cost_counts_tokens_at_their_prices() {
        assert_eq!(cost(1_000_000, 0, 3.0, 15.0), 3.0);
        assert_eq!(cost(2_000, 1_000, 3.0, 15.0), 0.021);
        assert_eq!(cost(0, 0, 3.0, 15.0), 0.0);
    }

    #[test]
    fn a_value_shows_the_places_of_its_step_and_range() {
        let knob = |value, min, max, step| Parameter {
            key: "k".into(),
            label: "K".into(),
            value,
            min,
            max,
            step,
            hint: None,
        };
        assert_eq!(
            places(&knob(0.25, 0.25, 1.25, 0.5)),
            2,
            "the range starts off the step's grid"
        );
        assert_eq!(
            places(&knob(1.0, 0.0, 1.25, 0.5)),
            2,
            "the top end needs two"
        );
        assert_eq!(places(&knob(0.7, 0.0, 2.0, 0.1)), 1);
        assert_eq!(
            places(&knob(-2.0, -2.0, 2.0, 1.0)),
            0,
            "whole numbers on both sides"
        );
        assert_eq!(
            places(&knob(0.1 + 0.2, 0.0, 1.0, 0.1)),
            1,
            "float noise adds no places"
        );
    }

    #[test]
    fn a_value_outside_its_range_pegs() {
        let knob = |value| Parameter {
            key: "temperature".into(),
            label: "Temperature".into(),
            value,
            min: 0.0,
            max: 2.0,
            step: 0.1,
            hint: None,
        };
        let panel =
            ParameterPanel::new("knobs", [knob(3.5), knob(-1.0), knob(0.7)], |_, _, _, _| {});
        let values: Vec<f64> = panel.parameters.iter().map(|knob| knob.value).collect();
        assert_eq!(values, [2.0, 0.0, 0.7]);
    }
}
