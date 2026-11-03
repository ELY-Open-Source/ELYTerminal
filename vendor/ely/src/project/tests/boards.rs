use gpui::{
    AnyElement, Entity, IntoElement, Modifiers, MouseButton, ParentElement, SharedString, Styled,
    TestAppContext, point, px,
};
use jiff::civil::date;

use super::{Desk, desk, heard, note, press, settle, tab_to};
use crate::{
    mail::Label,
    project::{
        IssueCard, KanbanBoard, KanbanCard, KanbanColumn, LabelManager, Person, Status, Task,
        TaskDetailPanel,
    },
};

fn card(key: &str) -> KanbanCard {
    let task = Task::new(SharedString::from(key.to_string()), format!("Task {key}"));
    let face =
        IssueCard::new(SharedString::from(format!("issue-{key}")), task).today(date(2026, 9, 27));
    KanbanCard::new(SharedString::from(key.to_string()), face)
}

fn board(_: &Desk, owner: Entity<Desk>) -> AnyElement {
    let (moved, opened) = (owner.clone(), owner);
    KanbanBoard::new("board")
        .column(
            KanbanColumn::new("todo", "Todo")
                .card(card("a"))
                .card(card("b")),
        )
        .column(KanbanColumn::new("doing", "In progress").card(card("c")))
        .on_move(move |key, column, index, _, cx| {
            note(&moved, format!("move {key} {column} {index}"), cx)
        })
        .on_open(move |key, _, cx| note(&opened, format!("open {key}"), cx))
        .into_any_element()
}

/// Presses at `from`, drags in steps to `to`, and lets go there.
fn drag(
    from: gpui::Point<gpui::Pixels>,
    to: gpui::Point<gpui::Pixels>,
    cx: &mut gpui::VisualTestContext,
) {
    cx.simulate_mouse_move(from, None, Modifiers::none());
    cx.simulate_mouse_down(from, MouseButton::Left, Modifiers::none());
    for step in 1..=6 {
        let share = step as f32 / 6.0;
        let at = point(
            from.x + (to.x - from.x) * share,
            from.y + (to.y - from.y) * share,
        );
        cx.simulate_mouse_move(at, Some(MouseButton::Left), Modifiers::none());
        settle(cx);
    }
    cx.simulate_mouse_up(to, MouseButton::Left, Modifiers::none());
    settle(cx);
}

#[gpui::test]
fn a_card_dragged_to_another_column_lands_where_it_is_let_go(cx: &mut TestAppContext) {
    let (host, cx) = desk(board, cx);
    let from = cx.debug_bounds("card a").expect("card a draws").center();
    let lane = cx.debug_bounds("column doing").expect("the column draws");
    drag(from, point(lane.center().x, lane.bottom() - px(8.0)), cx);
    assert_eq!(
        heard(&host, cx),
        ["move a doing 1"],
        "below the column's one card"
    );
}

#[gpui::test]
fn a_card_let_go_in_its_own_place_does_not_move(cx: &mut TestAppContext) {
    let (host, cx) = desk(board, cx);
    let from = cx.debug_bounds("card a").expect("card a draws").center();
    drag(from, point(from.x, from.y + px(6.0)), cx);
    assert!(heard(&host, cx).is_empty(), "{:?}", heard(&host, cx));
}

#[gpui::test]
fn option_arrows_move_the_focused_card_and_enter_opens_it(cx: &mut TestAppContext) {
    let (host, cx) = desk(board, cx);
    tab_to(1, cx);
    press("down", cx);
    press("alt-down", cx);
    press("alt-right", cx);
    press("enter", cx);
    assert_eq!(
        heard(&host, cx),
        ["move a todo 1", "move a doing 0", "open a"]
    );
}

fn narrow(desk: &Desk, owner: Entity<Desk>) -> AnyElement {
    gpui::div()
        .w(px(300.0))
        .child(board(desk, owner))
        .into_any_element()
}

