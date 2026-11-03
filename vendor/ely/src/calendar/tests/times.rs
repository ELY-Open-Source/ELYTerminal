use gpui::{
    Bounds, Entity, IntoElement, Modifiers, MouseButton, ParentElement, Pixels, Styled,
    TestAppContext, VisualTestContext, div, point, px,
};
use jiff::{civil::date, tz::TimeZone};

use super::{Planning, at, heard, note, planning, press, settle, tab_to};
use crate::calendar::{AllDayRow, CalendarWeekView, Event, TimeGrid};

/// Tuesday the 22nd with a review at 13:00, its clock at noon.
fn grid(_: &Planning, owner: Entity<Planning>) -> gpui::AnyElement {
    let (made, moved, pressed) = (owner.clone(), owner.clone(), owner);
    let review = Event::timed("review", "Review", at(22, 13), at(22, 14), 0);
    div()
        .h(px(600.0))
        .child(
            TimeGrid::new("grid", [date(2026, 9, 22)], [review])
                .zone(TimeZone::UTC)
                .now(at(22, 12))
                .on_create(move |start, end, _, cx| note(&made, format!("made {start} {end}"), cx))
                .on_resize(move |key, end, _, cx| note(&moved, format!("end {key} {end}"), cx))
                .on_event(move |key, _, cx| note(&pressed, format!("event {key}"), cx)),
        )
        .into_any_element()
}

fn column(cx: &mut VisualTestContext) -> Bounds<Pixels> {
    cx.debug_bounds("column 2026-09-22")
        .expect("the day's column draws")
}

/// Where minute `minutes` of the day lies down its column: an hour is 48 tall.
fn at_minute(column: Bounds<Pixels>, minutes: f32) -> Pixels {
    column.top() + px(48.0 * minutes / 60.0)
}

fn drag(from: gpui::Point<Pixels>, to: gpui::Point<Pixels>, cx: &mut VisualTestContext) {
    let none = Modifiers::none();
    cx.simulate_mouse_down(from, MouseButton::Left, none);
    for step in 1..=4 {
        let share = step as f32 / 4.0;
        let at = point(from.x, from.y + (to.y - from.y) * share);
        cx.simulate_mouse_move(at, MouseButton::Left, none);
    }
    cx.simulate_mouse_up(to, MouseButton::Left, none);
    settle(cx);
}

#[gpui::test]
fn a_drag_on_empty_time_makes_an_event_by_quarters(cx: &mut TestAppContext) {
    let (host, cx) = planning(grid, cx);
    let column = column(cx);
    let x = column.center().x;
    let from = point(x, at_minute(column, 16.0 * 60.0 + 4.0));
    let to = point(x, at_minute(column, 17.0 * 60.0 + 28.0));
    drag(from, to, cx);
    assert_eq!(
        heard(&host, cx),
        [format!(
            "made {} {}",
            at(22, 16),
            at(22, 17) + jiff::SignedDuration::from_mins(30)
        )]
    );
}

#[gpui::test]
fn a_drag_on_an_events_lower_edge_moves_its_end(cx: &mut TestAppContext) {
    let (host, cx) = planning(grid, cx);
    let edge = cx
        .debug_bounds("end review 2026-09-22")
        .expect("the review's edge draws");
    let from = edge.center();
    drag(from, point(from.x, from.y + px(48.0)), cx);
    let heard = heard(&host, cx);
    assert_eq!(
        heard.last().map(String::as_str),
        Some(format!("end review {}", at(22, 15)).as_str()),
        "{heard:?}"
    );
}

#[gpui::test]
fn a_press_on_a_card_names_its_event_and_makes_nothing(cx: &mut TestAppContext) {
    let (host, cx) = planning(grid, cx);
    let card = cx
        .debug_bounds("event-card review")
        .expect("the review draws");
    cx.simulate_click(card.center(), Modifiers::none());
    settle(cx);
    assert_eq!(heard(&host, cx), ["event review"]);
}

#[gpui::test]
fn the_now_line_sits_at_its_minute_on_today(cx: &mut TestAppContext) {
    let (_, cx) = planning(grid, cx);
    let column = column(cx);
    let line = cx.debug_bounds("now-line").expect("today carries the line");
    assert_eq!(line.top(), at_minute(column, 12.0 * 60.0));
}

