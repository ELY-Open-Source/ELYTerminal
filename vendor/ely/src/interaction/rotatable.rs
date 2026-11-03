use std::rc::Rc;

use gpui::{
    App, AppContext, Bounds, DragMoveEvent, ElementId, EmptyView, EntityId, InteractiveElement,
    IntoElement, ParentElement, Pixels, Point, RenderOnce, SharedString,
    StatefulInteractiveElement, Styled, Transformation, Window, div, radians, relative, svg,
};

use crate::{layout::seeded::use_seeded, primitives::FocusRing, theme::ActiveTheme};

/// What the arrows and Shift snap to, in degrees.
const NOTCH: f32 = 15.0;

struct Turn {
    owner: EntityId,
}

/// The angle, clockwise from straight up and in 0..360, at which `at` lies from the middle of `bounds`; `snap` rounds it to a notch.
fn angle_at(bounds: Bounds<Pixels>, at: Point<Pixels>, snap: bool) -> f32 {
    let middle = bounds.center();
    let (dx, dy) = (f32::from(at.x - middle.x), f32::from(at.y - middle.y));
    let angle = dx.atan2(-dy).to_degrees().rem_euclid(360.0);
    match snap {
        true => ((angle / NOTCH).round() * NOTCH).rem_euclid(360.0),
        false => angle,
    }
}

type OnTurn = Rc<dyn Fn(f32, &mut Window, &mut App)>;

/// An svg turned clockwise by degrees, with a knob.
#[derive(IntoElement)]
pub struct Rotatable {
    id: ElementId,
    path: SharedString,
    side: Pixels,
    angle: f32,
    on_turn: OnTurn,
}

impl Rotatable {
    /// `path` names the svg asset; `side` is the ring's width.
    pub fn new(
        id: impl Into<ElementId>,
        path: impl Into<SharedString>,
        side: Pixels,
        angle: f32,
        on_turn: impl Fn(f32, &mut Window, &mut App) + 'static,
    ) -> Self {
        assert!(
            angle.is_finite(),
            "rotatable: angle {angle} is not a number"
        );
        Self {
            id: id.into(),
            path: path.into(),
            side,
            angle: angle.rem_euclid(360.0),
            on_turn: Rc::new(on_turn),
        }
    }
}

impl RenderOnce for Rotatable {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id;
        let on_turn = self.on_turn;
        let held = use_seeded((id.clone(), "angle"), self.angle, window, cx);
        let (owner, angle) = (held.entity_id(), held.read(cx).value);
        let turn: OnTurn = Rc::new(move |next, window, cx| {
            log::info!("rotatable: {next:.0}°");
            held.update(cx, |held, cx| {
                held.value = next;
                cx.notify();
            });
            on_turn(next, window, cx);
        });
        let theme = cx.theme();
        let (side, knob) = (
            self.side,
            theme.interaction().grip.to_pixels(window.rem_size()),
        );
        let reach = (side - knob) / 2.0;
        let (sin, cos) = angle.to_radians().sin_cos();
        let (dragged, keyed) = (turn.clone(), turn);
        div()
            .id(id.clone())
            .relative()
            .flex()
            .flex_none()
            .items_center()
            .justify_center()
            .size(side)
            .rounded_full()
            .border_1()
            .border_color(theme.colors.border)
            .tab_index(0)
            .focus_ring(cx)
            .on_drag_move(move |event: &DragMoveEvent<Turn>, window, cx| {
                if event.drag(cx).owner != owner {
                    return;
                }
                let next = angle_at(
                    event.bounds,
                    event.event.position,
                    event.event.modifiers.shift,
                );
                if next != angle {
                    dragged(next, window, cx);
                }
            })
            .on_key_down(move |event, window, cx| {
                let by = match event.keystroke.key.as_str() {
                    "left" => -NOTCH,
                    "right" => NOTCH,
                    _ => return,
                };
                cx.stop_propagation();
                keyed(
                    ((angle / NOTCH).round() * NOTCH + by).rem_euclid(360.0),
                    window,
                    cx,
                );
            })
            .child(
                svg()
                    .path(self.path)
                    .size(side * 0.5)
                    .text_color(theme.colors.fg)
                    .with_transformation(Transformation::rotate(radians(angle.to_radians()))),
            )
            .child(
                div()
                    .absolute()
                    .left(relative(0.5))
                    .top(relative(0.5))
                    .ml(reach * sin)
                    .mt(-(reach * cos))
                    .size_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        div()
                            .id((id, "knob"))
                            .flex_none()
                            .size(knob)
                            .rounded_full()
                            .bg(theme.colors.accent)
                            .cursor(gpui::CursorStyle::OpenHand)
                            .on_drag(Turn { owner }, |_, _, _, cx| cx.new(|_| EmptyView)),
                    ),
            )
    }
}

#[cfg(test)]
mod tests {
    use gpui::{Bounds, point, px, size};

    use super::angle_at;

    #[test]
    fn an_angle_runs_clockwise_from_straight_up_and_snaps_to_a_notch() {
        let ring = Bounds::new(point(px(0.0), px(0.0)), size(px(100.0), px(100.0)));
        assert_eq!(angle_at(ring, point(px(50.0), px(0.0)), false), 0.0);
        assert_eq!(angle_at(ring, point(px(100.0), px(50.0)), false), 90.0);
        assert_eq!(angle_at(ring, point(px(50.0), px(100.0)), false), 180.0);
        assert_eq!(angle_at(ring, point(px(0.0), px(50.0)), false), 270.0);
        let eleven = point(
            px(50.0 + 50.0 * 11f32.to_radians().sin()),
            px(50.0 - 50.0 * 11f32.to_radians().cos()),
        );
        assert!(
            (angle_at(ring, eleven, false) - 11.0).abs() < 1e-3,
            "free, it stays at 11°"
        );
        let near = angle_at(ring, point(px(60.0), px(0.0)), true);
        assert_eq!(near, 15.0, "11° snaps to the nearest notch");
    }
}
