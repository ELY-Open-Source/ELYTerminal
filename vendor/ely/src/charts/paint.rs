use gpui::{
    Bounds, Entity, Hsla, IntoElement, PathBuilder, Pixels, Point, Styled, Window, canvas, fill,
    point, size,
};

use super::{
    geometry::{Geometry, Ink, Rect},
    series::tangents,
};
use crate::theme::Palette;

/// A place in a box's own pixels, moved to where the box sits.
pub(crate) fn at(origin: Point<Pixels>, (x, y): (f32, f32)) -> Point<Pixels> {
    origin + point(Pixels::from(x), Pixels::from(y))
}

/// A rectangle in a box's own pixels, moved to where the box sits.
pub(crate) fn place(origin: Point<Pixels>, rect: Rect) -> Bounds<Pixels> {
    Bounds::new(
        at(origin, (rect.x, rect.y)),
        size(Pixels::from(rect.w), Pixels::from(rect.h)),
    )
}

/// A canvas over its parent that keeps the parent's bounds in `state` and draws once more when they change.
pub(crate) fn measure<T: 'static>(
    state: Entity<T>,
    slot: fn(&mut T) -> &mut Bounds<Pixels>,
) -> impl IntoElement {
    canvas(
        move |bounds, window, cx| {
            let changed = state.update(cx, |state, cx| {
                let held = slot(state);
                let changed = *held != bounds;
                if changed {
                    *held = bounds;
                    cx.notify();
                }
                changed
            });
            if changed {
                window.request_animation_frame();
            }
        },
        |_, _, _, _| {},
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full()
}

/// The color series `ix` takes from the chart palette.
pub(crate) fn tint(colors: &Palette, ix: usize) -> Hsla {
    colors.chart[ix % colors.chart.len()]
}

impl Ink {
    pub(crate) fn color(self, colors: &Palette) -> Hsla {
        match self {
            Ink::Series(ix) => tint(colors, ix),
            Ink::Rise => colors.success,
            Ink::Fall => colors.danger,
            Ink::Rule => colors.fg_subtle,
            Ink::Strong => colors.fg,
        }
    }
}

/// How a geometry paints: its colors and measures in pixels, and whether lines curve.
#[derive(Clone)]
pub(crate) struct Pen {
    pub colors: Palette,
    pub stroke: Pixels,
    pub hairline: Pixels,
    pub corner: Pixels,
    pub smooth: bool,
    pub horizontal: bool,
}

/// Follows `points`, straight or through curves that never overshoot.
pub(crate) fn trace(
    path: &mut PathBuilder,
    points: &[(f32, f32)],
    smooth: bool,
    origin: Point<Pixels>,
) {
    let Some(first) = points.first() else {
        return;
    };
    path.move_to(at(origin, *first));
    if !smooth || points.len() < 3 {
        points[1..]
            .iter()
            .for_each(|next| path.line_to(at(origin, *next)));
        return;
    }
    let slopes = tangents(points);
    for ix in 0..points.len() - 1 {
        let ((x0, y0), (x1, y1)) = (points[ix], points[ix + 1]);
        let third = (x1 - x0) / 3.0;
        path.cubic_bezier_to(
            at(origin, (x1, y1)),
            at(origin, (x0 + third, y0 + slopes[ix] * third)),
            at(origin, (x1 - third, y1 - slopes[ix + 1] * third)),
        );
    }
}

/// Paints a path, or logs why it would not build.
pub(crate) fn finish(path: PathBuilder, color: Hsla, window: &mut Window) {
    match path.build() {
        Ok(path) => window.paint_path(path, color),
        Err(error) => log::error!("chart: a path failed to build: {error:#}"),
    }
}

/// A round dot filled with one color and edged with another.
pub(crate) fn ring(
    center: Point<Pixels>,
    radius: Pixels,
    (inside, edge): (Hsla, Hsla),
    width: Pixels,
    window: &mut Window,
) {
    let bounds = Bounds::new(
        center - point(radius, radius),
        size(radius * 2.0, radius * 2.0),
    );
    window.paint_quad(
        fill(bounds, inside)
            .corner_radii(radius)
            .border_widths(width)
            .border_color(edge),
    );
}

/// Paints a geometry laid out in its box's own pixels: gridlines, areas, lines, bars, shapes, strokes and dots.
pub(crate) fn paint(
    geometry: &Geometry,
    frame: Rect,
    pen: &Pen,
    origin: Point<Pixels>,
    window: &mut Window,
) {
    let colors = &pen.colors;
    let grid = colors.border.opacity(0.6);
    for (tick, _) in &geometry.ticks {
        let line = if pen.horizontal {
            Bounds::new(
                at(origin, (*tick, frame.y)),
                size(pen.hairline, Pixels::from(frame.h)),
            )
        } else {
            Bounds::new(
                at(origin, (frame.x, *tick)),
                size(Pixels::from(frame.w), pen.hairline),
            )
        };
        window.paint_quad(fill(line, grid));
    }
    for line in &geometry.lines {
        let Some(floor) = &line.floor else {
            continue;
        };
        let mut area = PathBuilder::fill();
        trace(&mut area, &line.points, pen.smooth, origin);
        floor
            .iter()
            .rev()
            .for_each(|place| area.line_to(at(origin, *place)));
        area.close();
        finish(area, line.ink.color(colors).opacity(0.14), window);
    }
    for line in geometry.lines.iter().filter(|line| line.points.len() > 1) {
        let mut path = PathBuilder::stroke(pen.stroke);
        trace(&mut path, &line.points, pen.smooth, origin);
        finish(path, line.ink.color(colors), window);
    }
    let seam = f32::from(pen.hairline);
    for (ink, rect) in &geometry.bars {
        let rect = if pen.horizontal {
            Rect {
                w: (rect.w - seam).max(0.0),
                ..*rect
            }
        } else {
            Rect {
                y: rect.y + seam,
                h: (rect.h - seam).max(0.0),
                ..*rect
            }
        };
        let corner = Pixels::from(f32::from(pen.corner).min(rect.w.min(rect.h) / 2.0));
        window.paint_quad(fill(place(origin, rect), ink.color(colors)).corner_radii(corner));
    }
    for (ink, outline) in geometry
        .shapes
        .iter()
        .filter(|(_, outline)| outline.len() > 2)
    {
        let mut shape = PathBuilder::fill();
        let mut edge = PathBuilder::stroke(pen.hairline);
        trace(&mut shape, outline, false, origin);
        trace(&mut edge, outline, false, origin);
        shape.close();
        edge.close();
        finish(shape, ink.color(colors).opacity(0.3), window);
        finish(edge, ink.color(colors), window);
    }
    for (ink, from, to) in &geometry.strokes {
        let width = if *ink == Ink::Rule {
            pen.hairline
        } else {
            pen.stroke
        };
        let mut path = PathBuilder::stroke(width);
        path.move_to(at(origin, *from));
        path.line_to(at(origin, *to));
        finish(path, ink.color(colors), window);
    }
    for dot in &geometry.dots {
        let color = dot.ink.color(colors);
        ring(
            at(origin, dot.center),
            Pixels::from(dot.radius),
            (color.opacity(0.6), color),
            pen.hairline,
            window,
        );
    }
}
