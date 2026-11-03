use std::{cell::Cell, f32::consts::PI, rc::Rc};

use gpui::{
    App, DragMoveEvent, ElementId, EmptyView, EntityId, InteractiveElement, IntoElement,
    ParentElement, Path, PathBuilder, Pixels, Point, RenderOnce, StatefulInteractiveElement,
    Styled, Window, canvas, div, point, prelude::*,
};

use super::{
    options::OnNumber,
    slider::{fraction, keyed, value_at},
};
use crate::{
    primitives::tab_stop,
    theme::{ActiveTheme, Elevation},
};

/// Where the dial starts, at lower left, and how far it turns.
const START: f32 = 0.75 * PI;
const SWEEP: f32 = 1.5 * PI;

fn at_angle(center: Point<Pixels>, radius: Pixels, angle: f32) -> Point<Pixels> {
    point(
        center.x + radius * angle.cos(),
        center.y + radius * angle.sin(),
    )
}

/// A stroked arc clockwise from `from` to `to`, in radians.
fn arc(center: Point<Pixels>, radius: Pixels, from: f32, to: f32, width: Pixels) -> Path<Pixels> {
    let mut path = PathBuilder::stroke(width);
    path.move_to(at_angle(center, radius, from));
    path.arc_to(
        point(radius, radius),
        Pixels::ZERO,
        to - from > PI,
        true,
        at_angle(center, radius, to),
    );
    path.build().expect("an arc is a simple open path")
}

struct Turn {
    owner: EntityId,
    start: Rc<Cell<Option<(Pixels, f64)>>>,
}

/// A dial for audio-style values. Drag up or down, or use the arrow keys.
#[derive(IntoElement)]
pub struct Knob {
    id: ElementId,
    value: f64,
    range: (f64, f64, f64),
    disabled: bool,
    on_change: Option<OnNumber>,
}

impl Knob {
    pub fn new(id: impl Into<ElementId>, value: f64) -> Self {
        Self {
            id: id.into(),
            value,
            range: (0.0, 100.0, 1.0),
            disabled: false,
            on_change: None,
        }
    }

    pub fn range(mut self, min: f64, max: f64) -> Self {
        assert!(min < max, "knob range {min}..{max} is empty");
        self.range.0 = min;
        self.range.1 = max;
        self
    }

    pub fn step(mut self, step: f64) -> Self {
        assert!(step > 0.0, "knob step {step} must be positive");
        self.range.2 = step;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn on_change(mut self, handler: impl Fn(f64, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for Knob {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let ((min, max, step), value) = (self.range, self.value);
        let focus = tab_stop(
            (self.id.clone(), "focus").into(),
            !self.disabled,
            window,
            cx,
        );
        let focused = focus.is_focused(window);
        let owner = window
            .use_keyed_state((self.id.clone(), "drag"), cx, |_, _| ())
            .entity_id();
        let theme = cx.theme();
        let colors = &theme.colors;
        let (diameter, width) = theme.knob();
        let (travel, stroke) = (
            diameter.to_pixels(window.rem_size()) * 4.0,
            width.to_pixels(window.rem_size()),
        );
        let turned = fraction(value, min, max);
        let (rail, ink, tick) = (colors.border, colors.accent, colors.fg);
        let commit = {
            let (id, on_change) = (self.id.clone(), self.on_change);
            Rc::new(move |next: f64, window: &mut Window, cx: &mut App| {
                if next == value {
                    return;
                }
                log::debug!("knob {id:?}: {next}");
                if let Some(on_change) = &on_change {
                    on_change(next, window, cx);
                }
            })
        };
        let (drag, keys) = (commit.clone(), commit);
        div()
            .id(self.id)
            .track_focus(&focus)
            .relative()
            .flex_none()
            .size(diameter)
            .rounded_full()
            .bg(colors.surface)
            .border_1()
            .border_color(if focused { colors.focus } else { colors.border })
            .shadow(theme.elevation(Elevation::Raised))
            .when(self.disabled, |knob| knob.opacity(0.5))
            .child(
                canvas(
                    |_, _, _| {},
                    move |bounds, _, window, _| {
                        let center = bounds.center();
                        let radius = bounds.size.width / 2.0 - stroke * 1.5;
                        window.paint_path(arc(center, radius, START, START + SWEEP, stroke), rail);
                        if turned > 0.0 {
                            let to = START + SWEEP * turned;
                            window.paint_path(arc(center, radius, START, to, stroke), ink);
                        }
                        let angle = START + SWEEP * turned;
                        let mut line = PathBuilder::stroke(stroke);
                        line.move_to(at_angle(center, radius * 0.3, angle));
                        line.line_to(at_angle(center, radius * 0.62, angle));
                        window.paint_path(line.build().expect("a tick is a straight line"), tick);
                    },
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            )
            .when(!self.disabled, |knob| {
                knob.cursor_ns_resize()
                    .on_drag(
                        Turn {
                            owner,
                            start: Rc::new(Cell::new(None)),
                        },
                        |turn, _, _, cx| {
                            turn.start.set(None);
                            cx.new(|_| EmptyView)
                        },
                    )
                    .on_drag_move(move |event: &DragMoveEvent<Turn>, window, cx| {
                        let turn = event.drag(cx);
                        if turn.owner != owner {
                            return;
                        }
                        let (start, y) = (turn.start.clone(), event.event.position.y);
                        let (anchor, from) = start.get().unwrap_or_else(|| {
                            start.set(Some((y, value)));
                            (y, value)
                        });
                        let moved = (anchor - y) / travel;
                        drag(
                            value_at(fraction(from, min, max) + moved, min, max, step),
                            window,
                            cx,
                        );
                    })
                    .on_key_down(move |event, window, cx| {
                        if let Some(next) = keyed(&event.keystroke.key, value, min, max, step) {
                            cx.stop_propagation();
                            keys(next, window, cx);
                        }
                    })
            })
    }
}
