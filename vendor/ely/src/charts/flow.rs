use std::rc::Rc;

use gpui::{
    App, Bounds, Div, ElementId, Entity, HoverListenerMode, InteractiveElement, IntoElement,
    MouseMoveEvent, ParentElement, PathBuilder, Pixels, Refineable, RenderOnce, SharedString,
    StatefulInteractiveElement, StyleRefinement, Styled, Window, canvas, div, fill, prelude::*,
};

use super::{
    axes::anchored,
    geometry::Rect,
    paint::{at, finish, measure, place, tint},
    parts::ChartTooltip,
    plot::Format,
    scale::compact,
    series::drawable,
};
use crate::{
    theme::{ActiveTheme, Radius, TextSize},
    typography::tabular,
};

/// A flow chart's own state: its box and what the pointer is on.
#[derive(Default)]
struct Flows {
    bounds: Bounds<Pixels>,
    hover: Option<Hover>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Hover {
    Node(usize),
    Link(usize),
}

/// A link's ribbon: where it leaves its source and meets its target, the tops of both ends, and its thickness.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Ribbon {
    pub from: (f32, f32),
    pub to: (f32, f32),
    pub thick: f32,
}

impl Ribbon {
    /// The ribbon's top edge at a share of the way along: a curve level at both ends.
    fn top(&self, share: f32) -> (f32, f32) {
        let middle = (self.from.0 + self.to.0) / 2.0;
        let (a, b, c, d) = (
            (self.from.0, self.from.1),
            (middle, self.from.1),
            (middle, self.to.1),
            (self.to.0, self.to.1),
        );
        let bez = |p: [f32; 4]| {
            let t = share;
            let u = 1.0 - t;
            u * u * u * p[0] + 3.0 * u * u * t * p[1] + 3.0 * u * t * t * p[2] + t * t * t * p[3]
        };
        (bez([a.0, b.0, c.0, d.0]), bez([a.1, b.1, c.1, d.1]))
    }

    /// Whether a place lies on the ribbon.
    fn holds(&self, (x, y): (f32, f32)) -> bool {
        if x < self.from.0 || x > self.to.0 {
            return false;
        }
        let (mut low, mut high) = (0.0f32, 1.0f32);
        for _ in 0..24 {
            let middle = (low + high) / 2.0;
            if self.top(middle).0 < x {
                low = middle
            } else {
                high = middle
            }
        }
        let top = self.top(low).1;
        y >= top && y <= top + self.thick
    }
}

/// What flows into node `ix` and out of it.
fn flows_at(links: &[(usize, usize, f64)], ix: usize) -> (f64, f64) {
    links
        .iter()
        .fold((0.0, 0.0), |(inward, outward), (from, to, value)| {
            (
                inward + if *to == ix { *value } else { 0.0 },
                outward + if *from == ix { *value } else { 0.0 },
            )
        })
}

/// What passes through node `ix`: the more of what comes in and what goes out.
fn passing(links: &[(usize, usize, f64)], ix: usize) -> f64 {
    let (inward, outward) = flows_at(links, ix);
    inward.max(outward)
}

