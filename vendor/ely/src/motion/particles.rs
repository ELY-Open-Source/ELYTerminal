use std::{f32::consts::TAU, time::Duration};

use gpui::{
    Animation, AnimationExt, AnyElement, App, Bounds, Div, ElementId, Hsla, InteractiveElement,
    IntoElement, MouseButton, ParentElement, PathBuilder, Pixels, Point, RenderOnce,
    StyleRefinement, Styled, Window, canvas, div, fill, linear_color_stop, linear_gradient, point,
    px, size,
};
use smallvec::SmallVec;
use web_time::Instant;

use super::{
    duration,
    skeleton::{radii, span},
};
use crate::theme::ActiveTheme;

/// How long a burst of confetti falls, and how many pieces it throws.
const FALL: Duration = Duration::from_millis(1_800);
const PIECES: usize = 42;
/// One slow round of drifting particles, and how many there are.
const DRIFT: Duration = Duration::from_secs(24);
const MOTES: usize = 36;
/// One full turn of a tonal drift.
const TIDE: Duration = Duration::from_secs(40);
/// How long a ripple takes to spread and fade.
const SPREAD: Duration = Duration::from_millis(560);

/// A small repeatable random stream, so a burst or a field looks the same each time.
struct Seed(u32);

impl Seed {
    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        (self.0 % 10_000) as f32 / 10_000.0
    }
}

/// A burst of paper pieces thrown up from the middle each time `bursts` grows; they tumble and fall in the chart's quiet colors. Under reduced motion nothing is thrown.
#[derive(IntoElement)]
pub struct Confetti {
    id: ElementId,
    bursts: usize,
}

impl Confetti {
    /// Place it over the area the pieces should fill; it takes no presses.
    pub fn new(id: impl Into<ElementId>, bursts: usize) -> Self {
        Self {
            id: id.into(),
            bursts,
        }
    }
}

/// The last count and the burst number after a new `bursts`: only growth throws a burst.
fn step((last, turn): (usize, usize), bursts: usize) -> (usize, usize) {
    (bursts, turn + usize::from(bursts > last))
}

fn piece(center: Point<Pixels>, side: Pixels, turn: f32, color: Hsla, window: &mut Window) {
    let (cos, sin) = (turn.cos(), turn.sin());
    let corner = |x: f32, y: f32| {
        let (dx, dy) = (side * x, side * 0.5 * y);
        point(
            center.x + dx * cos - dy * sin,
            center.y + dx * sin + dy * cos,
        )
    };
    let mut path = PathBuilder::fill();
    path.move_to(corner(-0.5, -0.5));
    path.line_to(corner(0.5, -0.5));
    path.line_to(corner(0.5, 0.5));
    path.line_to(corner(-0.5, 0.5));
    path.close();
    match path.build() {
        Ok(path) => window.paint_path(path, color),
        Err(error) => log::error!("confetti: a piece failed to build: {error:#}"),
    }
}

impl RenderOnce for Confetti {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let bursts = self.bursts;
        let count =
            window.use_keyed_state((self.id.clone(), "bursts"), cx, |_, _| (bursts, 0usize));
        let next = step(*count.read(cx), bursts);
        if next != *count.read(cx) {
            count.update(cx, |count, _| *count = next);
        }
        let turn = next.1;
        let theme = cx.theme();
        if turn == 0 || theme.reduced_motion {
            return div().absolute().inset_0().into_any_element();
        }
        let colors = theme.colors.chart;
        let side = theme.status_dot() * 1.5;
        let side = side.to_pixels(window.rem_size());
        div()
            .absolute()
            .inset_0()
            .with_animation(
                (self.id, format!("burst-{turn}")),
                Animation::new(FALL),
                move |sky, t| {
                    sky.child(
                        canvas(
                            |_, _, _| {},
                            move |bounds, _, window, _| {
                                let mut seed = Seed(0x9e37_79b9 ^ turn as u32);
                                let origin = bounds.center();
                                let height = f32::from(bounds.size.height);
                                let secs = t * FALL.as_secs_f32();
                                let fade = 1.0 - ((t - 0.7) / 0.3).clamp(0.0, 1.0);
                                for ix in 0..PIECES {
                                    let angle = -TAU / 4.0 + (seed.next() - 0.5) * 1.4;
                                    let speed = height * (2.4 + 1.2 * seed.next());
                                    let (vx, vy) = (angle.cos() * speed, angle.sin() * speed);
                                    let x = vx * secs;
                                    let y = vy * secs + 0.5 * height * 4.5 * secs * secs;
                                    let spin =
                                        seed.next() * TAU + secs * (6.0 + seed.next() * 10.0);
                                    let color = colors[ix % colors.len()].opacity(fade);
                                    let at = point(origin.x + px(x), origin.y + px(y));
                                    piece(at, side, spin, color, window);
                                }
                            },
                        )
                        .size_full(),
                    )
                },
            )
            .into_any_element()
    }
}

