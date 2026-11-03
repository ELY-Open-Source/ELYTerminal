use std::{
    f32::consts::{FRAC_PI_2, TAU},
    rc::Rc,
};

use gpui::{
    App, Bounds, Div, ElementId, Entity, IntoElement, ParentElement, Pixels, Refineable,
    RenderOnce, SharedString, StyleRefinement, Styled, Window, canvas, div, fill, point,
    prelude::*, size,
};

use super::{
    axes::anchored,
    geometry::Rect,
    glide::Glide,
    paint::{at, finish, measure, place, ring, tint},
    parts::ChartTooltip,
    pie::{bearing, toward, wedge},
    plot::Format,
    scale::{compact, nice},
    series::drawable,
    tiles::{Tiles, tracked},
};
use crate::{
    motion,
    theme::{ActiveTheme, Density, Radius, TextSize},
    typography::{format, tabular},
};

/// A measure against its target: its value, the target it aims for, and the bands from poor to good it falls in.
#[derive(Clone, Debug, PartialEq)]
pub struct Bullet {
    label: SharedString,
    value: f64,
    target: f64,
    bands: Vec<f64>,
}

impl Bullet {
    pub fn new(label: impl Into<SharedString>, value: f64, target: f64) -> Self {
        assert!(
            drawable(value) && drawable(target) && value >= 0.0 && target >= 0.0,
            "a bullet reads values of zero or more, within charts::LIMIT"
        );
        Self {
            label: label.into(),
            value,
            target,
            bands: Vec::new(),
        }
    }

    /// Where each band ends, rising: poor, then fair, then good.
    pub fn bands(mut self, ends: impl IntoIterator<Item = f64>) -> Self {
        self.bands = ends.into_iter().collect();
        assert!(
            self.bands.windows(2).all(|pair| pair[0] < pair[1])
                && self.bands.iter().all(|end| *end > 0.0 && drawable(*end)),
            "bands end in rising order, within charts::LIMIT"
        );
        self
    }

    /// The top of this bullet's scale: past its value, its target and its last band.
    fn top(&self) -> f64 {
        let most = self
            .bands
            .iter()
            .copied()
            .fold(self.value.max(self.target), f64::max);
        nice(0.0, most.max(f64::EPSILON), 4).0.1
    }
}

/// Measures against their targets, a row each: a thin bar for the value over bands from poor to good, and a mark at the target. Hover a row to read it.
#[derive(IntoElement)]
pub struct BulletChart {
    base: Div,
    id: ElementId,
    bullets: Vec<Bullet>,
    format: Format,
}

impl BulletChart {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            base: div(),
            id: id.into(),
            bullets: Vec::new(),
            format: Rc::new(compact),
        }
    }

    pub fn bullet(mut self, bullet: Bullet) -> Self {
        self.bullets.push(bullet);
        self
    }

    pub fn format(mut self, format: impl Fn(f64) -> String + 'static) -> Self {
        self.format = Rc::new(format);
        self
    }
}

