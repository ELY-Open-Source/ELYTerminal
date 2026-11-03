use gpui::SharedString;

use super::view::Frame;

/// A socket on a node that carries one kind of value in or out.
#[derive(Clone, Debug, PartialEq)]
pub struct Port {
    pub key: SharedString,
    pub name: SharedString,
    pub kind: SharedString,
}

impl Port {
    pub fn new(
        key: impl Into<SharedString>,
        name: impl Into<SharedString>,
        kind: impl Into<SharedString>,
    ) -> Self {
        Self {
            key: key.into(),
            name: name.into(),
            kind: kind.into(),
        }
    }
}

/// A box of work at a point on the canvas, its inputs down its left side and its outputs down its right.
#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    pub key: SharedString,
    pub title: SharedString,
    pub at: (f32, f32),
    pub inputs: Vec<Port>,
    pub outputs: Vec<Port>,
}

impl Node {
    pub fn new(
        key: impl Into<SharedString>,
        title: impl Into<SharedString>,
        at: (f32, f32),
    ) -> Self {
        Self {
            key: key.into(),
            title: title.into(),
            at,
            inputs: Vec::new(),
            outputs: Vec::new(),
        }
    }

    pub fn input(mut self, port: Port) -> Self {
        self.inputs.push(port);
        self
    }

    pub fn output(mut self, port: Port) -> Self {
        self.outputs.push(port);
        self
    }

    pub(crate) fn frame(&self) -> Frame {
        let rows = self.inputs.len().max(self.outputs.len()) as f32;
        Frame::new(self.at.0, self.at.1, WIDE, HEAD + ROW * rows + FOOT)
    }

    /// Where a port's socket sits, in canvas units: on the left edge for an input, the right for an output.
    pub(crate) fn socket(&self, out: bool, row: usize) -> (f32, f32) {
        let x = if out { self.at.0 + WIDE } else { self.at.0 };
        (x, self.at.1 + HEAD + ROW * row as f32 + ROW / 2.0)
    }

    fn row(&self, out: bool, port: &str) -> Option<usize> {
        let ports = if out { &self.outputs } else { &self.inputs };
        ports.iter().position(|each| each.key == port)
    }
}

/// A wire from one node's output to another's input.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Edge {
    pub from: (SharedString, SharedString),
    pub to: (SharedString, SharedString),
}

impl Edge {
    pub fn new(
        from: (impl Into<SharedString>, impl Into<SharedString>),
        to: (impl Into<SharedString>, impl Into<SharedString>),
    ) -> Self {
        Self {
            from: (from.0.into(), from.1.into()),
            to: (to.0.into(), to.1.into()),
        }
    }
}

/// A node's width, its title's height, a port row's height and the room under the last row, in canvas units.
pub(crate) const WIDE: f32 = 200.0;
pub(crate) const HEAD: f32 = 32.0;
pub(crate) const ROW: f32 = 28.0;
const FOOT: f32 = 8.0;

/// A port a press or a release reaches: its node and key, whether it is an output, and its kind.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Socket {
    pub node: SharedString,
    pub port: SharedString,
    pub out: bool,
    pub kind: SharedString,
}

/// Where a wire starts and ends in canvas units, and the kind it carries.
pub(crate) type Ends = ((f32, f32), (f32, f32), SharedString);

/// An edge's ends; none when a node or port it names left the graph.
pub(crate) fn ends(nodes: &[Node], edge: &Edge) -> Option<Ends> {
    let node = |key: &SharedString| nodes.iter().find(|node| node.key == *key);
    let ends = node(&edge.from.0)
        .zip(node(&edge.to.0))
        .and_then(|(from, to)| {
            let (out, input) = (from.row(true, &edge.from.1)?, to.row(false, &edge.to.1)?);
            Some((
                from.socket(true, out),
                to.socket(false, input),
                from.outputs[out].kind.clone(),
            ))
        });
    if ends.is_none() {
        log::error!("node editor: edge {edge:?} names a node or port gone from the graph");
    }
    ends
}