macro_rules! backdrop {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(IntoElement)]
        pub struct $name {
            id: ElementId,
            base: Div,
            body: SmallVec<[AnyElement; 2]>,
        }

        impl $name {
            pub fn new(id: impl Into<ElementId>) -> Self {
                Self {
                    id: id.into(),
                    base: div(),
                    body: SmallVec::new(),
                }
            }
        }

        impl Styled for $name {
            fn style(&mut self) -> &mut StyleRefinement {
                self.base.style()
            }
        }

        impl ParentElement for $name {
            fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
                self.body.extend(elements);
            }
        }
    };
}

backdrop!(
    ParticleBackground,
    "Faint motes that drift slowly behind what it holds. Under reduced motion they rest."
);

/// Whether a dot of `radius` at `at` lies wholly inside the rounded box.
fn inside(bounds: Bounds<Pixels>, radii: [Pixels; 4], at: Point<Pixels>, radius: Pixels) -> bool {
    [at.x - radius, at.x, at.x + radius].iter().all(|x| {
        let (top, bottom) = span(bounds, radii, *x);
        at.y - radius >= top && at.y + radius <= bottom
    })
}

fn motes(
    bounds: Bounds<Pixels>,
    radii: [Pixels; 4],
    t: f32,
    color: Hsla,
    radius: Pixels,
    window: &mut Window,
) {
    let mut seed = Seed(0x2545_f491);
    for _ in 0..MOTES {
        let (x, y, phase, reach) = (
            seed.next(),
            seed.next(),
            seed.next() * TAU,
            0.02 + seed.next() * 0.04,
        );
        let drift = TAU * t + phase;
        let at = point(
            bounds.left() + bounds.size.width * (x + reach * drift.cos()),
            bounds.top() + bounds.size.height * (y + reach * drift.sin()),
        );
        if !inside(bounds, radii, at, radius) {
            continue;
        }
        let corner = point(at.x - radius, at.y - radius);
        window.paint_quad(
            fill(Bounds::new(corner, size(radius * 2.0, radius * 2.0)), color).corner_radii(radius),
        );
    }
}

impl RenderOnce for ParticleBackground {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let radii = radii(self.base.style(), window.rem_size());
        let theme = cx.theme();
        let color = theme.colors.fg.opacity(0.08);
        let radius = (theme.status_dot() / 2.0).to_pixels(window.rem_size());
        let still = theme.reduced_motion;
        let field = move |t: f32| {
            canvas(
                |_, _, _| {},
                move |bounds, _, window, _| motes(bounds, radii, t, color, radius, window),
            )
            .absolute()
            .inset_0()
        };
        let layer = if still {
            div()
                .absolute()
                .inset_0()
                .child(field(0.0))
                .into_any_element()
        } else {
            div()
                .absolute()
                .inset_0()
                .with_animation(self.id, Animation::new(DRIFT).repeat(), move |layer, t| {
                    layer.child(field(t))
                })
                .into_any_element()
        };
        self.base
            .relative()
            .overflow_hidden()
            .child(layer)
            .children(self.body)
    }
}

backdrop!(
    AnimatedGradient,
    "A background whose tone drifts, slowly, between the surface and a breath of the accent. Under reduced motion it rests."
);

impl RenderOnce for AnimatedGradient {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let (from, to) = (theme.colors.surface, theme.colors.selection);
        let tone = move |t: f32| {
            linear_gradient(
                90.0 + 360.0 * t,
                linear_color_stop(from, 0.0),
                linear_color_stop(to, 1.0),
            )
        };
        let base = self.base.children(self.body);
        if theme.reduced_motion {
            return base.bg(tone(0.0)).into_any_element();
        }
        base.with_animation(self.id, Animation::new(TIDE).repeat(), move |base, t| {
            base.bg(tone(t))
        })
        .into_any_element()
    }
}

/// Where the last press landed, and when.
#[derive(Clone, Copy)]
struct Press {
    at: Point<Pixels>,
    since: Instant,
    turn: usize,
}

backdrop!(
    Ripple,
    "A circle of light that spreads from each press and fades, cut to the box's rounded corners. Under reduced motion none shows."
);

