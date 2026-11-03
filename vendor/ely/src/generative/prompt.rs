use std::{ops::Range, rc::Rc};

use gpui::{
    App, ElementId, Entity, Focusable, FontWeight, HighlightStyle, IntoElement, ParentElement,
    RenderOnce, SharedString, Styled, StyledText, Window, div,
};
use similar::{ChangeTag, TextDiff};

use crate::{
    buttons::{Button, ButtonVariant},
    forms::{Input, OnFlag, TextInput},
    motion::{Entrance, Transition},
    primitives::{Icon, IconName},
    theme::{ActiveTheme, ControlSize, IconSize, Radius, TextSize},
};

pub(super) type OnText = Rc<dyn Fn(&str, &mut Window, &mut App)>;

/// Byte ranges of `new` that `old` lacks, word by word: neighbors joined, and anything but letters and digits trimmed from each end.
pub(crate) fn added_words(old: &str, new: &str) -> Vec<Range<usize>> {
    let diff = TextDiff::from_unicode_words(old, new);
    let mut ranges: Vec<Range<usize>> = Vec::new();
    let mut at = 0;
    for change in diff.iter_all_changes() {
        let len = change.value().len();
        match change.tag() {
            ChangeTag::Delete => continue,
            ChangeTag::Equal => {}
            ChangeTag::Insert => match ranges.last_mut() {
                Some(last) if last.end == at => last.end = at + len,
                _ => ranges.push(at..at + len),
            },
        }
        at += len;
    }
    ranges
        .into_iter()
        .filter_map(|range| {
            let word = &new[range.clone()];
            let glue = |ch: char| !ch.is_alphanumeric();
            let start = range.start + word.len() - word.trim_start_matches(glue).len();
            let end = range.end - (word.len() - word.trim_end_matches(glue).len());
            (start < end).then_some(start..end)
        })
        .collect()
}

/// A prompt and a way to rewrite it: Enhance asks the owner for a fuller prompt, offered with the words it adds washed; Use this takes it, Keep mine lets it go.
#[derive(IntoElement)]
pub struct PromptEnhancer {
    id: ElementId,
    field: Entity<TextInput>,
    enhancing: bool,
    suggestion: Option<SharedString>,
    on_enhance: Option<OnText>,
    on_resolve: Option<OnFlag>,
}

impl PromptEnhancer {
    /// `field` is the owner's prompt.
    pub fn new(id: impl Into<ElementId>, field: &Entity<TextInput>) -> Self {
        Self {
            id: id.into(),
            field: field.clone(),
            enhancing: false,
            suggestion: None,
            on_enhance: None,
            on_resolve: None,
        }
    }

    /// While the owner writes a suggestion.
    pub fn enhancing(mut self, enhancing: bool) -> Self {
        self.enhancing = enhancing;
        self
    }

    /// A fuller prompt to offer.
    pub fn suggestion(mut self, text: impl Into<SharedString>) -> Self {
        self.suggestion = Some(text.into());
        self
    }

    /// Gets the prompt to rewrite, trimmed.
    pub fn on_enhance(mut self, handler: impl Fn(&str, &mut Window, &mut App) + 'static) -> Self {
        self.on_enhance = Some(Rc::new(handler));
        self
    }

