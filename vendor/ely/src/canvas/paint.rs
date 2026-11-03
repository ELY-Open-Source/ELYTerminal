use gpui::{Bounds, Hsla, PathBuilder, Pixels, Point, Window, fill, point, size};

use super::{
    shape::{Shape, ShapeKind, polygon},
    view::{Frame, Viewport},
};
use crate::theme::Palette;

/// The view pixels of a canvas frame.
pub(crate) fn in_view(view: &Viewport, frame: &Frame) -> Frame {
    let (x, y) = view.to_view((frame.x, frame.y));
    Frame::new(x, y, frame.w * view.zoom, frame.h * view.zoom)
}

pub(crate) fn at(origin: Point<Pixels>, (x, y): (f32, f32)) -> Point<Pixels> {
    origin + point(Pixels::from(x), Pixels::from(y))
}

pub(crate) fn quad(origin: Point<Pixels>, frame: &Frame) -> Bounds<Pixels> {
    Bounds::new(
        at(origin, (frame.x, frame.y)),
        size(Pixels::from(frame.w), Pixels::from(frame.h)),
    )
}

pub(crate) fn outline(
    points: &[(f32, f32)],
    origin: Point<Pixels>,
    width: Pixels,
    closed: bool,
) -> PathBuilder {
    let mut path = PathBuilder::stroke(width);
    if let Some(first) = points.first() {
        path.move_to(at(origin, *first));
        points[1..]
            .iter()
            .for_each(|next| path.line_to(at(origin, *next)));
        if closed {
            path.close();
        }
    }
    path
}

pub(crate) fn solid(points: &[(f32, f32)], origin: Point<Pixels>) -> PathBuilder {
    let mut path = PathBuilder::fill();
    if let Some(first) = points.first() {
        path.move_to(at(origin, *first));
        points[1..]
            .iter()
            .for_each(|next| path.line_to(at(origin, *next)));
        path.close();
    }
    path
}

pub(crate) fn finish(path: PathBuilder, color: Hsla, window: &mut Window) {
    match path.build() {
        Ok(path) => window.paint_path(path, color),
        Err(error) => log::error!("canvas: a path failed to build: {error:#}"),
    }
}

/// Paints a shape's outline and wash in view pixels.
pub(crate) fn paint_shape(
    shape: &Shape,
    view: &Viewport,
    origin: Point<Pixels>,
    palette: &Palette,
    stroke: Pixels,
    window: &mut Window,
) {
    let ink = palette.hue(shape.hue, format_args!("shape {}", shape.key));
    let wash = ink.alpha(0.14);
    let frame = in_view(view, &shape.frame);
    match &shape.kind {
        ShapeKind::Rect => {
            window.paint_quad(fill(quad(origin, &frame), wash));
            let corners = [
                (frame.x, frame.y),
                (frame.right(), frame.y),
                (frame.right(), frame.bottom()),
                (frame.x, frame.bottom()),
            ];
            finish(outline(&corners, origin, stroke, true), ink, window);
        }
        ShapeKind::Ellipse | ShapeKind::Polygon(_) => {
            let sides = match shape.kind {
                ShapeKind::Polygon(sides) => sides,
                _ => 64,
            };
            let corners = polygon(&frame, sides);
            finish(solid(&corners, origin), wash, window);
            finish(outline(&corners, origin, stroke, true), ink, window);
        }
        ShapeKind::Line(end) | ShapeKind::Arrow(end) => {
            let (from, to) = (end.opposite().of(&frame), end.of(&frame));
            finish(outline(&[from, to], origin, stroke, false), ink, window);
            if matches!(shape.kind, ShapeKind::Arrow(_)) {
                finish(solid(&head(from, to, stroke), origin), ink, window);
            }
        }
        ShapeKind::Path { points, width } => {
            let points: Vec<(f32, f32)> = points
                .iter()
                .map(|(x, y)| view.to_view((shape.frame.x + x, shape.frame.y + y)))
                .collect();
            let wide = Pixels::from(width * view.zoom).max(stroke);
            finish(outline(&points, origin, wide, false), ink, window);
        }
        ShapeKind::Text(_) | ShapeKind::Note(_) => {}
    }
}

/// An arrow's head at `to`, coming from `from`: its tip and its two back corners.
pub(crate) fn head(from: (f32, f32), to: (f32, f32), stroke: Pixels) -> [(f32, f32); 3] {
    let (dx, dy) = (to.0 - from.0, to.1 - from.1);
    let length = (dx * dx + dy * dy).sqrt().max(1.0);
    let (ux, uy) = (dx / length, dy / length);
    let reach = f32::from(stroke) * 5.0;
    let back = (to.0 - ux * reach, to.1 - uy * reach);
    [
        to,
        (back.0 - uy * reach / 2.0, back.1 + ux * reach / 2.0),
        (back.0 + uy * reach / 2.0, back.1 - ux * reach / 2.0),
    ]
}

/// A curve from `from` to `to` that leaves and arrives level, its handles `pull` along the way it runs.
pub(crate) fn wire(
    from: (f32, f32),
    to: (f32, f32),
    pull: f32,
    origin: Point<Pixels>,
    width: Pixels,
) -> PathBuilder {
    let mut path = PathBuilder::stroke(width);
    path.move_to(at(origin, from));
    path.cubic_bezier_to(
        at(origin, to),
        at(origin, (from.0 + pull, from.1)),
        at(origin, (to.0 - pull, to.1)),
    );
    path
}
