use std::rc::Rc;

use gpui::{
    AnyElement, App, AppContext as _, Bounds, ElementId, EmptyView, Entity, FontWeight,
    InteractiveElement, IntoElement, MouseButton, ParentElement, Pixels, Point, RenderOnce,
    SharedString, StatefulInteractiveElement, Styled, Window, canvas, div, prelude::*,
};

use super::{
    graph::{Edge, HEAD, Node, ROW, Socket, ends, kinds},
    paint::{finish, in_view, wire},
    plane::{InfiniteCanvas, OnViewport},
    view::Viewport,
    wiring::{Ending, Hold, OnEdge, OnNudge, Pull, Pulling},
};
use crate::{
    theme::{ActiveTheme, Radius, TextSize},
    typography::Ellipsis,
};

type OnKeys = Rc<dyn Fn(&[SharedString], &mut Window, &mut App)>;

/// Nodes joined by wires on an endless plane, as in a shader or pipeline editor. A node's title drags it; a drag out of an output draws a wire that lands on an input of the same kind. An input takes one wire, so a new one replaces the old; a press on a wired input lifts its wire to land elsewhere or be dropped. Each kind of value has its hue. The owner keeps the graph and hears each change.
#[derive(IntoElement)]
pub struct NodeGraph {
    id: ElementId,
    viewport: Viewport,
    nodes: Vec<Node>,
    edges: Vec<Edge>,
    selected: Vec<SharedString>,
    on_select: Option<OnKeys>,
    on_move: Option<OnNudge>,
    on_connect: Option<OnEdge>,
    on_disconnect: Option<OnEdge>,
    on_viewport: Option<OnViewport>,
    fixed: bool,
}

impl NodeGraph {
    pub fn new(
        id: impl Into<ElementId>,
        viewport: Viewport,
        nodes: impl IntoIterator<Item = Node>,
        edges: impl IntoIterator<Item = Edge>,
    ) -> Self {
        Self {
            id: id.into(),
            viewport,
            nodes: nodes.into_iter().collect(),
            edges: edges.into_iter().collect(),
            selected: Vec::new(),
            on_select: None,
            on_move: None,
            on_connect: None,
            on_disconnect: None,
            on_viewport: None,
            fixed: false,
        }
    }

    /// Wires stay as the owner gives them: only sockets that hold a wire show, none starts or lifts one, and a press there pans.
    pub fn fixed_wires(mut self) -> Self {
        self.fixed = true;
        self
    }

    pub fn selected(mut self, keys: impl IntoIterator<Item = impl Into<SharedString>>) -> Self {
        self.selected = keys.into_iter().map(Into::into).collect();
        self
    }

