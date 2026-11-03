use std::{ops::Range, rc::Rc};

use gpui::{Hsla, IntoElement, PathBuilder, Pixels, Point, Styled, Window, canvas, fill};

use super::{
    candles::Candle,
    market::ChartKind,
    series::Drawn,
    stage::Visible,
    tools::{self, Drawing},
};
use crate::{
    charts::{Linear, Rect, at, finish, place, tint},
    theme::Palette,
};

/// How a market chart paints: its colors and measures in pixels.
#[derive(Clone)]
pub(crate) struct Pen {
    pub palette: Palette,
    pub rise: Hsla,
    pub fall: Hsla,
    pub stroke: Pixels,
    pub hairline: Pixels,
}

/// What a market chart paints, in its box's own pixels: panes with their price scales, main first; the candles in view and how they draw; volume, profile, overlays, studies, a compared symbol, the last price, alerts, and the crosshair.
pub(crate) struct Scene {
    pub panes: Vec<(Rect, Linear, Vec<f64>)>,
    pub visible: Visible,
    pub range: Range<usize>,
    pub candles: Rc<Vec<Candle>>,
    pub kind: ChartKind,
    pub volume: Option<f64>,
    pub profile: Vec<(f64, f64, f64)>,
    pub overlays: Rc<Vec<Drawn>>,
    pub studies: Rc<Vec<Drawn>>,
    pub compare: Option<Vec<Option<f64>>>,
    pub last: Option<(f64, bool)>,
    pub alerts: Vec<f64>,
    pub cross: Option<(i64, Option<(usize, f32)>)>,
    pub drawings: Vec<Drawing>,
    pub pen: Pen,
}

impl Scene {
    fn x(&self, ix: usize) -> f32 {
        let main = self.panes[0].0;
        self.visible.x(ix as f64, (main.x, main.w))
    }

    fn slot(&self) -> f32 {
        self.visible.slot(self.panes[0].0.w)
    }
}

/// A straight line in dashes.
fn dashed(
    origin: Point<Pixels>,
    (from, to): ((f32, f32), (f32, f32)),
    color: Hsla,
    pen: &Pen,
    window: &mut Window,
) {
    let dash = [pen.stroke * 2.0, pen.stroke * 2.0];
    let mut path = PathBuilder::stroke(pen.hairline).dash_array(&dash);
    path.move_to(at(origin, from));
    path.line_to(at(origin, to));
    finish(path, color, window);
}

/// A reading's line through the candles in view, broken where it has no value.
fn reading(
    scene: &Scene,
    values: &[Option<f64>],
    scale: &Linear,
    width: Pixels,
    color: Hsla,
    origin: Point<Pixels>,
    window: &mut Window,
) {
    let mut path = PathBuilder::stroke(width);
    let mut open = false;
    for ix in scene.range.clone() {
        match values.get(ix).copied().flatten() {
            Some(value) => {
                let place = at(origin, (scene.x(ix), scale.at(value)));
                if open {
                    path.line_to(place)
                } else {
                    path.move_to(place)
                }
                open = true;
            }
            None => open = false,
        }
    }
    finish(path, color, window);
}

/// The price's own marks: candles, bars, or a line with or without its area.
fn prices(scene: &Scene, origin: Point<Pixels>, window: &mut Window) {
    let (main, scale) = (scene.panes[0].0, &scene.panes[0].1);
    let (pen, slot) = (&scene.pen, scene.slot());
    let body = (slot * 0.68).max(f32::from(pen.hairline));
    if matches!(scene.kind, ChartKind::Line | ChartKind::Area) {
        let closes: Vec<Option<f64>> = scene
            .candles
            .iter()
            .map(|candle| Some(candle.close))
            .collect();
        let ink = tint(&pen.palette, 0);
        if scene.kind == ChartKind::Area && scene.range.len() > 1 {
            let mut area = PathBuilder::fill();
            let (first, last) = (scene.range.start, scene.range.end - 1);
            area.move_to(at(origin, (scene.x(first), main.y + main.h)));
            scene.range.clone().for_each(|ix| {
                area.line_to(at(origin, (scene.x(ix), scale.at(scene.candles[ix].close))))
            });
            area.line_to(at(origin, (scene.x(last), main.y + main.h)));
            area.close();
            finish(area, ink.opacity(0.12), window);
        }
        reading(scene, &closes, scale, pen.stroke, ink, origin, window);
        return;
    }
    for ix in scene.range.clone() {
        let (candle, x) = (&scene.candles[ix], scene.x(ix));
        let ink = if candle.rose() { pen.rise } else { pen.fall };
        let (high, low, open, close) = (
            scale.at(candle.high),
            scale.at(candle.low),
            scale.at(candle.open),
            scale.at(candle.close),
        );
        let thin = f32::from(pen.hairline);
        window.paint_quad(fill(
            place(
                origin,
                Rect {
                    x: x - thin / 2.0,
                    y: high,
                    w: thin,
                    h: low - high,
                },
            ),
            ink,
        ));
        if scene.kind == ChartKind::Bars {
            let tick = body / 2.0;
            window.paint_quad(fill(
                place(
                    origin,
                    Rect {
                        x: x - tick,
                        y: open - thin / 2.0,
                        w: tick,
                        h: thin,
                    },
                ),
                ink,
            ));
            window.paint_quad(fill(
                place(
                    origin,
                    Rect {
                        x,
                        y: close - thin / 2.0,
                        w: tick,
                        h: thin,
                    },
                ),
                ink,
            ));
            continue;
        }
        let (top, bottom) = (open.min(close), open.max(close));
        let rect = Rect {
            x: x - body / 2.0,
            y: top,
            w: body,
            h: (bottom - top).max(thin),
        };
        window.paint_quad(fill(place(origin, rect), ink));
    }
}