    /// Gets whether the suggestion was taken; a taken one is already in the field.
    pub fn on_resolve(mut self, handler: impl Fn(bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_resolve = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for PromptEnhancer {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let last = window.use_keyed_state((self.id.clone(), "offer"), cx, |_, _| {
            SharedString::default()
        });
        if let Some(text) = self
            .suggestion
            .as_ref()
            .filter(|text| *last.read(cx) != **text)
        {
            last.update(cx, |last, _| *last = text.clone());
        }
        let (offered, shown) = (last.read(cx).clone(), self.suggestion.is_some());
        let prompt = self.field.read(cx).text().trim().to_string();
        let words = added_words(&prompt, &offered);
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let enhance = self.on_enhance.map(|enhance| {
            Button::new((self.id.clone(), "enhance"), "Enhance")
                .variant(ButtonVariant::Ghost)
                .size(ControlSize::Sm)
                .icon(IconName::Sparkles)
                .loading(self.enhancing)
                .disabled(prompt.is_empty() || self.enhancing)
                .on_click(move |_, window, cx| {
                    log::info!("prompt enhancer: asked with {} characters", prompt.len());
                    enhance(&prompt, window, cx)
                })
        });
        let answer = |taken: bool| {
            let (field, text, resolve) =
                (self.field.clone(), offered.clone(), self.on_resolve.clone());
            move |_: &gpui::ClickEvent, window: &mut Window, cx: &mut App| {
                if taken {
                    field.update(cx, |input, cx| input.set_text(text.to_string(), cx));
                }
                log::info!(
                    "prompt enhancer: suggestion {}",
                    if taken { "taken" } else { "kept out" }
                );
                window.focus(&field.focus_handle(cx), cx);
                if let Some(resolve) = &resolve {
                    resolve(taken, window, cx);
                }
            }
        };
        let wash = HighlightStyle {
            background_color: Some(colors.success_subtle),
            ..HighlightStyle::default()
        };
        let card = div()
            .mt_3()
            .flex()
            .flex_col()
            .gap_3()
            .p_3()
            .rounded(theme.radius(Radius::Md))
            .border_1()
            .border_color(colors.border)
            .bg(colors.surface)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1p5()
                    .text_size(theme.text_size(TextSize::Xs))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(colors.fg_muted)
                    .child(
                        Icon::new(IconName::Sparkles)
                            .size(IconSize::Xs)
                            .color(colors.fg_muted),
                    )
                    .child("Suggested"),
            )
            .child(
                div()
                    .text_size(theme.text_size(TextSize::Sm))
                    .text_color(colors.fg)
                    .child(
                        StyledText::new(offered.clone())
                            .with_highlights(words.into_iter().map(|range| (range, wash))),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_2()
                    .child(
                        Button::new((self.id.clone(), "take"), "Use this")
                            .variant(ButtonVariant::Primary)
                            .size(ControlSize::Sm)
                            .disabled(!shown)
                            .on_click(answer(true)),
                    )
                    .child(
                        Button::new((self.id.clone(), "keep"), "Keep mine")
                            .variant(ButtonVariant::Ghost)
                            .size(ControlSize::Sm)
                            .disabled(!shown)
                            .on_click(answer(false)),
                    ),
            );
        div()
            .flex()
            .flex_col()
            .child(Input::new(&self.field))
            .children(enhance.map(|button| div().mt_2().flex().justify_end().child(button)))
            .child(
                Transition::new((self.id.clone(), "offered"), shown)
                    .entrance(Entrance::Rise)
                    .child(card),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::added_words;

    #[test]
    fn added_words_are_the_new_runs_without_their_spaces() {
        let new = "a fluffy cat asleep on a sunlit mat";
        let ranges = added_words("a cat on a mat", new);
        let words: Vec<&str> = ranges.iter().map(|range| &new[range.clone()]).collect();
        assert_eq!(words, ["fluffy", "asleep", "sunlit"]);
        let new = "a white atrium, pale limestone";
        let ranges = added_words("a white atrium", new);
        let words: Vec<&str> = ranges.iter().map(|range| &new[range.clone()]).collect();
        assert_eq!(
            words,
            ["pale limestone"],
            "a comma added after a kept word stays plain"
        );
        let new = "白い中庭，朝の光";
        let ranges = added_words("白い中庭", new);
        let words: Vec<&str> = ranges.iter().map(|range| &new[range.clone()]).collect();
        assert_eq!(words, ["朝の光"], "any script's punctuation stays plain");
        assert!(added_words("same words", "same words").is_empty());
        let whole = 2..7;
        assert_eq!(added_words("", "  a cat "), [whole]);
    }
}
