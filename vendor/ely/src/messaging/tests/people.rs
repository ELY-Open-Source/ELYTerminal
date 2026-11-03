use gpui::{
    Context, IntoElement, ParentElement, Render, SharedString, Styled, TestAppContext, Window, div,
    px,
};

use super::{press, settle, setup, tab_to};
use crate::{
    data_display::Presence,
    forms,
    messaging::{ClearAfter, Member, MemberList, Status, StatusSetter, UserProfileCard},
};

/// Four members, and what the list asked.
struct Members {
    asked: Vec<String>,
}

impl Render for Members {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (select, activate) = (cx.entity(), cx.entity());
        div().w(px(320.0)).child(
            MemberList::new(
                "members",
                [
                    Member::new("dev", "Dev Rao", Presence::Offline),
                    Member::new("ben", "Ben Ito", Presence::Online).role("Admin"),
                    Member::new("ana", "Ana Lima", Presence::Away).status("🌴 Away till Monday"),
                ],
            )
            .on_select(move |key, _, cx| {
                select.update(cx, |host, _| host.asked.push(format!("select {key}")))
            })
            .on_activate(move |key, _, cx| {
                activate.update(cx, |host, _| host.asked.push(format!("activate {key}")))
            }),
        )
    }
}

#[gpui::test]
fn an_arrow_selects_a_member_and_enter_acts_on_them(cx: &mut TestAppContext) {
    setup(cx);
    let (host, cx) = cx.add_window_view(|_, _| Members { asked: Vec::new() });
    cx.update(|window, _| window.activate_window());
    settle(cx);
    tab_to(1, cx);
    press("down", cx);
    press("enter", cx);
    tab_to(2, cx);
    press("enter", cx);
    assert_eq!(
        host.read_with(cx, |host, _| host.asked.clone()),
        ["select ben", "activate ben", "activate dev"],
        "online comes first by name, then offline"
    );
}

#[test]
#[should_panic(expected = "member ana twice")]
fn a_member_is_listed_once() {
    let _ = MemberList::new(
        "members",
        [
            Member::new("ana", "Ana", Presence::Online),
            Member::new("ana", "Ana Lima", Presence::Away),
        ],
    );
}

/// A card with both ways to reach, and what it asked.
struct Card {
    asked: Vec<&'static str>,
}

impl Render for Card {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (message, call) = (cx.entity(), cx.entity());
        div().w(px(300.0)).child(
            UserProfileCard::new("card", "Ana Lima", Presence::Online)
                .title("Architect")
                .on_message(move |_, cx| message.update(cx, |host, _| host.asked.push("message")))
                .on_call(move |_, cx| call.update(cx, |host, _| host.asked.push("call"))),
        )
    }
}

#[gpui::test]
fn the_cards_ways_to_reach_each_ask_their_own(cx: &mut TestAppContext) {
    setup(cx);
    let (host, cx) = cx.add_window_view(|_, _| Card { asked: Vec::new() });
    cx.update(|window, _| window.activate_window());
    settle(cx);
    for nth in 1..=2 {
        tab_to(nth, cx);
        press("space", cx);
    }
    assert_eq!(
        host.read_with(cx, |host, _| host.asked.clone()),
        ["message", "call"]
    );
}

fn walk() -> Status {
    Status {
        emoji: Some("🚶".into()),
        text: "On a walk".into(),
        clear: ClearAfter::OneHour,
    }
}

/// A status setter with one suggestion, the status the owner holds, and what was saved.
struct Setting {
    status: Option<Status>,
    saved: Vec<Status>,
    cleared: usize,
}

impl Render for Setting {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (save, clear) = (cx.entity(), cx.entity());
        let setter = StatusSetter::new("status")
            .suggestions([walk()])
            .on_save(move |status, _, cx| {
                save.update(cx, |host, _| host.saved.push(status.clone()))
            })
            .on_clear(move |_, cx| clear.update(cx, |host, _| host.cleared += 1));
        let setter = match self.status.clone() {
            Some(status) => setter.status(status),
            None => setter,
        };
        div().w(px(360.0)).child(setter)
    }
}

