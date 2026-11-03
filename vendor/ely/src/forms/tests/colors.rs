use gpui::{
    Context, Hsla, IntoElement, Modifiers, MouseButton, ParentElement, Render, Styled,
    TestAppContext, Window, div, point, px, rgb, rgba,
};

use super::setup;
use crate::forms::{ColorPicker, GradientEditor, GradientStop};

struct Picked {
    color: Hsla,
}

impl Render for Picked {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        div()
            .size_full()
            .child(ColorPicker::new("picked", self.color).opaque().on_change(
                move |color, _, cx| {
                    view.update(cx, |view, cx| {
                        view.color = color;
                        cx.notify();
                    })
                },
            ))
    }
}

#[gpui::test]
fn an_opaque_picker_commits_opaque_colors(cx: &mut TestAppContext) {
    setup(cx);
    let (view, cx) = cx.add_window_view(|_, _| Picked {
        color: rgba(0x3772bb80).into(),
    });
    cx.simulate_click(point(px(60.0), px(60.0)), Modifiers::none());
    assert_eq!(view.read_with(cx, |view, _| view.color.a), 1.0);
}

struct Gradient {
    stops: Vec<GradientStop>,
}

impl Render for Gradient {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        div().size_full().child(
            GradientEditor::new("gradient", self.stops.clone()).on_change(move |next, _, cx| {
                view.update(cx, |view, cx| {
                    view.stops = next.to_vec();
                    cx.notify();
                })
            }),
        )
    }
}

#[gpui::test]
fn a_dragged_stop_keeps_its_grip_after_passing_another(cx: &mut TestAppContext) {
    setup(cx);
    let stop = |at, hex| GradientStop {
        at,
        color: rgb(hex).into(),
    };
    let (view, cx) = cx.add_window_view(|_, _| Gradient {
        stops: vec![
            stop(0.2, 0xff0000),
            stop(0.5, 0x00ff00),
            stop(0.8, 0x0000ff),
        ],
    });
    let width = cx.update(|window, _| window.viewport_size().width);
    let at = |t: f32| point(px(8.0) + (width - px(16.0)) * t, px(44.0));
    cx.simulate_mouse_down(at(0.2), MouseButton::Left, Modifiers::none());
    for t in [0.22, 0.6, 0.65] {
        cx.simulate_mouse_move(at(t), MouseButton::Left, Modifiers::none());
    }
    cx.simulate_mouse_up(at(0.65), MouseButton::Left, Modifiers::none());
    let stops = view.read_with(cx, |view, _| view.stops.clone());
    let place = |hex| {
        let color: Hsla = rgb(hex).into();
        stops.iter().find(|stop| stop.color == color).unwrap().at
    };
    assert!((place(0xff0000) - 0.65).abs() < 0.01, "{stops:?}");
    assert_eq!(place(0x00ff00), 0.5, "{stops:?}");
}
