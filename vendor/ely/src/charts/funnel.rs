use std::rc::Rc;

use gpui::{
    App, Div, ElementId, Entity, HoverListenerMode, InteractiveElement, IntoElement,
    MouseMoveEvent, ParentElement, PathBuilder, Pixels, Refineable, RenderOnce, SharedString,
    StatefulInteractiveElement, StyleRefinement, Styled, Window, canvas, div, fill,
};

use super::{
    axes::anchored,
    geometry::Rect,
    paint::{at, finish, measure, place, tint},
    parts::ChartTooltip,
    plot::Format,
    scale::compact,
    series::drawable,
    tiles::Tiles,
};
use crate::{
    theme::{ActiveTheme, Radius, TextSize},
    typography::{format, tabular},
};

/// Stages a group passes through, each bar as wide as those who reached it, centered, with how many carried on from the stage before. Hover reads a stage.
#[derive(IntoElement)]
pub struct FunnelChart {
    base: Div,
    id: ElementId,
    stages: Vec<(SharedString, f64)>,
    format: Format,
}

impl FunnelChart {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            base: div(),
            id: id.into(),
            stages: Vec::new(),
            format: Rc::new(compact),
        }
    }

    pub fn stage(mut self, name: impl Into<SharedString>, value: f64) -> Self {
        assert!(
            drawable(value) && value >= 0.0,
            "a stage needs a value of zero or more, within charts::LIMIT"
        );
        self.stages.push((name.into(), value));
        self
    }

    pub fn format(mut self, format: impl Fn(f64) -> String + 'static) -> Self {
        self.format = Rc::new(format);
        self
    }
}

