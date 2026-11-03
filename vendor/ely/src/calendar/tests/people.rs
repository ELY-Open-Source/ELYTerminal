use gpui::{Entity, IntoElement, TestAppContext};
use jiff::{civil::date, tz::TimeZone};

use super::{Planning, at, heard, note, planning, press, tab_to};
use crate::calendar::{AgendaView, Answer, Attendee, AttendeeList, Event, EventPopover};

fn guests(_: &Planning, _: Entity<Planning>) -> gpui::AnyElement {
    AttendeeList::new(
        "guests",
        [
            Attendee::new("Ben Ito", "ben@atrium.studio", Answer::Going),
            Attendee::new("Dev Rao", "dev@atrium.studio", Answer::Maybe),
            Attendee::new("Ana Lima", "ana@atrium.studio", Answer::Going).organizer(),
        ],
    )
    .into_any_element()
}

#[gpui::test]
fn the_one_who_asked_comes_first(cx: &mut TestAppContext) {
    let (_, cx) = planning(guests, cx);
    let ana = cx
        .debug_bounds("guest ana@atrium.studio")
        .expect("Ana draws");
    let ben = cx
        .debug_bounds("guest ben@atrium.studio")
        .expect("Ben draws");
    assert!(ana.top() < ben.top(), "{ana:?} {ben:?}");
}

fn agenda(_: &Planning, owner: Entity<Planning>) -> gpui::AnyElement {
    AgendaView::new(
        "agenda",
        date(2026, 9, 21),
        7,
        [
            Event::timed("crit", "Crit", at(24, 14), at(24, 15), 3),
            Event::timed("review", "Review", at(22, 11), at(22, 12), 0),
            Event::timed("standup", "Stand-up", at(22, 9), at(22, 10), 1),
        ],
    )
    .zone(TimeZone::UTC)
    .today(date(2026, 9, 22))
    .on_event(move |key, _, cx| note(&owner, format!("open {key}"), cx))
    .into_any_element()
}

#[gpui::test]
fn enter_opens_the_event_under_the_cursor(cx: &mut TestAppContext) {
    let (host, cx) = planning(agenda, cx);
    tab_to(1, cx);
    press("down", cx);
    press("enter", cx);
    tab_to(2, cx);
    press("enter", cx);
    assert_eq!(
        heard(&host, cx),
        ["open review", "open crit"],
        "today's stand-up then review; Thursday's crit in the next list"
    );
}

fn empty(_: &Planning, _: Entity<Planning>) -> gpui::AnyElement {
    AgendaView::new("agenda", date(2026, 9, 21), 7, [])
        .zone(TimeZone::UTC)
        .today(date(2026, 9, 22))
        .into_any_element()
}

#[gpui::test]
fn an_empty_stretch_says_nothing_is_planned(cx: &mut TestAppContext) {
    let (_, cx) = planning(empty, cx);
    assert!(cx.debug_bounds("agenda-none").is_some());
}

fn popover(_: &Planning, owner: Entity<Planning>) -> gpui::AnyElement {
    let (edits, deletes) = (owner.clone(), owner);
    EventPopover::new(
        "popover",
        Event::timed("review", "Review", at(22, 11), at(22, 12), 0).place("Model room"),
    )
    .zone(TimeZone::UTC)
    .attendees([Attendee::new("Ana Lima", "ana@atrium.studio", Answer::Going).organizer()])
    .on_edit(move |key, _, cx| note(&edits, format!("edit {key}"), cx))
    .on_delete(move |key, _, cx| note(&deletes, format!("delete {key}"), cx))
    .into_any_element()
}

#[gpui::test]
fn a_chip_opens_its_details_and_edit_names_it(cx: &mut TestAppContext) {
    let (host, cx) = planning(popover, cx);
    tab_to(1, cx);
    let chip = cx.update(|window, cx| window.focused(cx));
    press("space", cx);
    assert!(
        cx.debug_bounds("event-popover review").is_some(),
        "the details open"
    );
    cx.update(|window, cx| {
        window.focus_next(cx);
        window.focus_next(cx);
    });
    press("space", cx);
    assert_eq!(
        heard(&host, cx),
        ["edit review"],
        "Tab reaches Delete, then Edit"
    );
    let focused = cx.update(|window, cx| window.focused(cx));
    assert_eq!(
        focused, chip,
        "Edit closes the details and hands focus back to the chip"
    );
}
