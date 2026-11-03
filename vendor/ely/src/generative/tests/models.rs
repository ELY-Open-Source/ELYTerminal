use std::{cell::RefCell, rc::Rc};

use gpui::{Context, IntoElement, ParentElement, Render, Styled, TestAppContext, Window, div, px};

use super::{press, settle, setup, tab};
use crate::generative::{
    Download, DownloadState, FineTuneJobCard, HardwareMonitor, ModelCard, ModelDownloadManager,
    Reading, TunePhase,
};

type Heard = Rc<RefCell<Vec<String>>>;

/// Downloads in each state, and the actions heard.
struct Downloads(Heard);

impl Render for Downloads {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let download = |key: &str, state| Download {
            key: key.to_string().into(),
            name: key.to_string().into(),
            bytes: 1_000_000,
            state,
        };
        let heard = |what: &'static str| {
            let heard = self.0.clone();
            move |key: &gpui::SharedString, _: &mut Window, _: &mut gpui::App| {
                heard.borrow_mut().push(format!("{what} {key}"))
            }
        };
        div().w(px(420.0)).child(
            ModelDownloadManager::new(
                "downloads",
                [
                    download("a", DownloadState::Fetching(0.5, 1_000)),
                    download("b", DownloadState::Paused(0.2)),
                    download("c", DownloadState::Failed("disk full".into())),
                    download("d", DownloadState::Done),
                ],
            )
            .on_pause(heard("pause"))
            .on_resume(heard("resume"))
            .on_cancel(heard("cancel"))
            .on_retry(heard("retry"))
            .on_delete(heard("delete")),
        )
    }
}

#[gpui::test]
fn each_download_offers_what_its_state_allows(cx: &mut TestAppContext) {
    setup(cx);
    let heard = Heard::default();
    let store = heard.clone();
    let (_, cx) = cx.add_window_view(|_, _| Downloads(store));
    cx.update(|window, _| window.activate_window());
    settle(cx);
    for _ in 0..7 {
        tab(1, cx);
        press("enter", cx);
    }
    assert_eq!(
        *heard.borrow(),
        [
            "pause a", "cancel a", "resume b", "cancel b", "retry c", "delete c", "delete d"
        ]
    );
}

/// A model to use and one in use, a job training for one epoch and one done, and what they heard.
struct Cards(Heard);

impl Render for Cards {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let (used, canceled, opened) = (self.0.clone(), self.0.clone(), self.0.clone());
        div()
            .w(px(420.0))
            .child(
                ModelCard::new("free", "Orchid", "Orchid Labs", "A model")
                    .on_use(move |_, _| used.borrow_mut().push("use".into())),
            )
            .child(
                ModelCard::new("busy", "Juniper", "Juniper Co", "A model")
                    .in_use(true)
                    .on_use(|_, _| {}),
            )
            .child(
                FineTuneJobCard::new(
                    "training",
                    "tone",
                    "Orchid",
                    "notes",
                    TunePhase::Running(1, 1, 0.5),
                )
                .on_cancel(move |_, _| canceled.borrow_mut().push("cancel".into())),
            )
            .child(
                FineTuneJobCard::new("done", "tone", "Orchid", "notes", TunePhase::Done)
                    .loss(vec![1.2, 0.8, 0.6], vec![1.3, 0.9, 0.7])
                    .on_open(move |_, _| opened.borrow_mut().push("open".into())),
            )
    }
}

#[gpui::test]
fn cards_offer_what_their_state_allows(cx: &mut TestAppContext) {
    setup(cx);
    let heard = Heard::default();
    let store = heard.clone();
    let (_, cx) = cx.add_window_view(|_, _| Cards(store));
    cx.update(|window, _| window.activate_window());
    settle(cx);
    for _ in 0..3 {
        tab(1, cx);
        press("enter", cx);
    }
    assert_eq!(
        *heard.borrow(),
        ["use", "cancel", "open"],
        "a model in use offers no Use"
    );
}

#[test]
#[should_panic(expected = "GPU reads outside 0 to 1")]
fn a_reading_past_full_fails_loud() {
    let reading = Reading {
        name: "GPU".into(),
        share: 1.2,
        detail: "120%".into(),
        history: Vec::new(),
    };
    let _ = HardwareMonitor::new("machine", [reading]);
}
