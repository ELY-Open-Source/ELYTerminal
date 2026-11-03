use gpui::{Entity, IntoElement, Modifiers, ParentElement, Styled, TestAppContext, div, px};
use jiff::{SignedDuration, Timestamp, tz::TimeZone};

use super::{Mailing, heard, mailing, note, press, settle, tab_to};
use crate::{
    buttons::Button,
    mail::{Contact, MailReader, MailThreadView, Message, QuotedText},
};

fn ana() -> Contact {
    Contact::new("Ana Lima", "ana@atrium.studio")
}

fn ben() -> Contact {
    Contact::new("Ben Ito", "ben@atrium.studio")
}

fn review(to: Vec<Contact>) -> Message {
    Message::new("m1", ana(), Timestamp::UNIX_EPOCH)
        .to(to)
        .file("plan.pdf", 2_400_000)
}

fn reader(owner: Entity<Mailing>, to: Vec<Contact>) -> gpui::AnyElement {
    let (reply, all, forward, download) = (owner.clone(), owner.clone(), owner.clone(), owner);
    MailReader::new("reader", "Friday's review", review(to))
        .zone(TimeZone::UTC)
        .on_reply(move |key, _, cx| note(&reply, format!("reply {key}"), cx))
        .on_reply_all(move |key, _, cx| note(&all, format!("reply all {key}"), cx))
        .on_forward(move |key, _, cx| note(&forward, format!("forward {key}"), cx))
        .on_download(move |key, file, _, cx| note(&download, format!("download {key} {file}"), cx))
        .child("Moved to 10:00.")
        .into_any_element()
}

#[gpui::test]
fn a_readers_asks_each_reach_their_own(cx: &mut TestAppContext) {
    let (host, cx) = mailing(
        |owner| reader(owner, vec![Contact::new("Me", "me@atrium.studio"), ben()]),
        cx,
    );
    for nth in 1..=4 {
        tab_to(nth, cx);
        press("space", cx);
    }
    assert_eq!(
        heard(&host, cx),
        [
            "reply m1",
            "reply all m1",
            "forward m1",
            "download m1 plan.pdf"
        ]
    );
}

#[gpui::test]
fn reply_all_waits_for_more_than_one(cx: &mut TestAppContext) {
    let (host, cx) = mailing(|owner| reader(owner, vec![ben()]), cx);
    tab_to(2, cx);
    press("space", cx);
    assert_eq!(heard(&host, cx), ["forward m1"]);
}

fn thread(_: Entity<Mailing>) -> gpui::AnyElement {
    let message = |key: &'static str| {
        Message::new(key, ana(), Timestamp::UNIX_EPOCH)
            .to([ben()])
            .snippet("Earlier words")
    };
    MailThreadView::new("thread", "Friday's review")
        .zone(TimeZone::UTC)
        .message(message("m1"), div().child("First"))
        .message(message("m2"), div().child("Second"))
        .message(message("m3"), div().child("Newest"))
        .into_any_element()
}

#[gpui::test]
fn the_newest_stands_open_and_space_opens_a_fold(cx: &mut TestAppContext) {
    let (_, cx) = mailing(thread, cx);
    assert!(
        cx.debug_bounds("letter m3").is_some(),
        "the newest stands open"
    );
    assert!(
        cx.debug_bounds("letter m1").is_none(),
        "the first is folded"
    );
    tab_to(1, cx);
    press("space", cx);
    assert!(cx.debug_bounds("letter m1").is_some(), "Space opened it");
    assert!(
        cx.debug_bounds("letter m2").is_none(),
        "the middle stays folded"
    );
}

#[test]
#[should_panic(expected = "message m1 twice")]
fn a_message_sits_once_in_a_thread() {
    let _ = MailThreadView::new("thread", "Twice")
        .message(review(Vec::new()), div())
        .message(review(Vec::new()), div());
}

#[gpui::test]
fn quoted_words_stay_folded_until_asked(cx: &mut TestAppContext) {
    let (_, cx) = mailing(
        |_| {
            QuotedText::new("quote")
                .child("On Friday, Ana wrote: the samples are dry.")
                .into_any_element()
        },
        cx,
    );
    assert!(cx.debug_bounds("quoted quote").is_none());
    tab_to(1, cx);
    press("space", cx);
    assert!(cx.debug_bounds("quoted quote").is_some());
}

/// A button, then a thread of three whose Reply the host hears.
fn answered(owner: Entity<Mailing>) -> gpui::AnyElement {
    let (before, reply) = (owner.clone(), owner);
    let message = |key: &'static str| {
        Message::new(key, ana(), Timestamp::UNIX_EPOCH)
            .to([ben()])
            .snippet("Earlier words")
    };
    div()
        .child(
            Button::new("before", "Before")
                .on_click(move |_, _, cx| note(&before, "before".into(), cx)),
        )
        .child(
            MailThreadView::new("thread", "Friday's review")
                .zone(TimeZone::UTC)
                .on_reply(move |key, _, cx| note(&reply, format!("reply {key}"), cx))
                .message(message("m1"), div().child("First"))
                .message(message("m2"), div().child("Second"))
                .message(message("m3"), div().child("Newest")),
        )
        .into_any_element()
}

#[gpui::test]
fn a_press_on_a_fold_leaves_focus_alone(cx: &mut TestAppContext) {
    let (host, cx) = mailing(answered, cx);
    tab_to(1, cx);
    let fold = cx.debug_bounds("fold m1").expect("the first fold draws");
    cx.simulate_click(fold.center(), Modifiers::none());
    settle(cx);
    press("space", cx);
    assert_eq!(heard(&host, cx), ["before"], "the button kept focus");
}

#[gpui::test]
fn space_on_a_fold_keeps_focus_on_the_message(cx: &mut TestAppContext) {
    let (host, cx) = mailing(answered, cx);
    tab_to(2, cx);
    press("space", cx);
    cx.update(|window, cx| window.focus_next(cx));
    settle(cx);
    press("space", cx);
    assert_eq!(
        heard(&host, cx),
        ["reply m1"],
        "Tab went on to the opened message's Reply"
    );
}

#[gpui::test]
fn a_fold_keeps_its_parts_inside_at_280(cx: &mut TestAppContext) {
    let (_, cx) = mailing(
        |_| {
            let long = Contact::new(
                "Anastasia Lima-Fernández de la Torre",
                "anastasia@atrium.studio",
            );
            let old = Timestamp::now() - SignedDuration::from_hours(24 * 20);
            div()
                .w(px(280.0))
                .child(
                    MailThreadView::new("thread", "Samples")
                        .zone(TimeZone::UTC)
                        .message(
                            Message::new("m1", long, old)
                                .snippet("The lime reads warmest by the windows."),
                            div().child("Earlier"),
                        )
                        .message(Message::new("m2", ana(), old), div().child("Newest")),
                )
                .into_any_element()
        },
        cx,
    );
    let fold = cx.debug_bounds("fold m1").expect("the fold draws");
    let date = cx.debug_bounds("fold-date m1").expect("its date draws");
    assert!(date.right() <= fold.right(), "{date:?} inside {fold:?}");
}
