use std::{collections::VecDeque, time::Duration};

use gpui::{
    App, Bounds, ElementId, FontWeight, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    Styled, Window, canvas, div, fill, point, size,
};
use web_time::Instant;

use crate::{
    theme::{ActiveTheme, Radius, TextSize},
    typography::tabular,
};

/// A frame's budget at 60 Hz.
const BUDGET: Duration = Duration::from_micros(16_667);
/// How far back frames count.
const SPAN: Duration = Duration::from_secs(1);
/// Frames the graph shows.
const BARS: usize = 60;

/// Frames missed before this one came, allowing half a frame of jitter.
fn missed(interval: Duration) -> u32 {
    (interval.as_secs_f64() / BUDGET.as_secs_f64() - 0.5).max(0.0) as u32
}

/// Frame stamps over the last second and the one before it, oldest first, so a frame that ran into the second counts whole.
#[derive(Default)]
struct Frames(VecDeque<Instant>);

impl Frames {
    fn record(&mut self, now: Instant) {
        self.0.push_back(now);
        while self
            .0
            .get(1)
            .is_some_and(|second| now.duration_since(*second) >= SPAN)
        {
            self.0.pop_front();
        }
    }

    fn intervals(&self) -> impl Iterator<Item = Duration> + '_ {
        self.0
            .iter()
            .zip(self.0.iter().skip(1))
            .map(|(before, after)| after.duration_since(*before))
    }

    /// Frames per second over the stamps kept, once two frames span some time.
    fn fps(&self) -> Option<f64> {
        let (first, last) = (self.0.front()?, self.0.back()?);
        let span = last.duration_since(*first).as_secs_f64();
        (span > 0.0).then(|| (self.0.len() - 1) as f64 / span)
    }

    fn worst(&self) -> Option<Duration> {
        self.intervals().max()
    }
}

/// Frames per second and the worst frame over the last second, a frame that ran into it counted whole, and a bar per recent frame against the 60 Hz budget: success on time, warning a frame late, danger later. While shown it keeps its view drawing, so it counts the display's frames and holds the window awake; show it only while measuring. Its numbers are readings, so reduced motion leaves them live.
#[derive(IntoElement)]
pub struct FpsMeter {
    id: ElementId,
}

impl FpsMeter {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self { id: id.into() }
    }
}

impl RenderOnce for FpsMeter {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let frames =
            window.use_keyed_state((self.id.clone(), "frames"), cx, |_, _| Frames::default());
        let now = cx.background_executor().now();
        frames.update(cx, |frames, _| frames.record(now));
        window.request_animation_frame();
        let frames = frames.read(cx);
        let fps = frames
            .fps()
            .map_or("—".to_string(), |fps| format!("{fps:.0}"));
        let worst = frames.worst().map_or("—".to_string(), |worst| {
            format!("{:.1}", worst.as_secs_f64() * 1000.0)
        });
        let bars: Vec<Duration> = frames.intervals().collect();
        let bars = bars[bars.len().saturating_sub(BARS)..].to_vec();
        let theme = cx.theme();
        let colors = &theme.colors;
        let graph = theme.tooling().graph;
        div()
            .flex()
            .flex_col()
            .gap_1()
            .p_2()
            .rounded(theme.radius(Radius::Md))
            .border_1()
            .border_color(colors.border)
            .bg(colors.overlay)
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_baseline()
                    .justify_between()
                    .gap_x_3()
                    .child(
                        div()
                            .flex()
                            .items_baseline()
                            .gap_1()
                            .child(
                                tabular(div())
                                    .debug_selector({
                                        let fps = fps.clone();
                                        move || format!("fps-{fps}")
                                    })
                                    .text_size(theme.text_size(TextSize::Lg))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(colors.fg)
                                    .child(fps),
                            )
                            .child(
                                div()
                                    .text_size(theme.text_size(TextSize::Xs))
                                    .text_color(colors.fg_muted)
                                    .child("fps"),
                            ),
                    )
                    .child(
                        tabular(div())
                            .debug_selector({
                                let worst = worst.clone();
                                move || format!("fps-worst-{worst}")
                            })
                            .text_size(theme.text_size(TextSize::Xs))
                            .text_color(colors.fg_muted)
                            .child(format!("worst {worst} ms")),
                    ),
            )
            .child(
                canvas(
                    |_, _, _| {},
                    move |bounds, _, window, cx| {
                        let theme = cx.theme();
                        let colors = &theme.colors;
                        let slot = bounds.size.width / BARS as f32;
                        for (ix, bar) in bars.iter().enumerate() {
                            let share = (bar.as_secs_f32() / (BUDGET.as_secs_f32() * 3.0)).min(1.0);
                            let height = bounds.size.height * share;
                            let left = bounds.right() - slot * (bars.len() - ix) as f32;
                            let color = match missed(*bar) {
                                0 => colors.success,
                                1 => colors.warning,
                                _ => colors.danger,
                            };
                            window.paint_quad(fill(
                                Bounds::new(
                                    point(left, bounds.bottom() - height),
                                    size(slot * 0.7, height),
                                ),
                                color,
                            ));
                        }
                        let budget = bounds.bottom() - bounds.size.height / 3.0;
                        window.paint_quad(fill(
                            Bounds::new(
                                point(bounds.left(), budget),
                                size(bounds.size.width, theme.chart().hairline),
                            ),
                            colors.border_strong,
                        ));
                    },
                )
                .w(graph.0)
                .h(graph.1),
            )
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use web_time::Instant;

    use super::{Frames, missed};

    #[test]
    fn frames_count_the_last_second() {
        let (start, mut frames) = (Instant::now(), Frames::default());
        assert_eq!(frames.fps(), None);
        for n in 0..=100 {
            frames.record(start + Duration::from_millis(20) * n);
        }
        assert_eq!(
            frames.fps(),
            Some(50.0),
            "a frame each 20 ms over the last second"
        );
        assert_eq!(frames.0.len(), 51, "older stamps drop");
        frames.record(start + Duration::from_millis(2100));
        assert_eq!(frames.worst(), Some(Duration::from_millis(100)));
    }

    #[test]
    fn a_stall_longer_than_the_span_counts_whole() {
        let (start, mut frames) = (Instant::now(), Frames::default());
        for ms in [0, 500, 2000] {
            frames.record(start + Duration::from_millis(ms));
        }
        assert_eq!(frames.worst(), Some(Duration::from_millis(1500)));
        assert_eq!(frames.fps().map(|fps| (fps * 100.0).round()), Some(67.0));
    }

    #[test]
    fn a_frame_is_late_by_whole_frames() {
        let tenths = |tenths: u64| Duration::from_micros(tenths * 100);
        assert_eq!(missed(tenths(169)), 0, "jitter is on time");
        assert_eq!(missed(tenths(333)), 1);
        assert_eq!(missed(tenths(500)), 2);
    }
}
