use std::rc::Rc;

use gpui::{
    App, Bounds, Div, Entity, HoverListenerMode, InteractiveElement, IntoElement, MouseButton,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, PathBuilder, Pixels, Point, Stateful,
    StatefulInteractiveElement, Styled, Window, canvas, fill, prelude::*, size,
};

use super::{
    geometry::{Dot, Geometry, Ink, Rect},
    paint::{Pen, at, finish, paint, place, ring},
    plot::{Pick, Plot, State},
    scale::Band,
};

/// What the pointer at a place picks: a category's band, the nearest category, or the nearest dot within reach.
pub(crate) fn pick_at(
    pick: Pick,
    place: (f32, f32),
    frame: Rect,
    horizontal: bool,
    span: (usize, usize),
    dots: &[Dot],
    reach: f32,
) -> Option<usize> {
    if !frame.contains(place) {
        return None;
    }
    let count = span.1 - span.0 + 1;
    let (along, start, length) = if horizontal {
        (place.1, frame.y, frame.h)
    } else {
        (place.0, frame.x, frame.w)
    };
    match pick {
        Pick::Band => Band {
            count,
            range: (start, start + length),
            padding: 0.0,
        }
        .index(along)
        .map(|ix| ix + span.0),
        Pick::Point if count < 2 => Some(span.0),
        Pick::Point => {
            let ix = ((along - start) / (length / (count - 1) as f32)).round() as usize;
            Some(ix.min(count - 1) + span.0)
        }
        Pick::Dot => dots
            .iter()
            .map(|dot| (dot, (dot.center.0 - place.0).hypot(dot.center.1 - place.1)))
            .filter(|(dot, far)| *far <= dot.radius.max(reach))
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(dot, _)| dot.key),
    }
}

/// What the picked index lights: its band, its points on each line, or its dot.
pub(crate) enum Focus {
    Band(Rect),
    Line(f32, Vec<(Ink, (f32, f32))>),
    Dot(Dot),
}

/// What the picked index lights: its band, its points on each line, or its dot.
pub(crate) fn focus(
    plot: &Plot,
    geometry: &Geometry,
    hover: Option<usize>,
    (rect, span): (Rect, (usize, usize)),
) -> Option<Focus> {
    let ix = hover?;
    match plot.pick {
        Pick::Dot => geometry
            .dots
            .iter()
            .find(|dot| dot.key == ix)
            .map(|dot| Focus::Dot(*dot)),
        Pick::Band => {
            let range = if plot.horizontal {
                (rect.y, rect.y + rect.h)
            } else {
                (rect.x, rect.x + rect.w)
            };
            let lanes = Band {
                count: span.1 - span.0 + 1,
                range,
                padding: 0.0,
            };
            let (start, width) = lanes.slot(ix - span.0);
            Some(Focus::Band(if plot.horizontal {
                Rect {
                    x: rect.x,
                    y: start,
                    w: rect.w,
                    h: width,
                }
            } else {
                Rect {
                    x: start,
                    y: rect.y,
                    w: width,
                    h: rect.h,
                }
            }))
        }
        Pick::Point => {
            let x = geometry.labels.get(ix - span.0)?.0;
            let points = geometry
                .lines
                .iter()
                .filter_map(|line| Some((line.ink, *line.points.get(ix - span.0)?)))
                .collect();
            Some(Focus::Line(x, points))
        }
    }
}

/// The plot's canvas: the band under the pointer, the marks, dashed rules and note lines, the crosshair or the lit dot, and a brush.
pub(crate) fn drawing(
    geometry: Rc<Geometry>,
    pen: Pen,
    rect: Rect,
    (focus, brush): (Option<Focus>, Option<(f32, f32)>),
    (rules, notes): (Vec<f32>, Vec<f32>),
) -> impl IntoElement {
    canvas(
        |_, _, _| {},
        move |bounds, _, window, _| {
            let origin = bounds.origin;
            let colors = &pen.colors;
            if let Some(Focus::Band(band)) = &focus {
                window.paint_quad(fill(place(origin, *band), colors.hover));
            }
            paint(&geometry, rect, &pen, origin, window);
            let dash = [pen.stroke * 2.0, pen.stroke * 2.0];
            for level in &rules {
                let mut rule = PathBuilder::stroke(pen.hairline).dash_array(&dash);
                let (from, to) = if pen.horizontal {
                    ((*level, rect.y), (*level, rect.y + rect.h))
                } else {
                    ((rect.x, *level), (rect.x + rect.w, *level))
                };
                rule.move_to(at(origin, from));
                rule.line_to(at(origin, to));
                finish(rule, colors.fg_muted, window);
            }
            let upright = |x: f32| {
                Bounds::new(
                    at(origin, (x, rect.y)),
                    size(pen.hairline, Pixels::from(rect.h)),
                )
            };
            for x in &notes {
                window.paint_quad(fill(upright(*x), colors.fg_subtle.opacity(0.6)));
            }
            match &focus {
                Some(Focus::Line(x, points)) => {
                    window.paint_quad(fill(upright(*x), colors.border_strong));
                    for (ink, point) in points {
                        let ends = (colors.bg, ink.color(colors));
                        ring(
                            at(origin, *point),
                            pen.stroke * 2.0,
                            ends,
                            pen.stroke,
                            window,
                        );
                    }
                }
                Some(Focus::Dot(dot)) => {
                    let ends = (dot.ink.color(colors), colors.bg);
                    ring(
                        at(origin, dot.center),
                        Pixels::from(dot.radius),
                        ends,
                        pen.stroke,
                        window,
                    );
                }
                _ => {}
            }
            if let Some((a, b)) = brush {
                let band = Rect {
                    x: a.min(b),
                    y: rect.y,
                    w: (a - b).abs(),
                    h: rect.h,
                };
                window.paint_quad(fill(place(origin, band), colors.selection));
            }
        },
    )
    .absolute()
    .inset_0()
}

