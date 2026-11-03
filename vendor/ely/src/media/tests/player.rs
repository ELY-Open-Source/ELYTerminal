use std::{path::PathBuf, time::Duration};

use gpui::{
    Context, Entity, IntoElement, Modifiers, MouseButton, ParentElement, Pixels, Point, Render,
    Styled, TestAppContext, VisualTestContext, Window, div, point, px,
};

use super::{across, picture, press, settle, setup};
use crate::media::VideoPlayer;

const SECOND: Duration = Duration::from_secs(1);

/// A player of a hundred seconds, this wide, with this caption, and what it was told.
struct Watching {
    path: PathBuf,
    width: f32,
    caption: &'static str,
    at: Duration,
    playing: bool,
    plays: Vec<bool>,
    captions: bool,
    fullscreens: usize,
}

impl Render for Watching {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (play, seek, caption, full) = (cx.entity(), cx.entity(), cx.entity(), cx.entity());
        div().w(px(self.width)).child(
            VideoPlayer::new(
                "player",
                Some(self.path.clone().into()),
                1.6,
                SECOND * 100,
                self.at,
            )
            .playing(self.playing)
            .captions(self.captions, Some(self.caption.into()))
            .on_play(move |on, _, cx| {
                play.update(cx, |view, cx| {
                    view.playing = on;
                    view.plays.push(on);
                    cx.notify();
                })
            })
            .on_seek(move |time, _, cx| {
                seek.update(cx, |view, cx| {
                    view.at = time;
                    cx.notify();
                })
            })
            .on_captions(move |on, _, cx| {
                caption.update(cx, |view, cx| {
                    view.captions = on;
                    cx.notify();
                })
            })
            .on_fullscreen(move |_, cx| {
                full.update(cx, |view, cx| {
                    view.fullscreens += 1;
                    cx.notify();
                })
            }),
        )
    }
}

fn watching_at<'a>(
    width: f32,
    caption: &'static str,
    captions: bool,
    cx: &'a mut TestAppContext,
) -> (Entity<Watching>, &'a mut VisualTestContext) {
    setup(cx);
    let path = picture("frame", 160, 100);
    let (view, cx) = cx.add_window_view(|_, _| Watching {
        path,
        width,
        caption,
        at: SECOND * 30,
        playing: false,
        plays: Vec::new(),
        captions,
        fullscreens: 0,
    });
    cx.update(|window, _| window.activate_window());
    settle(cx);
    let frame = cx.debug_bounds("video-player").expect("the player draws");
    cx.simulate_click(frame.origin + point(px(8.0), px(8.0)), Modifiers::none());
    settle(cx);
    (view, cx)
}

fn watching(cx: &mut TestAppContext) -> (Entity<Watching>, &mut VisualTestContext) {
    watching_at(480.0, "Light falls on the stair", false, cx)
}

#[gpui::test]
fn space_on_a_focused_bar_button_presses_it_once(cx: &mut TestAppContext) {
    let (view, cx) = watching(cx);
    cx.update(|window, cx| {
        window.blur(cx);
        for _ in 0..3 {
            window.focus_next(cx);
        }
    });
    press("space", cx);
    assert_eq!(view.read_with(cx, |view, _| view.plays.clone()), [true]);
}

#[gpui::test]
fn command_letters_leave_the_player_to_the_app(cx: &mut TestAppContext) {
    let (view, cx) = watching(cx);
    for key in ["cmd-f", "cmd-c", "cmd-k", "ctrl-f"] {
        press(key, cx);
    }
    let heard = view.read_with(cx, |view, _| {
        (view.fullscreens, view.captions, view.plays.clone())
    });
    assert_eq!(heard, (0, false, vec![]));
}

#[gpui::test]
fn a_long_caption_wraps_inside_a_narrow_player(cx: &mut TestAppContext) {
    let (view, cx) = watching_at(280.0, "Light falls", true, cx);
    let line = cx.debug_bounds("video-caption").expect("the caption draws");
    view.update(cx, |view, cx| {
        view.caption = "The shadow walks the length of the hall while the morning comes in.";
        cx.notify();
    });
    settle(cx);
    let (chip, frame) = (
        cx.debug_bounds("video-caption").expect("the caption draws"),
        cx.debug_bounds("video-player").expect("the player draws"),
    );
    assert!(
        chip.left() >= frame.left() && chip.right() <= frame.right(),
        "{chip:?} in {frame:?}"
    );
    assert!(
        chip.size.height > line.size.height * 1.5,
        "{chip:?} wraps past {line:?}"
    );
}

