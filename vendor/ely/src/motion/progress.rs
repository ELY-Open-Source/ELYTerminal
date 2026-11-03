use std::f32::consts::{FRAC_PI_2, TAU};

use gpui::{
    Animation, AnimationExt, AnyElement, App, ElementId, IntoElement, ParentElement, Rems,
    RenderOnce, Styled, Window, canvas, div, prelude::*, relative,
};

use super::{BASE, SLOW, duration, ease_in_out_cubic, ease_out_cubic, spinner::arc};
use crate::{
    theme::{ActiveTheme, TextSize},
    typography::tabular,
};

/// Where a bar or ring glides from, to its latest value.
struct Glide {
    from: f32,
    to: f32,
    turn: usize,
}

/// Keeps `value`'s last change; returns where to glide from, and the change's number.
pub(crate) fn follow(
    id: &ElementId,
    value: f32,
    window: &mut Window,
    cx: &mut App,
) -> (f32, usize) {
    let state = window.use_keyed_state((id.clone(), "glide"), cx, |_, _| Glide {
        from: value,
        to: value,
        turn: 0,
    });
    if state.read(cx).to != value {
        state.update(cx, |glide, _| {
            glide.from = glide.to;
            glide.to = value;
            glide.turn += 1;
        });
    }
    let glide = state.read(cx);
    (glide.from, glide.turn)
}

fn checked(value: f32, what: &str) -> f32 {
    assert!((0.0..=1.0).contains(&value), "{what} {value} is not 0..=1");
    value
}

/// How far a task has come: a thin track that fills and glides to each new value, or sweeps when none is known. A buffer shows what has loaded ahead; segments split it into steps.
#[derive(IntoElement)]
pub struct ProgressBar {
    id: ElementId,
    value: Option<f32>,
    buffer: Option<f32>,
    segments: usize,
}

impl ProgressBar {
    pub fn new(id: impl Into<ElementId>, value: f32) -> Self {
        Self {
            id: id.into(),
            value: Some(checked(value, "progress")),
            buffer: None,
            segments: 1,
        }
    }

    /// No value known: a short fill sweeps the track.
    pub fn indeterminate(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            value: None,
            buffer: None,
            segments: 1,
        }
    }

    /// What has loaded ahead of the value, drawn fainter; none when it lies behind, as after a seek.
    pub fn buffer(mut self, buffer: f32) -> Self {
        let value = self.value.expect("an indeterminate bar has no buffer");
        self.buffer = (checked(buffer, "buffer") > value).then_some(buffer);
        self
    }

    /// Splits the track into `count` steps with gaps between.
    pub fn segments(mut self, count: usize) -> Self {
        assert!(count >= 2, "segments need at least two, got {count}");
        self.segments = count;
        self
    }
}

impl RenderOnce for ProgressBar {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let (fill, faint, track) = (
            theme.colors.fg,
            theme.colors.fg.opacity(0.25),
            theme.colors.border,
        );
        let thickness = theme.progress_thickness();
        let lane = move || {
            div()
                .relative()
                .h(thickness)
                .rounded_full()
                .overflow_hidden()
                .bg(track)
        };
        let bar = move |share: f32| {
            div()
                .absolute()
                .top_0()
                .left_0()
                .h_full()
                .rounded_full()
                .w(relative(share))
        };
        let Some(value) = self.value else {
            let sweep = bar(0.3).bg(fill);
            if theme.reduced_motion {
                return lane().child(sweep.left(relative(0.35))).into_any_element();
            }
            return lane()
                .child(
                    sweep.with_animation(
                        (self.id, "sweep"),
                        Animation::new(SLOW * 4)
                            .repeat()
                            .with_easing(ease_in_out_cubic),
                        |bar, t| bar.left(relative(-0.3 + 1.3 * t)),
                    ),
                )
                .into_any_element();
        };
        let (from, turn) = follow(&self.id, value, window, cx);
        let (buffer, segments) = (self.buffer, self.segments);
        let lanes = move |at: f32| {
            (0..segments).map(move |ix| {
                let share = |whole: f32| (whole * segments as f32 - ix as f32).clamp(0.0, 1.0);
                lane()
                    .flex_1()
                    .when_some(buffer, |lane, buffer| {
                        lane.child(bar(share(buffer)).bg(faint))
                    })
                    .child(
                        bar(share(at))
                            .bg(fill)
                            .debug_selector(move || format!("progress-fill-{ix}")),
                    )
            })
        };
        div()
            .flex()
            .gap_1()
            .with_animation(
                (self.id, format!("glide-{turn}")),
                Animation::new(duration(BASE, cx)).with_easing(ease_out_cubic),
                move |row, t| {
                    let at = if turn == 0 {
                        value
                    } else {
                        from + (value - from) * t
                    };
                    row.children(lanes(at))
                },
            )
            .into_any_element()
    }
}

