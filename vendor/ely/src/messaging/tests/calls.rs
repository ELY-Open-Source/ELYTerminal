use gpui::{
    App, Context, Entity, InteractiveElement, IntoElement, Modifiers, ParentElement, Render,
    Styled, TestAppContext, Window, div, px,
};
use web_time::Instant;

use super::{press, settle, setup, tab_to};
use crate::{
    data_display::Avatar,
    messaging::{
        CallControls, HuddleIndicator, IncomingCallDialog, ParticipantTile, ScreenShareView,
        VideoCallGrid, VoiceCallBar,
    },
};

/// A view that shows one call part and keeps what it asked.
struct Call {
    part: fn(Entity<Call>) -> gpui::AnyElement,
    asked: Vec<String>,
}

fn note(owner: &Entity<Call>, what: String, cx: &mut App) {
    owner.update(cx, |call, cx| {
        call.asked.push(what);
        cx.notify();
    });
}

impl Render for Call {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div().w(px(480.0)).child((self.part)(cx.entity()))
    }
}

fn call(
    part: fn(Entity<Call>) -> gpui::AnyElement,
    cx: &mut TestAppContext,
) -> (Entity<Call>, &mut gpui::VisualTestContext) {
    setup(cx);
    let (host, cx) = cx.add_window_view(|_, _| Call {
        part,
        asked: Vec::new(),
    });
    cx.update(|window, _| window.activate_window());
    settle(cx);
    (host, cx)
}

fn asked(host: &Entity<Call>, cx: &mut gpui::VisualTestContext) -> Vec<String> {
    host.read_with(cx, |call, _| call.asked.clone())
}

#[gpui::test]
fn the_call_controls_each_ask_their_own(cx: &mut TestAppContext) {
    let (host, cx) = call(
        |owner| {
            let (mic, camera, share, leave) = (owner.clone(), owner.clone(), owner.clone(), owner);
            CallControls::new("controls")
                .mic(true, move |on, _, cx| note(&mic, format!("mic {on}"), cx))
                .camera(true, move |on, _, cx| {
                    note(&camera, format!("camera {on}"), cx)
                })
                .share(false, move |on, _, cx| {
                    note(&share, format!("share {on}"), cx)
                })
                .on_leave(move |_, cx| note(&leave, "leave".into(), cx))
                .into_any_element()
        },
        cx,
    );
    for nth in 1..=4 {
        tab_to(nth, cx);
        press("space", cx);
    }
    assert_eq!(
        asked(&host, cx),
        ["mic false", "camera false", "share true", "leave"]
    );
}

#[gpui::test]
fn the_voice_bar_mutes_and_leaves(cx: &mut TestAppContext) {
    let (host, cx) = call(
        |owner| {
            let (mic, leave) = (owner.clone(), owner);
            VoiceCallBar::new("voice", "Huddle in # design", Instant::now())
                .people([
                    Avatar::new("ana", "Ana Lima"),
                    Avatar::new("ben", "Ben Ito"),
                ])
                .mic(true, move |on, _, cx| note(&mic, format!("mic {on}"), cx))
                .on_leave(move |_, cx| note(&leave, "leave".into(), cx))
                .into_any_element()
        },
        cx,
    );
    for nth in 1..=2 {
        tab_to(nth, cx);
        press("space", cx);
    }
    assert_eq!(asked(&host, cx), ["mic false", "leave"]);
}

#[gpui::test]
fn a_huddle_joins_on_a_press_or_space(cx: &mut TestAppContext) {
    let (host, cx) = call(
        |owner| {
            HuddleIndicator::new("huddle", [Avatar::new("ana", "Ana Lima")])
                .on_join(move |_, cx| note(&owner, "join".into(), cx))
                .into_any_element()
        },
        cx,
    );
    let chip = cx.debug_bounds("huddle").expect("the huddle draws");
    cx.simulate_click(chip.center(), Modifiers::none());
    settle(cx);
    tab_to(1, cx);
    press("space", cx);
    assert_eq!(asked(&host, cx), ["join", "join"]);
}

#[gpui::test]
fn a_huddle_without_a_handler_takes_no_tab_stop(cx: &mut TestAppContext) {
    let (_, cx) = call(
        |_| HuddleIndicator::new("huddle", [Avatar::new("ana", "Ana Lima")]).into_any_element(),
        cx,
    );
    tab_to(1, cx);
    assert!(cx.update(|window, cx| window.focused(cx).is_none()));
}

#[test]
#[should_panic(expected = "a huddle holds someone")]
fn a_huddle_holds_someone() {
    let _ = HuddleIndicator::new("huddle", Vec::new());
}

