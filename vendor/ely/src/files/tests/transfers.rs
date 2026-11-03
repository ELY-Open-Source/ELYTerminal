use gpui::{
    Context, Entity, IntoElement, Modifiers, ParentElement, Render, SharedString, Styled,
    TestAppContext, VisualTestContext, Window, div, px,
};

use super::{press, settle, setup};
use crate::files::{
    DownloadManager, FileOperation, FileOperationProgress, Transfer, TransferQueue, TransferState,
    operation::headline, transfer::status, transfers::tally,
};

/// Moves focus to the `nth` Tab stop from none.
fn tab_to(nth: usize, cx: &mut VisualTestContext) {
    cx.update(|window, cx| {
        window.blur(cx);
        for _ in 0..nth {
            window.focus_next(cx);
        }
    });
    settle(cx);
}

fn transfer(key: &str, state: TransferState) -> Transfer {
    Transfer {
        key: SharedString::from(key.to_string()),
        name: SharedString::from(format!("{key}.mov")),
        upload: false,
        moved: 4_000_000,
        total: 16_000_000,
        state,
    }
}

#[test]
fn a_status_says_how_far_how_fast_and_how_long() {
    let moving = transfer("a", TransferState::Moving { rate: 2_000_000 });
    assert_eq!(status(&moving), "4.0 MB of 16 MB · 2.0 MB/s · 6s left");
    assert_eq!(
        status(&transfer("a", TransferState::Paused)),
        "Paused at 4.0 MB of 16 MB"
    );
    assert_eq!(
        status(&transfer("a", TransferState::Queued)),
        "Waiting · 16 MB"
    );
    let failed = transfer("a", TransferState::Failed("The disk is full".into()));
    assert_eq!(status(&failed), "Failed: The disk is full");
    assert_eq!(status(&transfer("a", TransferState::Done)), "16 MB");
}

#[test]
fn a_tally_counts_the_moving_and_the_waiting() {
    let all = [
        transfer("a", TransferState::Moving { rate: 1 }),
        transfer("b", TransferState::Queued),
        transfer("c", TransferState::Paused),
        transfer("d", TransferState::Done),
    ];
    assert_eq!(tally(&all), "1 moving · 2 waiting");
}

#[test]
fn a_headline_says_what_an_operation_does_and_where() {
    let moving = TransferState::Moving { rate: 1 };
    assert_eq!(
        headline(FileOperation::Copy, 12, "Atrium", &moving),
        "Copying 12 items to Atrium"
    );
    assert_eq!(
        headline(FileOperation::Move, 1, "Atrium", &TransferState::Done),
        "Moved 1 item to Atrium"
    );
}

/// A queue of three, one moving, one paused, one failed, and what its buttons asked.
struct Queued {
    asked: Vec<String>,
}

impl Render for Queued {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let ask = |what: &'static str, cx: &mut Context<Self>| {
            let owner = cx.entity();
            move |key: &SharedString, _: &mut Window, cx: &mut gpui::App| {
                owner.update(cx, |host, _| host.asked.push(format!("{what} {key}")))
            }
        };
        let transfers = [
            transfer("a", TransferState::Moving { rate: 1_000_000 }),
            transfer("b", TransferState::Paused),
            transfer("c", TransferState::Failed("Lost the network".into())),
        ];
        div().w(px(420.0)).child(
            TransferQueue::new("queue", transfers)
                .pause(ask("pause", cx))
                .resume(ask("resume", cx))
                .retry(ask("retry", cx))
                .cancel(ask("cancel", cx)),
        )
    }
}

#[gpui::test]
fn each_state_offers_what_it_can_do(cx: &mut TestAppContext) {
    setup(cx);
    let (host, cx) = cx.add_window_view(|_, _| Queued { asked: Vec::new() });
    cx.update(|window, _| window.activate_window());
    settle(cx);
    for nth in 1..=6 {
        tab_to(nth, cx);
        press("space", cx);
    }
    let asked = host.read_with(cx, |host, _| host.asked.clone());
    assert_eq!(
        asked,
        [
            "pause a", "cancel a", "resume b", "cancel b", "retry c", "cancel c"
        ]
    );
}

/// Two downloads, one on its way and one done, and what was asked.
struct Downloading {
    asked: Vec<String>,
}

