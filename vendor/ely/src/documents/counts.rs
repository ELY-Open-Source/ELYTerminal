use gpui::{App, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window, div};
use unicode_segmentation::UnicodeSegmentation;

use super::markdown::{reading_minutes, words};
use crate::{
    theme::ActiveTheme,
    typography::{format::plural, tabular},
};

/// Share of a limit past which the count warns.
const NEAR: f32 = 0.9;

/// Words and characters in a text, as one quiet line; against a limit, characters warn near it and turn to danger past it.
#[derive(IntoElement)]
pub struct WordCount {
    text: SharedString,
    limit: Option<usize>,
}

impl WordCount {
    pub fn new(text: impl Into<SharedString>) -> Self {
        Self {
            text: text.into(),
            limit: None,
        }
    }

    /// Counts characters against `limit`.
    pub fn limit(mut self, limit: usize) -> Self {
        assert!(limit > 0, "a character limit of zero allows nothing");
        self.limit = Some(limit);
        self
    }
}

impl RenderOnce for WordCount {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let colors = &cx.theme().colors;
        let characters = self.text.graphemes(true).count();
        let words = plural(words(&self.text) as u64, "word", "words");
        let (characters, color) = match self.limit {
            Some(limit) => (
                format!("{characters}/{limit}"),
                if characters > limit {
                    colors.danger
                } else if characters as f32 >= limit as f32 * NEAR {
                    colors.warning
                } else {
                    colors.fg_subtle
                },
            ),
            None => (
                plural(characters as u64, "character", "characters"),
                colors.fg_subtle,
            ),
        };
        tabular(div().flex().gap_1().text_color(colors.fg_subtle))
            .child(words)
            .child("·")
            .child(div().text_color(color).child(characters))
    }
}

/// How long a text takes to read, at an adult's pace.
#[derive(IntoElement)]
pub struct ReadingTime {
    text: SharedString,
}

impl ReadingTime {
    pub fn new(text: impl Into<SharedString>) -> Self {
        Self { text: text.into() }
    }
}

impl RenderOnce for ReadingTime {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let minutes = reading_minutes(words(&self.text));
        tabular(div().text_color(cx.theme().colors.fg_subtle)).child(if minutes == 0 {
            "Nothing to read yet".to_string()
        } else {
            format!("{minutes} min read")
        })
    }
}
