use std::time::Duration;

use gpui::{
    Context, Entity, IntoElement, Modifiers, MouseButton, ParentElement, Render, SharedString,
    Styled, TestAppContext, VisualTestContext, Window, div, point, px,
};

use super::{across, press, settle, setup, tab_to};
use crate::media::{
    AudioPlayer, AudioWaveform, MediaControls, PlaybackSpeedControl, Playlist, Repeat, Track,
    VolumeControl,
};

const SECOND: Duration = Duration::from_secs(1);

/// A waveform of a hundred seconds, 400 wide, and the times it was sent to.
struct Waving {
    at: Duration,
    sought: Vec<Duration>,
}

impl Render for Waving {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let owner = cx.entity();
        let peaks: Vec<f32> = (0..120).map(|ix| (ix % 7) as f32 / 7.0).collect();
        div().w(px(400.0)).mt(px(40.0)).child(
            AudioWaveform::new("wave", peaks, SECOND * 100, self.at).on_seek(move |time, _, cx| {
                owner.update(cx, |view, cx| {
                    view.at = time;
                    view.sought.push(time);
                    cx.notify();
                })
            }),
        )
    }
}

fn waving(cx: &mut TestAppContext) -> (Entity<Waving>, &mut VisualTestContext) {
    setup(cx);
    let (view, cx) = cx.add_window_view(|_, _| Waving {
        at: Duration::ZERO,
        sought: Vec::new(),
    });
    cx.update(|window, _| window.activate_window());
    settle(cx);
    (view, cx)
}

#[gpui::test]
fn a_press_on_the_waveform_seeks_and_a_drag_past_its_end_stops_there(cx: &mut TestAppContext) {
    let (view, cx) = waving(cx);
    let quarter = across("audio-waveform", 0.25, cx);
    let none = Modifiers::none();
    cx.simulate_mouse_down(quarter, MouseButton::Left, none);
    cx.simulate_mouse_move(quarter + point(px(20.0), px(0.0)), MouseButton::Left, none);
    cx.simulate_mouse_move(quarter + point(px(900.0), px(0.0)), MouseButton::Left, none);
    cx.simulate_mouse_up(quarter + point(px(900.0), px(0.0)), MouseButton::Left, none);
    settle(cx);
    let sought = view.read_with(cx, |view, _| view.sought.clone());
    let first = sought.first().expect("a press seeks");
    assert!(first.abs_diff(SECOND * 25) < SECOND, "{sought:?}");
    assert_eq!(sought.last(), Some(&(SECOND * 100)), "{sought:?}");
}

#[gpui::test]
fn the_waveform_steps_five_seconds_by_arrow_and_home_goes_to_the_start(cx: &mut TestAppContext) {
    let (view, cx) = waving(cx);
    let half = across("audio-waveform", 0.5, cx);
    cx.simulate_click(half, Modifiers::none());
    settle(cx);
    let start = view.read_with(cx, |view, _| view.at);
    press("left", cx);
    assert_eq!(view.read_with(cx, |view, _| view.at), start - SECOND * 5);
    press("home", cx);
    assert_eq!(view.read_with(cx, |view, _| view.at), Duration::ZERO);
}

#[gpui::test]
fn the_waveform_tip_follows_the_pointer_and_stays_over_it(cx: &mut TestAppContext) {
    let (_, cx) = waving(cx);
    assert!(cx.debug_bounds("time-tip").is_none());
    for share in [0.01, 0.5, 0.99] {
        let at = across("audio-waveform", share, cx);
        cx.simulate_mouse_move(at, None, Modifiers::none());
        settle(cx);
        let tip = cx
            .debug_bounds("time-tip")
            .expect("a tip over the waveform");
        let wave = cx
            .debug_bounds("audio-waveform")
            .expect("the waveform draws");
        let inside = tip.left() >= wave.left() - px(0.5) && tip.right() <= wave.right() + px(0.5);
        assert!(inside, "at {share}: {tip:?} over {wave:?}");
        assert!(
            tip.left() <= at.x && at.x <= tip.right(),
            "under the pointer: {tip:?}"
        );
    }
}

