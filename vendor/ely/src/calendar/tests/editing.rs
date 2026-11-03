use gpui::{Entity, IntoElement, Modifiers, TestAppContext, VisualTestContext, point, px};
use jiff::{SignedDuration, Timestamp, civil::date, tz::TimeZone};

use super::{Planning, at, heard, note, planning, press, settle, tab_to};
use crate::calendar::{
    AvailabilityPicker, Event, EventDraft, EventEditor, RecurrenceEditor, ReminderPicker, When,
    editor::{moved, toggled},
};

fn draft(title: &str, from: i8, to: i8) -> EventDraft {
    EventDraft {
        title: title.to_string().into(),
        when: When::Timed {
            start: at(22, from),
            end: at(22, to),
        },
        place: "".into(),
        hue: 0,
        repeat: None,
        reminders: Vec::new(),
        notes: "".into(),
    }
}

fn said(draft: &EventDraft) -> String {
    match draft.when {
        When::Timed { start, end } => format!("save {} {start} {end}", draft.title),
        When::AllDay { first, last } => format!("save {} {first} to {last}", draft.title),
    }
}

fn editor(host: &Planning, owner: Entity<Planning>) -> gpui::AnyElement {
    let title = match host.month.day() {
        1 => "",
        _ => "Crit",
    };
    EventEditor::new("editor", draft(title, 9, 10))
        .zone(TimeZone::UTC)
        .on_save(move |draft, _, cx| note(&owner, said(draft), cx))
        .into_any_element()
}

/// Presses the editor's last action, Save, at the right end of its row.
fn save(cx: &mut VisualTestContext) {
    let row = cx.debug_bounds("editor-actions").expect("the actions draw");
    cx.simulate_click(
        point(row.right() - px(8.0), row.center().y),
        Modifiers::none(),
    );
    settle(cx);
}

#[gpui::test]
fn save_waits_for_a_title_then_hands_on_the_draft(cx: &mut TestAppContext) {
    let (host, cx) = planning(editor, cx);
    save(cx);
    assert!(heard(&host, cx).is_empty(), "no title, no save");
    tab_to(1, cx);
    cx.simulate_input("  Model review ");
    settle(cx);
    save(cx);
    assert_eq!(
        heard(&host, cx),
        [format!("save Model review {} {}", at(22, 9), at(22, 10))]
    );
}

#[gpui::test]
fn all_day_turns_the_hours_into_their_day(cx: &mut TestAppContext) {
    let (host, cx) = planning(editor, cx);
    tab_to(1, cx);
    cx.simulate_input("Fair");
    settle(cx);
    tab_to(2, cx);
    press("space", cx);
    save(cx);
    assert_eq!(heard(&host, cx), ["save Fair 2026-09-22 to 2026-09-22"]);
}

fn backward(_: &Planning, owner: Entity<Planning>) -> gpui::AnyElement {
    EventEditor::new("editor", draft("Crit", 10, 9))
        .zone(TimeZone::UTC)
        .on_save(move |draft, _, cx| note(&owner, said(draft), cx))
        .into_any_element()
}

#[gpui::test]
fn an_end_before_the_start_says_so_and_holds_save(cx: &mut TestAppContext) {
    let (host, cx) = planning(backward, cx);
    assert!(cx.debug_bounds("editor-fault").is_some());
    save(cx);
    assert!(heard(&host, cx).is_empty());
}

#[gpui::test]
fn a_new_draft_from_the_owner_starts_the_editor_over(cx: &mut TestAppContext) {
    let (host, cx) = planning(editor, cx);
    host.update(cx, |host, cx| {
        host.month = date(2026, 10, 2);
        cx.notify();
    });
    settle(cx);
    save(cx);
    assert_eq!(
        heard(&host, cx),
        [format!("save Crit {} {}", at(22, 9), at(22, 10))]
    );
}

fn reminders(_: &Planning, owner: Entity<Planning>) -> gpui::AnyElement {
    ReminderPicker::new("reminders", [10])
        .on_change(move |minutes, _, cx| note(&owner, format!("{minutes:?}"), cx))
        .into_any_element()
}

#[gpui::test]
fn add_brings_the_nearest_reminder_not_yet_set(cx: &mut TestAppContext) {
    let (host, cx) = planning(reminders, cx);
    tab_to(3, cx);
    press("space", cx);
    tab_to(2, cx);
    press("space", cx);
    assert_eq!(heard(&host, cx), ["[0, 10]", "[]"], "Add, then the way off");
}