#[gpui::test]
fn a_focused_card_past_the_fold_scrolls_into_view(cx: &mut TestAppContext) {
    let (_, cx) = desk(narrow, cx);
    tab_to(3, cx);
    let board = cx.debug_bounds("kanban-board").expect("the board draws");
    let card = cx.debug_bounds("card c").expect("card c draws");
    assert!(
        board.left() <= card.left() && card.right() <= board.right(),
        "card c at {card:?} shows inside the board at {board:?}"
    );
}

fn labels(_: &Desk, owner: Entity<Desk>) -> AnyElement {
    let (renamed, made) = (owner.clone(), owner);
    LabelManager::new(
        "labels",
        [Label::new("bug", "Bug", 3), Label::new("site", "Site", 1)],
    )
    .on_rename(move |key, name, _, cx| note(&renamed, format!("rename {key} {name}"), cx))
    .on_create(move |name, _, cx| note(&made, format!("make {name}"), cx))
    .into_any_element()
}

#[gpui::test]
fn a_label_keeps_a_name_of_its_own(cx: &mut TestAppContext) {
    let (host, cx) = desk(labels, cx);
    tab_to(2, cx);
    press("enter", cx);
    cx.simulate_input("site");
    press("enter", cx);
    assert!(
        cx.debug_bounds("refused bug").is_some(),
        "a taken name is refused"
    );
    tab_to(2, cx);
    press("enter", cx);
    cx.simulate_input("Defect");
    press("enter", cx);
    tab_to(7, cx);
    cx.simulate_input("bug");
    press("enter", cx);
    assert!(
        cx.debug_bounds("refused +new").is_some(),
        "a new label cannot take one"
    );
    cx.simulate_input(" report");
    press("enter", cx);
    assert_eq!(heard(&host, cx), ["rename bug Defect", "make bug report"]);
}

fn panel(_: &Desk, owner: Entity<Desk>) -> AnyElement {
    let task = Task::new("brief", "Write the brief").status(Status::Todo);
    TaskDetailPanel::new("panel", task, [Person::new("ana", "Ana Lima")])
        .on_change(move |task, _, cx| {
            let said = format!("{} {:?} {:?}", task.title, task.status, task.priority);
            note(&owner, said, cx)
        })
        .into_any_element()
}

#[gpui::test]
fn the_panel_hands_on_the_task_after_each_edit(cx: &mut TestAppContext) {
    let (host, cx) = desk(panel, cx);
    tab_to(1, cx);
    press("enter", cx);
    cx.simulate_input("   ");
    press("enter", cx);
    tab_to(1, cx);
    press("enter", cx);
    cx.simulate_input("Client brief");
    press("enter", cx);
    tab_to(2, cx);
    cx.simulate_keystrokes("down down enter");
    settle(cx);
    tab_to(3, cx);
    cx.simulate_keystrokes("down up enter");
    settle(cx);
    assert_eq!(
        heard(&host, cx),
        [
            "Client brief Todo None",
            "Write the brief InProgress None",
            "Write the brief Todo Low"
        ],
        "a blank title is not taken"
    );
}

fn roadmap(_: &Desk, _: Entity<Desk>) -> AnyElement {
    crate::project::Roadmap::new("roadmap", date(2026, 9, 1), 2)
        .lane(
            "Design",
            [
                crate::project::Initiative::new(
                    "early",
                    "Early",
                    date(2026, 7, 1),
                    date(2026, 8, 30),
                ),
                crate::project::Initiative::new(
                    "half",
                    "Half",
                    date(2026, 10, 1),
                    date(2026, 10, 31),
                ),
            ],
        )
        .into_any_element()
}

#[gpui::test]
fn a_roadmap_draws_only_what_falls_in_its_months(cx: &mut TestAppContext) {
    let (_, cx) = desk(roadmap, cx);
    assert!(
        cx.debug_bounds("initiative early").is_none(),
        "it ended before the span"
    );
    let map = cx.debug_bounds("roadmap").expect("the roadmap draws");
    let half = cx.debug_bounds("initiative half").expect("October draws");
    let share = (half.left() - map.left()) / map.size.width;
    assert!(
        (share - 30.0 / 61.0).abs() < 0.01,
        "October starts 30 of 61 days in: {share}"
    );
    let end = (half.right() - map.left()) / map.size.width;
    assert!(
        (end - 1.0).abs() < 0.01,
        "October runs through its last day: {end}"
    );
}

