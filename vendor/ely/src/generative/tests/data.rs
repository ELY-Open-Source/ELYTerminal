use gpui::{
    AppContext, Bounds, Context, Entity, IntoElement, Modifiers, MouseButton, ParentElement,
    Pixels, Render, Styled, TestAppContext, VisualTestContext, Window, div, point, px,
};
use jiff::Timestamp;

use super::{press, settle, setup, tab};
use crate::{
    forms::TextInput,
    generative::{
        Embedded, EmbeddingVisualizer, PromptPlayground, PromptVersion, PromptVersionHistory,
        lens::{DRAG, Lens, START, STEP, radius, turned},
    },
};

/// An embedding of two points, 400 wide, flat or turned.
struct Space(bool);

const A: [f32; 3] = [0.8, 0.3, 0.2];
const B: [f32; 3] = [-0.6, -0.4, 0.0];

fn points() -> Vec<Embedded> {
    let point = |key: &str, group, at| Embedded {
        key: key.to_string().into(),
        label: key.to_string().into(),
        group,
        at,
    };
    vec![point("a", 0, A), point("b", 1, B)]
}

impl Render for Space {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().w(px(400.0)).child(
            EmbeddingVisualizer::new("space", points(), ["Docs", "Code"]).three_dimensions(self.0),
        )
    }
}

fn space(three: bool, cx: &mut TestAppContext) -> &mut VisualTestContext {
    setup(cx);
    let (_, cx) = cx.add_window_view(|_, _| Space(three));
    cx.update(|window, _| window.activate_window());
    settle(cx);
    cx
}

/// The plot inside the area's border, and where `at` lands on it under `turn`.
fn aim(at: [f32; 3], turn: Option<(f32, f32)>, cx: &mut VisualTestContext) -> gpui::Point<Pixels> {
    let area = cx.debug_bounds("embedding-area").expect("the area draws");
    let plot = Bounds::new(
        area.origin + point(px(1.0), px(1.0)),
        area.size - gpui::size(px(2.0), px(2.0)),
    );
    let lens = Lens {
        turn,
        size: (f32::from(plot.size.width), f32::from(plot.size.height)),
        radius: radius(&points()),
    };
    let (x, y, _) = lens.place(at);
    plot.origin + point(px(x), px(y))
}

fn hover(at: gpui::Point<Pixels>, cx: &mut VisualTestContext) {
    cx.simulate_mouse_move(at, None, Modifiers::none());
    settle(cx);
}

#[gpui::test]
fn a_flat_view_names_the_point_under_the_pointer_and_skips_hidden_groups(cx: &mut TestAppContext) {
    let cx = space(false, cx);
    tab(1, cx);
    let first = cx.update(|window, cx| window.focused(cx));
    tab(2, cx);
    let wrapped = cx.update(|window, cx| window.focused(cx));
    assert!(
        first.is_some() && first == wrapped,
        "flat, the plot takes no Tab"
    );
    press("enter", cx);
    hover(aim(A, None, cx), cx);
    assert!(
        cx.debug_bounds("embedding-tip").is_none(),
        "Docs is hidden, so a names nothing"
    );
    let at = aim(B, None, cx);
    hover(at, cx);
    let tip = cx.debug_bounds("embedding-tip").expect("b is named");
    assert!((tip.origin - at).x.abs() < px(0.5) && (tip.origin - at).y.abs() < px(0.5));
}

#[gpui::test]
fn a_turned_view_takes_tab_and_spins_with_the_arrows(cx: &mut TestAppContext) {
    let cx = space(true, cx);
    tab(3, cx);
    for _ in 0..3 {
        press("right", cx);
    }
    let turn = (0..3).fold(START, |turn, _| turned(turn, (STEP, 0.0)));
    let at = aim(A, Some(turn), cx);
    hover(at, cx);
    let tip = cx
        .debug_bounds("embedding-tip")
        .expect("a is named where it turned to");
    assert!((tip.origin - at).x.abs() < px(0.5) && (tip.origin - at).y.abs() < px(0.5));
}

#[gpui::test]
fn a_drag_keeps_turning_past_the_plots_edge(cx: &mut TestAppContext) {
    let cx = space(true, cx);
    let middle = cx
        .debug_bounds("embedding-area")
        .expect("the area draws")
        .center();
    let (none, down) = (Modifiers::none(), |y: f32| middle + point(px(0.0), px(y)));
    cx.simulate_mouse_down(middle, MouseButton::Left, none);
    cx.simulate_mouse_move(down(50.0), MouseButton::Left, none);
    cx.simulate_mouse_move(down(250.0), MouseButton::Left, none);
    cx.simulate_mouse_up(down(250.0), MouseButton::Left, none);
    settle(cx);
    let at = aim(A, Some(turned(START, (0.0, 250.0 * DRAG))), cx);
    hover(at, cx);
    let tip = cx
        .debug_bounds("embedding-tip")
        .expect("a is named where the whole drag turned it");
    assert!((tip.origin - at).x.abs() < px(0.5) && (tip.origin - at).y.abs() < px(0.5));
}

