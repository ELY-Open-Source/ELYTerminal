use std::rc::Rc;

use gpui::{
    App, ElementId, Hsla, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window,
    div, prelude::*, relative,
};

use super::{
    panel::{OnEdit, caption, editing},
    view::Frame,
};
use crate::{
    forms::{Choice, Select},
    theme::{ActiveTheme, Radius},
};

/// How a shape keeps to its parent along one axis when the parent resizes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pin {
    /// To the left or top edge.
    Start,
    /// To the right or bottom edge.
    End,
    /// To both edges, stretching.
    Both,
    /// To the middle.
    Center,
    /// Growing with the parent.
    Scale,
}

impl Pin {
    pub const ALL: [Pin; 5] = [Pin::Start, Pin::End, Pin::Both, Pin::Center, Pin::Scale];

    fn value(self) -> &'static str {
        match self {
            Pin::Start => "start",
            Pin::End => "end",
            Pin::Both => "both",
            Pin::Center => "center",
            Pin::Scale => "scale",
        }
    }

    fn words(self, across: bool) -> &'static str {
        match (self, across) {
            (Pin::Start, true) => "Left",
            (Pin::Start, false) => "Top",
            (Pin::End, true) => "Right",
            (Pin::End, false) => "Bottom",
            (Pin::Both, true) => "Left and right",
            (Pin::Both, false) => "Top and bottom",
            (Pin::Center, _) => "Center",
            (Pin::Scale, _) => "Scale",
        }
    }

    /// Where a span `at` and `len` long inside a parent from `from`, `size` long, lands once the parent spans `to` and `now`.
    pub fn follow(
        self,
        (at, len): (f32, f32),
        (from, size): (f32, f32),
        (to, now): (f32, f32),
    ) -> (f32, f32) {
        let (before, after) = (at - from, from + size - at - len);
        match self {
            Pin::Start => (to + before, len),
            Pin::End => (to + now - after - len, len),
            Pin::Both => (to + before, (now - before - after).max(0.0)),
            Pin::Center => (to + (now - size) / 2.0 + before, len),
            Pin::Scale => {
                assert!(size > 0.0, "a parent {size} long cannot scale");
                let grown = now / size;
                (to + before * grown, len * grown)
            }
        }
    }
}

/// How a shape keeps to its parent across and down.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Constraints {
    pub across: Pin,
    pub down: Pin,
}

impl Constraints {
    /// Where `child` lands once its parent goes from `before` to `after`.
    pub fn placed(self, child: Frame, before: Frame, after: Frame) -> Frame {
        let (x, w) =
            self.across
                .follow((child.x, child.w), (before.x, before.w), (after.x, after.w));
        let (y, h) = self
            .down
            .follow((child.y, child.h), (before.y, before.h), (after.y, after.h));
        Frame::new(x, y, w, h)
    }
}

/// A shape's constraints: a picture of the pins beside a choice for each axis.
#[derive(IntoElement)]
pub struct ConstraintsEditor {
    id: ElementId,
    constraints: Constraints,
    on_change: OnEdit<Constraints>,
}

impl ConstraintsEditor {
    pub fn new(
        id: impl Into<ElementId>,
        constraints: Constraints,
        on_change: impl Fn(Constraints, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            constraints,
            on_change: Rc::new(on_change),
        }
    }
}

/// A hairline from `x`, `y` in shares of its box, `len` long across or down.
fn hairline(x: f32, y: f32, len: f32, across: bool, color: Hsla) -> gpui::Div {
    let line = div().absolute().left(relative(x)).top(relative(y));
    if across {
        line.w(relative(len)).border_t_1().border_color(color)
    } else {
        line.h(relative(len)).border_l_1().border_color(color)
    }
}

impl RenderOnce for ConstraintsEditor {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let on_change = self.on_change;
        let (id, now, on_change) = (self.id, self.constraints, &on_change);
        let theme = cx.theme();
        let (lit, rest) = (theme.colors.accent, theme.colors.border);
        let pin = |on: bool| if on { lit } else { rest };
        let (h, v) = (now.across, now.down);
        let picture = div()
            .relative()
            .flex_none()
            .size(theme.canvas().pins)
            .rounded(theme.radius(Radius::Md))
            .border_1()
            .border_color(theme.colors.border)
            .child(
                div()
                    .absolute()
                    .left(relative(0.35))
                    .top(relative(0.35))
                    .size(relative(0.3))
                    .border_1()
                    .border_color(theme.colors.fg_subtle),
            )
            .child(hairline(
                0.1,
                0.5,
                0.2,
                true,
                pin(matches!(h, Pin::Start | Pin::Both)),
            ))
            .child(hairline(
                0.7,
                0.5,
                0.2,
                true,
                pin(matches!(h, Pin::End | Pin::Both)),
            ))
            .child(hairline(
                0.5,
                0.1,
                0.2,
                false,
                pin(matches!(v, Pin::Start | Pin::Both)),
            ))
            .child(hairline(
                0.5,
                0.7,
                0.2,
                false,
                pin(matches!(v, Pin::End | Pin::Both)),
            ))
            .when(h == Pin::Center, |p| {
                p.child(hairline(0.4, 0.5, 0.2, true, lit))
            })
            .when(v == Pin::Center, |p| {
                p.child(hairline(0.5, 0.4, 0.2, false, lit))
            });
        let choice = |across: bool| {
            let pinned = if across { h } else { v };
            let set = editing(
                "constraints editor",
                &now,
                on_change,
                move |c: &mut Constraints, pin: Pin| {
                    if across { c.across = pin } else { c.down = pin }
                },
            );
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(caption(if across { "Horizontal" } else { "Vertical" }, cx))
                .child(
                    Select::new(
                        (id.clone(), if across { "across" } else { "down" }),
                        Pin::ALL
                            .iter()
                            .map(|pin| Choice::new(pin.value(), pin.words(across))),
                    )
                    .selected(pinned.value())
                    .on_change(move |value: &SharedString, window, cx| {
                        let pin = *Pin::ALL
                            .iter()
                            .find(|pin| pin.value() == value.as_ref())
                            .expect("a pin the select offers");
                        set(pin, window, cx)
                    }),
                )
        };
        div().flex().gap_3().child(picture).child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .gap_2()
                .child(choice(true))
                .child(choice(false)),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{Constraints, Pin};
    use crate::canvas::Frame;

    #[test]
    fn each_pin_keeps_its_own_distance_as_the_parent_grows() {
        let (child, before) = ((20.0, 10.0), (0.0, 100.0));
        let grown = (50.0, 200.0);
        assert_eq!(Pin::Start.follow(child, before, grown), (70.0, 10.0));
        assert_eq!(Pin::End.follow(child, before, grown), (170.0, 10.0));
        assert_eq!(Pin::Both.follow(child, before, grown), (70.0, 110.0));
        assert_eq!(Pin::Center.follow(child, before, grown), (120.0, 10.0));
        assert_eq!(Pin::Scale.follow(child, before, grown), (90.0, 20.0));
    }

    #[test]
    fn constraints_place_a_child_on_both_axes() {
        let pins = Constraints {
            across: Pin::Both,
            down: Pin::End,
        };
        let placed = pins.placed(
            Frame::new(10.0, 60.0, 80.0, 20.0),
            Frame::new(0.0, 0.0, 100.0, 100.0),
            Frame::new(0.0, 0.0, 300.0, 150.0),
        );
        assert_eq!(placed, Frame::new(10.0, 110.0, 280.0, 20.0));
    }
}
