use std::{
    collections::HashSet,
    f32::consts::{FRAC_PI_2, TAU},
    rc::Rc,
};

use gpui::{
    App, Bounds, Div, ElementId, Entity, HoverListenerMode, InteractiveElement, IntoElement,
    MouseMoveEvent, ParentElement, PathBuilder, Pixels, Point, Refineable, RenderOnce,
    SharedString, StatefulInteractiveElement, StyleRefinement, Styled, Window, canvas, div, point,
    prelude::*,
};

use super::{
    axes::anchored,
    glide::Glide,
    paint::{finish, measure, ring, tint},
    parts::{ChartLegend, ChartTooltip},
    plot::Format,
    scale::{compact, nice},
    series::Series,
};
use crate::{
    motion,
    theme::{ActiveTheme, TextSize},
    typography::tabular,
};

/// A radar's own state: its box, the spoke under the pointer, the series the legend hid, and a glide between values.
#[derive(Default)]
struct Radar {
    bounds: Bounds<Pixels>,
    hover: Option<usize>,
    hidden: HashSet<SharedString>,
    glide: Glide,
}

/// The spoke nearest the direction of `at` from `center`, of `count` spread clockwise from twelve o'clock.
fn spoke_at(count: usize, center: (f32, f32), at: (f32, f32)) -> usize {
    let angle = ((at.1 - center.1).atan2(at.0 - center.0) + FRAC_PI_2).rem_euclid(TAU);
    ((angle / (TAU / count as f32)).round() as usize) % count
}

/// Values on spokes around a center, a shape per series: rings mark the scale, spokes the axes. Hover a spoke to read every series on it; a press on a legend name hides it.
#[derive(IntoElement)]
pub struct RadarChart {
    base: Div,
    id: ElementId,
    axes: Vec<SharedString>,
    series: Vec<Series>,
    format: Format,
}

impl RadarChart {
    pub fn new(
        id: impl Into<ElementId>,
        axes: impl IntoIterator<Item = impl Into<SharedString>>,
    ) -> Self {
        let axes: Vec<SharedString> = axes.into_iter().map(Into::into).collect();
        assert!(axes.len() >= 3, "a radar needs three axes");
        Self {
            base: div(),
            id: id.into(),
            axes,
            series: Vec::new(),
            format: Rc::new(compact),
        }
    }

    pub fn series(mut self, series: Series) -> Self {
        assert_eq!(
            series.values.len(),
            self.axes.len(),
            "series {} needs a value per axis",
            series.name
        );
        assert!(
            series.values.iter().all(|value| *value >= 0.0),
            "a radar reads values of zero or more"
        );
        self.series.push(series);
        self
    }

    /// How values read on the rings and in the tooltip.
    pub fn format(mut self, format: impl Fn(f64) -> String + 'static) -> Self {
        self.format = Rc::new(format);
        self
    }
}