#[gpui::test]
#[should_panic(expected = "named twice")]
fn a_group_named_twice_fails_loud() {
    EmbeddingVisualizer::new("space", points(), ["Docs", "Docs"]);
}

#[gpui::test]
#[should_panic(expected = "sits outside -1 to 1")]
fn a_point_past_the_cube_fails_loud() {
    EmbeddingVisualizer::new(
        "space",
        [Embedded {
            key: "far".into(),
            label: "far".into(),
            group: 0,
            at: [1.2, 0.0, 0.0],
        }],
        ["Docs"],
    );
}

/// A playground over a template with two names, and the prompts it ran.
struct Trying {
    template: Entity<TextInput>,
    ran: Vec<String>,
}

impl Render for Trying {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let heard = cx.entity();
        div()
            .w(px(480.0))
            .child(
                PromptPlayground::new("try", &self.template).on_run(move |prompt, _, cx| {
                    heard.update(cx, |view, cx| {
                        view.ran.push(prompt.to_string());
                        cx.notify();
                    })
                }),
            )
    }
}

#[gpui::test]
fn run_waits_for_every_value_then_sends_the_filled_prompt(cx: &mut TestAppContext) {
    setup(cx);
    let (view, cx) = cx.add_window_view(|window, cx| {
        let template = cx.new(|cx| {
            let mut input = TextInput::new(window, cx);
            input.set_text("Notes on {topic} in a {tone} voice", cx);
            input
        });
        Trying {
            template,
            ran: Vec::new(),
        }
    });
    cx.update(|window, _| window.activate_window());
    settle(cx);
    tab(2, cx);
    cx.simulate_input("dunes");
    settle(cx);
    tab(2, cx);
    press("enter", cx);
    let ran = |cx: &mut VisualTestContext| view.read_with(cx, |view, _| view.ran.clone());
    assert!(ran(cx).is_empty(), "with tone blank, Run is out of reach");
    cx.update(|window, cx| window.blur(cx));
    tab(3, cx);
    cx.simulate_input("calm");
    settle(cx);
    tab(1, cx);
    press("enter", cx);
    assert_eq!(ran(cx), ["Notes on dunes in a calm voice"]);
}

/// Three versions of a prompt, the one chosen, and the versions chosen and restored.
struct History {
    chosen: usize,
    chose: Vec<usize>,
    restored: Vec<usize>,
}

impl Render for History {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (choose, restore) = (cx.entity(), cx.entity());
        let at = Timestamp::from_second(1_790_000_000).expect("a time");
        let version = |key: &str, text: &str, note: &str| PromptVersion {
            key: key.to_string().into(),
            text: text.to_string().into(),
            author: "Mira".into(),
            at,
            note: note.to_string().into(),
        };
        let versions = [
            version("v3", "Write plain, short notes.", "Shorter"),
            version("v2", "Write plain notes.", "Plain words"),
            version("v1", "Write notes.", "First"),
        ];
        div().w(px(480.0)).child(
            PromptVersionHistory::new("history", versions, self.chosen)
                .on_choose(move |ix, _, cx| {
                    choose.update(cx, |view, cx| {
                        view.chosen = ix;
                        view.chose.push(ix);
                        cx.notify();
                    })
                })
                .on_restore(move |ix, _, cx| {
                    restore.update(cx, |view, cx| {
                        view.restored.push(ix);
                        cx.notify();
                    })
                }),
        )
    }
}

#[gpui::test]
fn a_version_is_chosen_by_keys_and_only_an_older_one_restores(cx: &mut TestAppContext) {
    setup(cx);
    let (view, cx) = cx.add_window_view(|_, _| History {
        chosen: 0,
        chose: Vec::new(),
        restored: Vec::new(),
    });
    cx.update(|window, _| window.activate_window());
    settle(cx);
    let heard = |cx: &mut VisualTestContext| {
        view.read_with(cx, |view, _| (view.chose.clone(), view.restored.clone()))
    };
    tab(4, cx);
    press("enter", cx);
    assert_eq!(heard(cx), (vec![], vec![]), "the newest offers no Restore");
    cx.update(|window, cx| window.blur(cx));
    tab(2, cx);
    press("enter", cx);
    tab(2, cx);
    press("enter", cx);
    assert_eq!(heard(cx), (vec![1], vec![1]));
}