fn setting(
    status: Option<Status>,
    cx: &mut TestAppContext,
) -> (gpui::Entity<Setting>, &mut gpui::VisualTestContext) {
    setup(cx);
    cx.update(forms::bind_keys);
    let (host, cx) = cx.add_window_view(|_, _| Setting {
        status,
        saved: Vec::new(),
        cleared: 0,
    });
    cx.update(|window, _| window.activate_window());
    settle(cx);
    (host, cx)
}

#[gpui::test]
fn a_suggestion_fills_the_status_and_save_hands_it_over(cx: &mut TestAppContext) {
    let (host, cx) = setting(None, cx);
    tab_to(3, cx);
    press("space", cx);
    tab_to(5, cx);
    press("space", cx);
    assert_eq!(host.read_with(cx, |host, _| host.saved.clone()), [walk()]);
}

#[gpui::test]
fn typed_words_save_trimmed(cx: &mut TestAppContext) {
    let (host, cx) = setting(None, cx);
    tab_to(2, cx);
    cx.simulate_input("  Reading ");
    settle(cx);
    tab_to(5, cx);
    press("space", cx);
    let saved = Status {
        emoji: None,
        text: SharedString::from("Reading"),
        clear: ClearAfter::Today,
    };
    assert_eq!(host.read_with(cx, |host, _| host.saved.clone()), [saved]);
}

#[gpui::test]
fn a_new_status_from_the_owner_starts_the_edits_over(cx: &mut TestAppContext) {
    let (host, cx) = setting(None, cx);
    tab_to(2, cx);
    cx.simulate_input("Reading");
    settle(cx);
    host.update(cx, |host, cx| {
        host.status = Some(walk());
        cx.notify();
    });
    settle(cx);
    tab_to(6, cx);
    press("space", cx);
    assert_eq!(host.read_with(cx, |host, _| host.saved.clone()), [walk()]);
}

#[gpui::test]
fn clear_status_asks_only_while_a_status_is_set(cx: &mut TestAppContext) {
    let (host, cx) = setting(Some(walk()), cx);
    tab_to(5, cx);
    press("space", cx);
    assert_eq!(host.read_with(cx, |host, _| host.cleared), 1);
}

#[gpui::test]
fn an_emoji_is_searched_and_picked_and_focus_comes_back(cx: &mut TestAppContext) {
    let (host, cx) = setting(None, cx);
    tab_to(1, cx);
    press("enter", cx);
    cx.update(|window, cx| window.focus_next(cx));
    settle(cx);
    cx.simulate_input("palm tree");
    settle(cx);
    press("enter", cx);
    cx.update(|window, cx| window.focus_next(cx));
    settle(cx);
    cx.simulate_input("Away");
    settle(cx);
    tab_to(5, cx);
    press("space", cx);
    let saved = Status {
        emoji: Some("🌴".into()),
        text: "Away".into(),
        clear: ClearAfter::Today,
    };
    assert_eq!(
        host.read_with(cx, |host, _| host.saved.clone()),
        [saved],
        "the words follow the emoji field in Tab order"
    );
}

/// A chat list and a member list whose open chat and selected member left.
struct Departed;

impl Render for Departed {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .w(px(320.0))
            .child(
                crate::messaging::ChannelList::new("chats")
                    .section(
                        "Channels",
                        [(
                            "general",
                            crate::messaging::ChannelItem::new("general", "general"),
                        )],
                    )
                    .open("gone"),
            )
            .child(
                MemberList::new(
                    "members",
                    [Member::new("ana", "Ana Lima", Presence::Online)],
                )
                .selected("gone"),
            )
    }
}

#[gpui::test]
fn a_chat_or_member_that_left_marks_none(cx: &mut TestAppContext) {
    setup(cx);
    let (_, cx) = cx.add_window_view(|_, _| Departed);
    settle(cx);
    assert!(cx.debug_bounds("item-selected-general").is_none());
    assert!(cx.debug_bounds("item-selected-Ana Lima").is_none());
}