/// A place in window pixels as the plot's own.
fn local(position: Point<Pixels>, bounds: Bounds<Pixels>) -> (f32, f32) {
    let offset = position - bounds.origin;
    (f32::from(offset.x), f32::from(offset.y))
}

/// Wires the pointer: hover picks, and on a zoomable plot a drag brushes a span and a double press shows all.
pub(crate) fn answer(
    root: Stateful<Div>,
    state: &Entity<State>,
    (pick, horizontal, zoomable): (Pick, bool, bool),
    (rect, span, reach): (Rect, (usize, usize), f32),
    geometry: Rc<Geometry>,
) -> Stateful<Div> {
    let (moved, left, pressed, released) =
        (state.clone(), state.clone(), state.clone(), state.clone());
    let end_brush = move |_: &MouseUpEvent, _: &mut Window, cx: &mut App| {
        released.update(cx, |state, cx| {
            let Some((a, b)) = state.brush.take() else {
                return;
            };
            let middle = rect.y + rect.h / 2.0;
            let end = |x: f32| {
                let place = (x.clamp(rect.x, rect.x + rect.w), middle);
                pick_at(pick, place, rect, false, span, &[], 0.0)
            };
            if let (Some(from), Some(to)) = (end(a.min(b)), end(a.max(b)))
                && to > from
            {
                state.span = Some((from, to));
                log::info!("chart: zoomed to {from}..={to}");
            }
            cx.notify();
        })
    };
    root.on_mouse_move(move |event: &MouseMoveEvent, _, cx| {
        let place = local(event.position, moved.read(cx).bounds);
        moved.update(cx, |state, cx| {
            if event.pressed_button == Some(MouseButton::Left)
                && let Some(brush) = state.brush.as_mut()
            {
                brush.1 = place.0.clamp(rect.x, rect.x + rect.w);
                cx.notify();
                return;
            }
            let next = pick_at(pick, place, rect, horizontal, span, &geometry.dots, reach);
            if state.hover != next || (next.is_some() && pick != Pick::Dot) {
                state.hover = next;
                state.pointer = place;
                cx.notify();
            }
        })
    })
    .hover_listener_mode(HoverListenerMode::InputModalityIndependent)
    .on_hover(move |inside, _, cx| {
        if !*inside {
            left.update(cx, |state, cx| {
                state.hover = None;
                cx.notify();
            })
        }
    })
    .when(zoomable, |root| {
        root.cursor_crosshair()
            .on_mouse_down(MouseButton::Left, move |event: &MouseDownEvent, _, cx| {
                let place = local(event.position, pressed.read(cx).bounds);
                pressed.update(cx, |state, cx| {
                    if event.click_count == 2 {
                        state.span = None;
                        log::info!("chart: zoom reset");
                    } else if rect.contains(place) {
                        state.brush = Some((place.0, place.0));
                    }
                    cx.notify();
                })
            })
            .on_mouse_up(MouseButton::Left, end_brush.clone())
            .on_mouse_up_out(MouseButton::Left, end_brush)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_pointer_picks_bands_points_and_dots() {
        let frame = Rect {
            x: 0.0,
            y: 0.0,
            w: 100.0,
            h: 50.0,
        };
        assert_eq!(
            pick_at(Pick::Band, (30.0, 10.0), frame, false, (0, 3), &[], 0.0),
            Some(1)
        );
        assert_eq!(
            pick_at(Pick::Point, (40.0, 10.0), frame, false, (2, 4), &[], 0.0),
            Some(3),
            "the nearest of three, past the zoomed start"
        );
        assert_eq!(
            pick_at(Pick::Band, (30.0, 60.0), frame, false, (0, 3), &[], 0.0),
            None,
            "below the frame"
        );
        assert_eq!(
            pick_at(Pick::Band, (10.0, 30.0), frame, true, (0, 4), &[], 0.0),
            Some(3),
            "sideways bands run down"
        );
        let dots = [Dot {
            ink: Ink::Series(0),
            center: (10.0, 10.0),
            radius: 2.0,
            key: 7,
        }];
        assert_eq!(
            pick_at(Pick::Dot, (14.0, 10.0), frame, false, (0, 0), &dots, 6.0),
            Some(7)
        );
        assert_eq!(
            pick_at(Pick::Dot, (30.0, 10.0), frame, false, (0, 0), &dots, 6.0),
            None,
            "out of reach"
        );
    }
}
