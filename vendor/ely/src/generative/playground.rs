use std::{ops::Range, rc::Rc, time::Duration};

use gpui::{
    App, ElementId, Entity, FontWeight, HighlightStyle, IntoElement, ParentElement, RenderOnce,
    SharedString, Styled, StyledText, Window, div, prelude::*,
};

use super::prompt::OnText;
use crate::{
    buttons::{Button, ButtonVariant},
    forms::{Input, TextInput},
    motion::Spinner,
    primitives::IconName,
    theme::{ActiveTheme, ControlSize, IconSize, Radius, TextSize},
    typography::{
        Ellipsis,
        format::{plural, took},
    },
};

/// The `{names}` in a template, each once, in the order they first appear; a name is letters, digits and underscores.
pub(crate) fn variables(template: &str) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        let after = &rest[open + 1..];
        let Some(close) = after.find('}') else {
            break;
        };
        let name = &after[..close];
        let valid = !name.is_empty() && name.chars().all(|ch| ch.is_alphanumeric() || ch == '_');
        if !valid {
            rest = after;
            continue;
        }
        if !names.iter().any(|known| known == name) {
            names.push(name.to_string());
        }
        rest = &after[close + 1..];
    }
    names
}

/// The template with each named value put in; a name left blank keeps its `{name}`, and its byte range is returned so it can be marked.
pub(crate) fn fill(template: &str, values: &[(String, String)]) -> (String, Vec<Range<usize>>) {
    let (mut text, mut blanks) = (template.to_string(), Vec::new());
    for (name, value) in values.iter().filter(|(_, value)| !value.is_empty()) {
        text = text.replace(&format!("{{{name}}}"), value);
    }
    for (name, _) in values.iter().filter(|(_, value)| value.is_empty()) {
        let slot = format!("{{{name}}}");
        blanks.extend(text.match_indices(&slot).map(|(at, _)| at..at + slot.len()));
    }
    blanks.sort_by_key(|range| range.start);
    (text, blanks)
}

/// A prompt to try: a template whose `{names}` become fields, the prompt as it will go with blanks marked, Run once every name has a value, and what came back with its tokens and time.
#[derive(IntoElement)]
pub struct PromptPlayground {
    id: ElementId,
    template: Entity<TextInput>,
    running: bool,
    output: Option<SharedString>,
    stats: Option<(usize, Duration)>,
    on_run: Option<OnText>,
}

impl PromptPlayground {
    /// `template` is the owner's field for the prompt.
    pub fn new(id: impl Into<ElementId>, template: &Entity<TextInput>) -> Self {
        Self {
            id: id.into(),
            template: template.clone(),
            running: false,
            output: None,
            stats: None,
            on_run: None,
        }
    }

    /// While the model answers.
    pub fn running(mut self, running: bool) -> Self {
        self.running = running;
        self
    }

    /// What came back, and its tokens and time.
    pub fn output(mut self, text: impl Into<SharedString>, tokens: usize, took: Duration) -> Self {
        self.output = Some(text.into());
        self.stats = Some((tokens, took));
        self
    }

