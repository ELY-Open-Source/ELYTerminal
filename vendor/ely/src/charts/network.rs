use std::{collections::HashSet, f32::consts::TAU};

use gpui::{
    App, Bounds, Div, ElementId, Entity, HoverListenerMode, InteractiveElement, IntoElement,
    MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, ParentElement, PathBuilder, Pixels,
    Refineable, RenderOnce, SharedString, StatefulInteractiveElement, StyleRefinement, Styled,
    Window, canvas, div, prelude::*,
};

use super::{
    axes::anchored,
    paint::{at, finish, measure, ring, tint},
    parts::ChartTooltip,
};
use crate::theme::{ActiveTheme, TextSize};

/// Places for a graph's nodes in a unit square, pushed apart by every other node and pulled along their edges (Fruchterman and Reingold), cooling a step at a time.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Force {
    pub places: Vec<(f32, f32)>,
    heat: f32,
    pinned: HashSet<usize>,
}

impl Force {
    /// Nodes on a circle, hot enough to move.
    pub(crate) fn new(count: usize) -> Self {
        let places = (0..count)
            .map(|ix| {
                let angle = TAU * ix as f32 / count.max(1) as f32;
                (0.5 + 0.3 * angle.cos(), 0.5 + 0.3 * angle.sin())
            })
            .collect();
        Self {
            places,
            heat: 0.08,
            pinned: HashSet::new(),
        }
    }

    pub(crate) fn settled(&self) -> bool {
        self.heat < 0.001
    }

    /// Holds a node where the pointer put it and warms the rest to settle around it.
    pub(crate) fn pin(&mut self, ix: usize, place: (f32, f32)) {
        self.places[ix] = (place.0.clamp(0.0, 1.0), place.1.clamp(0.0, 1.0));
        self.pinned.insert(ix);
        self.heat = self.heat.max(0.02);
    }

    /// One step: every pair pushes apart, every edge pulls together, a little gravity keeps it centered, and each node moves no farther than the heat.
    pub(crate) fn step(&mut self, edges: &[(usize, usize)]) {
        let count = self.places.len();
        let k = 0.6 / (count.max(1) as f32).sqrt();
        let mut moves = vec![(0.0f32, 0.0f32); count];
        let apart = |a: (f32, f32), b: (f32, f32)| {
            let (dx, dy) = (a.0 - b.0, a.1 - b.1);
            let distance = dx.hypot(dy).max(1e-3);
            (dx / distance, dy / distance, distance)
        };
        for a in 0..count {
            for b in a + 1..count {
                let (x, y, distance) = apart(self.places[a], self.places[b]);
                let push = k * k / distance;
                moves[a] = (moves[a].0 + x * push, moves[a].1 + y * push);
                moves[b] = (moves[b].0 - x * push, moves[b].1 - y * push);
            }
        }
        for (a, b) in edges {
            let (x, y, distance) = apart(self.places[*a], self.places[*b]);
            let pull = distance * distance / k;
            moves[*a] = (moves[*a].0 - x * pull, moves[*a].1 - y * pull);
            moves[*b] = (moves[*b].0 + x * pull, moves[*b].1 + y * pull);
        }
        for (ix, place) in self.places.iter_mut().enumerate() {
            if self.pinned.contains(&ix) {
                continue;
            }
            let (x, y) = (
                moves[ix].0 + (0.5 - place.0) * k,
                moves[ix].1 + (0.5 - place.1) * k,
            );
            let length = x.hypot(y).max(1e-6);
            let reach = length.min(self.heat);
            *place = (
                (place.0 + x / length * reach).clamp(0.0, 1.0),
                (place.1 + y / length * reach).clamp(0.0, 1.0),
            );
        }
        self.heat *= 0.96;
    }
}

/// A graph's own state: its box, the forces for the shape it was built for, the node under the pointer and the one being dragged.
#[derive(Default)]
struct Graph {
    bounds: Bounds<Pixels>,
    shape: (usize, Vec<(usize, usize)>),
    force: Option<Force>,
    hover: Option<usize>,
    drag: Option<usize>,
}

/// Nodes and the edges between them, placed by forces: edges pull, nodes push apart, and the layout settles as you watch. Drag a node to move it; hover one to light its neighbors.
#[derive(IntoElement)]
pub struct NetworkGraph {
    base: Div,
    id: ElementId,
    nodes: Vec<(SharedString, usize)>,
    edges: Vec<(usize, usize)>,
}