fn repeat(_: &Planning, owner: Entity<Planning>) -> gpui::AnyElement {
    RecurrenceEditor::new("repeat", date(2026, 9, 27), None)
        .on_change(move |rule, _, cx| {
            let words = rule.map(|rule| rule.describe(date(2026, 9, 27)));
            note(&owner, format!("{words:?}"), cx)
        })
        .into_any_element()
}

#[gpui::test]
fn picking_a_frequency_starts_a_rule(cx: &mut TestAppContext) {
    let (host, cx) = planning(repeat, cx);
    tab_to(1, cx);
    cx.simulate_keystrokes("down down down enter");
    settle(cx);
    assert_eq!(heard(&host, cx), ["Some(\"Every week on Sunday\")"]);
}

fn slots(_: &Planning, owner: Entity<Planning>) -> gpui::AnyElement {
    AvailabilityPicker::new("slots", [date(2026, 9, 28), date(2026, 9, 29)])
        .zone(TimeZone::UTC)
        .hours(9, 12)
        .busy([(at(28, 9), at(28, 10))])
        .now(at(27, 12))
        .on_pick(move |start, _, cx| note(&owner, format!("pick {start}"), cx))
        .into_any_element()
}

#[gpui::test]
fn a_slot_pick_hands_on_its_start(cx: &mut TestAppContext) {
    let (host, cx) = planning(slots, cx);
    tab_to(3, cx);
    press("space", cx);
    assert_eq!(
        heard(&host, cx),
        [format!("pick {}", at(28, 10))],
        "09:00 to 10:00 is taken"
    );
}

fn paged(host: &Planning, owner: Entity<Planning>) -> gpui::AnyElement {
    let first = host.month;
    let dawn = date(2026, 8, 31).at(0, 0, 0, 0).to_zoned(TimeZone::UTC);
    AvailabilityPicker::new("paged", [first, first.tomorrow().expect("a day ahead")])
        .zone(TimeZone::UTC)
        .hours(9, 10)
        .now(dawn.expect("a UTC time").timestamp())
        .on_pick(move |start, _, cx| note(&owner, format!("pick {start}"), cx))
        .into_any_element()
}

#[gpui::test]
fn new_days_from_the_owner_start_the_choice_over(cx: &mut TestAppContext) {
    let (host, cx) = planning(paged, cx);
    tab_to(2, cx);
    press("space", cx);
    host.update(cx, |host, cx| {
        host.month = date(2026, 10, 5);
        cx.notify();
    });
    settle(cx);
    tab_to(3, cx);
    press("space", cx);
    let nine = date(2026, 10, 5).at(9, 0, 0, 0).to_zoned(TimeZone::UTC);
    assert_eq!(
        heard(&host, cx),
        [format!("pick {}", nine.expect("a UTC time").timestamp())],
        "the chosen day of the old run is gone; the first open day shows"
    );
}

#[test]
fn only_a_timed_event_moves_and_only_while_its_end_exists() {
    let all_day = When::AllDay {
        first: date(2026, 9, 1),
        last: date(2026, 9, 2),
    };
    assert_eq!(moved(all_day, Timestamp::UNIX_EPOCH), None);
    let start = Timestamp::UNIX_EPOCH;
    let hour = When::Timed {
        start,
        end: start + SignedDuration::from_hours(1),
    };
    assert_eq!(
        moved(hour, Timestamp::MAX),
        None,
        "no end past jiff's last moment"
    );
    let later = start + SignedDuration::from_hours(5);
    let expected = When::Timed {
        start: later,
        end: later + SignedDuration::from_hours(1),
    };
    assert_eq!(moved(hour, later), Some(expected));
}

#[test]
fn a_day_at_jiffs_end_stays_all_day_and_an_end_at_its_start_has_days() {
    let last = When::AllDay {
        first: date(9999, 12, 31),
        last: date(9999, 12, 31),
    };
    let west = TimeZone::fixed(jiff::tz::offset(-12));
    assert_eq!(toggled(last, &west), None);
    let start = Timestamp::MIN + SignedDuration::from_hours(1);
    let mut backward = Event::timed(
        "e",
        "Backward",
        start,
        start + SignedDuration::from_hours(1),
        0,
    );
    backward.when = When::Timed {
        start,
        end: Timestamp::MIN,
    };
    let first = start.to_zoned(TimeZone::UTC).date();
    assert_eq!(backward.days(&TimeZone::UTC), (first, first));
}