/// Volume as faint bars along the bottom of the price pane, and the volume at each price as bars from its right edge.
fn volume(scene: &Scene, origin: Point<Pixels>, window: &mut Window) {
    let (main, pen) = (scene.panes[0].0, &scene.pen);
    if let Some(most) = scene.volume.filter(|most| *most > 0.0) {
        let (body, room) = (
            (scene.slot() * 0.68).max(f32::from(pen.hairline)),
            main.h * 0.2,
        );
        for ix in scene.range.clone() {
            let candle = &scene.candles[ix];
            let tall = room * (candle.volume / most) as f32;
            let ink = if candle.rose() { pen.rise } else { pen.fall };
            let rect = Rect {
                x: scene.x(ix) - body / 2.0,
                y: main.y + main.h - tall,
                w: body,
                h: tall,
            };
            window.paint_quad(fill(place(origin, rect), ink.opacity(0.22)));
        }
    }
    let most = scene
        .profile
        .iter()
        .map(|(_, _, volume)| *volume)
        .fold(0.0, f64::max);
    for (low, high, traded) in scene.profile.iter().filter(|_| most > 0.0) {
        let scale = &scene.panes[0].1;
        let (top, bottom) = (scale.at(*high), scale.at(*low));
        let wide = main.w * 0.24 * (*traded / most) as f32;
        let strength = if *traded == most { 0.16 } else { 0.08 };
        let seam = f32::from(pen.hairline);
        let rect = Rect {
            x: main.x + main.w - wide,
            y: top + seam,
            w: wide,
            h: (bottom - top - seam).max(0.0),
        };
        window.paint_quad(fill(place(origin, rect), pen.palette.fg.opacity(strength)));
    }
}

/// Overlays over the prices: a shaded band, then their lines.
fn overlays(scene: &Scene, origin: Point<Pixels>, window: &mut Window) {
    let (scale, pen) = (&scene.panes[0].1, &scene.pen);
    for drawn in scene.overlays.iter() {
        let color = tint(
            &pen.palette,
            drawn.lines.first().map_or(0, |(color, _)| *color),
        );
        if let Some((low, high)) = &drawn.band {
            let shown: Vec<usize> = scene
                .range
                .clone()
                .filter(|ix| low[*ix].is_some() && high[*ix].is_some())
                .collect();
            if shown.len() > 1 {
                let mut band = PathBuilder::fill();
                band.move_to(at(
                    origin,
                    (scene.x(shown[0]), scale.at(high[shown[0]].expect("shown"))),
                ));
                shown.iter().for_each(|ix| {
                    band.line_to(at(
                        origin,
                        (scene.x(*ix), scale.at(high[*ix].expect("shown"))),
                    ))
                });
                shown.iter().rev().for_each(|ix| {
                    band.line_to(at(
                        origin,
                        (scene.x(*ix), scale.at(low[*ix].expect("shown"))),
                    ))
                });
                band.close();
                finish(band, color.opacity(0.08), window);
                reading(
                    scene,
                    high,
                    scale,
                    pen.hairline,
                    color.opacity(0.6),
                    origin,
                    window,
                );
                reading(
                    scene,
                    low,
                    scale,
                    pen.hairline,
                    color.opacity(0.6),
                    origin,
                    window,
                );
            }
        }
        for (ink, values) in &drawn.lines {
            reading(
                scene,
                values,
                scale,
                pen.stroke * 0.75,
                tint(&pen.palette, *ink),
                origin,
                window,
            );
        }
    }
    if let Some(compare) = &scene.compare {
        reading(
            scene,
            compare,
            scale,
            pen.stroke * 0.75,
            pen.palette.fg_muted,
            origin,
            window,
        );
    }
}