/// Lays out a flow: a column per step from the sources, each node as tall as what passes through it, each link a ribbon between its ends.
pub(crate) fn sankey(
    count: usize,
    links: &[(usize, usize, f64)],
    frame: Rect,
    (node, padding): (f32, f32),
) -> (Vec<Rect>, Vec<Ribbon>) {
    let mut column = vec![0usize; count];
    for _ in 0..count {
        for (from, to, _) in links {
            column[*to] = column[*to].max(column[*from] + 1);
        }
    }
    assert!(
        links
            .iter()
            .all(|(from, to, _)| column[*to] > column[*from]),
        "a flow cannot loop back"
    );
    let through: Vec<f64> = (0..count).map(|ix| passing(links, ix)).collect();
    let columns = column.iter().max().map_or(1, |last| last + 1);
    let placed = &column;
    let members = |c: usize| (0..count).filter(move |ix| placed[*ix] == c);
    let total = |c: usize| members(c).map(|ix| through[ix]).sum::<f64>();
    let biggest = (0..columns).map(total).fold(0.0, f64::max);
    let scale = (0..columns)
        .map(|c| {
            let gaps = members(c).count().saturating_sub(1);
            f64::from((frame.h - padding * gaps as f32).max(0.0)) / (total(c) / biggest)
        })
        .fold(f64::INFINITY, f64::min);
    let height = |value: f64| match biggest > 0.0 {
        true => (value / biggest * scale) as f32,
        false => 0.0,
    };
    let mut nodes = vec![Rect::default(); count];
    for c in 0..columns {
        let x = if columns > 1 {
            frame.x + (frame.w - node) * c as f32 / (columns - 1) as f32
        } else {
            frame.x
        };
        let tall: f32 = members(c).map(|ix| height(through[ix])).sum::<f32>()
            + padding * members(c).count().saturating_sub(1) as f32;
        let mut y = frame.y + (frame.h - tall) / 2.0;
        for ix in members(c) {
            nodes[ix] = Rect {
                x,
                y,
                w: node,
                h: height(through[ix]),
            };
            y += nodes[ix].h + padding;
        }
    }
    let mut ribbons: Vec<Ribbon> = links
        .iter()
        .map(|link| Ribbon {
            from: (0.0, 0.0),
            to: (0.0, 0.0),
            thick: height(link.2),
        })
        .collect();
    let mut order: Vec<usize> = (0..links.len()).collect();
    let mut stacked = vec![0.0f32; count];
    order.sort_by(|a, b| nodes[links[*a].1].y.total_cmp(&nodes[links[*b].1].y));
    for ix in &order {
        let from = links[*ix].0;
        ribbons[*ix].from = (nodes[from].x + node, nodes[from].y + stacked[from]);
        stacked[from] += ribbons[*ix].thick;
    }
    stacked.fill(0.0);
    order.sort_by(|a, b| nodes[links[*a].0].y.total_cmp(&nodes[links[*b].0].y));
    for ix in &order {
        let to = links[*ix].1;
        ribbons[*ix].to = (nodes[to].x, nodes[to].y + stacked[to]);
        stacked[to] += ribbons[*ix].thick;
    }
    (nodes, ribbons)
}

/// How things flow from where they start to where they end: nodes in columns, ribbons as thick as what passes. Hover a node or a ribbon to read it.
#[derive(IntoElement)]
pub struct SankeyChart {
    base: Div,
    id: ElementId,
    nodes: Vec<SharedString>,
    links: Vec<(usize, usize, f64)>,
    format: Format,
}

impl SankeyChart {
    pub fn new(
        id: impl Into<ElementId>,
        nodes: impl IntoIterator<Item = impl Into<SharedString>>,
    ) -> Self {
        let nodes = nodes.into_iter().map(Into::into).collect();
        Self {
            base: div(),
            id: id.into(),
            nodes,
            links: Vec::new(),
            format: Rc::new(compact),
        }
    }

    /// A flow of `value` from node `from` to node `to`, by their places in the list.
    pub fn link(mut self, from: usize, to: usize, value: f64) -> Self {
        if from >= self.nodes.len() || to >= self.nodes.len() || from == to {
            log::error!(
                "sankey: a link {from} -> {to} among {} nodes; skipped",
                self.nodes.len()
            );
            return self;
        }
        assert!(
            drawable(value) && value > 0.0,
            "a link carries a positive value, within charts::LIMIT"
        );
        self.links.push((from, to, value));
        self
    }

    pub fn format(mut self, format: impl Fn(f64) -> String + 'static) -> Self {
        self.format = Rc::new(format);
        self
    }
}

