use gpui::{
    App, Bounds, ContentMask, Div, ElementId, Entity, InteractiveElement, IntoElement,
    ParentElement, PathBuilder, Pixels, Refineable, RenderOnce, StyleRefinement, Styled, Window,
    canvas, div, fill, size,
};
use web_time::Instant;

use super::{
    axes::beside,
    geometry::{Rect, frame},
    paint::{at, finish, measure, place, ring, tint, trace},
    scale::{Linear, compact, nice},
    series::drawable,
};
use crate::{
    motion,
    theme::{ActiveTheme, TextSize},
    typography::tabular,
};

/// A live chart's own state: its box, how many values it has seen, and when the latest arrived.
#[derive(Default)]
struct Live {
    bounds: Bounds<Pixels>,
    seen: Option<u64>,
    since: Option<Instant>,
}

/// The latest values of a stream as a line that slides left as each new one arrives, the newest marked with a dot. The owner keeps the values and counts what it has pushed.
#[derive(IntoElement)]
pub struct RealtimeChart {
    base: Div,
    id: ElementId,
    values: Vec<f64>,
    pushed: u64,
}

impl RealtimeChart {
    /// `values` are the latest, oldest first; `pushed` counts every value so far, so the chart knows when a new one came.
    pub fn new(
        id: impl Into<ElementId>,
        values: impl IntoIterator<Item = f64>,
        pushed: u64,
    ) -> Self {
        let values: Vec<f64> = values.into_iter().collect();
        assert!(
            values.iter().all(|value| drawable(*value)),
            "a stream needs values, within charts::LIMIT"
        );
        Self {
            base: div(),
            id: id.into(),
            values,
            pushed,
        }
    }
}

impl Styled for RealtimeChart {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for RealtimeChart {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let live: Entity<Live> =
            window.use_keyed_state((self.id.clone(), "live"), cx, |_, _| Live::default());
        let seen = live.read(cx).seen;
        if seen != Some(self.pushed) {
            live.update(cx, |live, _| {
                live.seen = Some(self.pushed);
                live.since = seen.map(|_| Instant::now());
            });
        }
        let length = motion::duration(motion::BASE, cx).as_secs_f32();
        let share = live.read(cx).since.map_or(1.0, |since| {
            (since.elapsed().as_secs_f32() / length).min(1.0)
        });
        if share < 1.0 {
            window.request_animation_frame();
        }
        let slide = 1.0 - motion::ease_out_cubic(share);
        let theme = cx.theme();
        let (colors, sizes, rem) = (theme.colors.clone(), theme.chart(), window.rem_size());
        let pixels = |length: gpui::Rems| f32::from(length.to_pixels(rem));
        let bounds = live.read(cx).bounds;
        let (width, height) = (f32::from(bounds.size.width), f32::from(bounds.size.height));
        let rect = frame(
            (width, height),
            pixels(sizes.gutter),
            pixels(sizes.inset),
            pixels(sizes.inset),
        );
        let (low, high) = self
            .values
            .iter()
            .fold((f64::MAX, f64::MIN), |(low, high), value| {
                (low.min(*value), high.max(*value))
            });
        let ((low, high), ticks) = if self.values.is_empty() {
            ((0.0, 1.0), vec![0.0, 1.0])
        } else {
            nice(low, high, 4)
        };
        let scale = Linear::new((low, high), (rect.y + rect.h, rect.y));
        let step = rect.w / (self.values.len().max(2) - 1) as f32;
        let points: Vec<(f32, f32)> = self
            .values
            .iter()
            .enumerate()
            .map(|(ix, value)| (rect.x + step * (ix as f32 + slide), scale.at(*value)))
            .collect();
        let labels = ticks.iter().map(|tick| {
            let label = tabular(div())
                .whitespace_nowrap()
                .text_size(theme.text_size(TextSize::Xs))
                .text_color(colors.fg_subtle);
            beside(
                scale.at(*tick),
                sizes.gutter.to_pixels(rem),
                label.child(compact(*tick)),
            )
        });
        let (ink, levels) = (
            tint(&colors, 0),
            ticks.iter().map(|tick| scale.at(*tick)).collect::<Vec<_>>(),
        );
        let (grid, back) = (colors.border.opacity(0.6), colors.bg);
        let (stroke, hairline, inset) = (
            sizes.stroke.to_pixels(rem),
            sizes.hairline,
            pixels(sizes.inset),
        );
        let measured = live.clone();
        let mut chart = div()
            .debug_selector(|| "chart-root".into())
            .relative()
            .h(sizes.height * 0.6);
        chart.style().refine(self.base.style());
        chart
            .child(
                canvas(
                    |_, _, _| {},
                    move |bounds, _, window, _| {
                        let origin = bounds.origin;
                        for level in &levels {
                            let line = Bounds::new(
                                at(origin, (rect.x, *level)),
                                size(Pixels::from(rect.w), hairline),
                            );
                            window.paint_quad(fill(line, grid));
                        }
                        let (Some(first), Some(last)) = (points.first(), points.last()) else {
                            return;
                        };
                        let floor = rect.y + rect.h;
                        let room = Rect {
                            x: rect.x,
                            y: 0.0,
                            w: rect.w + inset,
                            h: floor + inset,
                        };
                        window.with_content_mask(
                            Some(ContentMask {
                                bounds: place(origin, room),
                            }),
                            |window| {
                                let mut area = PathBuilder::fill();
                                trace(&mut area, &points, false, origin);
                                area.line_to(at(origin, (last.0, floor)));
                                area.line_to(at(origin, (first.0, floor)));
                                area.close();
                                finish(area, ink.opacity(0.12), window);
                                let mut line = PathBuilder::stroke(stroke);
                                trace(&mut line, &points, false, origin);
                                finish(line, ink, window);
                                ring(at(origin, *last), stroke * 2.0, (ink, back), stroke, window);
                            },
                        );
                    },
                )
                .absolute()
                .inset_0(),
            )
            .children(labels)
            .child(measure(measured, |live| &mut live.bounds))
    }
}