/// A volume control and what it was told.
struct Turning {
    level: f32,
    muted: bool,
    levels: Vec<f64>,
}

impl Render for Turning {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (level, mute) = (cx.entity(), cx.entity());
        div().w(px(240.0)).child(
            VolumeControl::new("volume", self.level, self.muted)
                .on_level(move |to, _, cx| {
                    level.update(cx, |view, cx| {
                        view.level = to as f32;
                        view.levels.push(to);
                        cx.notify();
                    })
                })
                .on_mute(move |on, _, cx| {
                    mute.update(cx, |view, cx| {
                        view.muted = on;
                        cx.notify();
                    })
                }),
        )
    }
}

#[gpui::test]
fn the_speaker_mutes_and_moving_the_slider_unmutes(cx: &mut TestAppContext) {
    setup(cx);
    let (view, cx) = cx.add_window_view(|_, _| Turning {
        level: 0.6,
        muted: false,
        levels: Vec::new(),
    });
    cx.update(|window, _| window.activate_window());
    settle(cx);
    tab_to(1, cx);
    press("space", cx);
    assert!(
        view.read_with(cx, |view, _| view.muted),
        "the speaker mutes"
    );
    tab_to(2, cx);
    press("right", cx);
    let heard = view.read_with(cx, |view, _| (view.muted, view.levels.clone()));
    assert_eq!(
        heard,
        (false, vec![0.05]),
        "the slider starts from zero and unmutes"
    );
}

/// A speed control and the speed it holds.
struct Pacing {
    speed: f32,
}

impl Render for Pacing {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let owner = cx.entity();
        div()
            .size_full()
            .child(
                PlaybackSpeedControl::new("speed", self.speed).on_change(move |to, _, cx| {
                    owner.update(cx, |view, cx| {
                        view.speed = to;
                        cx.notify();
                    })
                }),
            )
    }
}

#[gpui::test]
fn the_speed_menu_picks_a_speed(cx: &mut TestAppContext) {
    setup(cx);
    let (view, cx) = cx.add_window_view(|_, _| Pacing { speed: 1.0 });
    cx.update(|window, _| window.activate_window());
    settle(cx);
    tab_to(1, cx);
    press("enter", cx);
    for _ in 0..4 {
        press("down", cx);
    }
    press("enter", cx);
    let speed = view.read_with(cx, |view, _| view.speed);
    assert_eq!(speed, 1.5, "it opens on the first speed");
}

/// A transport and what it was told.
#[derive(Default)]
struct Steering {
    playing: bool,
    shuffle: bool,
    repeat: Option<Repeat>,
    steps: Vec<&'static str>,
}

impl Render for Steering {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (play, back, forward, shuffle, repeat) = (
            cx.entity(),
            cx.entity(),
            cx.entity(),
            cx.entity(),
            cx.entity(),
        );
        let now = self.repeat.unwrap_or(Repeat::Off);
        div().w(px(320.0)).child(
            MediaControls::new("transport", self.playing)
                .on_play(move |on, _, cx| play.update(cx, |view, _| view.playing = on))
                .on_back(move |_, cx| back.update(cx, |view, _| view.steps.push("back")))
                .on_forward(move |_, cx| forward.update(cx, |view, _| view.steps.push("forward")))
                .shuffle(self.shuffle, move |on, _, cx| {
                    shuffle.update(cx, |view, _| view.shuffle = on)
                })
                .repeat(now, move |next, _, cx| {
                    repeat.update(cx, |view, cx| {
                        view.repeat = Some(next);
                        cx.notify();
                    })
                }),
        )
    }
}