impl Styled for SankeyChart {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for SankeyChart {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let flows: Entity<Flows> =
            window.use_keyed_state((self.id.clone(), "flows"), cx, |_, _| Flows::default());
        let (bounds, hover) = (flows.read(cx).bounds, flows.read(cx).hover);
        let hover = hover.filter(|hover| match hover {
            Hover::Node(ix) => *ix < self.nodes.len(),
            Hover::Link(ix) => *ix < self.links.len(),
        });
        let theme = cx.theme();
        let (colors, sizes, rem) = (theme.colors.clone(), theme.chart(), window.rem_size());
        let pixels = |length: gpui::Rems| f32::from(length.to_pixels(rem));
        let room = pixels(sizes.label) * 1.5;
        let frame = Rect {
            x: room,
            y: pixels(sizes.inset),
            w: (f32::from(bounds.size.width) - room * 2.0).max(0.0),
            h: (f32::from(bounds.size.height) - pixels(sizes.inset) * 2.0).max(0.0),
        };
        let hover = hover.filter(|_| frame.w > 0.0);
        let (nodes, ribbons) = if frame.w > 0.0 {
            sankey(
                self.nodes.len(),
                &self.links,
                frame,
                (pixels(sizes.inset), pixels(sizes.inset) * 1.5),
            )
        } else {
            (vec![Rect::default(); self.nodes.len()], Vec::new())
        };
        let lit = |link: usize| match hover {
            Some(Hover::Link(ix)) => ix == link,
            Some(Hover::Node(node)) => self.links[link].0 == node || self.links[link].1 == node,
            None => true,
        };
        let strengths: Vec<f32> = (0..ribbons.len())
            .map(|ix| {
                if hover.is_none() {
                    0.28
                } else if lit(ix) {
                    0.5
                } else {
                    0.08
                }
            })
            .collect();
        let format = self.format.clone();
        let text = |content: String, strong: bool| {
            div()
                .flex_none()
                .whitespace_nowrap()
                .text_size(theme.text_size(TextSize::Xs))
                .text_color(if strong { colors.fg } else { colors.fg_muted })
                .child(content)
        };
        let middle = frame.x + frame.w / 2.0;
        let labels: Vec<Div> = nodes
            .iter()
            .zip(&self.nodes)
            .enumerate()
            .map(|(ix, (rect, name))| {
                let right = rect.x < middle;
                let x = if right {
                    rect.x - pixels(sizes.inset)
                } else {
                    rect.x + rect.w + pixels(sizes.inset)
                };
                let label = div()
                    .flex()
                    .flex_col()
                    .when(right, |label| label.items_end())
                    .child(text(name.to_string(), hover == Some(Hover::Node(ix))))
                    .child(tabular(text(format(passing(&self.links, ix)), false)));
                div()
                    .absolute()
                    .left(Pixels::from(x))
                    .top(Pixels::from(rect.y + rect.h / 2.0))
                    .w_0()
                    .h_0()
                    .flex()
                    .items_center()
                    .when(right, |anchor| anchor.justify_end())
                    .child(label)
            })
            .collect();
        let tooltip = hover.map(|hover| match hover {
            Hover::Link(ix) => {
                let (from, to, value) = self.links[ix];
                let place = ribbons[ix].top(0.5);
                let card = ChartTooltip::new(format!("{} → {}", self.nodes[from], self.nodes[to]))
                    .row(Some(tint(&colors, from)), "Flow", format(value));
                anchored(
                    (place.0, place.1 + ribbons[ix].thick / 2.0),
                    place.0 > f32::from(bounds.size.width) * 0.6,
                    card,
                )
            }
            Hover::Node(ix) => {
                let (inward, outward) = flows_at(&self.links, ix);
                let rect = nodes[ix];
                let card = ChartTooltip::new(self.nodes[ix].clone())
                    .row(None, "In", format(inward))
                    .row(None, "Out", format(outward));
                anchored(
                    (rect.x + rect.w, rect.y + rect.h / 2.0),
                    rect.x > middle,
                    card,
                )
            }
        });
        let (painted, woven, corner) = (
            nodes.clone(),
            ribbons.clone(),
            theme.radius(Radius::Sm).to_pixels(rem),
        );
        let sources: Vec<usize> = self.links.iter().map(|link| link.0).collect();
        let palette = colors.clone();
        let (hit_nodes, hit_ribbons) = (nodes, ribbons);
        let (moved, left) = (flows.clone(), flows.clone());
        let mut root = div()
            .debug_selector(|| "chart-root".into())
            .relative()
            .h(sizes.height);
        root.style().refine(self.base.style());
        root.id((self.id.clone(), "sankey"))
            .on_mouse_move(move |event: &MouseMoveEvent, _, cx| {
                let offset = event.position - moved.read(cx).bounds.origin;
                let place = (f32::from(offset.x), f32::from(offset.y));
                let next = hit_nodes
                    .iter()
                    .position(|rect| rect.contains(place))
                    .map(Hover::Node)
                    .or_else(|| {
                        hit_ribbons
                            .iter()
                            .position(|ribbon| ribbon.holds(place))
                            .map(Hover::Link)
                    });
                moved.update(cx, |flows, cx| {
                    if flows.hover != next {
                        flows.hover = next;
                        cx.notify();
                    }
                })
            })
            .hover_listener_mode(HoverListenerMode::InputModalityIndependent)
            .on_hover(move |inside, _, cx| {
                if !*inside {
                    left.update(cx, |flows, cx| {
                        flows.hover = None;
                        cx.notify();
                    })
                }
            })
            .child(
                canvas(
                    |_, _, _| {},
                    move |bounds, _, window, _| {
                        let origin = bounds.origin;
                        for (ix, ribbon) in woven.iter().enumerate() {
                            let mut path = PathBuilder::fill();
                            path.move_to(at(origin, ribbon.from));
                            let middle = (ribbon.from.0 + ribbon.to.0) / 2.0;
                            path.cubic_bezier_to(
                                at(origin, ribbon.to),
                                at(origin, (middle, ribbon.from.1)),
                                at(origin, (middle, ribbon.to.1)),
                            );
                            path.line_to(at(origin, (ribbon.to.0, ribbon.to.1 + ribbon.thick)));
                            let low = |(x, y): (f32, f32)| (x, y + ribbon.thick);
                            path.cubic_bezier_to(
                                at(origin, low(ribbon.from)),
                                at(origin, low((middle, ribbon.to.1))),
                                at(origin, low((middle, ribbon.from.1))),
                            );
                            path.close();
                            finish(
                                path,
                                tint(&palette, sources[ix]).opacity(strengths[ix]),
                                window,
                            );
                        }
                        for (ix, rect) in painted.iter().enumerate() {
                            window.paint_quad(
                                fill(place(origin, *rect), tint(&palette, ix)).corner_radii(corner),
                            );
                        }
                    },
                )
                .absolute()
                .inset_0(),
            )
            .children(labels)
            .children(tooltip)
            .child(measure(flows, |flows| &mut flows.bounds))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_flow_stacks_columns_and_links_meet_their_ends() {
        let links = [(0, 2, 30.0), (1, 2, 10.0), (2, 3, 25.0), (2, 4, 15.0)];
        let frame = Rect {
            x: 0.0,
            y: 0.0,
            w: 300.0,
            h: 100.0,
        };
        let (nodes, ribbons) = sankey(5, &links, frame, (10.0, 20.0));
        assert_eq!(
            (nodes[0].x, nodes[2].x, nodes[3].x),
            (0.0, 145.0, 290.0),
            "three columns edge to edge"
        );
        assert_eq!(
            (nodes[2].h, nodes[0].h),
            (80.0, 60.0),
            "the tightest column sets the scale for all"
        );
        assert_eq!(
            ribbons[1].to.1,
            nodes[2].y + ribbons[0].thick,
            "the second inflow lands under the first"
        );
        assert!(ribbons[0].holds((ribbons[0].top(0.5).0, ribbons[0].top(0.5).1 + 1.0)));
        assert!(!ribbons[0].holds((5.0, 50.0)), "left of the ribbon");
    }
}