impl NetworkGraph {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            base: div(),
            id: id.into(),
            nodes: Vec::new(),
            edges: Vec::new(),
        }
    }

    /// A node, colored by its group.
    pub fn node(mut self, name: impl Into<SharedString>, group: usize) -> Self {
        self.nodes.push((name.into(), group));
        self
    }

    /// An edge between two nodes, by their places in the list.
    pub fn edge(mut self, a: usize, b: usize) -> Self {
        if a >= self.nodes.len() || b >= self.nodes.len() || a == b {
            log::error!(
                "network: an edge {a} - {b} among {} nodes; skipped",
                self.nodes.len()
            );
            return self;
        }
        self.edges.push((a, b));
        self
    }
}

impl Styled for NetworkGraph {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for NetworkGraph {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let graph: Entity<Graph> =
            window.use_keyed_state((self.id.clone(), "graph"), cx, |_, _| Graph::default());
        let shape = (self.nodes.len(), self.edges.clone());
        let still = cx.theme().reduced_motion;
        let places = graph.update(cx, |graph, _| {
            if graph.shape != shape || graph.force.is_none() {
                log::info!(
                    "network graph: {} nodes, {} edges; settling",
                    shape.0,
                    shape.1.len()
                );
                graph.force = Some(Force::new(shape.0));
                graph.shape = shape;
            }
            let force = graph.force.as_mut().expect("forces were just set");
            for _ in 0..if still { 400 } else { 3 } {
                if force.settled() {
                    break;
                }
                force.step(&graph.shape.1);
            }
            (force.places.clone(), force.settled())
        });
        let (places, settled) = places;
        if !settled {
            window.request_animation_frame();
        }
        let (bounds, hover) = (
            graph.read(cx).bounds,
            graph.read(cx).hover.filter(|ix| *ix < self.nodes.len()),
        );
        let theme = cx.theme();
        let (colors, sizes, rem) = (theme.colors.clone(), theme.chart(), window.rem_size());
        let pixels = |length: gpui::Rems| f32::from(length.to_pixels(rem));
        let margin = pixels(sizes.gutter);
        let (width, height) = (
            (f32::from(bounds.size.width) - margin * 2.0).max(0.0),
            (f32::from(bounds.size.height) - margin * 2.0).max(0.0),
        );
        let spot = move |(x, y): (f32, f32)| (margin + x * width, margin + y * height);
        let degree: Vec<usize> = (0..self.nodes.len())
            .map(|ix| {
                self.edges
                    .iter()
                    .filter(|(a, b)| *a == ix || *b == ix)
                    .count()
            })
            .collect();
        let dot = pixels(theme.status_dot());
        let radii: Vec<f32> = degree
            .iter()
            .map(|degree| dot * (1.0 + (*degree as f32).sqrt() * 0.35))
            .collect();
        let near: HashSet<usize> = hover.map_or_else(HashSet::new, |on| {
            self.edges
                .iter()
                .filter_map(|(a, b)| {
                    if *a == on {
                        Some(*b)
                    } else if *b == on {
                        Some(*a)
                    } else {
                        None
                    }
                })
                .chain([on])
                .collect()
        });
        let lit = move |ix: usize| hover.is_none() || near.contains(&ix);
        let labels: Vec<Div> = self
            .nodes
            .iter()
            .enumerate()
            .map(|(ix, (name, _))| {
                let (x, y) = spot(places[ix]);
                let label = div()
                    .flex_none()
                    .whitespace_nowrap()
                    .text_size(theme.text_size(TextSize::Xs))
                    .text_color(if hover == Some(ix) {
                        colors.fg
                    } else {
                        colors.fg_muted
                    });
                div()
                    .absolute()
                    .left(Pixels::from(x + radii[ix]))
                    .top(Pixels::from(y))
                    .h_0()
                    .flex()
                    .items_center()
                    .pl_1()
                    .when(!lit(ix), |label| label.opacity(0.35))
                    .child(label.child(name.clone()))
            })
            .collect();
        let tooltip = hover.map(|ix| {
            let card = ChartTooltip::new(self.nodes[ix].0.clone()).row(
                Some(tint(&colors, self.nodes[ix].1)),
                "Links",
                degree[ix].to_string(),
            );
            let (x, y) = spot(places[ix]);
            anchored((x, y), x > f32::from(bounds.size.width) * 0.6, card)
        });
        let edges = self.edges.clone();
        let groups: Vec<usize> = self.nodes.iter().map(|(_, group)| *group).collect();
        let (stroke, hairline, palette) =
            (sizes.stroke.to_pixels(rem), sizes.hairline, colors.clone());
        let (points, sized) = (places.clone(), radii.clone());
        let hit = move |place: (f32, f32)| {
            (0..points.len())
                .map(|ix| {
                    (ix, {
                        let (x, y) = spot(points[ix]);
                        (x - place.0).hypot(y - place.1)
                    })
                })
                .filter(|(ix, far)| *far <= sized[*ix] + dot)
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .map(|(ix, _)| ix)
        };
        let unit = move |(x, y): (f32, f32)| {
            (
                (x - margin) / width.max(1.0),
                (y - margin) / height.max(1.0),
            )
        };
        let local = |position: gpui::Point<Pixels>, bounds: Bounds<Pixels>| {
            let offset = position - bounds.origin;
            (f32::from(offset.x), f32::from(offset.y))
        };
        let count = self.nodes.len();
        let (moved, pressed, released, left, measured) = (
            graph.clone(),
            graph.clone(),
            graph.clone(),
            graph.clone(),
            graph.clone(),
        );
        let press_hit = hit.clone();
        let mut root = div()
            .debug_selector(|| "chart-root".into())
            .relative()
            .h(sizes.height);
        root.style().refine(self.base.style());
        root.id((self.id.clone(), "network"))
            .on_mouse_down(MouseButton::Left, move |event: &MouseDownEvent, _, cx| {
                let place = local(event.position, pressed.read(cx).bounds);
                if let Some(ix) = press_hit(place) {
                    log::info!("network graph: dragging node {ix}");
                    pressed.update(cx, |graph, cx| {
                        graph.drag = Some(ix);
                        cx.notify();
                    })
                }
            })
            .on_mouse_move(move |event: &MouseMoveEvent, _, cx| {
                let place = local(event.position, moved.read(cx).bounds);
                moved.update(cx, |graph, cx| {
                    if let (Some(ix), Some(MouseButton::Left)) =
                        (graph.drag.filter(|ix| *ix < count), event.pressed_button)
                    {
                        graph
                            .force
                            .as_mut()
                            .expect("a dragged graph has forces")
                            .pin(ix, unit(place));
                        cx.notify();
                        return;
                    }
                    let next = hit(place);
                    if graph.hover != next {
                        graph.hover = next;
                        cx.notify();
                    }
                })
            })
            .on_mouse_up(MouseButton::Left, move |_: &MouseUpEvent, _, cx| {
                released.update(cx, |graph, cx| {
                    if graph.drag.take().is_some() {
                        cx.notify();
                    }
                })
            })
            .hover_listener_mode(HoverListenerMode::InputModalityIndependent)
            .on_hover(move |inside, _, cx| {
                if !*inside {
                    left.update(cx, |graph, cx| {
                        (graph.hover, graph.drag) = (None, None);
                        cx.notify();
                    })
                }
            })
            .child(
                canvas(
                    |_, _, _| {},
                    move |bounds, _, window, _| {
                        let origin = bounds.origin;
                        for (a, b) in &edges {
                            let mut line = PathBuilder::stroke(hairline);
                            line.move_to(at(origin, spot(places[*a])));
                            line.line_to(at(origin, spot(places[*b])));
                            let strong =
                                hover.is_some() && (hover == Some(*a) || hover == Some(*b));
                            let color = if strong {
                                palette.fg_muted
                            } else if hover.is_some() {
                                palette.border.opacity(0.4)
                            } else {
                                palette.border_strong
                            };
                            finish(line, color, window);
                        }
                        for (ix, place) in places.iter().enumerate() {
                            let ink = tint(&palette, groups[ix]);
                            let fill = if lit(ix) { ink } else { ink.opacity(0.3) };
                            ring(
                                at(origin, spot(*place)),
                                Pixels::from(radii[ix]),
                                (fill, palette.bg),
                                stroke / 2.0,
                                window,
                            );
                        }
                    },
                )
                .absolute()
                .inset_0(),
            )
            .children(labels)
            .children(tooltip)
            .child(measure(measured, |graph| &mut graph.bounds))
    }
}

#[cfg(test)]
mod tests {
    use super::Force;

    #[test]
    fn edges_pull_their_nodes_closer_than_strangers() {
        let mut force = Force::new(4);
        let edges = [(0, 1), (1, 2)];
        while !force.settled() {
            force.step(&edges);
        }
        let far = |a: usize, b: usize| {
            (force.places[a].0 - force.places[b].0).hypot(force.places[a].1 - force.places[b].1)
        };
        assert!(
            far(0, 1) < far(0, 3) && far(1, 2) < far(2, 3),
            "linked nodes sit nearer than the loner: {:?}",
            force.places
        );
        force.pin(3, (0.1, 0.1));
        force.step(&edges);
        assert_eq!(force.places[3], (0.1, 0.1), "a pinned node stays put");
    }
}
