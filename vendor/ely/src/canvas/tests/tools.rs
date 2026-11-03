use gpui::{
    AnyElement, Entity, IntoElement, Modifiers, MouseButton, MouseDownEvent, MouseUpEvent,
    ParentElement, Styled, TestAppContext, VisualTestContext, div, point, px,
};

use super::{Stage, said, say, settle, stage};
use crate::canvas::{
    Frame, InfiniteCanvas, Shape, ShapeKind, Tool, ToolLayer, ToolPalette, Viewport,
};

pub(super) fn at(x: f32, y: f32) -> gpui::Point<gpui::Pixels> {
    point(px(x), px(y))
}

/// Presses at `from`, drags in steps to `to`, and lets go.
pub(super) fn drag(from: (f32, f32), to: (f32, f32), cx: &mut VisualTestContext) {
    cx.simulate_mouse_move(at(from.0, from.1), None, Modifiers::none());
    cx.simulate_mouse_down(at(from.0, from.1), MouseButton::Left, Modifiers::none());
    for step in 1..=4 {
        let share = step as f32 / 4.0;
        let x = from.0 + (to.0 - from.0) * share;
        let y = from.1 + (to.1 - from.1) * share;
        cx.simulate_mouse_move(at(x, y), Some(MouseButton::Left), Modifiers::none());
        settle(cx);
    }
    cx.simulate_mouse_up(at(to.0, to.1), MouseButton::Left, Modifiers::none());
    settle(cx);
}

pub(super) fn press(x: f32, y: f32, count: usize, cx: &mut VisualTestContext) {
    cx.simulate_event(MouseDownEvent {
        button: MouseButton::Left,
        position: at(x, y),
        modifiers: Modifiers::none(),
        click_count: count,
        first_mouse: false,
    });
    cx.simulate_event(MouseUpEvent {
        button: MouseButton::Left,
        position: at(x, y),
        modifiers: Modifiers::none(),
        click_count: count,
    });
    settle(cx);
}

pub(super) fn pair() -> Vec<Shape> {
    vec![
        Shape::new(
            "a",
            "A",
            ShapeKind::Rect,
            Frame::new(0.0, 0.0, 100.0, 100.0),
        ),
        Shape::new(
            "b",
            "B",
            ShapeKind::Rect,
            Frame::new(200.0, 0.0, 100.0, 100.0),
        ),
    ]
}