/// The socket within `reach` of `point`, the topmost node first.
pub(crate) fn socket_at(nodes: &[Node], point: (f32, f32), reach: f32) -> Option<Socket> {
    nodes.iter().rev().find_map(|node| {
        let near =
            |(x, y): (f32, f32)| (x - point.0).abs() <= reach && (y - point.1).abs() <= reach;
        let ins = node
            .inputs
            .iter()
            .enumerate()
            .map(|(row, port)| (false, row, port));
        let outs = node
            .outputs
            .iter()
            .enumerate()
            .map(|(row, port)| (true, row, port));
        ins.chain(outs)
            .find(|(out, row, _)| near(node.socket(*out, *row)))
            .map(|(out, _, port)| Socket {
                node: node.key.clone(),
                port: port.key.clone(),
                out,
                kind: port.kind.clone(),
            })
    })
}

/// Whether a wire may run from `from` to `to`: out of an output, into an input of the same kind, on another node.
pub(crate) fn fits(from: &Socket, to: &Socket) -> bool {
    from.out && !to.out && from.node != to.node && from.kind == to.kind
}

/// The kinds of value on the graph, in the order they first appear, which gives each its hue.
pub(crate) fn kinds(nodes: &[Node]) -> Vec<SharedString> {
    nodes
        .iter()
        .flat_map(|node| node.inputs.iter().chain(&node.outputs))
        .fold(Vec::new(), |mut kinds, port| {
            if !kinds.contains(&port.kind) {
                kinds.push(port.kind.clone());
            }
            kinds
        })
}

#[cfg(test)]
mod tests {
    use super::{Edge, HEAD, Node, Port, ROW, WIDE, ends, fits, kinds, socket_at};

    fn graph() -> Vec<Node> {
        vec![
            Node::new("load", "Load image", (0.0, 0.0))
                .output(Port::new("image", "Image", "image")),
            Node::new("blur", "Blur", (300.0, 0.0))
                .input(Port::new("image", "Image", "image"))
                .input(Port::new("radius", "Radius", "number"))
                .output(Port::new("image", "Image", "image")),
        ]
    }

    #[test]
    fn sockets_sit_on_the_sides_a_row_apart() {
        let nodes = graph();
        let (from, to, kind) =
            ends(&nodes, &Edge::new(("load", "image"), ("blur", "image"))).expect("both ends");
        assert_eq!(from, (WIDE, HEAD + ROW / 2.0));
        assert_eq!(to, (300.0, HEAD + ROW / 2.0));
        assert_eq!(kind, "image");
        let radius =
            socket_at(&nodes, (302.0, HEAD + ROW * 1.5 - 3.0), 6.0).expect("the radius input");
        assert_eq!(
            (radius.node.as_ref(), radius.port.as_ref(), radius.out),
            ("blur", "radius", false)
        );
        assert_eq!(socket_at(&nodes, (150.0, 40.0), 6.0), None);
        assert_eq!(
            ends(&nodes, &Edge::new(("load", "image"), ("gone", "image"))),
            None
        );
        assert_eq!(
            ends(&nodes, &Edge::new(("load", "gone"), ("blur", "image"))),
            None
        );
    }

    #[test]
    fn a_wire_runs_out_to_in_between_nodes_of_one_kind() {
        let nodes = graph();
        let at = |x: f32, row: f32| {
            socket_at(&nodes, (x, HEAD + ROW * row + ROW / 2.0), 4.0).expect("a socket")
        };
        let (image_out, blur_in, radius_in) = (at(WIDE, 0.0), at(300.0, 0.0), at(300.0, 1.0));
        assert!(fits(&image_out, &blur_in));
        assert!(!fits(&image_out, &radius_in), "an image is no number");
        assert!(!fits(&blur_in, &image_out), "wires run out to in");
        assert_eq!(kinds(&nodes), ["image", "number"]);
    }
}
