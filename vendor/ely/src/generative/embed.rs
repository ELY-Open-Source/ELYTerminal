use std::{collections::HashSet, rc::Rc};

use gpui::{
    App, Bounds, DragMoveEvent, ElementId, EmptyView, EntityId, HoverListenerMode,
    InteractiveElement, IntoElement, MouseButton, ParentElement, Pixels, Point, RenderOnce,
    SharedString, StatefulInteractiveElement, Styled, Window, canvas, div, fill, point, prelude::*,
    size, transparent_black,
};

use super::lens::{DRAG, Lens, START, STEP, radius, turned};
use crate::{
    charts::{ChartLegend, ChartTooltip, anchored},
    primitives::{FocusRing, checked_ratio, tab_stop},
    theme::{ActiveTheme, Radius},
};

/// A point of an embedding: its key, its label, its group, and where it sits, each axis from -1 to 1.
#[derive(Clone, Debug, PartialEq)]
pub struct Embedded {
    pub key: SharedString,
    pub label: SharedString,
    pub group: usize,
    pub at: [f32; 3],
}

/// A box's width and height in pixels.
fn extent(bounds: Bounds<Pixels>) -> (f32, f32) {
    (f32::from(bounds.size.width), f32::from(bounds.size.height))
}

/// A turning drag, marked with its view.
struct Spin(EntityId);

/// The view's own state: its turn, where a drag was last, the point under the pointer, the groups the legend hid, and the plot's bounds.
struct View {
    turn: (f32, f32),
    from: Point<Pixels>,
    hovered: Option<usize>,
    hidden: HashSet<usize>,
    bounds: Bounds<Pixels>,
}

/// Points of an embedding scattered by where they sit, colored by group, flat or turned in three dimensions. Turned, a drag or the arrow keys spin it, nearer points drawn larger and stronger. The pointer names the point under it; the legend hides and shows groups.
#[derive(IntoElement)]
pub struct EmbeddingVisualizer {
    id: ElementId,
    points: Rc<Vec<Embedded>>,
    radius: f32,
    groups: Vec<SharedString>,
    three: bool,
    ratio: f32,
}

impl EmbeddingVisualizer {
    /// `groups` names each group a point's `group` indexes.
    pub fn new(
        id: impl Into<ElementId>,
        points: impl IntoIterator<Item = Embedded>,
        groups: impl IntoIterator<Item = impl Into<SharedString>>,
    ) -> Self {
        let points: Vec<Embedded> = points.into_iter().collect();
        let groups: Vec<SharedString> = groups.into_iter().map(Into::into).collect();
        for (ix, name) in groups.iter().enumerate() {
            assert!(!groups[..ix].contains(name), "group {name} named twice");
        }
        let points: Vec<Embedded> = points
            .into_iter()
            .filter(|point| {
                let held = point.group < groups.len();
                if !held {
                    log::error!(
                        "embedding: point {} in group {} of {}; left out",
                        point.key,
                        point.group,
                        groups.len()
                    );
                }
                held
            })
            .collect();
        for point in &points {
            assert!(
                point.at.iter().all(|axis| (-1.0..=1.0).contains(axis)),
                "point {} sits outside -1 to 1",
                point.key
            );
        }
        Self {
            id: id.into(),
            radius: radius(&points),
            points: Rc::new(points),
            groups,
            three: false,
            ratio: 4.0 / 3.0,
        }
    }

    /// Turns the view in three dimensions.
    pub fn three_dimensions(mut self, three: bool) -> Self {
        self.three = three;
        self
    }

    /// The view's width over height.
    pub fn ratio(mut self, ratio: f32) -> Self {
        self.ratio = checked_ratio(ratio);
        self
    }
}