impl Styled for BulletChart {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for BulletChart {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let tiles: Entity<Tiles> =
            window.use_keyed_state((self.id.clone(), "rows"), cx, |_, _| Tiles::default());
        let (bounds, hover) = (
            tiles.read(cx).bounds,
            tiles.read(cx).pointed(self.bullets.len()),
        );
        let theme = cx.theme();
        let (colors, sizes, rem) = (theme.colors.clone(), theme.chart(), window.rem_size());
        let pixels = |length: gpui::Rems| f32::from(length.to_pixels(rem));
        let (names, row, inset, count) = (
            pixels(theme.label_width()),
            pixels(theme.table_row(Density::Comfortable)),
            pixels(sizes.inset),
            self.bullets.len(),
        );
        let length = (f32::from(bounds.size.width) - names - inset).max(0.0);
        let format = self.format.clone();
        let labels: Vec<Div> = self
            .bullets
            .iter()
            .enumerate()
            .map(|(ix, bullet)| {
                let reading = format!("{} / {}", format(bullet.value), format(bullet.target));
                div()
                    .absolute()
                    .left_0()
                    .top(Pixels::from(row * ix as f32))
                    .w(Pixels::from(names))
                    .h(Pixels::from(row))
                    .flex()
                    .flex_col()
                    .justify_center()
                    .pr_3()
                    .child(
                        div()
                            .truncate()
                            .text_size(theme.text_size(TextSize::Sm))
                            .text_color(colors.fg)
                            .child(bullet.label.clone()),
                    )
                    .child(
                        tabular(div())
                            .text_size(theme.text_size(TextSize::Xs))
                            .text_color(colors.fg_muted)
                            .child(reading),
                    )
            })
            .collect();
        let tooltip = hover.map(|ix| {
            let bullet = &self.bullets[ix];
            let reach = if bullet.target > 0.0 {
                format::percent(bullet.value / bullet.target, 0, false)
            } else {
                "—".into()
            };
            let card = ChartTooltip::new(bullet.label.clone())
                .row(Some(tint(&colors, 0)), "Value", format(bullet.value))
                .row(None, "Target", format(bullet.target))
                .row(None, "Of target", reach);
            let x = names + length * (bullet.value / bullet.top()) as f32;
            anchored(
                (x, row * ix as f32 + row / 2.0),
                x > f32::from(bounds.size.width) * 0.6,
                card,
            )
        });
        let rows: Vec<(Vec<f32>, f32, f32)> = self
            .bullets
            .iter()
            .map(|bullet| {
                let share = |value: f64| (value / bullet.top()) as f32;
                (
                    bullet.bands.iter().map(|end| share(*end)).collect(),
                    share(bullet.value),
                    share(bullet.target),
                )
            })
            .collect();
        let (stroke, corner, palette) = (
            sizes.stroke.to_pixels(rem),
            theme.radius(Radius::Sm).to_pixels(rem),
            colors.clone(),
        );
        let mut root = div()
            .debug_selector(|| "chart-root".into())
            .relative()
            .h(theme.table_row(Density::Comfortable) * count as f32);
        root.style().refine(self.base.style());
        tracked(
            root,
            (self.id.clone(), "bullets").into(),
            &tiles,
            move |(_, y)| {
                let at = (y / row).floor();
                (at >= 0.0 && (at as usize) < count).then_some(at as usize)
            },
        )
        .child(
            canvas(
                |_, _, _| {},
                move |bounds, _, window, _| {
                    let origin = bounds.origin;
                    for (ix, (bands, value, target)) in rows.iter().enumerate() {
                        let top = row * ix as f32;
                        let band = |share: f32| Rect {
                            x: names,
                            y: top + row * 0.2,
                            w: length * share,
                            h: row * 0.6,
                        };
                        for (step, end) in bands.iter().enumerate().rev() {
                            let strength = 0.04 + 0.04 * (bands.len() - 1 - step) as f32;
                            window.paint_quad(
                                fill(place(origin, band(*end)), palette.fg.opacity(strength))
                                    .corner_radii(corner),
                            );
                        }
                        let bar = Rect {
                            x: names,
                            y: top + row * 0.38,
                            w: length * value,
                            h: row * 0.24,
                        };
                        let ink = if hover.is_none_or(|on| on == ix) {
                            tint(&palette, 0)
                        } else {
                            tint(&palette, 0).opacity(0.5)
                        };
                        window.paint_quad(fill(place(origin, bar), ink).corner_radii(corner));
                        let mark = Bounds::new(
                            at(
                                origin,
                                (
                                    names + length * target - f32::from(stroke) / 2.0,
                                    top + row * 0.14,
                                ),
                            ),
                            size(stroke, Pixels::from(row * 0.72)),
                        );
                        window.paint_quad(fill(mark, palette.fg));
                    }
                },
            )
            .absolute()
            .inset_0(),
        )
        .children(labels)
        .children(tooltip)
        .child(measure(tiles, |tiles| &mut tiles.bounds))
    }
}

/// Goals as rings around one center, each filling clockwise toward its target; the center reads the goal under the pointer, or the first.
#[derive(IntoElement)]
pub struct ProgressChart {
    base: Div,
    id: ElementId,
    goals: Vec<(SharedString, f64, f64)>,
    format: Format,
}

impl ProgressChart {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            base: div(),
            id: id.into(),
            goals: Vec::new(),
            format: Rc::new(compact),
        }
    }

    /// A goal: how far it has come of its target.
    pub fn goal(mut self, name: impl Into<SharedString>, value: f64, target: f64) -> Self {
        assert!(
            drawable(value) && value >= 0.0 && drawable(target) && target > 0.0,
            "a goal has a positive target and a value of zero or more, within charts::LIMIT"
        );
        self.goals.push((name.into(), value, target));
        self
    }

    pub fn format(mut self, format: impl Fn(f64) -> String + 'static) -> Self {
        self.format = Rc::new(format);
        self
    }
}