impl Styled for RadarChart {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for RadarChart {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let radar: Entity<Radar> =
            window.use_keyed_state((self.id.clone(), "radar"), cx, |_, _| Radar::default());
        let values: Vec<Vec<f64>> = self
            .series
            .iter()
            .map(|series| series.values.clone())
            .collect();
        let slow = motion::duration(motion::SLOW, cx);
        let (glided, moving) = radar.update(cx, |radar, _| radar.glide.follow(&values, slow));
        if moving {
            window.request_animation_frame();
        }
        let (bounds, hover, hidden) = {
            let radar = radar.read(cx);
            let hover = radar.hover.filter(|spoke| *spoke < self.axes.len());
            (radar.bounds, hover, radar.hidden.clone())
        };
        let theme = cx.theme();
        let (colors, sizes, rem) = (theme.colors.clone(), theme.chart(), window.rem_size());
        let pixels = |length: gpui::Rems| f32::from(length.to_pixels(rem));
        let (width, height) = (f32::from(bounds.size.width), f32::from(bounds.size.height));
        let (room, gap) = (pixels(sizes.foot), pixels(sizes.inset));
        let radius = (width.min(height) / 2.0 - room).max(0.0);
        let shown: Vec<usize> = (0..self.series.len())
            .filter(|ix| !hidden.contains(&self.series[*ix].name))
            .collect();
        let most = shown
            .iter()
            .flat_map(|ix| glided[*ix].iter())
            .fold(0.0f64, |most, value| most.max(*value));
        let ((_, top), ticks) = nice(0.0, most.max(f64::EPSILON), 4);
        let count = self.axes.len();
        let angle = move |ix: usize| -FRAC_PI_2 + TAU * ix as f32 / count as f32;
        let text = |content: String, strong: bool| {
            div()
                .flex_none()
                .whitespace_nowrap()
                .text_size(theme.text_size(TextSize::Xs))
                .text_color(if strong { colors.fg } else { colors.fg_muted })
                .child(content)
        };
        let axis_labels = self.axes.iter().enumerate().map(|(ix, name)| {
            let (dx, dy) = (angle(ix).cos(), angle(ix).sin());
            let (x, y) = (
                width / 2.0 + (radius + gap) * dx,
                height / 2.0 + (radius + gap) * dy,
            );
            div()
                .absolute()
                .left(Pixels::from(x))
                .top(Pixels::from(y))
                .w_0()
                .h_0()
                .flex()
                .map(|anchor| match dx {
                    dx if dx > 0.3 => anchor.justify_start(),
                    dx if dx < -0.3 => anchor.justify_end(),
                    _ => anchor.justify_center(),
                })
                .map(|anchor| match dy {
                    dy if dy > 0.3 => anchor.items_start(),
                    dy if dy < -0.3 => anchor.items_end(),
                    _ => anchor.items_center(),
                })
                .child(text(name.to_string(), hover == Some(ix)))
        });
        let format = self.format.clone();
        let ring_labels = ticks.iter().skip(1).map(|tick| {
            let y = height / 2.0 - radius * (*tick / top) as f32;
            div()
                .absolute()
                .left(Pixels::from(width / 2.0))
                .top(Pixels::from(y))
                .h_0()
                .flex()
                .items_center()
                .pl_1p5()
                .child(tabular(text(format(*tick), false)).text_color(colors.fg_subtle))
        });
        let tooltip = hover.map(|ix| {
            let card =
                shown
                    .iter()
                    .fold(ChartTooltip::new(self.axes[ix].clone()), |card, series| {
                        card.row(
                            Some(tint(&colors, *series)),
                            self.series[*series].name.clone(),
                            format(glided[*series][ix]),
                        )
                    });
            let reach = radius + gap;
            anchored(
                (
                    width / 2.0 + reach * angle(ix).cos(),
                    height / 2.0 + reach * angle(ix).sin(),
                ),
                angle(ix).cos() < 0.0,
                card,
            )
        });
        let legend = (self.series.len() > 1).then(|| {
            let toggled = radar.clone();
            let legend = self.series.iter().enumerate().fold(
                ChartLegend::new((self.id.clone(), "legend")),
                |legend, (ix, series)| {
                    legend.entry(
                        tint(&colors, ix),
                        series.name.clone(),
                        hidden.contains(&series.name),
                    )
                },
            );
            legend.on_toggle(move |name, _, cx| {
                toggled.update(cx, |radar, cx| {
                    if !radar.hidden.remove(name) {
                        radar.hidden.insert(name.clone());
                    }
                    log::info!(
                        "radar: {name} {}",
                        if radar.hidden.contains(name) {
                            "hidden"
                        } else {
                            "shown"
                        }
                    );
                    cx.notify();
                })
            })
        });
        let shapes: Vec<(usize, Vec<f32>)> = shown
            .iter()
            .map(|ix| {
                (
                    *ix,
                    glided[*ix]
                        .iter()
                        .map(|value| (*value / top) as f32)
                        .collect(),
                )
            })
            .collect();
        let levels: Vec<f32> = ticks
            .iter()
            .skip(1)
            .map(|tick| (*tick / top) as f32)
            .collect();
        let (stroke, hairline) = (sizes.stroke.to_pixels(rem), sizes.hairline);
        let palette = colors.clone();
        let (moved, left, measured) = (radar.clone(), radar.clone(), radar.clone());
        let mut frame = div().debug_selector(|| "chart-root".into()).h(sizes.height);
        frame.style().refine(self.base.style());
        frame.flex().flex_col().gap_2().children(legend).child(
            div()
                .id((self.id.clone(), "radar"))
                .relative()
                .flex_1()
                .min_h_0()
                .on_mouse_move(move |event: &MouseMoveEvent, _, cx| {
                    let center = moved.read(cx).bounds.center();
                    let (from, at) = (
                        (f32::from(center.x), f32::from(center.y)),
                        (f32::from(event.position.x), f32::from(event.position.y)),
                    );
                    let near = (at.0 - from.0).hypot(at.1 - from.1) <= radius + room;
                    let next = near.then(|| spoke_at(count, from, at));
                    moved.update(cx, |radar, cx| {
                        if radar.hover != next {
                            radar.hover = next;
                            cx.notify();
                        }
                    })
                })
                .hover_listener_mode(HoverListenerMode::InputModalityIndependent)
                .on_hover(move |inside, _, cx| {
                    if !*inside {
                        left.update(cx, |radar, cx| {
                            radar.hover = None;
                            cx.notify();
                        })
                    }
                })
                .child(
                    canvas(
                        |_, _, _| {},
                        move |bounds, _, window, _| {
                            let center = bounds.center();
                            let at = |share: f32, ix: usize| -> Point<Pixels> {
                                center
                                    + point(
                                        Pixels::from(radius * share * angle(ix).cos()),
                                        Pixels::from(radius * share * angle(ix).sin()),
                                    )
                            };
                            let outline =
                                |shares: &dyn Fn(usize) -> f32, path: &mut PathBuilder| {
                                    path.move_to(at(shares(0), 0));
                                    (1..count).for_each(|ix| path.line_to(at(shares(ix), ix)));
                                    path.close();
                                };
                            for level in &levels {
                                let mut path = PathBuilder::stroke(hairline);
                                outline(&|_| *level, &mut path);
                                finish(path, palette.border, window);
                            }
                            for ix in 0..count {
                                let mut spoke = PathBuilder::stroke(hairline);
                                spoke.move_to(center);
                                spoke.line_to(at(1.0, ix));
                                finish(
                                    spoke,
                                    if hover == Some(ix) {
                                        palette.border_strong
                                    } else {
                                        palette.border
                                    },
                                    window,
                                );
                            }
                            for (color, shares) in &shapes {
                                let ink = tint(&palette, *color);
                                let (mut area, mut edge) =
                                    (PathBuilder::fill(), PathBuilder::stroke(stroke));
                                outline(&|ix| shares[ix], &mut area);
                                outline(&|ix| shares[ix], &mut edge);
                                finish(area, ink.opacity(0.14), window);
                                finish(edge, ink, window);
                            }
                            if let Some(spoke) = hover {
                                for (color, shares) in &shapes {
                                    ring(
                                        at(shares[spoke], spoke),
                                        stroke * 2.0,
                                        (palette.bg, tint(&palette, *color)),
                                        stroke,
                                        window,
                                    );
                                }
                            }
                        },
                    )
                    .absolute()
                    .inset_0(),
                )
                .children(ring_labels)
                .children(axis_labels)
                .children(tooltip)
                .child(measure(measured, |radar| &mut radar.bounds)),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::spoke_at;

    #[test]
    fn the_pointer_picks_the_nearest_spoke() {
        assert_eq!(spoke_at(4, (0.0, 0.0), (0.0, -10.0)), 0, "twelve o'clock");
        assert_eq!(spoke_at(4, (0.0, 0.0), (10.0, 1.0)), 1, "three o'clock");
        assert_eq!(
            spoke_at(4, (0.0, 0.0), (-1.0, -10.0)),
            0,
            "just left of twelve wraps to the first"
        );
    }
}
