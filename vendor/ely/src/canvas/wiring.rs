use std::rc::Rc;

use gpui::{App, Bounds, Entity, EntityId, Pixels, SharedString, Window};

use super::graph::{Edge, Node, Socket, fits, socket_at};

pub(super) type OnNudge = Rc<dyn Fn(&SharedString, (f32, f32), &mut Window, &mut App)>;
pub(super) type OnEdge = Rc<dyn Fn(&Edge, &mut Window, &mut App)>;

/// A socket's reach for a press or a release, in view pixels.
pub(super) const REACH: f32 = 10.0;

/// What a press on the graph set going.
#[derive(Clone, Debug, Default)]
pub(super) enum Pull {
    #[default]
    Idle,
    Moving {
        key: SharedString,
        from: (f32, f32),
        to: (f32, f32),
    },
    Wiring {
        from: Socket,
        lifted: Option<Edge>,
        to: (f32, f32),
    },
}

/// The graph's own state: the pull under way and the box it covers.
#[derive(Default)]
pub(super) struct Hold {
    pub(super) pull: Pull,
    pub(super) bounds: Bounds<Pixels>,
}

/// A drag on a graph, and whose graph it is.
pub(super) struct Pulling {
    pub(super) owner: EntityId,
}

/// What a finished pull asks the owner, given the graph as it was.
pub(super) struct Ending {
    pub(super) nodes: Rc<Vec<Node>>,
    pub(super) edges: Rc<Vec<Edge>>,
    pub(super) zoom: f32,
    pub(super) on_move: Option<OnNudge>,
    pub(super) on_connect: Option<OnEdge>,
    pub(super) on_disconnect: Option<OnEdge>,
}

impl Ending {
    pub(super) fn run(&self, hold: &Entity<Hold>, window: &mut Window, cx: &mut App) {
        let pull = hold.update(cx, |hold, cx| {
            cx.notify();
            std::mem::take(&mut hold.pull)
        });
        match pull {
            Pull::Moving { key, from, to } => {
                let by = (to.0 - from.0, to.1 - from.1);
                if by != (0.0, 0.0) {
                    log::info!("node graph: move {key} by {by:?}");
                    if let Some(on_move) = &self.on_move {
                        on_move(&key, by, window, cx);
                    }
                }
            }
            Pull::Wiring { from, lifted, to } => {
                let kind = self
                    .nodes
                    .iter()
                    .find(|node| node.key == from.node)
                    .and_then(|node| {
                        let port = node.outputs.iter().find(|port| port.key == from.port)?;
                        Some(port.kind.clone())
                    });
                let Some(kind) = kind else {
                    log::error!(
                        "node graph: the wire's port {}.{} left the graph",
                        from.node,
                        from.port
                    );
                    return;
                };
                let from = Socket { kind, ..from };
                let target =
                    socket_at(&self.nodes, to, REACH / self.zoom).filter(|to| fits(&from, to));
                let made = target.map(|to| {
                    Edge::new((from.node.clone(), from.port.clone()), (to.node, to.port))
                });
                if made.is_some() && made == lifted {
                    return;
                }
                let replaced = made.as_ref().and_then(|made| {
                    self.edges
                        .iter()
                        .find(|edge| edge.to == made.to && Some(*edge) != lifted.as_ref())
                });
                for gone in lifted.iter().chain(replaced) {
                    log::info!("node graph: unwire {gone:?}");
                    if let Some(on_disconnect) = &self.on_disconnect {
                        on_disconnect(gone, window, cx);
                    }
                }
                if let Some(made) = &made {
                    log::info!("node graph: wire {made:?}");
                    if let Some(on_connect) = &self.on_connect {
                        on_connect(made, window, cx);
                    }
                }
            }
            Pull::Idle => {}
        }
    }
}