impl Render for Downloading {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (open, show, clear) = (cx.entity(), cx.entity(), cx.entity());
        let downloads = [
            transfer("done", TransferState::Done),
            transfer("going", TransferState::Moving { rate: 1_000_000 }),
        ];
        div().w(px(420.0)).child(
            DownloadManager::new("downloads", downloads)
                .open(move |key, _, cx| {
                    open.update(cx, |host, _| host.asked.push(format!("open {key}")))
                })
                .show(move |key, _, cx| {
                    show.update(cx, |host, _| host.asked.push(format!("show {key}")))
                })
                .on_clear(move |_, cx| clear.update(cx, |host, _| host.asked.push("clear".into()))),
        )
    }
}

#[gpui::test]
fn the_finished_sit_below_and_open_show_and_clear(cx: &mut TestAppContext) {
    setup(cx);
    let (host, cx) = cx.add_window_view(|_, _| Downloading { asked: Vec::new() });
    cx.update(|window, _| window.activate_window());
    settle(cx);
    for nth in 1..=3 {
        tab_to(nth, cx);
        press("space", cx);
    }
    let asked = host.read_with(cx, |host, _| host.asked.clone());
    assert_eq!(
        asked,
        ["clear", "open done", "show done"],
        "the one on its way has no handlers"
    );
    let going = cx
        .debug_bounds("transfer-row going")
        .expect("the one on its way draws");
    let done = cx
        .debug_bounds("transfer-row done")
        .expect("the finished one draws");
    assert!(going.bottom() <= done.top(), "{going:?} above {done:?}");
}

/// A copy in one state, and what its buttons asked.
struct Copying {
    state: TransferState,
    asked: Vec<&'static str>,
}

impl Render for Copying {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (pause, resume, cancel) = (cx.entity(), cx.entity(), cx.entity());
        div().w(px(420.0)).child(
            FileOperationProgress::new(
                "copy",
                FileOperation::Copy,
                3,
                "Atrium",
                (4_000, 16_000),
                self.state.clone(),
            )
            .current("Walkthrough.mov")
            .on_pause(move |_, cx| pause.update(cx, |host, _| host.asked.push("pause")))
            .on_resume(move |_, cx| resume.update(cx, |host, _| host.asked.push("resume")))
            .on_cancel(move |_, cx| cancel.update(cx, |host, _| host.asked.push("cancel"))),
        )
    }
}

fn copying(
    state: TransferState,
    cx: &mut TestAppContext,
) -> (Entity<Copying>, &mut VisualTestContext) {
    setup(cx);
    let (host, cx) = cx.add_window_view(move |_, _| Copying {
        state,
        asked: Vec::new(),
    });
    cx.update(|window, _| window.activate_window());
    settle(cx);
    (host, cx)
}

#[gpui::test]
fn a_running_copy_pauses_and_cancels_and_a_paused_one_resumes(cx: &mut TestAppContext) {
    let (host, cx) = copying(TransferState::Moving { rate: 1_000 }, cx);
    for nth in 1..=2 {
        tab_to(nth, cx);
        press("space", cx);
    }
    host.update(cx, |host, cx| {
        host.state = TransferState::Paused;
        cx.notify();
    });
    settle(cx);
    tab_to(1, cx);
    press("space", cx);
    assert_eq!(
        host.read_with(cx, |host, _| host.asked.clone()),
        ["pause", "cancel", "resume"]
    );
}

#[gpui::test]
fn a_finished_copy_offers_nothing(cx: &mut TestAppContext) {
    let (_, cx) = copying(TransferState::Done, cx);
    tab_to(1, cx);
    assert!(cx.update(|window, cx| window.focused(cx).is_none()));
}

#[test]
#[should_panic(expected = "transfer a twice")]
fn a_transfer_is_listed_once() {
    let _ = TransferQueue::new(
        "queue",
        [
            transfer("a", TransferState::Queued),
            transfer("a", TransferState::Done),
        ],
    );
}

#[test]
#[should_panic(expected = "a goes up, not down")]
fn a_download_comes_down() {
    let mut up = transfer("a", TransferState::Queued);
    up.upload = true;
    let _ = DownloadManager::new("downloads", [up]);
}

#[gpui::test]
fn a_press_on_show_shows_and_opens_nothing(cx: &mut TestAppContext) {
    setup(cx);
    let (host, cx) = cx.add_window_view(|_, _| Downloading { asked: Vec::new() });
    cx.update(|window, _| window.activate_window());
    settle(cx);
    let actions = cx
        .debug_bounds("transfer-actions done")
        .expect("the finished row's actions draw");
    cx.simulate_click(actions.center(), Modifiers::none());
    settle(cx);
    assert_eq!(
        host.read_with(cx, |host, _| host.asked.clone()),
        ["show done"]
    );
}