#[gpui::test]
fn a_caption_stays_while_the_controls_rest(cx: &mut TestAppContext) {
    let (view, cx) = watching_at(480.0, "Light falls on the stair", true, cx);
    cx.update(|window, cx| window.blur(cx));
    view.update(cx, |view, cx| {
        view.playing = true;
        cx.notify();
    });
    let frame = cx.debug_bounds("video-player").expect("the player draws");
    cx.simulate_mouse_move(frame.center(), None, Modifiers::none());
    settle(cx);
    cx.executor().advance_clock(Duration::from_millis(2600));
    cx.run_until_parked();
    let faded = cx
        .debug_bounds("video-controls-resting")
        .expect("the bar rests");
    let caption = cx.debug_bounds("video-caption").expect("the caption draws");
    assert!(
        caption.bottom() <= faded.top(),
        "{caption:?} outside {faded:?}"
    );
}

#[gpui::test]
fn space_plays_l_and_j_jump_ten_seconds_and_c_turns_captions(cx: &mut TestAppContext) {
    let (view, cx) = watching(cx);
    press("space", cx);
    assert!(view.read_with(cx, |view, _| view.playing));
    press("l", cx);
    press("l", cx);
    press("j", cx);
    assert_eq!(view.read_with(cx, |view, _| view.at), SECOND * 40);
    press("c", cx);
    assert!(view.read_with(cx, |view, _| view.captions));
}

#[gpui::test]
fn controls_rest_once_the_pointer_is_still_while_it_plays(cx: &mut TestAppContext) {
    let (view, cx) = watching(cx);
    cx.update(|window, cx| window.blur(cx));
    view.update(cx, |view, cx| {
        view.playing = true;
        cx.notify();
    });
    let frame = cx.debug_bounds("video-player").expect("the player draws");
    cx.simulate_mouse_move(frame.center(), None, Modifiers::none());
    settle(cx);
    assert!(
        cx.debug_bounds("video-controls-resting").is_none(),
        "a moving pointer keeps them up"
    );
    cx.executor().advance_clock(Duration::from_millis(2600));
    cx.run_until_parked();
    assert!(
        cx.debug_bounds("video-controls-resting").is_some(),
        "a still one lets them rest"
    );
}

/// Rests a playing player's bar, focus outside it and the pointer still at `spot`.
fn rest(spot: Point<Pixels>, view: &Entity<Watching>, cx: &mut VisualTestContext) {
    cx.update(|window, cx| window.blur(cx));
    view.update(cx, |view, cx| {
        view.playing = true;
        cx.notify();
    });
    cx.simulate_mouse_move(spot, None, Modifiers::none());
    settle(cx);
    cx.executor().advance_clock(Duration::from_millis(2600));
    cx.run_until_parked();
    settle(cx);
    assert!(
        cx.debug_bounds("video-controls-resting").is_some(),
        "the bar rests"
    );
}

#[gpui::test]
fn a_drag_begun_on_a_resting_bar_follows_the_pointer(cx: &mut TestAppContext) {
    let (view, cx) = watching(cx);
    rest(across("scrubber-rail", 0.1, cx), &view, cx);
    let none = Modifiers::none();
    let start = across("scrubber-rail", 0.1, cx);
    cx.simulate_mouse_down(start, MouseButton::Left, none);
    settle(cx);
    for step in 1..=5 {
        let to = across("scrubber-rail", 0.1 + 0.1 * step as f32, cx);
        cx.simulate_mouse_move(to, MouseButton::Left, none);
        settle(cx);
    }
    let end = across("scrubber-rail", 0.6, cx);
    cx.simulate_mouse_up(end, MouseButton::Left, none);
    settle(cx);
    let at = view.read_with(cx, |view, _| view.at);
    assert!(at.abs_diff(SECOND * 60) < SECOND * 2, "{at:?}");
}