#[gpui::test]
fn a_tile_marks_only_its_own_mute_and_speech(cx: &mut TestAppContext) {
    let (_, cx) = call(
        |_| {
            div()
                .flex()
                .child(
                    div().w(px(200.0)).child(
                        ParticipantTile::new("ana", "Ana Lima", 16.0 / 9.0)
                            .muted(true)
                            .speaking(true),
                    ),
                )
                .child(
                    div()
                        .w(px(200.0))
                        .child(ParticipantTile::new("ben", "Ben Ito", 16.0 / 9.0)),
                )
                .into_any_element()
        },
        cx,
    );
    assert!(cx.debug_bounds("tile-muted ana").is_some());
    assert!(cx.debug_bounds("tile-speaking ana").is_some());
    assert!(cx.debug_bounds("tile-muted ben").is_none());
    assert!(cx.debug_bounds("tile-speaking ben").is_none());
}

/// Five tiles, 40 tall, in a grid `wide` across.
fn five(wide: f32) -> gpui::AnyElement {
    let tile = |name: &'static str| div().debug_selector(move || name.into()).h(px(40.0));
    div()
        .w(px(wide))
        .child(
            VideoCallGrid::new("grid")
                .child(tile("t0"))
                .child(tile("t1"))
                .child(tile("t2"))
                .child(tile("t3"))
                .child(tile("t4")),
        )
        .into_any_element()
}

fn at(name: &'static str, cx: &mut gpui::VisualTestContext) -> gpui::Bounds<gpui::Pixels> {
    cx.debug_bounds(name).expect("the tile draws")
}

#[gpui::test]
fn a_short_last_row_centers_at_the_full_rows_width(cx: &mut TestAppContext) {
    let (_, cx) = call(|_| five(720.0), cx);
    let (t0, t2, t3, t4) = (at("t0", cx), at("t2", cx), at("t3", cx), at("t4", cx));
    assert_eq!(t0.origin.y, t2.origin.y, "three share the first row");
    assert!(t3.origin.y > t0.origin.y, "two drop to the second");
    assert!(
        (t3.size.width - t0.size.width).abs() < px(0.5),
        "{t3:?} {t0:?}"
    );
    let grid = at("video-call-grid", cx);
    let middle = (t3.left() + t4.right()) / 2.0;
    assert!(
        (middle - grid.center().x).abs() < px(0.5),
        "{middle:?} {grid:?}"
    );
}

#[gpui::test]
fn a_narrow_grid_keeps_to_the_columns_that_fit(cx: &mut TestAppContext) {
    let (_, cx) = call(|_| five(360.0), cx);
    let (t0, t1, t2) = (at("t0", cx), at("t1", cx), at("t2", cx));
    assert_eq!(t0.origin.y, t1.origin.y, "two fit across");
    assert!(t2.origin.y > t1.origin.y, "the third drops");
}

#[gpui::test]
fn a_share_waits_for_its_screen_and_stops_on_ask(cx: &mut TestAppContext) {
    let (host, cx) = call(
        |owner| {
            ScreenShareView::new("share", "Ana Lima", 16.0 / 10.0)
                .on_stop(move |_, cx| note(&owner, "stop".into(), cx))
                .into_any_element()
        },
        cx,
    );
    assert!(cx.debug_bounds("screen-waiting").is_some());
    tab_to(1, cx);
    press("space", cx);
    assert_eq!(asked(&host, cx), ["stop"]);
}

/// A call that rings until it is answered, and the answers.
struct Ringing {
    ringing: bool,
    asked: Vec<&'static str>,
}

impl Render for Ringing {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (accept, decline) = (cx.entity(), cx.entity());
        let answer = |owner: Entity<Ringing>, what: &'static str, cx: &mut App| {
            owner.update(cx, |ringing, cx| {
                ringing.ringing = false;
                ringing.asked.push(what);
                cx.notify();
            })
        };
        div().size_full().children(self.ringing.then(|| {
            IncomingCallDialog::new(
                "incoming",
                "Ana Lima",
                move |_, cx| answer(accept.clone(), "accept", cx),
                move |_, cx| answer(decline.clone(), "decline", cx),
            )
            .video()
        }))
    }
}

fn ringing(cx: &mut TestAppContext) -> (Entity<Ringing>, &mut gpui::VisualTestContext) {
    setup(cx);
    let (host, cx) = cx.add_window_view(|_, _| Ringing {
        ringing: true,
        asked: Vec::new(),
    });
    cx.update(|window, _| window.activate_window());
    settle(cx);
    (host, cx)
}

#[gpui::test]
fn accept_answers_once(cx: &mut TestAppContext) {
    let (host, cx) = ringing(cx);
    tab_to(2, cx);
    press("space", cx);
    assert_eq!(
        host.read_with(cx, |ringing, _| ringing.asked.clone()),
        ["accept"]
    );
}

#[gpui::test]
fn decline_answers_once(cx: &mut TestAppContext) {
    let (host, cx) = ringing(cx);
    tab_to(1, cx);
    press("space", cx);
    assert_eq!(
        host.read_with(cx, |ringing, _| ringing.asked.clone()),
        ["decline"]
    );
}

#[gpui::test]
fn escape_declines(cx: &mut TestAppContext) {
    let (host, cx) = ringing(cx);
    press("escape", cx);
    assert_eq!(
        host.read_with(cx, |ringing, _| ringing.asked.clone()),
        ["decline"]
    );
}
