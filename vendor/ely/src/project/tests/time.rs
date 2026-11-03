use std::time::Duration;

use gpui::{
    AnyElement, Entity, InteractiveElement, IntoElement, ParentElement, Styled, TestAppContext,
};
use jiff::civil::date;

use super::{Desk, desk, heard, note, press, settle, tab_to};
use crate::project::{DatabaseView, PomodoroTimer, Status, Task, TimeEntry, TimeTracker};

fn pomodoro(_: &Desk, owner: Entity<Desk>) -> AnyElement {
    PomodoroTimer::new("pomodoro")
        .minutes(2, 1, 3)
        .rounds(2)
        .on_phase(move |phase, _, cx| note(&owner, phase.words().into(), cx))
        .into_any_element()
}

#[gpui::test]
fn a_paused_phase_keeps_its_time_and_one_run_out_hands_on_the_next(cx: &mut TestAppContext) {
    let (host, cx) = desk(pomodoro, cx);
    tab_to(2, cx);
    press("space", cx);
    cx.executor().advance_clock(Duration::from_secs(61));
    settle(cx);
    press("space", cx);
    cx.executor().advance_clock(Duration::from_secs(600));
    settle(cx);
    assert!(
        heard(&host, cx).is_empty(),
        "a paused minute of two stays short"
    );
    press("space", cx);
    cx.executor().advance_clock(Duration::from_secs(60));
    settle(cx);
    assert_eq!(heard(&host, cx), ["Short break"]);
}

fn tracker(desk: &Desk, owner: Entity<Desk>) -> AnyElement {
    let entry = TimeEntry::new("e1", "Brief", Duration::from_secs(600));
    let (started, stopped) = (owner.clone(), owner);
    let tracker = TimeTracker::new("tracker", [entry])
        .on_start(move |task, at, _, cx| {
            started.update(cx, |desk, cx| {
                desk.heard.push(format!("start {task}"));
                desk.clock = Some((task.clone(), at));
                cx.notify();
            })
        })
        .on_stop(move |at, _, cx| {
            stopped.update(cx, |desk, cx| {
                let (task, since) = desk.clock.take().expect("a task runs");
                desk.heard.push(format!(
                    "stop {task} {}s",
                    at.duration_since(since).as_secs()
                ));
                cx.notify();
            })
        });
    match &desk.clock {
        Some((task, since)) => tracker.running(task.clone(), *since),
        None => tracker,
    }
    .into_any_element()
}

#[gpui::test]
fn enter_starts_and_stops_and_the_field_takes_the_keys_again(cx: &mut TestAppContext) {
    let (host, cx) = desk(tracker, cx);
    tab_to(1, cx);
    press("enter", cx);
    cx.simulate_input("  Site photos ");
    press("enter", cx);
    cx.executor().advance_clock(Duration::from_secs(90));
    settle(cx);
    press("enter", cx);
    cx.simulate_input("Budget");
    press("enter", cx);
    assert_eq!(
        heard(&host, cx),
        ["start Site photos", "stop Site photos 90s", "start Budget"],
        "a blank start does nothing, and focus follows each turn"
    );
}

#[gpui::test]
fn skip_moves_on_at_once(cx: &mut TestAppContext) {
    let (host, cx) = desk(pomodoro, cx);
    tab_to(3, cx);
    press("space", cx);
    assert_eq!(heard(&host, cx), ["Short break"]);
}

fn database(_: &Desk, owner: Entity<Desk>) -> AnyElement {
    let tasks = [
        Task::new("a", "Alpha")
            .starts(date(2026, 9, 20))
            .due(date(2026, 9, 30)),
        Task::new("b", "Beta").status(Status::Done),
    ];
    DatabaseView::new("db", tasks)
        .today(date(2026, 9, 27))
        .on_status(move |key, status, _, cx| note(&owner, format!("{key} {}", status.words()), cx))
        .into_any_element()
}

#[gpui::test]
fn the_switch_shows_each_way_and_the_board_moves_a_record(cx: &mut TestAppContext) {
    let (host, cx) = desk(database, cx);
    assert!(cx.debug_bounds("table-row a").is_some(), "a table first");
    tab_to(5, cx);
    press("space", cx);
    assert!(
        cx.debug_bounds("initiative a").is_some(),
        "a dated record on the timeline"
    );
    assert!(
        cx.debug_bounds("initiative b").is_none(),
        "an undated one left off"
    );
    tab_to(2, cx);
    press("space", cx);
    tab_to(6, cx);
    press("alt-right", cx);
    assert_eq!(heard(&host, cx), ["a In progress"]);
}

fn narrow_database(desk: &Desk, owner: Entity<Desk>) -> AnyElement {
    gpui::div()
        .debug_selector(|| "narrow".into())
        .w(gpui::px(280.0))
        .child(database(desk, owner))
        .into_any_element()
}

#[gpui::test]
fn a_focused_segment_past_the_fold_comes_into_view(cx: &mut TestAppContext) {
    let (_, cx) = desk(narrow_database, cx);
    tab_to(5, cx);
    let frame = cx.debug_bounds("narrow").expect("the box draws");
    let segment = cx
        .debug_bounds("segment Timeline")
        .expect("the segment draws");
    assert!(
        frame.left() <= segment.left() && segment.right() <= frame.right(),
        "Timeline at {segment:?} inside {frame:?}"
    );
}