    pub fn on_select(
        mut self,
        handler: impl Fn(&[SharedString], &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_select = Some(Rc::new(handler));
        self
    }

    /// Gets a dragged node's key and how far it went, in canvas units.
    pub fn on_move(
        mut self,
        handler: impl Fn(&SharedString, (f32, f32), &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_move = Some(Rc::new(handler));
        self
    }

    pub fn on_connect(mut self, handler: impl Fn(&Edge, &mut Window, &mut App) + 'static) -> Self {
        self.on_connect = Some(Rc::new(handler));
        self
    }

    pub fn on_disconnect(
        mut self,
        handler: impl Fn(&Edge, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_disconnect = Some(Rc::new(handler));
        self
    }

    pub fn on_viewport(
        mut self,
        handler: impl Fn(Viewport, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_viewport = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for NodeGraph {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id;
        let view = self.viewport;
        let hold: Entity<Hold> =
            window.use_keyed_state((id.clone(), "hold"), cx, |_, _| Hold::default());
        let owner = hold.entity_id();
        let nodes = Rc::new(self.nodes);
        let edges = Rc::new(self.edges);
        let kinds = kinds(&nodes);
        let theme = cx.theme();
        let rem = window.rem_size();
        let colors = theme.colors.clone();
        let hue = move |kind: &SharedString| {
            let ix = kinds
                .iter()
                .position(|each| each == kind)
                .expect("a kind on the graph");
            colors.hue(ix % 8, format_args!("kind {kind}"))
        };
        let text = theme.text_size(TextSize::Sm).to_pixels(rem) * view.zoom;
        let radius = theme.radius(Radius::Lg).to_pixels(rem) * view.zoom;
        let stroke = theme.canvas().stroke.to_pixels(rem) * 1.5;
        let dot = Pixels::from(10.0 * view.zoom);
        let pull = hold.read(cx).pull.clone();
        let point_of = move |at: Point<Pixels>, bounds: Bounds<Pixels>| {
            let local = at - bounds.origin;
            view.to_canvas((f32::from(local.x), f32::from(local.y)))
        };
        let ending = Rc::new(Ending {
            nodes: nodes.clone(),
            edges: edges.clone(),
            zoom: view.zoom,
            on_move: self.on_move,
            on_connect: self.on_connect,
            on_disconnect: self.on_disconnect,
        });
        let (fg, surface, border, accent) = (
            theme.colors.fg,
            theme.colors.surface,
            theme.colors.border,
            theme.colors.accent,
        );
        let cards: Vec<AnyElement> = nodes
            .iter()
            .map(|node| {
                let frame = in_view(&view, &node.frame());
                let chosen = self.selected.contains(&node.key);
                let (grab, key, on_select) =
                    (hold.clone(), node.key.clone(), self.on_select.clone());
                let head = div()
                    .id((id.clone(), format!("head-{}", node.key)))
                    .h(Pixels::from(HEAD * view.zoom))
                    .flex()
                    .items_center()
                    .px(text * 0.75)
                    .border_b_1()
                    .border_color(border)
                    .font_weight(FontWeight::SEMIBOLD)
                    .cursor_grab()
                    .on_mouse_down(MouseButton::Left, move |event, window, cx| {
                        cx.stop_propagation();
                        let bounds = grab.read(cx).bounds;
                        let at = point_of(event.position, bounds);
                        log::info!("node graph: select {key}");
                        if let Some(on_select) = &on_select {
                            on_select(std::slice::from_ref(&key), window, cx);
                        }
                        grab.update(cx, |hold, _| {
                            hold.pull = Pull::Moving {
                                key: key.clone(),
                                from: at,
                                to: at,
                            }
                        });
                    })
                    .on_drag(Pulling { owner }, |_, _, _, cx| cx.new(|_| EmptyView))
                    .child(Ellipsis::new(node.title.clone()));
                let socket = |out: bool, row: usize, port: &super::graph::Port| {
                    let fixed = self.fixed;
                    let (press, edges, node_key) = (hold.clone(), edges.clone(), node.key.clone());
                    let (port_key, kind) = (port.key.clone(), port.kind.clone());
                    let nodes = nodes.clone();
                    div()
                        .id((
                            id.clone(),
                            format!(
                                "socket-{}-{}-{}",
                                node.key,
                                if out { "out" } else { "in" },
                                port.key
                            ),
                        ))
                        .absolute()
                        .top(Pixels::from((ROW * row as f32 + ROW / 2.0) * view.zoom) - dot / 2.0)
                        .when(out, |socket| socket.right(-dot / 2.0))
                        .when(!out, |socket| socket.left(-dot / 2.0))
                        .size(dot)
                        .rounded_full()
                        .border_1()
                        .border_color(surface)
                        .bg(hue(&kind))
                        .cursor_crosshair()
                        .on_mouse_down(MouseButton::Left, move |event, _, cx| {
                            if fixed {
                                return;
                            }
                            cx.stop_propagation();
                            let bounds = press.read(cx).bounds;
                            let at = point_of(event.position, bounds);
                            let here = Socket {
                                node: node_key.clone(),
                                port: port_key.clone(),
                                out,
                                kind: kind.clone(),
                            };
                            let pull = if out {
                                Pull::Wiring {
                                    from: here,
                                    lifted: None,
                                    to: at,
                                }
                            } else {
                                let wired = edges
                                    .iter()
                                    .find(|edge| edge.to == (node_key.clone(), port_key.clone()));
                                let source = wired.and_then(|edge| {
                                    let port = nodes
                                        .iter()
                                        .find(|node| node.key == edge.from.0)?
                                        .outputs
                                        .iter()
                                        .find(|port| port.key == edge.from.1);
                                    if port.is_none() {
                                        log::error!("node editor: edge {edge:?} starts at a port gone from the graph");
                                    }
                                    Some(edge).zip(port)
                                });
                                match source {
                                    Some((edge, port)) => {
                                        Pull::Wiring {
                                            from: Socket {
                                                node: edge.from.0.clone(),
                                                port: edge.from.1.clone(),
                                                out: true,
                                                kind: port.kind.clone(),
                                            },
                                            lifted: Some(edge.clone()),
                                            to: at,
                                        }
                                    }
                                    None => Pull::Idle,
                                }
                            };
                            press.update(cx, |hold, cx| {
                                hold.pull = pull;
                                cx.notify();
                            });
                        })
                        .on_drag(Pulling { owner }, |_, _, _, cx| cx.new(|_| EmptyView))
                };
                let rows = node.inputs.len().max(node.outputs.len());
                let body = div().relative().children((0..rows).map(|row| {
                    let (input, output) = (node.inputs.get(row), node.outputs.get(row));
                    div()
                        .h(Pixels::from(ROW * view.zoom))
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(text * 0.5)
                        .px(text * 0.75)
                        .child(
                            div()
                                .min_w_0()
                                .children(input.map(|port| Ellipsis::new(port.name.clone()))),
                        )
                        .child(
                            div()
                                .min_w_0()
                                .children(output.map(|port| Ellipsis::new(port.name.clone()))),
                        )
                }));
                let wired = |out: bool, port: &super::graph::Port| {
                    edges.iter().any(|edge| {
                        let end = if out { &edge.from } else { &edge.to };
                        end.0 == node.key && end.1 == port.key
                    })
                };
                let shown = |out: bool, port: &super::graph::Port| !self.fixed || wired(out, port);
                let sockets: Vec<AnyElement> = node
                    .inputs
                    .iter()
                    .enumerate()
                    .filter(|(_, port)| shown(false, port))
                    .map(|(row, port)| socket(false, row, port).into_any_element())
                    .chain(
                        node.outputs
                            .iter()
                            .enumerate()
                            .filter(|(_, port)| shown(true, port))
                            .map(|(row, port)| socket(true, row, port).into_any_element()),
                    )
                    .collect();
                div()
                    .absolute()
                    .left(Pixels::from(frame.x))
                    .top(Pixels::from(frame.y))
                    .w(Pixels::from(frame.w))
                    .h(Pixels::from(frame.h))
                    .rounded(radius)
                    .border_1()
                    .border_color(if chosen { accent } else { border })
                    .bg(surface)
                    .text_size(text)
                    .line_height(text * 1.3)
                    .text_color(fg)
                    .child(head)
                    .child(body.children(sockets))
                    .into_any_element()
            })
            .collect();
        let wires = {
            let (nodes, edges, hue) = (nodes.clone(), edges.clone(), hue.clone());
            let pending = match &pull {
                Pull::Wiring { from, to, .. } => {
                    let source = nodes.iter().find(|node| node.key == from.node);
                    let wiring = source.and_then(|source| {
                        let row = source
                            .outputs
                            .iter()
                            .position(|port| port.key == from.port)?;
                        Some((
                            source.socket(true, row),
                            *to,
                            source.outputs[row].kind.clone(),
                        ))
                    });
                    if wiring.is_none() {
                        log::error!(
                            "node editor: the wire's port {}.{} left the graph",
                            from.node,
                            from.port
                        );
                    }
                    wiring
                }
                _ => None,
            };
            let lifted = match &pull {
                Pull::Wiring { lifted, .. } => lifted.clone(),
                _ => None,
            };
            canvas(
                |_, _, _| {},
                move |bounds, _, window, _| {
                    let drawn = edges
                        .iter()
                        .filter(|edge| Some(*edge) != lifted.as_ref())
                        .filter_map(|edge| ends(&nodes, edge))
                        .map(|(from, to, kind)| (from, to, hue(&kind)))
                        .chain(
                            pending
                                .iter()
                                .map(|(from, to, kind)| (*from, *to, hue(kind).opacity(0.7))),
                        );
                    for (from, to, ink) in drawn {
                        let (from, to) = (view.to_view(from), view.to_view(to));
                        let pull = ((to.0 - from.0).abs() / 2.0).max(40.0 * view.zoom);
                        finish(wire(from, to, pull, bounds.origin, stroke), ink, window);
                    }
                },
            )
            .absolute()
            .top_0()
            .left_0()
            .size_full()
        };
        let (moved, measured, dropped, released, out) = (
            hold.clone(),
            hold.clone(),
            hold.clone(),
            hold.clone(),
            hold.clone(),
        );
        let (drop_end, up_end, out_end) = (ending.clone(), ending.clone(), ending);
        let layer = div()
            .id((id.clone(), "layer"))
            .absolute()
            .inset_0()
            .on_drag_move::<Pulling>(move |event, _, cx| {
                if event.drag(cx).owner != owner {
                    return;
                }
                let at = point_of(event.event.position, moved.read(cx).bounds);
                moved.update(cx, |hold, cx| {
                    match &mut hold.pull {
                        Pull::Moving { to, .. } | Pull::Wiring { to, .. } => *to = at,
                        Pull::Idle => {}
                    }
                    cx.notify();
                });
            })
            .on_drop(move |_: &Pulling, window, cx| drop_end.run(&dropped, window, cx))
            .on_mouse_up(MouseButton::Left, move |_, window, cx| {
                up_end.run(&released, window, cx)
            })
            .on_mouse_up_out(MouseButton::Left, move |_, window, cx| {
                out_end.run(&out, window, cx)
            })
            .child(
                canvas(
                    move |bounds, window, cx| {
                        if measured.read(cx).bounds != bounds {
                            measured.update(cx, |hold, _| hold.bounds = bounds);
                            window.request_animation_frame();
                        }
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            )
            .child(wires)
            .children(cards);
        let plane = InfiniteCanvas::new(id, view).layer(layer);
        match self.on_viewport {
            Some(on_viewport) => {
                plane.on_viewport(move |next, window, cx| on_viewport(next, window, cx))
            }
            None => plane,
        }
    }
}
