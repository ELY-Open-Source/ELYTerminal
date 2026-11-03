use gpui::{
    AnyElement, App, ElementId, FocusHandle, InteractiveElement, IntoElement, ParentElement,
    Pixels, StatefulInteractiveElement, Styled, Window, div, prelude::*,
};

use super::{
    MapViewport,
    geo::{project, unproject},
    marker::{MapMarker, OnMarker},
    view::OnMapViewport,
};
use crate::{
    primitives::{FocusRing, tab_stop},
    theme::{ActiveTheme, TextSize},
};

/// Pins gathered by squares of the world `side` pixels wide at the view's zoom, so a pan keeps each group: each square that holds pins, keyed by its column and row, with its members in order.
pub(crate) fn gather(points: &[(f64, f64)], side: f64) -> Vec<((i64, i64), Vec<usize>)> {
    assert!(side > 0.0, "a gathering square of {side}");
    let mut groups: Vec<((i64, i64), Vec<usize>)> = Vec::new();
    for (ix, (x, y)) in points.iter().enumerate() {
        let key = ((x / side).floor() as i64, (y / side).floor() as i64);
        match groups.iter_mut().find(|(each, _)| *each == key) {
            Some((_, members)) => members.push(ix),
            None => groups.push((key, vec![ix])),
        }
    }
    groups
}

/// The map's pins as drawn, and when `gathered`, the focus of each pin and cluster drawn.
pub(crate) struct Pins {
    pub elements: Vec<AnyElement>,
    pub gathered: Vec<FocusHandle>,
}

/// The pins a map lays: its markers, what hears a pin's press, and whether pins that crowd gather.
pub(crate) struct Laying {
    pub markers: Vec<MapMarker>,
    pub on_marker: Option<OnMarker>,
    pub gathered: bool,
}

impl Laying {
    /// Lays the pins at their places, or gathers pins that crowd one square of the world into a cluster that shows their count. Each keeps its own focus in view and out, and takes Tab and presses while in view; a cluster's press shows its members whole.
    pub(crate) fn lay(
        self,
        owner: &ElementId,
        (view, size, tile): (MapViewport, (f32, f32), f32),
        set: &OnMapViewport,
        window: &mut Window,
        cx: &mut App,
    ) -> Pins {
        let Laying {
            markers,
            on_marker,
            gathered,
        } = self;
        let seen = |(x, y): (f32, f32)| (0.0..=size.0).contains(&x) && (0.0..=size.1).contains(&y);
        let rem = window.rem_size();
        let sizes = cx.theme().maps();
        let groups = match gathered {
            true => {
                let side = f64::from(tile) * view.zoom.exp2();
                let points: Vec<(f64, f64)> = markers
                    .iter()
                    .map(|marker| {
                        let (x, y) = project(marker.at);
                        (x * side, y * side)
                    })
                    .collect();
                gather(&points, f64::from(sizes.gather.to_pixels(rem)))
            }
            false => (0..markers.len())
                .map(|ix| ((0, ix as i64), vec![ix]))
                .collect(),
        };
        let mut pins = Pins {
            elements: Vec::new(),
            gathered: Vec::new(),
        };
        for (key, members) in groups {
            if let [only] = members[..] {
                let marker = markers[only].clone();
                let at = view.to_view(marker.at, size, tile);
                let press = on_marker.clone().map(|on_marker| {
                    let key = (owner.clone(), format!("pin-{}", marker.key));
                    let focus = tab_stop(key.into(), seen(at), window, cx);
                    if gathered {
                        pins.gathered.push(focus.clone());
                    }
                    (focus, on_marker)
                });
                pins.elements
                    .push(marker.place(owner, at, press, seen(at), cx));
                continue;
            }
            let places: Vec<_> = members.iter().map(|ix| markers[*ix].at).collect();
            let mean =
                places
                    .iter()
                    .map(|place| project(*place))
                    .fold((0.0, 0.0), |sum, (x, y)| {
                        (
                            sum.0 + x / places.len() as f64,
                            sum.1 + y / places.len() as f64,
                        )
                    });
            let at = view.to_view(unproject(mean), size, tile);
            let id = (owner.clone(), format!("cluster-{}-{}", key.0, key.1));
            let focus = tab_stop(id.clone().into(), seen(at), window, cx);
            pins.gathered.push(focus.clone());
            let margin = f32::from(sizes.margin.to_pixels(rem));
            let set = set.clone();
            let fit = move |window: &mut Window, cx: &mut App| {
                set(view.fitting(&places, size, tile, margin * 4.0), window, cx)
            };
            pins.elements.push(bubble(
                id.into(),
                members.len(),
                at,
                focus,
                seen(at),
                fit,
                cx,
            ));
        }
        pins
    }
}

/// A round cluster at a view point, its count inside, pressable while `seen`.
fn bubble(
    id: ElementId,
    count: usize,
    (x, y): (f32, f32),
    focus: FocusHandle,
    seen: bool,
    fit: impl Fn(&mut Window, &mut App) + 'static,
    cx: &App,
) -> AnyElement {
    let theme = cx.theme();
    let side = theme.maps().cluster;
    div()
        .absolute()
        .left(Pixels::from(x))
        .top(Pixels::from(y))
        .size_0()
        .flex()
        .items_center()
        .justify_center()
        .child(
            div()
                .id(id)
                .track_focus(&focus)
                .flex_none()
                .size(side)
                .flex()
                .items_center()
                .justify_center()
                .rounded_full()
                .bg(theme.colors.accent)
                .border_1()
                .border_color(theme.colors.on_accent)
                .focus_ring(cx)
                .text_size(theme.text_size(TextSize::Sm))
                .text_color(theme.colors.on_accent)
                .child(count.to_string())
                .when(seen, |bubble| {
                    bubble
                        .cursor_pointer()
                        .on_click(move |_, window, cx| fit(window, cx))
                }),
        )
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::gather;

    #[test]
    fn pins_in_one_square_of_the_world_gather() {
        let points = [
            (10.0, 10.0),
            (50.0, 20.0),
            (70.0, 10.0),
            (130.0, 10.0),
            (20.0, 55.0),
            (20.0, 65.0),
        ];
        assert_eq!(
            gather(&points, 60.0),
            vec![
                ((0, 0), vec![0, 1, 4]),
                ((1, 0), vec![2]),
                ((2, 0), vec![3]),
                ((0, 1), vec![5])
            ],
            "each square keeps its members in order, the squares in the order first met"
        );
    }

    #[test]
    #[should_panic(expected = "a gathering square of 0")]
    fn a_square_of_nothing_fails() {
        gather(&[(0.0, 0.0)], 0.0);
    }
}
