use gpui::{
    Context, Entity, IntoElement, Render, SharedString, TestAppContext, VisualTestContext, Window,
};
use jiff::{Timestamp, tz::TimeZone};

use super::{Mailing, heard, mailing, note, press, settle, tab_to};
use crate::{
    forms,
    mail::{Contact, MailComposer, ScheduleSend, SignatureEditor},
};

fn contacts() -> [Contact; 2] {
    [
        Contact::new("Ana Lima", "ana@atrium.studio"),
        Contact::new("Ben Ito", "ben@atrium.studio"),
    ]
}

fn composer(owner: Entity<Mailing>) -> gpui::AnyElement {
    let (send, discard) = (owner.clone(), owner);
    MailComposer::new("composer")
        .contacts(contacts())
        .on_send(move |draft, _, cx| {
            note(
                &send,
                format!(
                    "send {} | {} | {}",
                    draft.to.join(","),
                    draft.subject,
                    draft.body
                ),
                cx,
            )
        })
        .on_discard(move |_, cx| note(&discard, "discard".into(), cx))
        .into_any_element()
}

fn writing(
    part: fn(Entity<Mailing>) -> gpui::AnyElement,
    cx: &mut TestAppContext,
) -> (Entity<Mailing>, &mut VisualTestContext) {
    let (host, cx) = mailing(part, cx);
    cx.update(|_, cx| forms::bind_keys(cx));
    (host, cx)
}

fn next(cx: &mut VisualTestContext) {
    cx.update(|window, cx| window.focus_next(cx));
    settle(cx);
}

#[gpui::test]
fn a_suggested_person_a_subject_and_words_go_out_as_one_draft(cx: &mut TestAppContext) {
    let (host, cx) = writing(composer, cx);
    tab_to(1, cx);
    cx.simulate_input("be");
    settle(cx);
    press("enter", cx);
    next(cx);
    next(cx);
    cx.simulate_input("Samples");
    next(cx);
    cx.simulate_input("They are dry.");
    next(cx);
    press("space", cx);
    assert_eq!(
        heard(&host, cx),
        ["send ben@atrium.studio | Samples | They are dry."]
    );
}

#[gpui::test]
fn cc_and_bcc_show_once_asked(cx: &mut TestAppContext) {
    let (_, cx) = writing(composer, cx);
    assert!(cx.debug_bounds("composer-Cc").is_none());
    tab_to(2, cx);
    press("space", cx);
    assert!(cx.debug_bounds("composer-Cc").is_some());
    assert!(cx.debug_bounds("composer-Bcc").is_some());
}

#[gpui::test]
fn with_nobody_to_send_to_send_steps_aside_for_discard(cx: &mut TestAppContext) {
    let (host, cx) = writing(composer, cx);
    tab_to(5, cx);
    press("space", cx);
    assert_eq!(
        heard(&host, cx),
        ["discard"],
        "Send is off, so the stop after the words is Discard"
    );
}

fn scheduling(owner: Entity<Mailing>) -> gpui::AnyElement {
    let (send, later) = (owner.clone(), owner);
    ScheduleSend::new("send")
        .zone(TimeZone::UTC)
        .on_send(move |_, cx| note(&send, "send".into(), cx))
        .on_schedule(move |at, _, cx| note(&later, format!("at {at}"), cx))
        .into_any_element()
}

#[gpui::test]
fn the_first_time_to_send_later_is_tomorrow_morning(cx: &mut TestAppContext) {
    let (host, cx) = writing(scheduling, cx);
    tab_to(2, cx);
    press("enter", cx);
    press("enter", cx);
    let morning = Timestamp::now()
        .to_zoned(TimeZone::UTC)
        .date()
        .tomorrow()
        .expect("a day follows")
        .at(8, 0, 0, 0)
        .to_zoned(TimeZone::UTC)
        .expect("a UTC time")
        .timestamp();
    assert_eq!(heard(&host, cx), [format!("at {morning}")]);
}

/// A signature the host keeps, and every change it heard.
struct Signing {
    words: SharedString,
    heard: Vec<String>,
}

impl Render for Signing {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let owner = cx.entity();
        SignatureEditor::new("signature", self.words.clone()).on_change(move |words, _, cx| {
            owner.update(cx, |host, cx| {
                host.heard.push(words.to_string());
                host.words = words.clone();
                cx.notify();
            })
        })
    }
}

fn signing<'a>(
    words: &str,
    cx: &'a mut TestAppContext,
) -> (Entity<Signing>, &'a mut VisualTestContext) {
    super::setup(cx);
    cx.update(forms::bind_keys);
    let (host, cx) = cx.add_window_view(|_, _| Signing {
        words: SharedString::from(words.to_string()),
        heard: Vec::new(),
    });
    cx.update(|window, _| window.activate_window());
    settle(cx);
    (host, cx)
}

#[gpui::test]
fn an_echo_keeps_the_caret_where_it_is(cx: &mut TestAppContext) {
    let (host, cx) = signing("", cx);
    tab_to(1, cx);
    cx.simulate_input("ab");
    settle(cx);
    press("left", cx);
    cx.simulate_input("X");
    settle(cx);
    cx.simulate_input("Y");
    settle(cx);
    let heard = host.read_with(cx, |host, _| host.heard.clone());
    assert_eq!(
        heard.last().map(String::as_str),
        Some("aXYb"),
        "the caret stayed after X"
    );
}

#[gpui::test]
fn a_new_signature_from_the_owner_starts_the_field_over(cx: &mut TestAppContext) {
    let (host, cx) = signing("Ana", cx);
    host.update(cx, |host, cx| {
        host.words = "Ben Ito".into();
        cx.notify();
    });
    settle(cx);
    tab_to(1, cx);
    cx.simulate_input("!");
    settle(cx);
    let heard = host.read_with(cx, |host, _| host.heard.clone());
    assert_eq!(heard, ["Ben Ito!"]);
}

/// A composer the host starts from an address, and what it sent.
struct Replying {
    to: SharedString,
    sent: Vec<String>,
}

impl Render for Replying {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let owner = cx.entity();
        MailComposer::new("composer")
            .to([self.to.clone()])
            .subject("Re: Samples")
            .on_send(move |draft, _, cx| {
                owner.update(cx, |host, _| {
                    host.sent
                        .push(format!("{} | {}", draft.to.join(","), draft.subject))
                })
            })
    }
}

#[gpui::test]
fn a_new_start_from_the_owner_begins_the_draft_over(cx: &mut TestAppContext) {
    super::setup(cx);
    cx.update(forms::bind_keys);
    let (host, cx) = cx.add_window_view(|_, _| Replying {
        to: "ana@atrium.studio".into(),
        sent: Vec::new(),
    });
    cx.update(|window, _| window.activate_window());
    settle(cx);
    host.update(cx, |host, cx| {
        host.to = "ben@atrium.studio".into();
        cx.notify();
    });
    settle(cx);
    tab_to(5, cx);
    press("space", cx);
    assert_eq!(
        host.read_with(cx, |host, _| host.sent.clone()),
        ["ben@atrium.studio | Re: Samples"]
    );
}
