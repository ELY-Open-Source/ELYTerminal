use std::rc::Rc;

use gpui::{
    App, Bounds, Corners, Div, DragMoveEvent, ElementId, EmptyView, Entity, EntityId, Hsla,
    InteractiveElement, IntoElement, MouseButton, ParentElement, Path, PathBuilder, Pixels, Point,
    RenderOnce, Stateful, StatefulInteractiveElement, Styled, Window, canvas, div, fill, point,
    prelude::*, size,
};

use crate::{
    buttons::{Button, ButtonVariant},
    i18n,
    layout::seeded::use_seeded,
    theme::{ActiveTheme, ControlSize, Radius, TextSize},
};

/// A pen stroke: points from the pad's top left corner.
pub type Stroke = Vec<Point<Pixels>>;

type OnStrokes = Rc<dyn Fn(&[Stroke], &mut Window, &mut App)>;
type Lift = Rc<dyn Fn(Stroke, &mut Window, &mut App)>;

struct Pen {
    owner: EntityId,
}

/// Where a pad sits, and the stroke under the pen.
#[derive(Default)]
pub(crate) struct Drawing {
    pub bounds: Bounds<Pixels>,
    pub stroke: Stroke,
}

impl Drawing {
    /// Adds `at` to the stroke under the pen, unless the pen has not moved.
    fn reach(&mut self, at: Point<Pixels>) {
        let at = local(self.bounds, at);
        if self.stroke.last() != Some(&at) {
            self.stroke.push(at);
        }
    }
}

/// `at` from the pad's corner, kept inside the pad.
fn local(bounds: Bounds<Pixels>, at: Point<Pixels>) -> Point<Pixels> {
    point(
        (at.x - bounds.left())
            .max(Pixels::ZERO)
            .min(bounds.size.width),
        (at.y - bounds.top())
            .max(Pixels::ZERO)
            .min(bounds.size.height),
    )
}

/// A line through `points`, bent through their midpoints so it reads smooth.
pub(crate) fn ink(
    points: &[Point<Pixels>],
    origin: Point<Pixels>,
    width: Pixels,
) -> Option<Path<Pixels>> {
    let [first, .., last] = points else {
        return None;
    };
    let at = |p: Point<Pixels>| point(origin.x + p.x, origin.y + p.y);
    let mut path = PathBuilder::stroke(width);
    path.move_to(at(*first));
    for pair in points.windows(2) {
        let middle = point((pair[0].x + pair[1].x) / 2.0, (pair[0].y + pair[1].y) / 2.0);
        path.curve_to(at(middle), at(pair[0]));
    }
    path.line_to(at(*last));
    Some(path.build().expect("a stroke is a simple open path"))
}

