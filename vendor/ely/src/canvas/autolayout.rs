use std::rc::Rc;

use gpui::{
    App, ElementId, InteractiveElement, IntoElement, MouseButton, ParentElement, RenderOnce,
    SharedString, Styled, Window, div,
};

use super::{
    panel::{OnEdit, caption, editing},
    view::Frame,
};
use crate::{
    buttons::SegmentedControl,
    forms::ScrubInput,
    primitives::{IconName, tab_stop},
    theme::{ActiveTheme, Radius},
};

/// Which way auto layout runs its children.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Flow {
    Row,
    Column,
}

/// Where children sit along a side of their frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Place {
    Start,
    Center,
    End,
}

const PLACES: [Place; 3] = [Place::Start, Place::Center, Place::End];

impl Place {
    /// How far into `free` room it puts what it places.
    fn offset(self, free: f32) -> f32 {
        match self {
            Place::Start => 0.0,
            Place::Center => free / 2.0,
            Place::End => free,
        }
    }

    /// The place a step of `by` reaches, stopping at either end.
    fn stepped(self, by: isize) -> Place {
        let at = PLACES
            .iter()
            .position(|place| *place == self)
            .expect("a place") as isize;
        PLACES[(at + by).clamp(0, 2) as usize]
    }
}

/// Children in a row or a column, a gap apart inside padding, placed across and down.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AutoLayout {
    pub flow: Flow,
    pub gap: f32,
    /// Across and down, in pixels.
    pub padding: (f32, f32),
    /// Across and down.
    pub place: (Place, Place),
}

impl AutoLayout {
    /// Children of `sizes` laid inside `frame`.
    pub fn laid(&self, frame: Frame, sizes: &[(f32, f32)]) -> Vec<Frame> {
        let row = self.flow == Flow::Row;
        let (x, y) = (frame.x + self.padding.0, frame.y + self.padding.1);
        let room = (
            frame.w - 2.0 * self.padding.0,
            frame.h - 2.0 * self.padding.1,
        );
        let (along, wide) = if row { room } else { (room.1, room.0) };
        let (lead, side) = if row {
            self.place
        } else {
            (self.place.1, self.place.0)
        };
        let long = |(w, h): (f32, f32)| if row { w } else { h };
        let run: f32 = sizes.iter().map(|size| long(*size)).sum::<f32>()
            + self.gap * sizes.len().saturating_sub(1) as f32;
        let mut next = lead.offset(along - run);
        sizes
            .iter()
            .map(|&(w, h)| {
                let off = side.offset(wide - if row { h } else { w });
                let laid = if row {
                    Frame::new(x + next, y + off, w, h)
                } else {
                    Frame::new(x + off, y + next, w, h)
                };
                next += long((w, h)) + self.gap;
                laid
            })
            .collect()
    }
}

/// Auto layout's flow, gap and padding, and where children sit: nine places in a grid, one Tab stop that the arrows move through.
#[derive(IntoElement)]
pub struct AutoLayoutControls {
    id: ElementId,
    layout: AutoLayout,
    on_change: OnEdit<AutoLayout>,
}

impl AutoLayoutControls {
    pub fn new(
        id: impl Into<ElementId>,
        layout: AutoLayout,
        on_change: impl Fn(AutoLayout, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            layout,
            on_change: Rc::new(on_change),
        }
    }
}