    /// Gets the prompt with its values in.
    pub fn on_run(mut self, handler: impl Fn(&str, &mut Window, &mut App) + 'static) -> Self {
        self.on_run = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for PromptPlayground {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let template = self.template.read(cx).text().to_string();
        let names = variables(&template);
        let fields: Vec<(String, Entity<TextInput>)> = names
            .into_iter()
            .map(|name| {
                let placeholder = name.clone();
                let field = window.use_keyed_state(
                    (self.id.clone(), format!("var-{name}")),
                    cx,
                    |window, cx| TextInput::new(window, cx).placeholder(placeholder),
                );
                (name, field)
            })
            .collect();
        let values: Vec<(String, String)> = fields
            .iter()
            .map(|(name, field)| (name.clone(), field.read(cx).text().to_string()))
            .collect();
        let (filled, blanks) = fill(&template, &values);
        let ready = !template.trim().is_empty() && blanks.is_empty() && !self.running;
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let label = |text: &'static str| {
            div()
                .text_size(theme.text_size(TextSize::Xs))
                .font_weight(FontWeight::MEDIUM)
                .text_color(colors.fg_muted)
                .child(text)
        };
        let blank = HighlightStyle {
            background_color: Some(colors.warning_subtle),
            ..HighlightStyle::default()
        };
        let sent = filled.clone();
        let run = self.on_run.map(|run| {
            Button::new((self.id.clone(), "run"), "Run")
                .variant(ButtonVariant::Primary)
                .size(ControlSize::Sm)
                .icon(IconName::Play)
                .disabled(!ready)
                .on_click(move |_, window, cx| {
                    log::info!("prompt playground: ran {} characters", sent.len());
                    run(&sent, window, cx)
                })
        });
        let stats = self.stats.map(|(tokens, spent)| {
            format!(
                "{} · {}",
                plural(tokens as u64, "token", "tokens"),
                took(spent)
            )
        });
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(label("Template"))
            .child(Input::new(&self.template))
            .when(!fields.is_empty(), |column| {
                column
                    .child(label("Values"))
                    .child(div().flex().flex_col().gap_2().children(fields.iter().map(
                        |(name, field)| {
                            div()
                                .flex()
                                .items_center()
                                .gap_3()
                                .child(
                                    div()
                                        .flex_none()
                                        .w(theme.label_width() * 0.6)
                                        .font_family(theme.mono_family.clone())
                                        .text_size(theme.text_size(TextSize::Xs))
                                        .text_color(colors.fg_muted)
                                        .child(Ellipsis::new(format!("{{{name}}}"))),
                                )
                                .child(div().flex_1().min_w_0().child(Input::new(field)))
                        },
                    )))
            })
            .child(label("Prompt"))
            .child(
                div()
                    .p_3()
                    .rounded(theme.radius(Radius::Md))
                    .bg(colors.sunken)
                    .text_size(theme.text_size(TextSize::Sm))
                    .text_color(colors.fg)
                    .child(
                        StyledText::new(filled)
                            .with_highlights(blanks.into_iter().map(|range| (range, blank))),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .child(div().flex().children(run))
                    .children(stats.map(|stats| {
                        div()
                            .text_size(theme.text_size(TextSize::Xs))
                            .text_color(colors.fg_muted)
                            .child(stats)
                    })),
            )
            .when(self.running, |column| {
                column.child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .text_size(theme.text_size(TextSize::Sm))
                        .text_color(colors.fg_muted)
                        .child(Spinner::new((self.id.clone(), "running")).size(IconSize::Sm))
                        .child("Running"),
                )
            })
            .when_some(self.output.filter(|_| !self.running), |column, output| {
                column.child(
                    div()
                        .p_3()
                        .rounded(theme.radius(Radius::Md))
                        .border_1()
                        .border_color(colors.border)
                        .bg(colors.surface)
                        .text_size(theme.text_size(TextSize::Sm))
                        .text_color(colors.fg)
                        .child(output),
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::{fill, variables};

    #[test]
    fn names_come_once_in_order_and_odd_braces_are_skipped() {
        assert_eq!(
            variables("Write {tone} notes on {topic} in a {tone} voice"),
            ["tone", "topic"]
        );
        assert_eq!(variables("{a b} then {c} and {} and {d"), ["c"]);
        assert_eq!(variables("{{x}}"), ["x"]);
    }

    #[test]
    fn values_go_in_and_blanks_keep_their_names() {
        let values = [
            ("topic".to_string(), "dunes".to_string()),
            ("tone".to_string(), String::new()),
        ];
        let (text, blanks) = fill("On {topic}, {tone}; {topic}.", &values);
        assert_eq!(text, "On dunes, {tone}; dunes.");
        let marked: Vec<&str> = blanks.iter().map(|range| &text[range.clone()]).collect();
        assert_eq!(marked, ["{tone}"]);
    }
}
