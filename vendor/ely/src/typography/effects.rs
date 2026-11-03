use std::{
    hash::{DefaultHasher, Hash, Hasher},
    ops::Range,
    time::Duration,
};

use gpui::{
    Animation, AnimationExt, App, ElementId, HighlightStyle, Hsla, IntoElement, ParentElement,
    RenderOnce, SharedString, Styled, StyledText, Window, div, prelude::*, pulsating_between,
};
use unicode_segmentation::UnicodeSegmentation;

use crate::{
    motion,
    theme::{ActiveTheme, Mix, TextSize},
};

const SHIMMER: Duration = Duration::from_millis(1800);
const BLINK: Duration = Duration::from_millis(1100);

fn graphemes(text: &str) -> Vec<Range<usize>> {
    text.grapheme_indices(true)
        .map(|(start, grapheme)| start..start + grapheme.len())
        .collect()
}

fn tinted(text: &SharedString, color: impl Fn(usize, usize) -> Hsla) -> StyledText {
    let spans = graphemes(text);
    let count = spans.len();
    let runs: Vec<_> = spans
        .into_iter()
        .enumerate()
        .map(|(ix, span)| {
            let style = HighlightStyle {
                color: Some(color(ix, count)),
                ..Default::default()
            };
            (span, style)
        })
        .collect();
    StyledText::new(text.clone()).with_highlights(runs)
}

/// Reveals text one grapheme at a time, with a blinking caret.
#[derive(IntoElement)]
pub struct Typewriter {
    id: ElementId,
    text: SharedString,
    per_second: f32,
}

impl Typewriter {
    pub fn new(id: impl Into<ElementId>, text: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
            per_second: 32.0,
        }
    }

    /// Graphemes per second.
    pub fn speed(mut self, per_second: f32) -> Self {
        assert!(per_second > 0.0, "typewriter speed must be positive");
        self.per_second = per_second;
        self
    }
}

impl RenderOnce for Typewriter {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let text = self.text;
        let spans = graphemes(&text);
        let count = spans.len();
        let full = Duration::from_secs_f32(count as f32 / self.per_second);
        let reveal = motion::duration(full, cx);
        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        let theme = cx.theme();
        let body = theme.text_size(TextSize::Base);
        let caret = div().w(body * 0.15).h(body * 1.3).bg(theme.colors.fg);
        let caret = if theme.reduced_motion {
            caret.into_any_element()
        } else {
            caret
                .with_animation(
                    "caret",
                    Animation::new(BLINK)
                        .repeat()
                        .with_easing(pulsating_between(0.15, 1.0)),
                    |caret, t| caret.opacity(t),
                )
                .into_any_element()
        };
        div()
            .id(self.id)
            .flex()
            .items_center()
            .gap_0p5()
            .child(div().with_animation(
                ("typing", hasher.finish()),
                Animation::new(reveal),
                move |line, t| {
                    let shown = ((count as f32 * t).ceil() as usize).min(count);
                    let end = if shown == 0 { 0 } else { spans[shown - 1].end };
                    line.child(SharedString::from(text[..end].to_string()))
                },
            ))
            .child(caret)
    }
}

/// Tonal ramp across the glyphs, ink to muted by default.
#[derive(IntoElement)]
pub struct GradientText {
    text: SharedString,
    ends: Option<(Hsla, Hsla)>,
}

impl GradientText {
    pub fn new(text: impl Into<SharedString>) -> Self {
        Self {
            text: text.into(),
            ends: None,
        }
    }

    pub fn colors(mut self, from: impl Into<Hsla>, to: impl Into<Hsla>) -> Self {
        self.ends = Some((from.into(), to.into()));
        self
    }
}

impl RenderOnce for GradientText {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let colors = &cx.theme().colors;
        let (from, to) = self.ends.unwrap_or((colors.fg, colors.fg_subtle));
        tinted(&self.text, |ix, count| {
            from.mix(&to, ix as f32 / count.saturating_sub(1).max(1) as f32)
        })
    }
}

/// A band of light that sweeps the text while it waits.
#[derive(IntoElement)]
pub struct ShimmerText {
    id: ElementId,
    text: SharedString,
}

impl ShimmerText {
    pub fn new(id: impl Into<ElementId>, text: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
        }
    }
}

impl RenderOnce for ShimmerText {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let (dim, lit) = (theme.colors.fg_subtle, theme.colors.fg);
        let text = self.text;
        if theme.reduced_motion {
            return div()
                .id(self.id)
                .text_color(dim)
                .child(text)
                .into_any_element();
        }
        div()
            .id(self.id)
            .with_animation(
                "shimmer",
                Animation::new(SHIMMER).repeat(),
                move |line, t| {
                    let band = motion::lerp(-0.3, 1.3, t);
                    line.child(tinted(&text, |ix, count| {
                        let x = ix as f32 / count.saturating_sub(1).max(1) as f32;
                        dim.mix(&lit, (1.0 - (x - band).abs() / 0.25).max(0.0))
                    }))
                },
            )
            .into_any_element()
    }
}
