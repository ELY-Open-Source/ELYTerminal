use std::{
    f32::consts::{FRAC_PI_2, PI, TAU},
    time::Duration,
};

use gpui::{
    App, Bounds, Div, ElementId, Entity, HoverListenerMode, Hsla, InteractiveElement, IntoElement,
    MouseMoveEvent, ParentElement, PathBuilder, Pixels, Point, Refineable, RenderOnce,
    SharedString, StatefulInteractiveElement, StyleRefinement, Styled, Window, canvas, div, point,
};
use web_time::Instant;

use super::{
    axes::anchored,
    glide::Glide,
    paint::{finish, measure, tint},
    parts::{ChartLegend, ChartTooltip},
    scale::compact,
    series::drawable,
};
use crate::{
    motion,
    theme::{ActiveTheme, TextSize},
    typography::{format, tabular},
};

/// Arcs for shares of a whole, clockwise from twelve o'clock: each a start and a sweep in radians.
pub(crate) fn sweeps(values: &[f64]) -> Vec<(f32, f32)> {
    let total: f64 = values.iter().filter(|value| **value > 0.0).sum();
    let mut start = -FRAC_PI_2;
    values
        .iter()
        .map(|value| {
            let sweep = if total > 0.0 {
                (value.max(0.0) / total) as f32 * TAU
            } else {
                0.0
            };
            let arc = (start, sweep);
            start += sweep;
            arc
        })
        .collect()
}

/// The angle toward `at` from `center`, from twelve o'clock on, and how far it lies.
pub(crate) fn bearing(center: (f32, f32), at: (f32, f32)) -> (f32, f32) {
    let (dx, dy) = (at.0 - center.0, at.1 - center.1);
    (
        (dy.atan2(dx) + FRAC_PI_2).rem_euclid(TAU) - FRAC_PI_2,
        dx.hypot(dy),
    )
}

/// Which arc holds the direction of `at` from `center`, between radii `inner` and `outer`.
pub(crate) fn arc_at(
    arcs: &[(f32, f32)],
    center: (f32, f32),
    at: (f32, f32),
    inner: f32,
    outer: f32,
) -> Option<usize> {
    let (angle, distance) = bearing(center, at);
    if distance < inner || distance > outer {
        return None;
    }
    arcs.iter()
        .position(|(start, sweep)| angle >= *start && angle < start + sweep)
}

/// A point `radius` from `center` toward `angle`.
pub(crate) fn toward(center: Point<Pixels>, radius: f32, angle: f32) -> Point<Pixels> {
    center
        + point(
            Pixels::from(radius * angle.cos()),
            Pixels::from(radius * angle.sin()),
        )
}

/// A ring segment's outline with `gap` taken evenly off both sides: out along the outer arc and back along the inner one, or to where the sides meet; none when too thin to draw.
pub(crate) fn wedge(
    center: Point<Pixels>,
    (inner, outer): (f32, f32),
    (start, sweep): (f32, f32),
    gap: f32,
) -> Option<PathBuilder> {
    let half = gap / 2.0;
    let meet = if sweep < PI {
        half / (sweep / 2.0).sin().max(f32::EPSILON)
    } else {
        half
    };
    let inner = inner.max(meet);
    if sweep <= 0.0 || inner >= outer {
        return None;
    }
    let arc = |radius: f32| {
        let inset = if radius > 0.0 {
            (half / radius).min(1.0).asin().min(sweep / 2.0)
        } else {
            sweep / 2.0
        };
        (start + inset, sweep - inset * 2.0)
    };
    let steps = ((sweep / TAU) * 120.0).ceil().max(2.0) as usize;
    let along = |step: usize| step as f32 / steps as f32;
    let mut path = PathBuilder::fill();
    let (from, span) = arc(outer);
    path.move_to(toward(center, outer, from));
    (1..=steps).for_each(|step| path.line_to(toward(center, outer, from + span * along(step))));
    let (from, span) = arc(inner);
    (0..=steps)
        .rev()
        .for_each(|step| path.line_to(toward(center, inner, from + span * along(step))));
    path.close();
    Some(path)
}

/// The slice under the pointer, the one it left, and when it moved: the first lifts out as the second settles.
#[derive(Default)]
struct Lift {
    on: Option<usize>,
    off: Option<usize>,
    since: Option<Instant>,
}

impl Lift {
    /// Moves to `next`; whether anything changed.
    fn point(&mut self, next: Option<usize>) -> bool {
        if self.on == next {
            return false;
        }
        (self.off, self.on, self.since) = (self.on, next, Some(Instant::now()));
        true
    }