impl Styled for ProgressChart {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for ProgressChart {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let rings: Entity<Tiles> =
            window.use_keyed_state((self.id.clone(), "rings"), cx, |_, _| Tiles::default());
        let glide: Entity<Glide> =
            window.use_keyed_state((self.id.clone(), "glide"), cx, |_, _| Glide::default());
        let shares: Vec<f64> = self
            .goals
            .iter()
            .map(|(_, value, target)| value / target)
            .collect();
        let drawn: Vec<f64> = shares.iter().map(|share| share.min(1.0)).collect();
        let slow = motion::duration(motion::SLOW, cx);
        let (glided, moving) = glide.update(cx, |glide, _| {
            glide.follow(std::slice::from_ref(&drawn), slow)
        });
        if moving {
            window.request_animation_frame();
        }
        let glided = glided.into_iter().next().expect("one row of shares");
        let (bounds, hover) = (
            rings.read(cx).bounds,
            rings.read(cx).pointed(self.goals.len()),
        );
        let theme = cx.theme();
        let (colors, sizes, rem) = (theme.colors.clone(), theme.chart(), window.rem_size());
        let side = f32::from(bounds.size.width).min(f32::from(bounds.size.height));
        let count = self.goals.len();
        let gap = f32::from(sizes.stroke.to_pixels(rem)) * 2.0;
        let thick = (side / 2.0 / count.max(1) as f32 - gap)
            .min(side * 0.09)
            .max(0.0);
        let radius = move |ix: usize| side / 2.0 - thick / 2.0 - (thick + gap) * ix as f32;
        let shown = hover.unwrap_or(0);
        let format = self.format.clone();
        let middle = self.goals.get(shown).map(|(name, _, _)| {
            div()
                .absolute()
                .inset_0()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .child(
                    tabular(div())
                        .text_size(theme.text_size(TextSize::Lg))
                        .text_color(colors.fg)
                        .child(format::percent(shares[shown], 0, false)),
                )
                .child(
                    div()
                        .text_size(theme.text_size(TextSize::Xs))
                        .text_color(colors.fg_muted)
                        .child(name.clone()),
                )
        });
        let legend = self
            .goals
            .iter()
            .enumerate()
            .map(|(ix, (name, value, target))| {
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .text_size(theme.text_size(TextSize::Sm))
                    .when(hover.is_some_and(|on| on != ix), |row| row.opacity(0.5))
                    .child(
                        div()
                            .size(theme.status_dot())
                            .rounded_full()
                            .bg(tint(&colors, ix)),
                    )
                    .child(div().text_color(colors.fg).child(name.clone()))
                    .child(tabular(div()).text_color(colors.fg_muted).child(format!(
                        "{} / {}",
                        format(*value),
                        format(*target)
                    )))
            });
        let tooltip = hover.map(|ix| {
            let (name, value, target) = &self.goals[ix];
            let card = ChartTooltip::new(name.clone())
                .row(
                    Some(tint(&colors, ix)),
                    "Done",
                    format::percent(value / target, 0, false),
                )
                .row(None, "Of", format(*target));
            anchored((side / 2.0 + radius(ix), side / 2.0), false, card)
        });
        let palette = colors.clone();
        let mut root = div().flex().items_center().gap_8();
        root.style().refine(self.base.style());
        let dial = tracked(
            div().relative().flex_none().size(sizes.height * 0.7),
            (self.id.clone(), "rings").into(),
            &rings,
            move |place| {
                let (_, distance) = bearing((side / 2.0, side / 2.0), place);
                (0..count).find(|ix| (distance - radius(*ix)).abs() <= thick / 2.0 + gap / 2.0)
            },
        );
        root.child(
            dial.child(
                canvas(
                    |_, _, _| {},
                    move |bounds, _, window, _| {
                        let center = bounds.origin
                            + point(Pixels::from(side / 2.0), Pixels::from(side / 2.0));
                        for (ix, share) in glided.iter().enumerate() {
                            let (middle, ink) = (radius(ix), tint(&palette, ix));
                            let ink = if hover.is_some_and(|on| on != ix) {
                                ink.opacity(0.35)
                            } else {
                                ink
                            };
                            let band = (middle - thick / 2.0, middle + thick / 2.0);
                            if let Some(track) = wedge(center, band, (-FRAC_PI_2, TAU), 0.0) {
                                finish(track, palette.fg.opacity(0.06), window);
                            }
                            let sweep = (*share as f32).clamp(0.0, 1.0) * TAU;
                            if sweep <= 0.0 {
                                continue;
                            }
                            if let Some(arc) = wedge(center, band, (-FRAC_PI_2, sweep), 0.0) {
                                finish(arc, ink, window);
                            }
                            for angle in [-FRAC_PI_2, -FRAC_PI_2 + sweep] {
                                ring(
                                    toward(center, middle, angle),
                                    Pixels::from(thick / 2.0),
                                    (ink, ink),
                                    Pixels::ZERO,
                                    window,
                                );
                            }
                        }
                    },
                )
                .absolute()
                .inset_0(),
            )
            .children(middle)
            .children(tooltip)
            .child(measure(rings, |rings| &mut rings.bounds)),
        )
        .child(div().flex().flex_col().gap_2().children(legend))
    }
}
