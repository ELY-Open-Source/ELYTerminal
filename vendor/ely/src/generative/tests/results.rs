use std::{cell::RefCell, rc::Rc, time::Duration};

use gpui::{
    Context, IntoElement, Modifiers, ParentElement, Render, Styled, TestAppContext, Window, div,
    point, px,
};

use super::{press, settle, setup, tab};
use crate::generative::{
    ABCompareView, GenerationGrid, GenerationQueue, Job, JobState, Outcome, VariationPicker,
    Verdict,
};

type Heard = Rc<RefCell<Vec<String>>>;

/// A queue of a running, a failed and a finished job, and the actions heard.
struct Queue(Heard);

impl Render for Queue {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let job = |key: &str, state| Job {
            key: key.to_string().into(),
            prompt: "a white atrium".into(),
            state,
            picture: None,
        };
        let (canceled, retried, removed) = (self.0.clone(), self.0.clone(), self.0.clone());
        div().w(px(420.0)).child(
            GenerationQueue::new(
                "queue",
                [
                    job("run", JobState::Running(0.4, Some(Duration::from_secs(12)))),
                    job("fail", JobState::Failed("out of memory".into())),
                    job("done", JobState::Done(Duration::from_secs(30))),
                ],
            )
            .on_cancel(move |key, _, _| canceled.borrow_mut().push(format!("cancel {key}")))
            .on_retry(move |key, _, _| retried.borrow_mut().push(format!("retry {key}")))
            .on_remove(move |key, _, _| removed.borrow_mut().push(format!("remove {key}"))),
        )
    }
}

#[gpui::test]
fn each_job_offers_what_its_state_allows(cx: &mut TestAppContext) {
    setup(cx);
    let heard = Heard::default();
    let store = heard.clone();
    let (_, cx) = cx.add_window_view(|_, _| Queue(store));
    cx.update(|window, _| window.activate_window());
    settle(cx);
    for _ in 0..4 {
        tab(1, cx);
        press("enter", cx);
    }
    assert_eq!(
        *heard.borrow(),
        ["cancel run", "retry fail", "remove fail", "remove done"]
    );
}

/// A grid of a picture, a pending result and a failure, and what it heard.
struct Grid(Heard);

impl Render for Grid {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let (opened, retried) = (self.0.clone(), self.0.clone());
        let results = [
            Outcome::Pending(Some(0.3)),
            Outcome::Done("missing.jpg".into()),
            Outcome::Failed("blocked".into()),
        ];
        div().w(px(420.0)).child(
            GenerationGrid::new("grid", results, 1.5)
                .on_open(move |ix, _, _| opened.borrow_mut().push(format!("open {ix}")))
                .on_retry(move |ix, _, _| retried.borrow_mut().push(format!("retry {ix}"))),
        )
    }
}

#[gpui::test]
fn only_done_results_open_and_a_failure_retries(cx: &mut TestAppContext) {
    setup(cx);
    let heard = Heard::default();
    let store = heard.clone();
    let (_, cx) = cx.add_window_view(|_, _| Grid(store));
    cx.update(|window, _| window.activate_window());
    settle(cx);
    tab(1, cx);
    press("enter", cx);
    tab(1, cx);
    press("enter", cx);
    assert_eq!(
        *heard.borrow(),
        ["open 1", "retry 2"],
        "the pending result takes no Tab"
    );
}

/// A single picture with nothing to open it.
struct Still;

impl Render for Still {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().w(px(420.0)).child(GenerationGrid::new(
            "still",
            [Outcome::Done("missing.jpg".into())],
            1.5,
        ))
    }
}

#[gpui::test]
fn a_picture_with_nothing_to_open_takes_no_focus(cx: &mut TestAppContext) {
    setup(cx);
    let (_, cx) = cx.add_window_view(|_, _| Still);
    cx.update(|window, _| window.activate_window());
    settle(cx);
    cx.simulate_click(point(px(100.0), px(60.0)), Modifiers::none());
    settle(cx);
    assert!(
        cx.update(|window, cx| window.focused(cx).is_none()),
        "no ring on a picture the keys cannot use"
    );
}

/// Three variations with the first chosen, and what it heard.
struct Variations(Heard);

impl Render for Variations {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let (chosen, varied) = (self.0.clone(), self.0.clone());
        div().w(px(420.0)).child(
            VariationPicker::new("variations", ["a.jpg", "b.jpg", "c.jpg"], 1.5, 0)
                .on_choose(move |ix, _, _| chosen.borrow_mut().push(format!("choose {ix}")))
                .on_vary(move |strong, _, _| varied.borrow_mut().push(format!("vary {strong}"))),
        )
    }
}

#[gpui::test]
fn a_thumbnail_chooses_and_the_chosen_one_is_quiet(cx: &mut TestAppContext) {
    setup(cx);
    let heard = Heard::default();
    let store = heard.clone();
    let (_, cx) = cx.add_window_view(|_, _| Variations(store));
    cx.update(|window, _| window.activate_window());
    settle(cx);
    tab(1, cx);
    press("enter", cx);
    tab(2, cx);
    press("enter", cx);
    tab(2, cx);
    press("enter", cx);
    assert_eq!(*heard.borrow(), ["choose 2", "vary true"]);
}

/// A blind comparison of two answers, deciding what it hears.
struct Compared {
    verdict: Option<Verdict>,
    heard: Heard,
}

impl Render for Compared {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        let compare = ABCompareView::new("compare")
            .side("Orchid", "one answer")
            .side("Juniper", "another")
            .blind()
            .on_verdict(move |verdict, _, cx| {
                view.update(cx, |compared, cx| {
                    compared.heard.borrow_mut().push(format!("{verdict:?}"));
                    compared.verdict = Some(verdict);
                    cx.notify();
                })
            });
        let compare = match self.verdict {
            Some(verdict) => compare.verdict(verdict),
            None => compare,
        };
        div().w(px(420.0)).child(compare)
    }
}

#[gpui::test]
fn a_verdict_is_given_once(cx: &mut TestAppContext) {
    setup(cx);
    let heard = Heard::default();
    let store = heard.clone();
    let (_, cx) = cx.add_window_view(|_, _| Compared {
        verdict: None,
        heard: store,
    });
    cx.update(|window, _| window.activate_window());
    settle(cx);
    tab(2, cx);
    press("enter", cx);
    tab(1, cx);
    press("enter", cx);
    assert_eq!(
        *heard.borrow(),
        ["Side(1)"],
        "the buttons leave with the verdict"
    );
}