#[gpui::test]
fn shift_tab_into_a_resting_bar_keeps_focus_on_its_last_button(cx: &mut TestAppContext) {
    let (view, cx) = watching(cx);
    let frame = cx.debug_bounds("video-player").expect("the player draws");
    rest(frame.center(), &view, cx);
    cx.update(|window, cx| window.focus_prev(cx));
    settle(cx);
    press("enter", cx);
    assert_eq!(view.read_with(cx, |view, _| view.fullscreens), 1);
}

#[gpui::test]
fn the_first_move_onto_a_resting_bar_shows_the_tip(cx: &mut TestAppContext) {
    let (view, cx) = watching(cx);
    let frame = cx.debug_bounds("video-player").expect("the player draws");
    rest(frame.origin + point(px(8.0), px(8.0)), &view, cx);
    let half = across("scrubber-rail", 0.5, cx);
    cx.simulate_mouse_move(half, None, Modifiers::none());
    settle(cx);
    assert!(cx.debug_bounds("time-tip").is_some());
}

/// A playing player whose only control is its speed.
struct Paced {
    path: PathBuf,
}

impl Render for Paced {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let player = VideoPlayer::new(
            "paced",
            Some(self.path.clone().into()),
            1.6,
            SECOND * 100,
            SECOND * 30,
        );
        div()
            .w(px(480.0))
            .child(player.playing(true).on_speed(|_, _, _| {}))
    }
}

#[gpui::test]
fn the_bar_stays_up_while_a_button_in_it_has_focus(cx: &mut TestAppContext) {
    setup(cx);
    let path = picture("paced", 160, 100);
    let (_, cx) = cx.add_window_view(|_, _| Paced { path });
    cx.update(|window, _| window.activate_window());
    settle(cx);
    cx.update(|window, cx| window.focus_next(cx));
    settle(cx);
    cx.executor().advance_clock(Duration::from_millis(2600));
    settle(cx);
    assert!(cx.debug_bounds("video-controls-resting").is_none());
}

#[gpui::test]
fn a_focused_stage_lets_the_bar_rest(cx: &mut TestAppContext) {
    let (view, cx) = watching(cx);
    view.update(cx, |view, cx| {
        view.playing = true;
        cx.notify();
    });
    let frame = cx.debug_bounds("video-player").expect("the player draws");
    cx.simulate_mouse_move(frame.center(), None, Modifiers::none());
    settle(cx);
    cx.executor().advance_clock(Duration::from_millis(2600));
    settle(cx);
    assert!(cx.debug_bounds("video-controls-resting").is_some());
}

#[gpui::test]
fn a_press_on_a_resting_bar_wakes_it_before_a_button_runs(cx: &mut TestAppContext) {
    let (view, cx) = watching(cx);
    let frame = cx.debug_bounds("video-player").expect("the player draws");
    rest(frame.origin + point(px(8.0), px(8.0)), &view, cx);
    let play = cx.debug_bounds("media-button play").expect("a play button");
    cx.simulate_click(play.center(), Modifiers::none());
    settle(cx);
    let plays = view.read_with(cx, |view, _| view.plays.clone());
    assert!(plays.is_empty(), "the press woke it: {plays:?}");
    cx.simulate_click(play.center(), Modifiers::none());
    settle(cx);
    assert_eq!(view.read_with(cx, |view, _| view.plays.clone()), [false]);
}

#[gpui::test]
fn a_key_wakes_a_resting_bar(cx: &mut TestAppContext) {
    let (view, cx) = watching(cx);
    view.update(cx, |view, cx| {
        view.playing = true;
        cx.notify();
    });
    let frame = cx.debug_bounds("video-player").expect("the player draws");
    cx.simulate_mouse_move(frame.center(), None, Modifiers::none());
    settle(cx);
    cx.executor().advance_clock(Duration::from_millis(2600));
    settle(cx);
    press("l", cx);
    let play = cx.debug_bounds("media-button play").expect("a play button");
    cx.simulate_click(play.center(), Modifiers::none());
    settle(cx);
    assert_eq!(view.read_with(cx, |view, _| view.plays.clone()), [false]);
}