fn week(_: &Planning, owner: Entity<Planning>) -> gpui::AnyElement {
    let steps = owner;
    div()
        .h(px(600.0))
        .child(
            CalendarWeekView::new("week", date(2026, 9, 23), [])
                .zone(TimeZone::UTC)
                .now(at(16, 9))
                .on_step(move |first, _, cx| note(&steps, format!("step {first}"), cx)),
        )
        .into_any_element()
}

#[gpui::test]
fn a_week_steps_on_and_comes_back_to_today(cx: &mut TestAppContext) {
    let (host, cx) = planning(week, cx);
    tab_to(3, cx);
    press("space", cx);
    tab_to(1, cx);
    press("space", cx);
    assert_eq!(
        heard(&host, cx),
        ["step 2026-09-28", "step 2026-09-14"],
        "on from the week of the 23rd, then back to the week of the 16th"
    );
}

fn whole_days(_: &Planning, _: Entity<Planning>) -> gpui::AnyElement {
    let days = (21..=27).map(|day| date(2026, 9, day));
    AllDayRow::new(
        "all-day",
        days,
        [
            Event::all_day("fair", "Fair", date(2026, 9, 22), date(2026, 9, 24), 0),
            Event::all_day("trip", "Trip", date(2026, 9, 23), date(2026, 9, 25), 1),
        ],
    )
    .zone(TimeZone::UTC)
    .into_any_element()
}

#[gpui::test]
fn whole_days_that_meet_take_lanes(cx: &mut TestAppContext) {
    let (_, cx) = planning(whole_days, cx);
    let fair = cx.debug_bounds("event-chip fair").expect("the fair draws");
    let trip = cx.debug_bounds("event-chip trip").expect("the trip draws");
    assert!(trip.top() > fair.bottom(), "{fair:?} {trip:?}");
    assert!(trip.left() > fair.left(), "the trip starts a day later");
}

/// Two grids, each with a review of the same key.
fn twins(_: &Planning, owner: Entity<Planning>) -> gpui::AnyElement {
    let one = |id: &'static str, owner: Entity<Planning>| {
        let review = Event::timed("review", "Review", at(22, 13), at(22, 14), 0);
        div().h(px(600.0)).w(px(300.0)).child(
            TimeGrid::new(id, [date(2026, 9, 22)], [review])
                .zone(TimeZone::UTC)
                .now(at(22, 12))
                .on_resize(move |key, end, _, cx| note(&owner, format!("{id} {key} {end}"), cx)),
        )
    };
    div()
        .flex()
        .child(one("left", owner.clone()))
        .child(one("right", owner))
        .into_any_element()
}

#[gpui::test]
fn a_drag_moves_only_its_own_grids_event(cx: &mut TestAppContext) {
    let (host, cx) = planning(twins, cx);
    let edge = cx
        .debug_bounds("end review 2026-09-22")
        .expect("an edge draws");
    let from = edge.center();
    drag(from, point(from.x, from.y + px(48.0)), cx);
    let heard = heard(&host, cx);
    let grids: Vec<&str> = heard
        .iter()
        .map(|said| &said[..said.find(' ').expect("a grid")])
        .collect();
    assert!(
        !grids.is_empty() && grids.iter().all(|grid| *grid == grids[0]),
        "{heard:?}"
    );
}

/// A night from Tuesday 21:00 to Wednesday 02:00.
fn nights(_: &Planning, owner: Entity<Planning>) -> gpui::AnyElement {
    let night = Event::timed("night", "Night", at(22, 21), at(23, 2), 0);
    div()
        .h(px(600.0))
        .child(
            TimeGrid::new("nights", [date(2026, 9, 22), date(2026, 9, 23)], [night])
                .zone(TimeZone::UTC)
                .now(at(16, 12))
                .on_resize(move |key, end, _, cx| note(&owner, format!("end {key} {end}"), cx)),
        )
        .into_any_element()
}