/// A roadmap at the end of jiff's range, a milestone past its count and an assignee who left.
fn far(_: &Desk, _: Entity<Desk>) -> AnyElement {
    let people = [crate::project::Person::new("ana", "Ana Lima")];
    gpui::div()
        .w(gpui::px(640.0))
        .child(
            crate::project::Roadmap::new("roadmap", date(9999, 11, 1), 3).lane(
                "Design",
                [crate::project::Initiative::new(
                    "last",
                    "Last",
                    date(9999, 12, 1),
                    date(9999, 12, 31),
                )],
            ),
        )
        .child(
            crate::project::MilestoneProgress::new(
                "milestone",
                "Launch",
                (date(2026, 9, 1), date(2026, 10, 1)),
                (6, 5),
            )
            .today(date(2026, 9, 15)),
        )
        .child(crate::project::AssigneePicker::new("assignee", people).assignee("gone"))
        .into_any_element()
}

#[gpui::test]
fn the_end_of_time_an_overcount_and_a_gone_assignee_draw(cx: &mut TestAppContext) {
    let (_, cx) = desk(far, cx);
    settle(cx);
}

/// A roadmap past jiff's last month with a bar on its last day alone.
fn last_day(_: &Desk, _: Entity<Desk>) -> AnyElement {
    crate::project::Roadmap::new("last-day-map", date(9999, 11, 1), 3)
        .lane(
            "Last",
            [crate::project::Initiative::new(
                "only-last-day",
                "Only last day",
                date(9999, 12, 31),
                date(9999, 12, 31),
            )],
        )
        .into_any_element()
}

#[gpui::test]
fn the_last_day_jiff_holds_keeps_its_bar(cx: &mut TestAppContext) {
    let (_, cx) = desk(last_day, cx);
    let bar = cx
        .debug_bounds("initiative only-last-day")
        .expect("the last day belongs to December");
    assert!(bar.size.width > px(0.0), "one day has a width");
}

/// Tasks a database lays out: one due at jiff's last day, one due before it starts.
fn far_tasks(layout: crate::project::DatabaseLayout) -> AnyElement {
    let tasks = [
        Task::new("a", "A")
            .starts(date(2026, 9, 1))
            .due(date(9999, 12, 31)),
        Task::new("r", "R")
            .starts(date(2026, 9, 27))
            .due(date(2026, 8, 27)),
    ];
    crate::project::DatabaseView::new("database", tasks)
        .layout(layout)
        .today(date(2026, 9, 1))
        .into_any_element()
}

#[gpui::test]
fn a_timeline_caps_long_spans_and_leaves_out_backward_tasks(cx: &mut TestAppContext) {
    let (_, cx) = desk(
        |_, _| far_tasks(crate::project::DatabaseLayout::Timeline),
        cx,
    );
    assert!(cx.debug_bounds("initiative a").is_some());
    assert!(cx.debug_bounds("initiative r").is_none());
}

#[gpui::test]
fn a_calendar_leaves_out_days_past_its_own(cx: &mut TestAppContext) {
    let _ = desk(
        |_, _| {
            let last = Task::new("a", "A").due(date(9999, 12, 31));
            crate::project::DatabaseView::new("database", [last])
                .layout(crate::project::DatabaseLayout::Calendar)
                .today(date(2026, 9, 1))
                .into_any_element()
        },
        cx,
    );
}

/// A board whose second column leaves once anything is heard.
fn changing_board(desk: &Desk, _: Entity<Desk>) -> AnyElement {
    let board = KanbanBoard::new("changing-board")
        .column(KanbanColumn::new("todo", "Todo").card(card("a")));
    let board = match desk.heard.is_empty() {
        true => board.column(KanbanColumn::new("gone", "Gone").card(card("b"))),
        false => board,
    };
    board.into_any_element()
}

