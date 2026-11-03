use gpui::{
    Animation, AnimationExt, App, ElementId, Hsla, InteractiveElement, IntoElement, ParentElement,
    RenderOnce, SharedString, Styled, Window, div,
};

use super::{
    format::{self, Separators},
    text::tabular,
};
use crate::{
    motion,
    theme::{ActiveTheme, TextSize},
};

const LEADING: f32 = 1.2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Tween {
    Roll,
    Count,
}

struct Change {
    from: f64,
    to: f64,
    generation: u64,
}

/// Number that rolls its digits, or counts, to each new value.
#[derive(IntoElement)]
pub struct AnimatedNumber {
    id: ElementId,
    value: f64,
    decimals: usize,
    pad: usize,
    color: Option<Hsla>,
    tween: Tween,
    size: TextSize,
}

impl AnimatedNumber {
    pub fn new(id: impl Into<ElementId>, value: f64) -> Self {
        Self {
            id: id.into(),
            value,
            decimals: 0,
            pad: 0,
            color: None,
            tween: Tween::Roll,
            size: TextSize::Xxl,
        }
    }

    pub fn decimals(mut self, decimals: usize) -> Self {
        self.decimals = decimals;
        self
    }

    /// Zero-pads to `width` characters, as a clock shows minutes.
    pub fn pad(mut self, width: usize) -> Self {
        self.pad = width;
        self
    }

    /// Defaults to the text color.
    pub fn color(mut self, color: Hsla) -> Self {
        self.color = Some(color);
        self
    }

    /// Counts through the values instead of rolling digits.
    pub fn count_up(mut self) -> Self {
        self.tween = Tween::Count;
        self
    }

    pub fn size(mut self, size: TextSize) -> Self {
        self.size = size;
        self
    }
}

/// Zero-pads `text` to `width` characters, after its minus sign.
fn padded(text: &str, width: usize) -> String {
    match text.strip_prefix(format::MINUS) {
        Some(digits) => format!("{}{digits:0>1$}", format::MINUS, width.saturating_sub(1)),
        None => format!("{text:0>width$}"),
    }
}

/// Pads the shorter string on the left so digits align by place.
fn align(from: &str, to: &str) -> (Vec<char>, Vec<char>) {
    let width = from.chars().count().max(to.chars().count());
    let pad = |text: &str| {
        let mut chars = vec![' '; width - text.chars().count()];
        chars.extend(text.chars());
        chars
    };
    (pad(from), pad(to))
}

impl RenderOnce for AnimatedNumber {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let value = self.value;
        let state = window.use_keyed_state(self.id.clone(), cx, |_, _| Change {
            from: value,
            to: value,
            generation: 0,
        });
        if state.read(cx).to.to_bits() != value.to_bits() {
            state.update(cx, |change, _| {
                change.from = change.to;
                change.to = value;
                change.generation += 1;
            });
        }
        let (from, to, generation) = {
            let change = state.read(cx);
            (change.from, change.to, change.generation)
        };
        let (decimals, pad) = (self.decimals, self.pad);
        let text =
            move |number: f64| padded(&format::number(number, decimals, Separators::EN), pad);
        let duration = motion::duration(motion::SLOW * 2, cx);
        let theme = cx.theme();
        let size = theme.text_size(self.size);
        let scale = window.scale_factor();
        // gpui snaps rows and margin apart; keep whole device pixels.
        let line = (size.to_pixels(window.rem_size()) * LEADING * scale).round() / scale;
        let row = tabular(div())
            .debug_selector(|| format!("animated-number {}", self.id))
            .id(self.id)
            .flex()
            .h(line)
            .overflow_hidden()
            .text_size(size)
            .line_height(line)
            .text_color(self.color.unwrap_or(theme.colors.fg));

        if self.tween == Tween::Count {
            return row.child(div().with_animation(
                ("count", generation),
                Animation::new(duration),
                move |label, t| {
                    let eased = f64::from(motion::ease_out_cubic(t));
                    label.child(text(from + (to - from) * eased))
                },
            ));
        }
        let (before, after) = align(&text(from), &text(to));
        row.children(
            before
                .into_iter()
                .zip(after)
                .enumerate()
                .map(move |(ix, (old, new))| {
                    let Some(end) = new.to_digit(10) else {
                        return div()
                            .child(SharedString::from(new.to_string()))
                            .into_any_element();
                    };
                    let start = old.to_digit(10).unwrap_or(0);
                    // A block column lends the cell no baseline.
                    let stack = div().children((0..10).map(|digit| {
                        div()
                            .debug_selector(move || format!("rolling-{digit}"))
                            .h(line)
                            .child(SharedString::from(digit.to_string()))
                    }));
                    div()
                        .debug_selector(|| "rolling-cell".into())
                        .h(line)
                        .overflow_hidden()
                        .child(stack.with_animation(
                            ("roll", generation * 1_000 + ix as u64),
                            Animation::new(duration),
                            move |stack, t| {
                                let place = motion::lerp(
                                    start as f32,
                                    end as f32,
                                    motion::ease_out_cubic(t),
                                );
                                stack.mt(-(line * place))
                            },
                        ))
                        .into_any_element()
                }),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{align, padded};

    #[test]
    fn padding_keeps_the_minus_first() {
        assert_eq!(padded("7", 2), "07");
        assert_eq!(padded("\u{2212}12", 5), "\u{2212}0012");
        assert_eq!(padded("123", 2), "123");
    }

    #[test]
    fn align_pads_by_place() {
        let (from, to) = align("999", "1,000");
        assert_eq!(from.iter().collect::<String>(), "  999");
        assert_eq!(to.iter().collect::<String>(), "1,000");
    }
}
