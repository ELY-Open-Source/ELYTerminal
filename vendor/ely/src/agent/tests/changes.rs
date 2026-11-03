use std::{cell::Cell, rc::Rc};

use gpui::{
    Context, Entity, IntoElement, KeyBinding, Modifiers, ParentElement, Pixels, Render, Styled,
    TestAppContext, VisualTestContext, Window, canvas, div, point, px,
};

use super::{press, settle};
use crate::{
    agent::{ChangeState, FileChange, FileChangeCard, MultiFileDiffReview},
    collab::Decision,
    forms,
    primitives::FocusNext,
    theme::Theme,
};

fn change(path: &str) -> FileChange {
    FileChange {
        path: path.to_string().into(),
        old: "let lift = 0.12;\n".into(),
        new: "let lift = 0.18;\n".into(),
        state: ChangeState::Proposed,
    }
}

fn setup(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        forms::bind_keys(cx);
        cx.bind_keys([KeyBinding::new("tab", FocusNext, None)]);
    });
}

/// One change card in a 640px column, its verdicts and the column's height.
struct Card {
    verdicts: Vec<bool>,
    height: Rc<Cell<Pixels>>,
}

impl Render for Card {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (view, height) = (cx.entity(), self.height.clone());
        // gpui fills the window with an auto-sized root, so the measured box sits inside one.
        div().child(
            div()
                .relative()
                .w(px(640.0))
                .child(
                    FileChangeCard::new("card", change("src/theme/lift.rs")).on_decide(
                        move |accepted, _, cx| {
                            view.update(cx, |card, _| card.verdicts.push(accepted))
                        },
                    ),
                )
                .child(
                    canvas(
                        move |bounds, _, _| height.set(bounds.size.height),
                        |_, _, _, _| {},
                    )
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full(),
                ),
        )
    }
}

#[gpui::test]
fn a_press_on_accept_decides_and_leaves_the_card_shut(cx: &mut TestAppContext) {
    setup(cx);
    let height = Rc::new(Cell::new(Pixels::ZERO));
    let seen = height.clone();
    let (view, cx) = cx.add_window_view(|_, _| Card {
        verdicts: Vec::new(),
        height: seen,
    });
    settle(cx);
    let shut = height.get();
    let accept = point(px(640.0 - 16.0), shut / 2.0);
    cx.simulate_mouse_move(accept, None, Modifiers::none());
    cx.simulate_click(accept, Modifiers::none());
    settle(cx);
    assert_eq!(view.read_with(cx, |card, _| card.verdicts.clone()), [true]);
    assert_eq!(height.get(), shut, "the press stays with the button");
    let chevron = point(px(14.0), shut / 2.0);
    cx.simulate_mouse_move(chevron, None, Modifiers::none());
    cx.simulate_click(chevron, Modifiers::none());
    settle(cx);
    assert!(height.get() > shut, "the header opens the diff");
}

/// Three proposed changes under review, and the decisions made.
struct Review {
    decisions: Vec<Decision>,
}

impl Render for Review {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        div().w(px(560.0)).child(
            MultiFileDiffReview::new("review", ["a.rs", "b.rs", "c.rs"].map(change)).on_decide(
                move |decision, _, cx| view.update(cx, |review, _| review.decisions.push(decision)),
            ),
        )
    }
}

fn review(cx: &mut TestAppContext) -> (Entity<Review>, &mut VisualTestContext) {
    setup(cx);
    let (view, cx) = cx.add_window_view(|_, _| Review {
        decisions: Vec::new(),
    });
    cx.update(|window, _| window.activate_window());
    settle(cx);
    (view, cx)
}

#[gpui::test]
fn accept_all_decides_every_file(cx: &mut TestAppContext) {
    let (view, cx) = review(cx);
    cx.update(|window, cx| window.focus_next(cx));
    cx.update(|window, cx| window.focus_next(cx));
    press("enter", cx);
    assert_eq!(
        view.read_with(cx, |review, _| review.decisions.clone()),
        [Decision::AcceptAll]
    );
}

#[gpui::test]
fn a_cards_verdict_names_its_file(cx: &mut TestAppContext) {
    let (view, cx) = review(cx);
    for _ in 0..8 {
        cx.update(|window, cx| window.focus_next(cx));
    }
    press("enter", cx);
    assert_eq!(
        view.read_with(cx, |review, _| review.decisions.clone()),
        [Decision::Accept(1)]
    );
}

/// A review at `width`, and its height as drawn.
struct Narrow {
    width: Pixels,
    height: Rc<Cell<Pixels>>,
}

impl Render for Narrow {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let height = self.height.clone();
        div().child(
            div()
                .relative()
                .w(self.width)
                .child(
                    MultiFileDiffReview::new(
                        "narrow",
                        ["src/theme/lift.rs", "src/theme/tokens.rs"].map(change),
                    )
                    .on_decide(|_, _, _| {}),
                )
                .child(
                    canvas(
                        move |bounds, _, _| height.set(bounds.size.height),
                        |_, _, _, _| {},
                    )
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full(),
                ),
        )
    }
}

#[gpui::test]
fn a_narrow_review_wraps_its_rows_inside_its_box(cx: &mut TestAppContext) {
    setup(cx);
    let height = Rc::new(Cell::new(Pixels::ZERO));
    let seen = height.clone();
    let (view, cx) = cx.add_window_view(|_, _| Narrow {
        width: px(640.0),
        height: seen,
    });
    settle(cx);
    let wide = height.get();
    view.update(cx, |narrow, cx| {
        narrow.width = px(280.0);
        cx.notify();
    });
    settle(cx);
    assert!(
        height.get() > wide,
        "at 280px the counts and verdicts wrap below: {:?} vs {wide:?}",
        height.get()
    );
}