impl Styled for FunnelChart {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

/// Each stage's bar inside `frame`: centered, as wide as its share of the widest, a row each with a gap under.
pub(crate) fn funnel(values: &[f64], frame: Rect) -> Vec<Rect> {
    let widest = values.iter().copied().fold(0.0, f64::max);
    let row = frame.h / values.len().max(1) as f32;
    values
        .iter()
        .enumerate()
        .map(|(ix, value)| {
            let w = if widest > 0.0 {
                frame.w * (*value / widest) as f32
            } else {
                0.0
            };
            Rect {
                x: frame.x + (frame.w - w) / 2.0,
                y: frame.y + row * ix as f32,
                w,
                h: row * 0.62,
            }
        })
        .collect()
}

impl RenderOnce for FunnelChart {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let flows: Entity<Tiles> =
            window.use_keyed_state((self.id.clone(), "flows"), cx, |_, _| Tiles::default());
        let (bounds, hover) = (
            flows.read(cx).bounds,
            flows.read(cx).pointed(self.stages.len()),
        );
        let theme = cx.theme();
        let (colors, sizes, rem) = (theme.colors.clone(), theme.chart(), window.rem_size());
        let pixels = |length: gpui::Rems| f32::from(length.to_pixels(rem));
        let (names, room) = (pixels(theme.label_width()), pixels(sizes.label) * 1.5);
        let frame = Rect {
            x: names,
            y: 0.0,
            w: (f32::from(bounds.size.width) - names - room).max(0.0),
            h: f32::from(bounds.size.height),
        };
        let values: Vec<f64> = self.stages.iter().map(|(_, value)| *value).collect();
        let bars = funnel(&values, frame);
        let first = values.first().copied().unwrap_or(0.0);
        let share = |part: f64, whole: f64| {
            if whole > 0.0 {
                format::percent(part / whole, 1, false)
            } else {
                "—".to_string()
            }
        };
        let format = self.format.clone();
        let text = |content: String, color| {
            div()
                .flex_none()
                .whitespace_nowrap()
                .text_size(theme.text_size(TextSize::Xs))
                .text_color(color)
                .child(content)
        };
        let names: Vec<Div> = bars
            .iter()
            .zip(&self.stages)
            .map(|(bar, (name, _))| {
                div()
                    .absolute()
                    .left_0()
                    .w(Pixels::from(names))
                    .top(Pixels::from(bar.y + bar.h / 2.0))
                    .h_0()
                    .flex()
                    .items_center()
                    .justify_end()
                    .pr_3()
                    .child(text(name.to_string(), colors.fg))
            })
            .collect();
        let readings: Vec<Div> = bars
            .iter()
            .zip(&self.stages)
            .map(|(bar, (_, value))| {
                let reading = div()
                    .flex()
                    .gap_1p5()
                    .child(tabular(text(format(*value), colors.fg)))
                    .child(tabular(text(share(*value, first), colors.fg_subtle)));
                div()
                    .absolute()
                    .left(Pixels::from(frame.x + frame.w))
                    .top(Pixels::from(bar.y + bar.h / 2.0))
                    .h_0()
                    .flex()
                    .items_center()
                    .pl_3()
                    .child(reading)
            })
            .collect();
        let tooltip = match hover {
            Some(ix) => {
                let bar = bars[ix];
                let before = if ix > 0 { values[ix - 1] } else { values[ix] };
                let card = ChartTooltip::new(self.stages[ix].0.clone())
                    .row(Some(tint(&colors, 0)), "Reached", format(values[ix]))
                    .row(None, "Of the first", share(values[ix], first))
                    .row(None, "From the stage before", share(values[ix], before));
                Some(anchored(
                    (frame.x + frame.w / 2.0 + bar.w / 2.0, bar.y + bar.h / 2.0),
                    false,
                    card,
                ))
            }
            _ => None,
        };
        let (painted, corner, ink) = (
            bars.clone(),
            theme.radius(Radius::Sm).to_pixels(rem),
            tint(&colors, 0),
        );
        let (moved, left, rows) = (
            flows.clone(),
            flows.clone(),
            frame.h / self.stages.len().max(1) as f32,
        );
        let count = self.stages.len();
        let mut root = div()
            .debug_selector(|| "chart-root".into())
            .relative()
            .h(sizes.height);
        root.style().refine(self.base.style());
        root.id((self.id.clone(), "funnel"))
            .on_mouse_move(move |event: &MouseMoveEvent, _, cx| {
                let offset = event.position - moved.read(cx).bounds.origin;
                let row = (f32::from(offset.y) / rows).floor();
                let next = (row >= 0.0 && (row as usize) < count).then_some(row as usize);
                moved.update(cx, |flows, cx| {
                    if flows.hover != next {
                        flows.hover = next;
                        cx.notify();
                    }
                })
            })
            .hover_listener_mode(HoverListenerMode::InputModalityIndependent)
            .on_hover(move |inside, _, cx| {
                if !*inside {
                    left.update(cx, |flows, cx| {
                        flows.hover = None;
                        cx.notify();
                    })
                }
            })
            .child(
                canvas(
                    |_, _, _| {},
                    move |bounds, _, window, _| {
                        let origin = bounds.origin;
                        for pair in painted.windows(2) {
                            let (upper, lower) = (pair[0], pair[1]);
                            let mut neck = PathBuilder::fill();
                            neck.move_to(at(origin, (upper.x, upper.y + upper.h)));
                            neck.line_to(at(origin, (upper.x + upper.w, upper.y + upper.h)));
                            neck.line_to(at(origin, (lower.x + lower.w, lower.y)));
                            neck.line_to(at(origin, (lower.x, lower.y)));
                            neck.close();
                            finish(neck, ink.opacity(0.1), window);
                        }
                        for (ix, bar) in painted.iter().enumerate() {
                            let strength = match hover {
                                Some(on) if on != ix => 0.45,
                                _ => 0.9,
                            };
                            window.paint_quad(
                                fill(place(origin, *bar), ink.opacity(strength))
                                    .corner_radii(corner),
                            );
                        }
                    },
                )
                .absolute()
                .inset_0(),
            )
            .children(names)
            .children(readings)
            .children(tooltip)
            .child(measure(flows, |flows| &mut flows.bounds))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn funnel_bars_center_and_shrink_by_share() {
        let bars = funnel(
            &[100.0, 50.0],
            Rect {
                x: 0.0,
                y: 0.0,
                w: 200.0,
                h: 100.0,
            },
        );
        assert_eq!((bars[0].x, bars[0].w), (0.0, 200.0));
        assert_eq!((bars[1].x, bars[1].w, bars[1].y), (50.0, 100.0, 50.0));
    }
}