    /// How far each of `count` slices has lifted, zero to one, and whether any still moves.
    fn shares(&self, count: usize, length: Duration) -> (Vec<f32>, bool) {
        let share = self.since.map_or(1.0, |since| {
            (since.elapsed().as_secs_f32() / length.as_secs_f32()).min(1.0)
        });
        let eased = motion::ease_out_cubic(share);
        let lift = |ix: usize| match (self.on == Some(ix), self.off == Some(ix)) {
            (true, _) => eased,
            (_, true) => 1.0 - eased,
            _ => 0.0,
        };
        ((0..count).map(lift).collect(), share < 1.0)
    }
}

/// How much each slice fades while another lifts: by the most any other has lifted, less as it lifts itself.
fn fades(lifts: &[f32]) -> Vec<f32> {
    (0..lifts.len())
        .map(|ix| {
            let others = lifts
                .iter()
                .enumerate()
                .filter(|(other, _)| *other != ix)
                .fold(0.0f32, |most, (_, lift)| most.max(*lift));
            others * (1.0 - lifts[ix])
        })
        .collect()
}

/// A ring chart's own state: its box, the lift under the pointer, and a glide between values.
#[derive(Default)]
struct Ring {
    bounds: Bounds<Pixels>,
    lift: Lift,
    glide: Glide,
}

/// Parts of a whole as slices, clockwise from twelve in the order given; `donut` hollows the middle for a total. Hover lifts a slice and reads its share.
#[derive(IntoElement)]
pub struct PieChart {
    base: Div,
    id: ElementId,
    slices: Vec<(SharedString, f64)>,
    center: Option<(SharedString, SharedString)>,
}

impl PieChart {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            base: div(),
            id: id.into(),
            slices: Vec::new(),
            center: None,
        }
    }

    pub fn slice(mut self, name: impl Into<SharedString>, value: f64) -> Self {
        assert!(
            drawable(value) && value >= 0.0,
            "a slice needs a value of zero or more, within charts::LIMIT"
        );
        self.slices.push((name.into(), value));
        self
    }

    /// Hollows the middle, which reads `value` over `label` until a slice is hovered.
    pub fn donut(mut self, value: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        self.center = Some((value.into(), label.into()));
        self
    }
}

