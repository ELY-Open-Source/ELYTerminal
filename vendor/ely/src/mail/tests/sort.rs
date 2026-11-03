use gpui::{
    Context, Entity, IntoElement, Modifiers, ParentElement, Render, SharedString, Styled,
    TestAppContext, VisualTestContext, Window, div, px,
};
use jiff::{Timestamp, ToSpan, tz::TimeZone};

use super::{Mailing, heard, mailing, note, press, settle, setup, tab_to};
use crate::{
    forms,
    mail::{Label, LabelPicker, SnoozePicker, snooze::snoozes, times::at_hour},
    typography::format::system_zone,
};

/// Labels the host keeps, the keys on, and what it heard.
struct Labeling {
    labels: Vec<Label>,
    on: Vec<SharedString>,
    heard: Vec<String>,
}

impl Render for Labeling {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (changing, making) = (cx.entity(), cx.entity());
        div().w(px(320.0)).child(
            LabelPicker::new("labels", self.labels.clone())
                .selected(self.on.clone())
                .on_change(move |keys, _, cx| {
                    changing.update(cx, |host, cx| {
                        host.heard.push(format!("on [{}]", keys.join(" ")));
                        host.on = keys.to_vec();
                        cx.notify();
                    })
                })
                .on_create(move |name, _, cx| {
                    making.update(cx, |host, cx| {
                        host.heard.push(format!("make {name}"));
                        let key = SharedString::from(name.to_lowercase());
                        let hue = host.labels.len();
                        host.labels.push(Label::new(key.clone(), name.clone(), hue));
                        host.on.push(key);
                        cx.notify();
                    })
                }),
        )
    }
}

fn labeling(cx: &mut TestAppContext) -> (Entity<Labeling>, &mut VisualTestContext) {
    setup(cx);
    let (host, cx) = cx.add_window_view(|_, _| Labeling {
        labels: vec![
            Label::new("clients", "Clients", 0),
            Label::new("finance", "Finance", 1),
            Label::new("press", "Press", 2),
            Label::new("travel", "Travel", 3),
        ],
        on: vec!["press".into()],
        heard: Vec::new(),
    });
    cx.update(|window, cx| {
        window.activate_window();
        forms::bind_keys(cx);
    });
    settle(cx);
    tab_to(1, cx);
    (host, cx)
}

fn told(host: &Entity<Labeling>, cx: &mut VisualTestContext) -> Vec<String> {
    host.read_with(cx, |host, _| host.heard.clone())
}

fn find(words: &str, cx: &mut VisualTestContext) {
    cx.simulate_input(words);
    settle(cx);
}

#[gpui::test]
fn typing_narrows_the_labels_and_enter_turns_the_first_on(cx: &mut TestAppContext) {
    let (host, cx) = labeling(cx);
    find("fin", cx);
    press("enter", cx);
    assert_eq!(told(&host, cx), ["on [finance press]"]);
}

#[gpui::test]
fn arrows_wrap_and_a_press_turns_a_label_off(cx: &mut TestAppContext) {
    let (host, cx) = labeling(cx);
    press("up", cx);
    press("enter", cx);
    let row = cx
        .debug_bounds("label press")
        .expect("the press label draws");
    cx.simulate_click(row.center(), Modifiers::none());
    settle(cx);
    assert_eq!(told(&host, cx), ["on [press travel]", "on [travel]"]);
}

#[gpui::test]
fn a_name_no_label_has_makes_one_and_the_field_starts_over(cx: &mut TestAppContext) {
    let (host, cx) = labeling(cx);
    find("Suppliers", cx);
    press("enter", cx);
    press("enter", cx);
    assert_eq!(
        told(&host, cx),
        ["make Suppliers", "on [clients press suppliers]"],
        "with the field empty again, Enter takes the first label"
    );
}

#[gpui::test]
fn a_name_a_label_has_is_not_offered_again(cx: &mut TestAppContext) {
    let (host, cx) = labeling(cx);
    find("PRESS", cx);
    press("down", cx);
    press("enter", cx);
    assert_eq!(told(&host, cx), ["on []"], "Press is the only row");
}

#[test]
#[should_panic(expected = "label name clients twice")]
fn a_label_name_is_listed_once() {
    let _ = LabelPicker::new(
        "labels",
        [Label::new("a", "Clients", 0), Label::new("b", "clients", 1)],
    );
}

#[gpui::test]
fn a_label_mail_or_box_that_left_draws_unmarked(cx: &mut TestAppContext) {
    let _ = mailing(
        |_| {
            let mail = crate::mail::Mail::new("m1", "Ada", "Plans", Timestamp::UNIX_EPOCH);
            let inbox =
                crate::mail::Mailbox::new("inbox", "Inbox", crate::primitives::IconName::Inbox);
            div()
                .w(px(480.0))
                .child(LabelPicker::new("labels", [Label::new("a", "A", 0)]).selected(["ghost"]))
                .child(crate::mail::MailList::new("mails", [mail]).selected(["gone"]))
                .child(
                    crate::mail::MailboxList::new("boxes")
                        .section("Mail", [inbox])
                        .open("gone"),
                )
                .into_any_element()
        },
        cx,
    );
}

fn snoozing(owner: Entity<Mailing>) -> gpui::AnyElement {
    SnoozePicker::new("snooze")
        .zone(TimeZone::UTC)
        .on_snooze(move |at, _, cx| note(&owner, format!("until {at}"), cx))
        .into_any_element()
}

#[gpui::test]
fn the_first_time_offered_puts_the_message_off(cx: &mut TestAppContext) {
    let (host, cx) = mailing(snoozing, cx);
    tab_to(1, cx);
    press("enter", cx);
    press("enter", cx);
    let (_, first) = snoozes(Timestamp::now(), &TimeZone::UTC).remove(0);
    assert_eq!(heard(&host, cx), [format!("until {first}")]);
}

#[gpui::test]
fn the_last_row_asks_for_a_time_of_ones_own(cx: &mut TestAppContext) {
    let (host, cx) = mailing(snoozing, cx);
    tab_to(1, cx);
    press("enter", cx);
    press("up", cx);
    press("enter", cx);
    assert!(cx.debug_bounds("time-dialog").is_some(), "the dialog shows");
    assert!(heard(&host, cx).is_empty(), "no time yet");
}

#[gpui::test]
fn the_dialog_takes_only_a_time_ahead_of_now(cx: &mut TestAppContext) {
    let (host, cx) = mailing(snoozing, cx);
    tab_to(1, cx);
    for key in [
        "enter", "up", "enter", "tab", "enter", "left", "enter", "escape",
    ] {
        press(key, cx);
    }
    assert!(
        cx.debug_bounds("time-late").is_some(),
        "yesterday has passed"
    );
    for key in ["tab", "tab", "enter"] {
        press(key, cx);
    }
    assert!(heard(&host, cx).is_empty(), "Snooze waits for a time ahead");
    for key in [
        "right", "right", "right", "enter", "escape", "tab", "tab", "enter",
    ] {
        press(key, cx);
    }
    let today = Timestamp::now().to_zoned(system_zone("a test")).date();
    let ahead = at_hour(
        today.checked_add(2.days()).expect("a day"),
        0,
        &TimeZone::UTC,
    );
    assert_eq!(heard(&host, cx), [format!("until {ahead}")]);
}
