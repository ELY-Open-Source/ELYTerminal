use gpui::{
    AppContext as _, Context, Entity, IntoElement, KeyBinding, ParentElement, Render, Styled,
    TestAppContext, VisualTestContext, Window, div, px,
};

use super::{press, settle};
use crate::{
    agent::{CostBreakdown, HumanInputRequest},
    forms::{self, TextInput},
    primitives::FocusNext,
    theme::Theme,
};

/// A question with two answers to pick, and the answers given; once answered, it shows the first.
struct Asked {
    field: Entity<TextInput>,
    answers: Vec<String>,
}

impl Render for Asked {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        let request = HumanInputRequest::new(
            "asked",
            "Keep the accents?",
            &self.field,
            move |answer, _, cx| {
                view.update(cx, |asked, cx| {
                    asked.answers.push(answer.to_string());
                    cx.notify();
                })
            },
        )
        .choices(["Keep them", "Lift further"]);
        let request = match self.answers.first() {
            Some(answer) => request.answered(answer.clone()),
            None => request,
        };
        div().w(px(420.0)).child(request)
    }
}

fn asked(cx: &mut TestAppContext) -> (Entity<Asked>, &mut VisualTestContext) {
    cx.update(|cx| {
        Theme::init(cx);
        forms::bind_keys(cx);
        cx.bind_keys([KeyBinding::new("tab", FocusNext, None)]);
    });
    let (view, cx) = cx.add_window_view(|window, cx| Asked {
        field: cx.new(|cx| TextInput::new(window, cx)),
        answers: Vec::new(),
    });
    cx.update(|window, _| window.activate_window());
    settle(cx);
    (view, cx)
}

#[gpui::test]
fn a_pick_answers_the_question_and_the_answer_stands_alone(cx: &mut TestAppContext) {
    let (view, cx) = asked(cx);
    cx.update(|window, cx| window.focus_next(cx));
    cx.update(|window, cx| window.focus_next(cx));
    press("enter", cx);
    cx.update(|window, cx| window.focus_next(cx));
    press("enter", cx);
    assert_eq!(
        view.read_with(cx, |asked, _| asked.answers.clone()),
        ["Lift further"],
        "once answered, no choice is left to press"
    );
}

#[gpui::test]
fn enter_sends_a_written_answer_and_skips_a_blank_one(cx: &mut TestAppContext) {
    let (view, cx) = asked(cx);
    for _ in 0..3 {
        cx.update(|window, cx| window.focus_next(cx));
    }
    cx.simulate_input(" ");
    press("enter", cx);
    cx.simulate_input("Lift them to 18%");
    press("enter", cx);
    view.read_with(cx, |asked, cx| {
        assert_eq!(asked.answers, ["Lift them to 18%"]);
        assert_eq!(asked.field.read(cx).text(), "");
    });
}

/// A breakdown of a session that has cost nothing yet.
struct Nothing;

impl Render for Nothing {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .w(px(320.0))
            .child(CostBreakdown::new("This session", "USD"))
    }
}

#[gpui::test]
fn a_session_that_cost_nothing_draws_its_total(cx: &mut TestAppContext) {
    cx.update(Theme::init);
    let (_, cx) = cx.add_window_view(|_, _| Nothing);
    settle(cx);
}