/// Turns a press and drag on `pad` into a stroke of points from its corner, kept inside it; `lift` gets the stroke as the pen leaves. The pad's owner keeps `drawing`'s bounds.
pub(crate) fn pen(
    pad: Stateful<Div>,
    drawing: &Entity<Drawing>,
    lift: impl Fn(Stroke, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    let owner = drawing.entity_id();
    let lift: Lift = Rc::new(lift);
    let finish = {
        let drawing = drawing.clone();
        move |at: Point<Pixels>, window: &mut Window, cx: &mut App| {
            let stroke = drawing.update(cx, |drawing, cx| {
                if !drawing.stroke.is_empty() {
                    drawing.reach(at);
                }
                cx.notify();
                std::mem::take(&mut drawing.stroke)
            });
            if !stroke.is_empty() {
                lift(stroke, window, cx);
            }
        }
    };
    let (press, start, moving) = (drawing.clone(), drawing.clone(), drawing.clone());
    let up = Rc::new(finish);
    let leave = up.clone();
    pad.on_mouse_down(MouseButton::Left, move |event, _, cx| {
        press.update(cx, |drawing, cx| {
            drawing.stroke = vec![local(drawing.bounds, event.position)];
            cx.notify();
        })
    })
    .on_drag(Pen { owner }, move |_, _, window, cx| {
        let at = window.mouse_position();
        start.update(cx, |drawing, cx| {
            drawing.reach(at);
            cx.notify();
        });
        cx.new(|_| EmptyView)
    })
    .on_drag_move(move |event: &DragMoveEvent<Pen>, _, cx| {
        if event.drag(cx).owner != owner {
            return;
        }
        moving.update(cx, |drawing, cx| {
            drawing.reach(event.event.position);
            cx.notify();
        })
    })
    .on_mouse_up(MouseButton::Left, move |event, window, cx| {
        up(event.position, window, cx)
    })
    .on_mouse_up_out(MouseButton::Left, move |event, window, cx| {
        leave(event.position, window, cx)
    })
}

fn paint(
    strokes: &[Stroke],
    bounds: Bounds<Pixels>,
    width: Pixels,
    color: Hsla,
    window: &mut Window,
) {
    let dot = |at: Point<Pixels>| {
        let center = point(bounds.left() + at.x, bounds.top() + at.y);
        fill(Bounds::centered_at(center, size(width, width)), color)
            .corner_radii(Corners::all(width / 2.0))
    };
    for stroke in strokes {
        if let Some(path) = ink(stroke, bounds.origin, width) {
            window.paint_path(path, color);
        }
        for end in [stroke.first(), stroke.last()].into_iter().flatten() {
            window.paint_quad(dot(*end));
        }
    }
}

/// A pad to sign with the pointer. A stroke lands when the pen lifts; Clear wipes them all.
#[derive(IntoElement)]
pub struct SignaturePad {
    id: ElementId,
    strokes: Vec<Stroke>,
    on_change: Option<OnStrokes>,
}

impl SignaturePad {
    pub fn new(id: impl Into<ElementId>, strokes: impl IntoIterator<Item = Stroke>) -> Self {
        Self {
            id: id.into(),
            strokes: strokes.into_iter().collect(),
            on_change: None,
        }
    }

    pub fn on_change(
        mut self,
        handler: impl Fn(&[Stroke], &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for SignaturePad {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let strokes = use_seeded((self.id.clone(), "strokes"), self.strokes, window, cx);
        let drawing =
            window.use_keyed_state((self.id.clone(), "drawing"), cx, |_, _| Drawing::default());
        let set: OnStrokes = {
            let (id, strokes, on_change) = (self.id.clone(), strokes.clone(), self.on_change);
            Rc::new(move |next, window, cx| {
                log::info!("signature pad {id:?}: {} strokes", next.len());
                strokes.update(cx, |strokes, cx| {
                    strokes.value = next.to_vec();
                    cx.notify();
                });
                if let Some(on_change) = &on_change {
                    on_change(next, window, cx);
                }
            })
        };
        let lift = {
            let (strokes, set) = (strokes.clone(), set.clone());
            move |stroke: Stroke, window: &mut Window, cx: &mut App| {
                let mut next = strokes.read(cx).value.clone();
                next.push(stroke);
                set(&next, window, cx);
            }
        };
        let theme = cx.theme();
        let colors = &theme.colors;
        let (height, width) = theme.signature();
        let width = width.to_pixels(window.rem_size());
        let empty = strokes.read(cx).value.is_empty();
        let mut shown = strokes.read(cx).value.clone();
        shown.push(drawing.read(cx).stroke.clone());
        let measure = drawing.clone();
        let color = colors.fg;
        let pad = div()
            .id(self.id.clone())
            .relative()
            .w_full()
            .h(height)
            .rounded(theme.radius(Radius::Lg))
            .border_1()
            .border_color(colors.border_strong)
            .bg(colors.surface)
            .cursor_crosshair()
            .child(
                div()
                    .absolute()
                    .left_6()
                    .right_6()
                    .bottom_8()
                    .flex()
                    .items_end()
                    .gap_2()
                    .pb_1()
                    .border_b_1()
                    .border_dashed()
                    .border_color(colors.border_strong)
                    .text_size(theme.text_size(TextSize::Sm))
                    .text_color(colors.fg_subtle)
                    .child("×")
                    .when(empty, |line| {
                        line.child(i18n::text(cx, "signature.here", &[]))
                    }),
            )
            .child(
                canvas(
                    move |bounds, _, cx| {
                        if measure.read(cx).bounds != bounds {
                            measure.update(cx, |drawing, _| drawing.bounds = bounds);
                        }
                    },
                    move |bounds, _, window, _| paint(&shown, bounds, width, color, window),
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            );
        let pad = pen(pad, &drawing, lift);
        div()
            .flex()
            .flex_col()
            .items_end()
            .gap_1()
            .child(pad)
            .child(
                Button::new((self.id, "clear"), i18n::text(cx, "signature.clear", &[]))
                    .size(ControlSize::Sm)
                    .variant(ButtonVariant::Ghost)
                    .disabled(empty)
                    .on_click(move |_, window, cx| set(&[], window, cx)),
            )
    }
}

#[cfg(test)]
mod tests {
    use gpui::{Bounds, point, px, size};

    use super::{ink, local};

    #[test]
    fn points_stay_inside_the_pad() {
        let bounds = Bounds::new(point(px(10.0), px(20.0)), size(px(100.0), px(50.0)));
        assert_eq!(
            local(bounds, point(px(30.0), px(40.0))),
            point(px(20.0), px(20.0))
        );
        assert_eq!(
            local(bounds, point(px(-5.0), px(90.0))),
            point(px(0.0), px(50.0))
        );
    }

    #[test]
    fn a_stroke_needs_two_points_for_a_line() {
        let origin = point(px(0.0), px(0.0));
        assert!(ink(&[point(px(1.0), px(1.0))], origin, px(2.0)).is_none());
        let line = [
            point(px(1.0), px(1.0)),
            point(px(9.0), px(4.0)),
            point(px(12.0), px(9.0)),
        ];
        assert!(ink(&line, origin, px(2.0)).is_some());
    }
}