#[gpui::test]
fn each_transport_button_says_what_it_does(cx: &mut TestAppContext) {
    setup(cx);
    let (view, cx) = cx.add_window_view(|_, _| Steering::default());
    cx.update(|window, _| window.activate_window());
    settle(cx);
    for nth in 1..=5 {
        tab_to(nth, cx);
        press("space", cx);
    }
    let heard = view.read_with(cx, |view, _| {
        (view.shuffle, view.playing, view.steps.clone(), view.repeat)
    });
    assert_eq!(
        heard,
        (true, true, vec!["back", "forward"], Some(Repeat::All))
    );
    press("space", cx);
    press("space", cx);
    assert_eq!(view.read_with(cx, |view, _| view.repeat), Some(Repeat::Off));
}

fn track(key: &str, title: &str) -> Track {
    Track {
        key: SharedString::from(key.to_string()),
        title: SharedString::from(title.to_string()),
        artist: "Aster Quartet".into(),
        length: SECOND * 200,
    }
}

/// A playlist and the songs it was told to play.
struct Queueing {
    played: Vec<SharedString>,
}

impl Render for Queueing {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let owner = cx.entity();
        div().w(px(320.0)).child(
            Playlist::new(
                "list",
                [track("a", "Tall Windows"), track("b", "Stone Stair")],
            )
            .current("a", true)
            .on_play(move |key, _, cx| owner.update(cx, |view, _| view.played.push(key.clone()))),
        )
    }
}

#[gpui::test]
fn down_and_enter_play_the_next_song(cx: &mut TestAppContext) {
    setup(cx);
    let (view, cx) = cx.add_window_view(|_, _| Queueing { played: Vec::new() });
    cx.update(|window, _| window.activate_window());
    settle(cx);
    tab_to(1, cx);
    press("down", cx);
    assert!(
        view.read_with(cx, |view, _| view.played.is_empty()),
        "a move plays nothing"
    );
    press("enter", cx);
    assert_eq!(view.read_with(cx, |view, _| view.played.clone()), ["b"]);
}

/// Four songs with the third under way, and the songs it was told to play.
struct Midway {
    played: Vec<SharedString>,
}

impl Render for Midway {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let owner = cx.entity();
        let songs = [
            track("a", "One"),
            track("b", "Two"),
            track("c", "Three"),
            track("d", "Four"),
        ];
        div().w(px(320.0)).child(
            Playlist::new("list", songs)
                .current("c", true)
                .on_play(move |key, _, cx| {
                    owner.update(cx, |view, _| view.played.push(key.clone()))
                }),
        )
    }
}

#[gpui::test]
fn the_keys_start_from_the_song_under_way(cx: &mut TestAppContext) {
    setup(cx);
    let (view, cx) = cx.add_window_view(|_, _| Midway { played: Vec::new() });
    cx.update(|window, _| window.activate_window());
    settle(cx);
    tab_to(1, cx);
    press("down", cx);
    press("enter", cx);
    assert_eq!(view.read_with(cx, |view, _| view.played.clone()), ["d"]);
}

#[test]
#[should_panic(expected = "track a twice")]
fn a_song_is_listed_once() {
    let _ = Playlist::new("list", [track("a", "One"), track("a", "Two")]);
}

/// An audio player and where it was sent.
struct Hearing {
    at: Duration,
}

impl Render for Hearing {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let owner = cx.entity();
        div().w(px(280.0)).child(
            AudioPlayer::new(
                "song",
                "Tall Windows",
                "Aster Quartet",
                SECOND * 200,
                self.at,
            )
            .on_seek(move |time, _, cx| {
                owner.update(cx, |view, cx| {
                    view.at = time;
                    cx.notify();
                })
            }),
        )
    }
}

#[gpui::test]
fn a_press_on_the_players_track_seeks(cx: &mut TestAppContext) {
    setup(cx);
    let (view, cx) = cx.add_window_view(|_, _| Hearing { at: SECOND * 10 });
    cx.update(|window, _| window.activate_window());
    settle(cx);
    let half = across("scrubber-rail", 0.5, cx);
    cx.simulate_click(half, Modifiers::none());
    settle(cx);
    let at = view.read_with(cx, |view, _| view.at);
    assert!(at.abs_diff(SECOND * 100) < SECOND * 2, "{at:?}");
}