#[gpui::test]
fn only_the_day_an_event_ends_holds_its_edge(cx: &mut TestAppContext) {
    let (_, cx) = planning(nights, cx);
    assert!(
        cx.debug_bounds("end night 2026-09-22").is_none(),
        "it goes on past midnight"
    );
    assert!(cx.debug_bounds("end night 2026-09-23").is_some());
}

fn opened(cx: &mut VisualTestContext) -> Pixels {
    let top = cx.debug_bounds("time-grid").expect("the grid draws").top();
    top - column(cx).top()
}

#[gpui::test]
fn a_grid_on_today_opens_an_hour_and_a_half_before_now(cx: &mut TestAppContext) {
    let (_, cx) = planning(grid, cx);
    assert_eq!(opened(cx), px(48.0 * 10.5), "at noon, from 10:30");
}

#[gpui::test]
fn a_grid_without_today_opens_on_the_morning(cx: &mut TestAppContext) {
    let (_, cx) = planning(nights, cx);
    assert_eq!(opened(cx), px(48.0 * 7.5), "from 07:30");
}

/// How often the host was told to redraw across a clock tick and a half, with no refresh of its own.
fn redraws(
    part: fn(&Planning, Entity<Planning>) -> gpui::AnyElement,
    cx: &mut TestAppContext,
) -> usize {
    let (host, cx) = planning(part, cx);
    let told = std::rc::Rc::new(std::cell::Cell::new(0));
    let count = told.clone();
    cx.update(|_, cx| {
        cx.observe(&host, move |_, _| count.set(count.get() + 1))
            .detach()
    });
    cx.executor()
        .advance_clock(std::time::Duration::from_secs(45));
    cx.run_until_parked();
    told.get()
}

fn clocked_day(_: &Planning, _: Entity<Planning>) -> gpui::AnyElement {
    let today = jiff::Timestamp::now().to_zoned(TimeZone::UTC).date();
    div()
        .h(px(600.0))
        .child(crate::calendar::CalendarDayView::new("day", today, []).zone(TimeZone::UTC))
        .into_any_element()
}

fn still_day(_: &Planning, _: Entity<Planning>) -> gpui::AnyElement {
    div()
        .h(px(600.0))
        .child(
            crate::calendar::CalendarDayView::new("day", date(2026, 9, 22), [])
                .zone(TimeZone::UTC)
                .now(at(22, 12)),
        )
        .into_any_element()
}

#[gpui::test]
fn a_day_on_the_clock_redraws_as_it_runs(cx: &mut TestAppContext) {
    assert!(
        redraws(clocked_day, cx) > 0,
        "the line at now moves with the clock"
    );
}

#[gpui::test]
fn a_day_given_its_moment_holds_still(cx: &mut TestAppContext) {
    assert_eq!(redraws(still_day, cx), 0);
}

fn clocked_month(_: &Planning, _: Entity<Planning>) -> gpui::AnyElement {
    crate::calendar::CalendarMonthView::new("month", date(2026, 9, 1), [])
        .zone(TimeZone::UTC)
        .into_any_element()
}

#[gpui::test]
fn a_month_on_the_clock_redraws_so_today_turns_over(cx: &mut TestAppContext) {
    assert!(redraws(clocked_month, cx) > 0);
}

fn clocked_year(_: &Planning, _: Entity<Planning>) -> gpui::AnyElement {
    crate::calendar::YearView::new("year", 2026, [])
        .zone(TimeZone::UTC)
        .into_any_element()
}

#[gpui::test]
fn a_year_on_the_clock_redraws_so_today_turns_over(cx: &mut TestAppContext) {
    assert!(redraws(clocked_year, cx) > 0);
}

fn clocked_other_day(_: &Planning, _: Entity<Planning>) -> gpui::AnyElement {
    div()
        .h(px(600.0))
        .child(
            crate::calendar::CalendarDayView::new("day", date(2026, 9, 22), []).zone(TimeZone::UTC),
        )
        .into_any_element()
}

#[gpui::test]
fn a_day_before_today_still_watches_the_clock_for_midnight(cx: &mut TestAppContext) {
    assert!(
        redraws(clocked_other_day, cx) > 0,
        "no line to tick, the grid itself does"
    );
}
