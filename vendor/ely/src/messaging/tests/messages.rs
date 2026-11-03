use gpui::{
    Context, IntoElement, Modifiers, ParentElement, Render, Styled, TestAppContext, Window, div, px,
};
use jiff::{Timestamp, civil::date, tz::TimeZone};

use super::{press, settle, setup, tab_to};
use crate::{
    data_display::Avatar,
    messaging::{ChatMessage, Delivery, MessageThread, ReadReceipt, ThreadPanel},
};

fn at(minute: i8) -> Timestamp {
    date(2026, 9, 26)
        .at(9, minute, 0, 0)
        .to_zoned(TimeZone::UTC)
        .expect("a UTC time")
        .timestamp()
}

/// Ana twice within a minute, then Ben.
struct Run;

impl Render for Run {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let message = |id: &'static str, author: &'static str, minute| {
            ChatMessage::new(id, author, at(minute))
                .zone(TimeZone::UTC)
                .child("Words")
        };
        div()
            .w(px(360.0))
            .child(message("m1", "Ana", 0))
            .child(message("m2", "Ana", 1).after("Ana", at(0)))
            .child(message("m3", "Ben", 2).after("Ana", at(1)))
    }
}

#[gpui::test]
fn a_message_in_a_run_drops_its_head(cx: &mut TestAppContext) {
    setup(cx);
    let (_, cx) = cx.add_window_view(|_, _| Run);
    settle(cx);
    assert!(cx.debug_bounds("message-head m1").is_some());
    assert!(
        cx.debug_bounds("message-head m2").is_none(),
        "Ana's second message joins her run"
    );
    assert!(
        cx.debug_bounds("message-head m3").is_some(),
        "Ben starts his own"
    );
}

#[gpui::test]
fn a_run_shows_its_time_on_hover(cx: &mut TestAppContext) {
    setup(cx);
    let (_, cx) = cx.add_window_view(|_, _| Run);
    settle(cx);
    assert!(
        cx.debug_bounds("run-time m2").is_none(),
        "the time rests hidden"
    );
    let row = cx
        .debug_bounds("message m2")
        .expect("the second message draws");
    cx.simulate_mouse_move(row.center(), None, Modifiers::none());
    settle(cx);
    assert!(cx.debug_bounds("run-time m2").is_some());
}

/// A thread of three replies, and how often it opened.
struct Threaded {
    opened: usize,
}

impl Render for Threaded {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let owner = cx.entity();
        div().w(px(360.0)).child(
            MessageThread::new("thread", 3, Timestamp::now())
                .repliers([Avatar::new("r1", "Ana Lima"), Avatar::new("r2", "Ben Ito")])
                .on_open(move |_, cx| owner.update(cx, |host, _| host.opened += 1)),
        )
    }
}

#[gpui::test]
fn a_thread_opens_on_a_press_or_space(cx: &mut TestAppContext) {
    setup(cx);
    let (host, cx) = cx.add_window_view(|_, _| Threaded { opened: 0 });
    cx.update(|window, _| window.activate_window());
    settle(cx);
    let thread = cx.debug_bounds("message-thread").expect("the thread draws");
    cx.simulate_click(thread.center(), Modifiers::none());
    settle(cx);
    tab_to(1, cx);
    press("space", cx);
    assert_eq!(host.read_with(cx, |host, _| host.opened), 2);
}

/// A thread with nothing to open.
struct Unthreaded;

impl Render for Unthreaded {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .w(px(360.0))
            .child(MessageThread::new("thread", 1, Timestamp::now()))
    }
}

#[gpui::test]
fn a_thread_without_a_handler_takes_no_tab_stop(cx: &mut TestAppContext) {
    setup(cx);
    let (_, cx) = cx.add_window_view(|_, _| Unthreaded);
    cx.update(|window, _| window.activate_window());
    settle(cx);
    tab_to(1, cx);
    assert!(cx.update(|window, cx| window.focused(cx).is_none()));
}

#[test]
#[should_panic(expected = "a thread holds a reply")]
fn a_thread_holds_a_reply() {
    let _ = MessageThread::new("thread", 0, Timestamp::UNIX_EPOCH);
}

#[test]
#[should_panic(expected = "more repliers than replies")]
fn repliers_fit_their_replies() {
    let _ = MessageThread::new("thread", 1, Timestamp::UNIX_EPOCH)
        .repliers([Avatar::new("a", "Ana"), Avatar::new("b", "Ben")]);
}

/// A thread panel with its replies, and whether it asked to close.
struct Paneled {
    replies: usize,
    closed: bool,
}

impl Render for Paneled {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let owner = cx.entity();
        div().w(px(320.0)).h(px(400.0)).child(
            ThreadPanel::new("panel", div().child("The first message"))
                .chat("design")
                .children((0..self.replies).map(|ix| div().child(format!("Reply {ix}"))))
                .on_close(move |_, cx| owner.update(cx, |host, _| host.closed = true)),
        )
    }
}

#[gpui::test]
fn a_panel_counts_its_replies_and_closes_on_ask(cx: &mut TestAppContext) {
    setup(cx);
    let (host, cx) = cx.add_window_view(|_, _| Paneled {
        replies: 2,
        closed: false,
    });
    cx.update(|window, _| window.activate_window());
    settle(cx);
    assert!(cx.debug_bounds("thread-replies").is_some());
    tab_to(1, cx);
    press("space", cx);
    assert!(host.read_with(cx, |host, _| host.closed));
}

#[gpui::test]
fn a_panel_without_replies_draws_no_rule(cx: &mut TestAppContext) {
    setup(cx);
    let (_, cx) = cx.add_window_view(|_, _| Paneled {
        replies: 0,
        closed: false,
    });
    settle(cx);
    assert!(cx.debug_bounds("thread-panel").is_some());
    assert!(cx.debug_bounds("thread-replies").is_none());
}

#[test]
#[should_panic(expected = "only a read message has readers")]
fn only_a_read_message_has_readers() {
    let _ = ReadReceipt::new(Delivery::Delivered).readers([Avatar::new("a", "Ana")]);
}