/// Progress as a ring that fills clockwise from the top and glides to each new value; the percent, or any content, can sit inside.
#[derive(IntoElement)]
pub struct ProgressRing {
    id: ElementId,
    value: f32,
    percent: bool,
    size: Option<Rems>,
    inside: Option<AnyElement>,
}

impl ProgressRing {
    pub fn new(id: impl Into<ElementId>, value: f32) -> Self {
        Self {
            id: id.into(),
            value: checked(value, "progress"),
            percent: false,
            size: None,
            inside: None,
        }
    }

    /// Shows the value as a percent inside the ring.
    pub fn percent(mut self) -> Self {
        assert!(
            self.inside.is_none(),
            "progress ring {:?}: a percent or content inside, not both",
            self.id
        );
        self.percent = true;
        self
    }

    /// A ring this wide; the theme's otherwise. Its stroke keeps the theme ring's weight.
    pub fn size(mut self, size: Rems) -> Self {
        self.size = Some(size);
        self
    }

    /// What sits in the ring's middle.
    pub fn inside(mut self, content: impl IntoElement) -> Self {
        assert!(
            !self.percent,
            "progress ring {:?}: a percent or content inside, not both",
            self.id
        );
        self.inside = Some(content.into_any_element());
        self
    }
}

impl RenderOnce for ProgressRing {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let (value, percent) = (self.value, self.percent);
        let (from, turn) = follow(&self.id, value, window, cx);
        let theme = cx.theme();
        let (fill, track) = (theme.colors.fg, theme.colors.border);
        let text = theme.text_size(TextSize::Xs);
        let stroke = theme.progress_ring().to_pixels(window.rem_size()) / 12.0;
        let glide = div()
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .with_animation(
                (self.id, format!("glide-{turn}")),
                Animation::new(duration(BASE, cx)).with_easing(ease_out_cubic),
                move |ring, t| {
                    let at = if turn == 0 {
                        value
                    } else {
                        from + (value - from) * t
                    };
                    ring.child(
                        canvas(
                            |_, _, _| {},
                            move |bounds, _, window, _| {
                                let side = bounds.size.width.min(bounds.size.height);
                                let width = stroke;
                                let (center, radius) = (bounds.center(), side / 2.0 - width);
                                arc(center, radius, 0.0, TAU, width, track, window);
                                if at > 0.0 {
                                    arc(center, radius, -FRAC_PI_2, TAU * at, width, fill, window);
                                }
                            },
                        )
                        .absolute()
                        .inset_0(),
                    )
                    .when(percent, |ring| {
                        ring.child(
                            tabular(div())
                                .text_size(text)
                                .child(format!("{}%", (at * 100.0).round())),
                        )
                    })
                },
            );
        div()
            .relative()
            .flex()
            .flex_none()
            .items_center()
            .justify_center()
            .size(self.size.unwrap_or(theme.progress_ring()))
            .child(glide)
            .children(self.inside)
    }
}

#[cfg(test)]
mod tests {
    use gpui::div;

    use super::{ProgressBar, ProgressRing};

    #[test]
    #[should_panic(expected = "a percent or content inside, not both")]
    fn a_ring_holds_a_percent_or_content_not_both() {
        let _ = ProgressRing::new("ring", 0.5).inside(div()).percent();
    }

    #[test]
    fn a_buffer_ahead_shows_and_one_behind_does_not() {
        assert_eq!(ProgressBar::new("bar", 0.4).buffer(0.7).buffer, Some(0.7));
        assert_eq!(ProgressBar::new("bar", 0.6).buffer(0.4).buffer, None);
    }
}
