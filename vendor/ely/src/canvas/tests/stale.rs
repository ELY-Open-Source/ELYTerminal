use gpui::{AnyElement, Entity, IntoElement, ParentElement, Styled, TestAppContext, div, px};

use super::{Stage, settle, stage};
use crate::canvas::{
    Edge, Frame, HistoryPanel, InfiniteCanvas, Link, MindMap, Node, NodeGraph, Port, Shape,
    ShapeKind, Tool, ToolPalette, Topic, Viewport,
};

/// Each part handed a key or an edge that its list no longer holds.
fn lagging(_: &Stage, _: Entity<Stage>) -> AnyElement {
    let view = Viewport::new(0.0, 0.0, 1.0);
    let load = Node::new("load", "Load", (0.0, 0.0)).output(Port::new("image", "Image", "image"));
    let shape = Shape::new("a", "A", ShapeKind::Rect, Frame::new(0.0, 0.0, 40.0, 40.0));
    div()
        .size_full()
        .child(div().h(px(120.0)).child(NodeGraph::new(
            "graph",
            view,
            [load],
            [Edge::new(("load", "image"), ("gone", "image"))],
        )))
        .child(
            div().h(px(120.0)).child(
                InfiniteCanvas::new("plane", view)
                    .shapes([shape])
                    .links([Link::new("l", "a", "gone")]),
            ),
        )
        .child(
            div().h(px(120.0)).child(
                MindMap::new("map", Topic::new("root", "Root"), view).selected(Some("gone")),
            ),
        )
        .child(ToolPalette::new("tools", Tool::Brush).tools([Tool::Select]))
        .child(HistoryPanel::new("history", ["Draw"], 4))
        .into_any_element()
}

#[gpui::test]
fn gone_nodes_shapes_topics_tools_and_steps_draw(cx: &mut TestAppContext) {
    let (_, cx) = stage(lagging, cx);
    settle(cx);
}