#[gpui::test]
fn a_column_that_leaves_during_a_drag_takes_no_card(cx: &mut TestAppContext) {
    let (host, cx) = desk(changing_board, cx);
    let from = cx.debug_bounds("card a").expect("card a").center();
    let gone = cx
        .debug_bounds("column gone")
        .expect("column gone")
        .center();
    cx.simulate_mouse_move(from, None, Modifiers::none());
    cx.simulate_mouse_down(from, MouseButton::Left, Modifiers::none());
    for shift in [12.0, 16.0] {
        let at = point(from.x + px(shift), from.y);
        cx.simulate_mouse_move(at, Some(MouseButton::Left), Modifiers::none());
        settle(cx);
    }
    host.update(cx, |host, cx| {
        host.heard.push("removed".into());
        cx.notify();
    });
    settle(cx);
    assert!(cx.debug_bounds("column gone").is_none());
    cx.simulate_mouse_move(gone, Some(MouseButton::Left), Modifiers::none());
    settle(cx);
    cx.simulate_mouse_up(gone, MouseButton::Left, Modifiers::none());
    settle(cx);
}

/// Drags `card a` toward `to` in six moves, as a hand would.
fn drag_toward(to: gpui::Point<gpui::Pixels>, cx: &mut gpui::VisualTestContext) {
    let from = cx.debug_bounds("card a").expect("card a").center();
    cx.simulate_mouse_move(from, None, Modifiers::none());
    cx.simulate_mouse_down(from, MouseButton::Left, Modifiers::none());
    for step in 1..=6 {
        let share = step as f32 / 6.0;
        let at = point(
            from.x + (to.x - from.x) * share,
            from.y + (to.y - from.y) * share,
        );
        cx.simulate_mouse_move(at, Some(MouseButton::Left), Modifiers::none());
        settle(cx);
    }
}

/// A board whose second column leaves, and the moves it asks.
fn leaving_column(host: &Desk, owner: Entity<Desk>) -> AnyElement {
    let board =
        KanbanBoard::new("drop-board").column(KanbanColumn::new("todo", "Todo").card(card("a")));
    let board = match host.heard.is_empty() {
        true => board.column(KanbanColumn::new("gone", "Gone").card(card("b"))),
        false => board,
    };
    board
        .on_move(move |key, column, index, _, cx| {
            note(&owner, format!("move {key} {column} {index}"), cx)
        })
        .into_any_element()
}

#[gpui::test]
fn a_drop_on_a_column_that_left_moves_nothing(cx: &mut TestAppContext) {
    let (host, cx) = desk(leaving_column, cx);
    let gone = cx
        .debug_bounds("column gone")
        .expect("column gone")
        .center();
    drag_toward(gone, cx);
    host.update(cx, |host, cx| {
        host.heard.push("removed".into());
        cx.notify();
    });
    settle(cx);
    cx.simulate_mouse_up(gone, MouseButton::Left, Modifiers::none());
    settle(cx);
    assert_eq!(heard(&host, cx), ["removed"]);
}

/// A target column that loses a card, and the moves it asks.
fn shrinking_target(host: &Desk, owner: Entity<Desk>) -> AnyElement {
    let target = KanbanColumn::new("target", "Target").card(card("b"));
    let target = match host.heard.is_empty() {
        true => target.card(card("c")),
        false => target,
    };
    let todo = KanbanColumn::new("todo", "Todo")
        .card(card("a"))
        .card(card("d"))
        .card(card("e"));
    KanbanBoard::new("shrinking-board")
        .column(todo)
        .column(target)
        .on_move(move |key, column, index, _, cx| {
            note(&owner, format!("move {key} {column} {index}"), cx)
        })
        .into_any_element()
}

#[gpui::test]
fn a_drop_past_a_column_that_lost_a_card_moves_nothing(cx: &mut TestAppContext) {
    let (host, cx) = desk(shrinking_target, cx);
    let target = cx.debug_bounds("column target").expect("column target");
    let to = point(target.center().x, target.bottom() - px(8.0));
    drag_toward(to, cx);
    host.update(cx, |host, cx| {
        host.heard.push("removed c".into());
        cx.notify();
    });
    settle(cx);
    cx.simulate_mouse_up(to, MouseButton::Left, Modifiers::none());
    settle(cx);
    assert_eq!(heard(&host, cx), ["removed c"]);
}