impl Styled for PieChart {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for PieChart {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let ring: Entity<Ring> =
            window.use_keyed_state((self.id.clone(), "ring"), cx, |_, _| Ring::default());
        let raw: Vec<f64> = self.slices.iter().map(|(_, value)| *value).collect();
        let (slow, fast) = (
            motion::duration(motion::SLOW, cx),
            motion::duration(motion::FAST, cx),
        );
        let (glided, gliding) = ring.update(cx, |ring, _| {
            ring.glide.follow(std::slice::from_ref(&raw), slow)
        });
        let arcs = sweeps(&glided.into_iter().next().expect("one ring of values"));
        let (lifts, lifting) = ring.read(cx).lift.shares(arcs.len(), fast);
        if gliding || lifting {
            window.request_animation_frame();
        }
        let hover = ring.read(cx).lift.on.filter(|ix| *ix < arcs.len());
        let theme = cx.theme();
        let (colors, sizes, rem) = (theme.colors.clone(), theme.chart(), window.rem_size());
        let pixels = |length: gpui::Rems| f32::from(length.to_pixels(rem));
        let bounds = ring.read(cx).bounds;
        let (width, height) = (f32::from(bounds.size.width), f32::from(bounds.size.height));
        let lift = pixels(sizes.lift);
        let outer = (width.min(height) / 2.0 - lift).max(0.0);
        let inner = if self.center.is_some() {
            outer * 0.64
        } else {
            0.0
        };
        let gap = if raw.iter().filter(|value| **value > 0.0).count() > 1 {
            pixels(sizes.stroke)
        } else {
            0.0
        };
        let total: f64 = raw.iter().sum();
        let share = |ix: usize| if total > 0.0 { raw[ix] / total } else { 0.0 };
        let fills: Vec<Hsla> = fades(&lifts)
            .iter()
            .enumerate()
            .map(|(ix, fade)| tint(&colors, ix).opacity(1.0 - 0.45 * fade))
            .collect();
        let tooltip = hover.filter(|_| self.center.is_none()).map(|ix| {
            let (start, sweep) = arcs[ix];
            let middle = start + sweep / 2.0;
            let reach = outer + lift;
            let place = (
                width / 2.0 + reach * middle.cos(),
                height / 2.0 + reach * middle.sin(),
            );
            let card = ChartTooltip::new(self.slices[ix].0.clone())
                .row(
                    Some(tint(&colors, ix)),
                    "Share",
                    format::percent(share(ix), 1, false),
                )
                .row(None, "Value", compact(raw[ix]));
            anchored(place, middle.cos() < 0.0, card)
        });
        let middle = self.center.clone().map(|(value, label)| {
            let (big, small) = match hover {
                Some(ix) => (
                    format::percent(share(ix), 1, false).into(),
                    self.slices[ix].0.clone(),
                ),
                None => (value, label),
            };
            div()
                .absolute()
                .inset_0()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .child(
                    tabular(div())
                        .text_size(theme.text_size(TextSize::Xl))
                        .text_color(colors.fg)
                        .child(big),
                )
                .child(
                    div()
                        .text_size(theme.text_size(TextSize::Xs))
                        .text_color(colors.fg_muted)
                        .child(small),
                )
        });
        let legend = self.slices.iter().enumerate().fold(
            ChartLegend::new((self.id.clone(), "legend")),
            |legend, (ix, (name, _))| legend.entry(tint(&colors, ix), name.clone(), false),
        );
        let (moved, left, measured, hit) = (ring.clone(), ring.clone(), ring.clone(), arcs.clone());
        let mut frame = div().debug_selector(|| "chart-root".into()).h(sizes.height);
        frame.style().refine(self.base.style());
        frame.flex().flex_col().gap_2().child(legend).child(
            div()
                .id((self.id.clone(), "ring"))
                .relative()
                .flex_1()
                .min_h_0()
                .on_mouse_move(move |event: &MouseMoveEvent, _, cx| {
                    let center = moved.read(cx).bounds.center();
                    let at = (f32::from(event.position.x), f32::from(event.position.y));
                    let next = arc_at(
                        &hit,
                        (f32::from(center.x), f32::from(center.y)),
                        at,
                        inner,
                        outer + lift,
                    );
                    moved.update(cx, |ring, cx| {
                        if ring.lift.point(next) {
                            cx.notify();
                        }
                    })
                })
                .hover_listener_mode(HoverListenerMode::InputModalityIndependent)
                .on_hover(move |inside, _, cx| {
                    if !*inside {
                        left.update(cx, |ring, cx| {
                            if ring.lift.point(None) {
                                cx.notify();
                            }
                        })
                    }
                })
                .child(
                    canvas(
                        |_, _, _| {},
                        move |bounds, _, window, _| {
                            let center = bounds.center();
                            for (ix, (start, sweep)) in arcs.iter().enumerate() {
                                let shifted = toward(center, lift * lifts[ix], start + sweep / 2.0);
                                if let Some(path) =
                                    wedge(shifted, (inner, outer), (*start, *sweep), gap)
                                {
                                    finish(path, fills[ix], window);
                                }
                            }
                        },
                    )
                    .absolute()
                    .inset_0(),
                )
                .children(middle)
                .children(tooltip)
                .child(measure(measured, |ring| &mut ring.bounds)),
        )
    }
}

#[cfg(test)]
mod tests {
    use std::f32::consts::{FRAC_PI_2, TAU};

    use super::*;

    #[test]
    fn slices_sweep_by_share_from_twelve_oclock() {
        let arcs = sweeps(&[1.0, 3.0]);
        assert_eq!(arcs[0], (-FRAC_PI_2, TAU / 4.0));
        assert_eq!(arcs[1].1, TAU * 0.75);
        assert_eq!(
            arc_at(&arcs, (0.0, 0.0), (5.0, -5.0), 0.0, 20.0),
            Some(0),
            "up and right is the first quarter"
        );
        assert_eq!(arc_at(&arcs, (0.0, 0.0), (-5.0, 5.0), 0.0, 20.0), Some(1));
        assert_eq!(
            arc_at(&arcs, (0.0, 0.0), (50.0, 0.0), 0.0, 20.0),
            None,
            "outside the ring"
        );
    }

    #[test]
    fn a_lifted_slice_fades_the_others() {
        assert_eq!(fades(&[0.0, 1.0, 0.0]), [1.0, 0.0, 1.0]);
        assert_eq!(
            fades(&[0.5, 0.5]),
            [0.25, 0.25],
            "a slice settling as another lifts"
        );
        assert_eq!(fades(&[0.0, 0.0]), [0.0, 0.0]);
    }
}