impl RenderOnce for EmbeddingVisualizer {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let view = window.use_keyed_state((self.id.clone(), "view"), cx, |_, _| View {
            turn: START,
            from: Point::default(),
            hovered: None,
            hidden: HashSet::new(),
            bounds: Bounds::default(),
        });
        let focus = tab_stop((self.id.clone(), "focus").into(), self.three, window, cx);
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let rem = window.rem_size();
        let (dot, reach) = (
            theme.status_dot().to_pixels(rem),
            f32::from(theme.chart().inset.to_pixels(rem)),
        );
        let (three, points, radius) = (self.three, self.points.clone(), self.radius);
        let (turn, hovered, hidden, bounds) = {
            let view = view.read(cx);
            (
                three.then_some(view.turn),
                view.hovered,
                view.hidden.clone(),
                view.bounds,
            )
        };
        let lens = Lens {
            turn,
            size: extent(bounds),
            radius,
        };
        let tint = move |group: usize| colors.chart[group % colors.chart.len()];
        let tooltip = hovered.filter(|ix| *ix < points.len()).map(|ix| {
            let point = &points[ix];
            let (x, y, _) = lens.place(point.at);
            let card = ChartTooltip::new(point.label.clone()).row(
                Some(tint(point.group)),
                self.groups[point.group].clone(),
                "",
            );
            anchored((x, y), x > lens.size.0 * 0.6, card).debug_selector(|| "embedding-tip".into())
        });
        let legend = self.groups.iter().enumerate().fold(
            ChartLegend::new((self.id.clone(), "legend")),
            |legend, (ix, name)| legend.entry(tint(ix), name.clone(), hidden.contains(&ix)),
        );
        let (toggled, groups) = (view.clone(), self.groups.clone());
        let legend = legend.on_toggle(move |name, _, cx| {
            let ix = groups
                .iter()
                .position(|group| group == name)
                .expect("a listed group");
            toggled.update(cx, |view, cx| {
                if !view.hidden.remove(&ix) {
                    view.hidden.insert(ix);
                }
                view.hovered = None;
                log::info!("embedding: {name} hidden {}", view.hidden.contains(&ix));
                cx.notify();
            })
        });
        let owner = view.entity_id();
        let (measure, moved, pressed, spun, keys, left) = (
            view.clone(),
            view.clone(),
            view.clone(),
            view.clone(),
            view.clone(),
            view.clone(),
        );
        let (painted, hover_points) = (points.clone(), points.clone());
        let mut area = div().id(self.id.clone());
        area.style().aspect_ratio = Some(self.ratio);
        let area = area
            .debug_selector(|| "embedding-area".into())
            .relative()
            .w_full()
            .rounded(theme.radius(Radius::Lg))
            .bg(theme.colors.sunken)
            .border_1()
            .border_color(transparent_black())
            .when(three, |area| {
                area.track_focus(&focus).focus_ring(cx).cursor_grab()
            })
            .child(
                canvas(
                    move |bounds, window, cx| {
                        if measure.read(cx).bounds != bounds {
                            measure.update(cx, |view, cx| {
                                view.bounds = bounds;
                                cx.notify();
                            });
                            window.request_animation_frame();
                        }
                    },
                    move |bounds, _, window, _| {
                        let lens = Lens {
                            size: extent(bounds),
                            ..lens
                        };
                        let mut placed: Vec<_> = painted
                            .iter()
                            .enumerate()
                            .filter(|(_, point)| !hidden.contains(&point.group))
                            .map(|(ix, point)| (ix, point.group, lens.place(point.at)))
                            .collect();
                        placed.sort_by(|a, b| a.2.2.total_cmp(&b.2.2));
                        for (ix, group, (x, y, near)) in placed {
                            let depth = if turn.is_some() {
                                0.6 + 0.8 * near
                            } else {
                                1.0
                            };
                            let side = dot * depth * if Some(ix) == hovered { 1.8 } else { 1.0 };
                            let center = point(
                                bounds.left() + Pixels::from(x),
                                bounds.top() + Pixels::from(y),
                            );
                            let alpha = if turn.is_some() {
                                0.35 + 0.65 * near
                            } else {
                                0.85
                            };
                            window.paint_quad(
                                fill(
                                    Bounds::centered_at(center, size(side, side)),
                                    tint(group).alpha(alpha),
                                )
                                .corner_radii(side / 2.0),
                            );
                        }
                    },
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            )
            .children(tooltip)
            .on_mouse_move(move |event, _, cx| {
                if event.pressed_button.is_some() {
                    return;
                }
                moved.update(cx, |view, cx| {
                    let bounds = view.bounds;
                    let at = (
                        f32::from(event.position.x - bounds.left()),
                        f32::from(event.position.y - bounds.top()),
                    );
                    let lens = Lens {
                        turn: three.then_some(view.turn),
                        size: extent(bounds),
                        radius,
                    };
                    let under = lens.nearest(&hover_points, &view.hidden, at, reach);
                    if under != view.hovered {
                        view.hovered = under;
                        cx.notify();
                    }
                })
            })
            .when(three, |area| {
                area.on_mouse_down(MouseButton::Left, move |event, _, cx| {
                    pressed.update(cx, |view, _| view.from = event.position)
                })
                .on_drag(Spin(owner), |_, _, _, cx| {
                    log::info!("embedding: a drag turns the view");
                    cx.new(|_| EmptyView)
                })
                .on_drag_move(move |event: &DragMoveEvent<Spin>, _, cx| {
                    if event.drag(cx).0 != owner {
                        return;
                    }
                    spun.update(cx, |view, cx| {
                        let delta = event.event.position - view.from;
                        view.turn = turned(
                            view.turn,
                            (f32::from(delta.x) * DRAG, f32::from(delta.y) * DRAG),
                        );
                        (view.from, view.hovered) = (event.event.position, None);
                        cx.notify();
                    })
                })
            })
            .hover_listener_mode(HoverListenerMode::InputModalityIndependent)
            .on_hover(move |inside, _, cx| {
                if !inside {
                    left.update(cx, |view, cx| {
                        view.hovered = None;
                        cx.notify();
                    })
                }
            })
            .on_key_down(move |event, _, cx| {
                let by = match event.keystroke.key.as_str() {
                    "left" => (-STEP, 0.0),
                    "right" => (STEP, 0.0),
                    "up" => (0.0, -STEP),
                    "down" => (0.0, STEP),
                    _ => return,
                };
                cx.stop_propagation();
                keys.update(cx, |view, cx| {
                    view.turn = turned(view.turn, by);
                    view.hovered = None;
                    log::info!(
                        "embedding: turned to {:.2}, {:.2}",
                        view.turn.0,
                        view.turn.1
                    );
                    cx.notify();
                })
            });
        div().flex().flex_col().gap_3().child(legend).child(area)
    }
}