impl RenderOnce for Ripple {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let press = window.use_keyed_state((self.id.clone(), "press"), cx, |_, _| None::<Press>);
        let theme = cx.theme();
        let (still, color) = (theme.reduced_motion, theme.colors.fg);
        let radii = radii(self.base.style(), window.rem_size());
        let held = press.clone();
        let base = self
            .base
            .id(self.id.clone())
            .relative()
            .children(self.body)
            .on_mouse_down(MouseButton::Left, move |event, _, cx| {
                held.update(cx, |press, cx| {
                    let turn = press.map_or(1, |press| press.turn + 1);
                    *press = Some(Press {
                        at: event.position,
                        since: Instant::now(),
                        turn,
                    });
                    cx.notify();
                });
            });
        let Some(last) = *press.read(cx) else {
            return base.into_any_element();
        };
        let length = duration(SPREAD, cx);
        if still || last.since.elapsed() >= length {
            return base.into_any_element();
        }
        window.request_animation_frame();
        base.child(div().absolute().inset_0().with_animation(
            (self.id, format!("ripple-{}", last.turn)),
            Animation::new(length),
            move |layer, t| {
                layer.child(
                    canvas(
                        |_, _, _| {},
                        move |bounds, _, window, _| ring(bounds, radii, last.at, t, color, window),
                    )
                    .size_full(),
                )
            },
        ))
        .into_any_element()
    }
}

/// Where a disc of `reach` around `at` covers the rounded box: each column's x, top and bottom, left to right.
fn disc_columns(
    bounds: Bounds<Pixels>,
    radii: [Pixels; 4],
    at: Point<Pixels>,
    reach: f32,
) -> Vec<(Pixels, Pixels, Pixels)> {
    let (left, right) = (
        (at.x - px(reach)).max(bounds.left()),
        (at.x + px(reach)).min(bounds.right()),
    );
    if left >= right {
        return Vec::new();
    }
    (0..=24)
        .map(|ix| left + (right - left) * (ix as f32 / 24.0))
        .map(|x| {
            let dx = f32::from(x - at.x);
            let half = px((reach * reach - dx * dx).max(0.0).sqrt());
            let (top, bottom) = span(bounds, radii, x);
            (x, (at.y - half).max(top), (at.y + half).min(bottom))
        })
        .filter(|(_, top, bottom)| top < bottom)
        .collect()
}

/// The ripple at `t`: a disc from `at`, cut to the rounded box, fading as it grows.
fn ring(
    bounds: Bounds<Pixels>,
    radii: [Pixels; 4],
    at: Point<Pixels>,
    t: f32,
    color: Hsla,
    window: &mut Window,
) {
    let far = [
        bounds.origin,
        bounds.top_right(),
        bounds.bottom_left(),
        bounds.bottom_right(),
    ]
    .iter()
    .map(|corner| {
        let (dx, dy) = (f32::from(corner.x - at.x), f32::from(corner.y - at.y));
        (dx * dx + dy * dy).sqrt()
    })
    .fold(0.0_f32, f32::max);
    let reach = far * (1.0 - (1.0 - t).powi(3));
    let columns = disc_columns(bounds, radii, at, reach);
    if columns.len() < 2 {
        return;
    }
    let mut path = PathBuilder::fill();
    path.move_to(point(columns[0].0, columns[0].1));
    for (x, top, _) in &columns[1..] {
        path.line_to(point(*x, *top));
    }
    for (x, _, bottom) in columns.iter().rev() {
        path.line_to(point(*x, *bottom));
    }
    path.close();
    match path.build() {
        Ok(path) => window.paint_path(path, color.opacity(0.12 * (1.0 - t))),
        Err(error) => log::error!("ripple: the ring failed to build: {error:#}"),
    }
}

#[cfg(test)]
mod tests {
    use super::Seed;

    use gpui::{Bounds, point, px, size};

    use super::{disc_columns, inside, step};

    #[test]
    fn only_a_growing_count_throws_a_burst() {
        assert_eq!(step((2, 1), 3), (3, 2));
        assert_eq!(step((2, 1), 1), (1, 1));
        assert_eq!(step((2, 1), 2), (2, 1));
    }

    #[test]
    fn a_ripple_and_a_mote_keep_inside_rounded_corners() {
        let bounds = Bounds::new(point(px(0.0), px(0.0)), size(px(100.0), px(40.0)));
        let pill = [px(20.0); 4];
        assert!(
            disc_columns(bounds, pill, point(px(1.0), px(1.0)), 4.0).is_empty(),
            "the corner is outside the pill"
        );
        assert!(!disc_columns(bounds, pill, point(px(50.0), px(20.0)), 10.0).is_empty());
        assert!(!inside(bounds, pill, point(px(3.0), px(3.0)), px(2.0)));
        assert!(inside(bounds, pill, point(px(50.0), px(20.0)), px(2.0)));
    }

    #[test]
    fn a_seed_gives_the_same_stream_each_time() {
        let draw = |seed| {
            let mut seed = Seed(seed);
            (0..4).map(|_| seed.next()).collect::<Vec<f32>>()
        };
        assert_eq!(draw(7), draw(7));
        assert_ne!(draw(7), draw(8));
        assert!(draw(7).iter().all(|value| (0.0..1.0).contains(value)));
    }
}
