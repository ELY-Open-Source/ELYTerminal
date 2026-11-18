use std::time::Duration;

use gpui::WindowId;

use super::{look::Mark, *};
use crate::sessions::orb::Look;

/// Her, `mood` since `since`, come long before.
fn ely(mood: Mood, since: Instant) -> Pet {
    Pet {
        rig: Rc::new(Rig::new()),
        mood,
        since,
        shown: since - Duration::from_secs(60),
        petted: None,
        act: Act::Calm,
        last: None,
        eased: None,
        edge: Edge::Top,
        rise: HEAD + ROOM,
        anchor: None,
        at: 140.0,
        drag: None,
        hovered: false,
        low: false,
        main: WindowHandle::new(WindowId::from(1)),
        glue: None,
        _watch: Vec::new(),
    }
}

#[test]
fn she_follows_the_session_shown_and_leaves_the_others_alone() {
    let looks = |shown| {
        shown_mood(
            [Some(Look::Idle), Some(Look::Error), Some(Look::Waiting)].into_iter(),
            shown,
        )
    };
    assert_eq!(looks(0), Mood::Calm, "the session shown is calm");
    assert_eq!(looks(1), Mood::Error, "the one shown frets her");
    assert_eq!(looks(2), Mood::Waiting, "the one shown makes her wait");
    assert_eq!(
        shown_mood([None].into_iter(), 0),
        Mood::Calm,
        "a session with no mark"
    );
}

#[test]
fn peeking_low_sinks_her_by_low_and_a_hover_eases_her_back() {
    let start = Instant::now();
    let at = |ms: u64| start + Duration::from_millis(ms);
    let whole = ely(Mood::Calm, start).pose(at(1000)).lift;
    let mut pet = ely(Mood::Calm, start);
    pet.low = true;
    let low = pet.pose(at(1000)).lift;
    assert!((whole - low - LOW).abs() < 0.01, "{whole} then {low}");
    pet.low = false;
    pet.ease(at(1000));
    let rising = pet.pose(at(1100)).lift;
    assert!(
        low < rising && rising < whole,
        "eased: {low}, {rising}, {whole}"
    );
    assert!(
        (pet.pose(at(1400)).lift - whole).abs() < 1.0,
        "back by EASE"
    );
}

#[test]
fn a_change_of_mood_or_a_press_eases_her_from_where_she_is_though_the_act_stays() {
    let start = Instant::now();
    let at = |ms: u64| start + Duration::from_millis(ms);
    // A wait hops her, a press winks, and a program starts working inside the wink.
    let mut pet = ely(Mood::Waiting, start);
    pet.press(at(2250));
    let before = pet.pose(at(2750));
    pet.feel(Mood::Working, at(2750));
    let after = pet.pose(at(2751));
    assert_eq!((before.act, after.act), (Act::Wink, Act::Wink));
    assert!(
        (after.lift - before.lift).abs() < 1.0,
        "no drop: {} then {}",
        before.lift,
        after.lift
    );
    let marks: Vec<Mark> = after.marks.iter().map(|(mark, _)| *mark).collect();
    assert!(
        marks.contains(&Mark::Bang),
        "the wait's mark fades: {marks:?}"
    );
    // An old finish clears mid-glance: calm before and after, her eyes turn back at her pace.
    let mut pet = ely(Mood::Done, start);
    let before = pet.pose(at(13_600));
    pet.feel(Mood::Calm, at(13_600));
    let after = pet.pose(at(13_601));
    assert_eq!((before.act, after.act), (Act::Calm, Act::Calm));
    assert!(
        before.look.0 < -5.0 && (after.look.0 - before.look.0).abs() < 0.5,
        "no snap: {:?} then {:?}",
        before.look,
        after.look
    );
    let later = pet.pose(at(14_000));
    assert!(
        pet.eased.is_none() && later.look.0.abs() < 0.5,
        "eased in by then"
    );
    // A second press mid-hop restarts the wink without a drop.
    let mut pet = ely(Mood::Calm, start);
    pet.press(at(1000));
    let before = pet.pose(at(1300));
    pet.press(at(1300));
    let after = pet.pose(at(1301));
    assert!(
        before.lift > 10.0 && (after.lift - before.lift).abs() < 1.0,
        "no drop: {} then {}",
        before.lift,
        after.lift
    );
}

#[test]
fn a_press_on_her_while_she_dozes_wakes_her() {
    let start = Instant::now();
    let at = |secs: u64| start + Duration::from_secs(secs);
    let mut pet = ely(Mood::Calm, start);
    assert_eq!(pet.pose(at(400)).act, Act::Doze);
    pet.press(at(400));
    assert_eq!(pet.pose(at(402)).act, Act::Calm, "awake");
    assert_eq!(
        pet.pose(at(701)).act,
        Act::Doze,
        "five calm minutes on, asleep again"
    );
}

#[test]
fn she_stands_on_the_top_edge_while_her_box_fits_above_it() {
    assert_eq!(edge(HEAD + ROOM), Edge::Top, "just room for her");
    assert_eq!(edge(400.0), Edge::Top);
    assert_eq!(edge(HEAD + ROOM - 0.5), Edge::Bottom, "half a point short");
    assert_eq!(edge(0.0), Edge::Bottom, "a window at the work area's top");
    assert_eq!(edge(-30.0), Edge::Bottom, "a window above it");
}

#[test]
fn she_holds_the_edge_of_any_pane_where_a_drag_left_her() {
    assert_eq!(left(px(1200.), 140.0), 140.0);
    assert_eq!(left(px(1200.), 900.0), 900.0, "dragged right");
    assert_eq!(left(px(1200.), 0.0), 0.0, "dragged to its left end");
    assert_eq!(left(px(600.), 900.0), 600.0 - WIDTH, "a narrower pane");
    assert_eq!(left(px(200.), 140.0), 200.0 - WIDTH);
    assert_eq!(
        left(px(100.), 140.0),
        0.0,
        "narrower than she is, she stands at its left"
    );
}