impl RenderOnce for AutoLayoutControls {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let on_change = self.on_change;
        let (id, layout, on_change) = (self.id, self.layout, &on_change);
        let place = Rc::new(editing(
            "auto layout",
            &layout,
            on_change,
            |layout, place| layout.place = place,
        ));
        let flow = editing("auto layout", &layout, on_change, |layout, flow: Flow| {
            layout.flow = flow
        });
        let at = |edit: fn(&mut AutoLayout, f64)| editing("auto layout", &layout, on_change, edit);
        let scrub = |name: &'static str, value: f32| {
            ScrubInput::new((id.clone(), name), name, value as f64)
                .range(0.0, 1_000.0)
                .precision(0)
        };
        let focus = tab_stop((id.clone(), "places").into(), true, window, cx);
        let focused = focus.is_focused(window);
        let theme = cx.theme();
        let sizes = theme.canvas();
        let (accent, subtle, hover) = (
            theme.colors.accent,
            theme.colors.fg_subtle,
            theme.colors.hover,
        );
        let keys = place.clone();
        let grid = div()
            .id((id.clone(), "places"))
            .debug_selector(|| "places".into())
            .track_focus(&focus)
            .flex_none()
            .flex()
            .flex_col()
            .p_0p5()
            .rounded(theme.radius(Radius::Md))
            .border_1()
            .border_color(if focused {
                theme.colors.focus
            } else {
                theme.colors.border
            })
            .on_key_down(move |event, window, cx| {
                let (across, down) = layout.place;
                let next = match event.keystroke.key.as_str() {
                    "left" => (across.stepped(-1), down),
                    "right" => (across.stepped(1), down),
                    "up" => (across, down.stepped(-1)),
                    "down" => (across, down.stepped(1)),
                    _ => return,
                };
                cx.stop_propagation();
                if next != layout.place {
                    keys(next, window, cx);
                }
            })
            .children(PLACES.iter().map(|down| {
                div().flex().children(PLACES.iter().map(|across| {
                    let spot = (*across, *down);
                    let chosen = spot == layout.place;
                    let press = place.clone();
                    div()
                        .id((id.clone(), format!("place-{across:?}-{down:?}")))
                        .flex()
                        .items_center()
                        .justify_center()
                        .size(sizes.place)
                        .rounded(theme.radius(Radius::Sm))
                        .cursor_pointer()
                        .hover(move |style| style.bg(hover))
                        .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                            press(spot, window, cx)
                        })
                        .child(
                            div()
                                .size(if chosen {
                                    sizes.place_mark
                                } else {
                                    sizes.place_pip
                                })
                                .rounded(theme.radius(Radius::Sm))
                                .bg(if chosen { accent } else { subtle }),
                        )
                }))
            }));
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                SegmentedControl::new(
                    (id.clone(), "flow"),
                    if layout.flow == Flow::Row {
                        "row"
                    } else {
                        "column"
                    },
                )
                .segment("row", "Row", Some(IconName::ArrowRight))
                .segment("column", "Column", Some(IconName::ArrowDown))
                .on_change(move |value: &SharedString, window, cx| {
                    let chosen = if value.as_ref() == "row" {
                        Flow::Row
                    } else {
                        Flow::Column
                    };
                    flow(chosen, window, cx)
                }),
            )
            .child(
                div().flex().items_start().gap_3().child(grid).child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(scrub("Gap", layout.gap).on_change(at(|l, v| l.gap = v as f32)))
                        .child(caption("Padding", cx))
                        .child(
                            scrub("Across", layout.padding.0)
                                .on_change(at(|l, v| l.padding.0 = v as f32)),
                        )
                        .child(
                            scrub("Down", layout.padding.1)
                                .on_change(at(|l, v| l.padding.1 = v as f32)),
                        ),
                ),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::{AutoLayout, Flow, Place};
    use crate::canvas::Frame;

    #[test]
    fn a_row_runs_its_children_a_gap_apart_and_places_the_run() {
        let row = AutoLayout {
            flow: Flow::Row,
            gap: 10.0,
            padding: (20.0, 10.0),
            place: (Place::Center, Place::End),
        };
        let laid = row.laid(
            Frame::new(0.0, 0.0, 200.0, 100.0),
            &[(40.0, 20.0), (60.0, 40.0)],
        );
        assert_eq!(
            laid,
            [
                Frame::new(45.0, 70.0, 40.0, 20.0),
                Frame::new(95.0, 50.0, 60.0, 40.0)
            ],
            "the run of 110 sits in the middle of 160, each child on the bottom padding"
        );
    }

    #[test]
    fn a_column_runs_down_from_its_top() {
        let column = AutoLayout {
            flow: Flow::Column,
            gap: 8.0,
            padding: (0.0, 0.0),
            place: (Place::Start, Place::Start),
        };
        let laid = column.laid(
            Frame::new(10.0, 10.0, 100.0, 100.0),
            &[(30.0, 10.0), (50.0, 20.0)],
        );
        assert_eq!(
            laid,
            [
                Frame::new(10.0, 10.0, 30.0, 10.0),
                Frame::new(10.0, 28.0, 50.0, 20.0)
            ]
        );
    }

    #[test]
    fn a_step_stops_at_either_end() {
        assert_eq!(Place::Start.stepped(-1), Place::Start);
        assert_eq!(Place::Start.stepped(1), Place::Center);
        assert_eq!(Place::End.stepped(1), Place::End);
    }
}