/// Each study in its pane: guides, bars about zero, then lines.
fn studies(scene: &Scene, origin: Point<Pixels>, window: &mut Window) {
    let pen = &scene.pen;
    for ((pane, scale, _), drawn) in scene.panes.iter().skip(1).zip(scene.studies.iter()) {
        for guide in &drawn.guides {
            let y = scale.at(*guide);
            dashed(
                origin,
                ((pane.x, y), (pane.x + pane.w, y)),
                pen.palette.border_strong,
                pen,
                window,
            );
        }
        if let Some(bars) = &drawn.bars {
            let (body, zero) = (
                (scene.slot() * 0.6).max(f32::from(pen.hairline)),
                scale.at(0.0),
            );
            for ix in scene.range.clone() {
                let Some(value) = bars[ix] else {
                    continue;
                };
                let y = scale.at(value);
                let ink = if value >= 0.0 { pen.rise } else { pen.fall };
                let rect = Rect {
                    x: scene.x(ix) - body / 2.0,
                    y: y.min(zero),
                    w: body,
                    h: (y - zero).abs(),
                };
                window.paint_quad(fill(place(origin, rect), ink.opacity(0.45)));
            }
        }
        for (ink, values) in &drawn.lines {
            reading(
                scene,
                values,
                scale,
                pen.stroke * 0.75,
                tint(&pen.palette, *ink),
                origin,
                window,
            );
        }
    }
}

/// The market chart's canvas: gridlines, volume and profile, prices, overlays, studies, the last price, alerts, and the crosshair.
pub(crate) fn drawing(scene: Scene) -> impl IntoElement {
    canvas(
        |_, _, _| {},
        move |bounds, _, window, _| {
            let origin = bounds.origin;
            let pen = &scene.pen;
            for (pane, scale, ticks) in &scene.panes {
                for tick in ticks {
                    let y = scale.at(*tick);
                    let line = Rect {
                        x: pane.x,
                        y,
                        w: pane.w,
                        h: f32::from(pen.hairline),
                    };
                    window.paint_quad(fill(place(origin, line), pen.palette.border.opacity(0.5)));
                }
            }
            volume(&scene, origin, window);
            let main = scene.panes[0].0;
            window.with_content_mask(
                Some(gpui::ContentMask {
                    bounds: place(origin, main),
                }),
                |window| {
                    prices(&scene, origin, window);
                    overlays(&scene, origin, window);
                    tools::drawings(&scene, origin, window);
                },
            );
            studies(&scene, origin, window);
            let edge = main.x + main.w;
            let domain = scene.panes[0].1.domain;
            let shown = |price: &f64| (domain.0..=domain.1).contains(price);
            for price in scene.alerts.iter().filter(|price| shown(price)) {
                let y = scene.panes[0].1.at(*price);
                dashed(
                    origin,
                    ((main.x, y), (edge, y)),
                    pen.palette.warning,
                    pen,
                    window,
                );
            }
            if let Some((price, rose)) = scene.last.filter(|(price, _)| shown(price)) {
                let y = scene.panes[0].1.at(price);
                dashed(
                    origin,
                    ((main.x, y), (edge, y)),
                    if rose { pen.rise } else { pen.fall },
                    pen,
                    window,
                );
            }
            if let Some((ix, row)) = scene.cross {
                let x = scene.visible.x(ix as f64, (main.x, main.w));
                let bottom = scene
                    .panes
                    .last()
                    .map_or(main.y + main.h, |(pane, _, _)| pane.y + pane.h);
                let upright = Rect {
                    x,
                    y: main.y,
                    w: f32::from(pen.hairline),
                    h: bottom - main.y,
                };
                window.paint_quad(fill(place(origin, upright), pen.palette.fg_subtle));
                if let Some((pane, y)) = row {
                    let across = scene.panes[pane].0;
                    dashed(
                        origin,
                        ((across.x, y), (across.x + across.w, y)),
                        pen.palette.fg_subtle,
                        pen,
                        window,
                    );
                }
            }
        },
    )
    .absolute()
    .inset_0()
}