pub(super) fn layer(tool: Tool, selected: &[&'static str], owner: Entity<Stage>) -> AnyElement {
    let view = Viewport::new(0.0, 0.0, 1.0);
    let [drawn, chosen, moved, sized, linked, written] = [(); 6].map(|_| owner.clone());
    let layer = ToolLayer::new("layer", tool, view, pair())
        .selected(selected.iter().copied())
        .on_draw(move |kind, frame, _, cx| {
            let what = match kind {
                ShapeKind::Path { points, .. } => format!("path of {}", points.len()),
                other => format!("{other:?}"),
            };
            say(
                &drawn,
                format!(
                    "draw {what} {} {} {} {}",
                    frame.x, frame.y, frame.w, frame.h
                ),
                cx,
            )
        })
        .on_select(move |keys, _, cx| say(&chosen, format!("select {keys:?}"), cx))
        .on_move(move |keys, by, _, cx| say(&moved, format!("move {keys:?} {by:?}"), cx))
        .on_resize(move |key, frame, _, cx| {
            say(&sized, format!("resize {key} {} {}", frame.w, frame.h), cx)
        })
        .on_link(move |from, to, _, cx| say(&linked, format!("link {from} {to}"), cx))
        .on_text(move |key, text, _, cx| say(&written, format!("text {key} {text}"), cx));
    InfiniteCanvas::new("plane", view)
        .shapes(pair())
        .layer(layer)
        .into_any_element()
}

#[gpui::test]
fn a_shape_tool_draws_what_the_drag_spans(cx: &mut TestAppContext) {
    let (host, cx) = stage(|_, owner| layer(Tool::Ellipse, &[], owner), cx);
    drag((110.0, 120.0), (190.0, 180.0), cx);
    assert_eq!(said(&host, cx), ["draw Ellipse 110 120 80 60"]);
}

#[gpui::test]
fn a_selected_shape_moves_and_snaps_to_its_neighbour(cx: &mut TestAppContext) {
    let (host, cx) = stage(|_, owner| layer(Tool::Select, &["a"], owner), cx);
    drag((50.0, 50.0), (147.0, 50.0), cx);
    assert_eq!(
        said(&host, cx),
        ["move [\"a\"] (100.0, 0.0)"],
        "its right edge meets b's left"
    );
}

#[gpui::test]
fn a_drag_on_empty_space_selects_what_it_holds(cx: &mut TestAppContext) {
    let (host, cx) = stage(|_, owner| layer(Tool::Select, &[], owner), cx);
    drag((350.0, 150.0), (150.0, -10.0), cx);
    assert_eq!(
        said(&host, cx),
        ["select [\"b\"]"],
        "a runs past the marquee's left"
    );
}

#[gpui::test]
fn a_corner_handle_resizes_the_selection(cx: &mut TestAppContext) {
    let (host, cx) = stage(|_, owner| layer(Tool::Select, &["a"], owner), cx);
    drag((100.0, 100.0), (150.0, 120.0), cx);
    assert_eq!(said(&host, cx), ["resize a 150 120"]);
}

#[gpui::test]
fn the_pen_adds_a_point_a_press_and_a_double_press_ends_the_path(cx: &mut TestAppContext) {
    let (host, cx) = stage(|_, owner| layer(Tool::Pen, &[], owner), cx);
    for (x, y) in [(110.0, 150.0), (160.0, 150.0), (160.0, 200.0)] {
        press(x, y, 1, cx);
    }
    press(160.0, 200.0, 2, cx);
    assert_eq!(said(&host, cx), ["draw path of 3 110 150 50 50"]);
}

#[gpui::test]
fn a_two_shape_selection_shows_no_handles(cx: &mut TestAppContext) {
    let (_, cx) = stage(|_, owner| layer(Tool::Select, &["a", "b"], owner), cx);
    assert!(
        cx.debug_bounds("handle North").is_none(),
        "handles resize one shape"
    );
    press(150.0, 0.0, 1, cx);
    assert!(cx.debug_bounds("handle North").is_none());
}

#[gpui::test]
fn one_selected_shape_shows_its_handles(cx: &mut TestAppContext) {
    let (_, cx) = stage(|_, owner| layer(Tool::Select, &["a"], owner), cx);
    let north = cx
        .debug_bounds("handle North")
        .expect("a handle on the top side");
    assert_eq!(north.center(), at(50.0, 0.0));
}

fn switch(host: &Entity<Stage>, tool: Tool, cx: &mut VisualTestContext) {
    host.update(cx, |stage, cx| {
        stage.tool = tool;
        cx.notify();
    });
    settle(cx);
}

#[gpui::test]
fn a_path_left_open_goes_to_the_owner_when_the_tool_changes(cx: &mut TestAppContext) {
    let (host, cx) = stage(|stage, owner| layer(stage.tool, &[], owner), cx);
    switch(&host, Tool::Pen, cx);
    for (x, y) in [(110.0, 150.0), (160.0, 150.0), (160.0, 200.0)] {
        press(x, y, 1, cx);
    }
    switch(&host, Tool::Select, cx);
    assert_eq!(said(&host, cx), ["draw path of 3 110 150 50 50"]);
    switch(&host, Tool::Pen, cx);
    press(120.0, 160.0, 1, cx);
    switch(&host, Tool::Hand, cx);
    switch(&host, Tool::Pen, cx);
    press(130.0, 170.0, 1, cx);
    press(130.0, 170.0, 2, cx);
    assert_eq!(
        said(&host, cx),
        ["draw path of 3 110 150 50 50"],
        "a lone point is dropped, so the next path starts afresh"
    );
}

fn palette(_: &Stage, _: Entity<Stage>) -> AnyElement {
    div()
        .w(px(280.0))
        .child(ToolPalette::new("palette", Tool::Select))
        .into_any_element()
}

#[gpui::test]
fn the_palette_wraps_inside_a_narrow_box(cx: &mut TestAppContext) {
    let (_, cx) = stage(palette, cx);
    for tool in Tool::ALL {
        let selector = format!("toggle {}", tool.words()).leak();
        let face = cx.debug_bounds(selector).expect("a face per tool");
        assert!(
            face.right() <= px(280.0),
            "{} ends at {:?}",
            tool.words(),
            face.right()
        );
    }
}
