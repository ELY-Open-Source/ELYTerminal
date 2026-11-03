use std::{path::PathBuf, time::Duration};

use gpui::{
    Context, Entity, InteractiveElement, IntoElement, Modifiers, MouseButton, ParentElement,
    Render, Styled, TestAppContext, VisualTestContext, Window, div, point, px,
};

use super::{across, picture, press, settle, setup};
use crate::media::{Cue, Scrubber, SubtitleEditor, VideoThumbnailStrip};

const SECOND: Duration = Duration::from_secs(1);

/// A scrubber over a hundred seconds, 400 wide, and the times it was sent to.
struct Scrubbing {
    at: Duration,
    sought: Vec<Duration>,
}

impl Render for Scrubbing {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let owner = cx.entity();
        div().w(px(400.0)).mt(px(40.0)).child(
            Scrubber::new("scrub", SECOND * 100, self.at)
                .chapter(Duration::ZERO, "Intro")
                .chapter(SECOND * 40, "Stair")
                .on_seek(move |time, _, cx| {
                    owner.update(cx, |view, cx| {
                        (view.at, view.sought) = (time, [view.sought.clone(), vec![time]].concat());
                        cx.notify();
                    })
                }),
        )
    }
}

fn scrubbing(cx: &mut TestAppContext) -> (Entity<Scrubbing>, &mut VisualTestContext) {
    setup(cx);
    let (view, cx) = cx.add_window_view(|_, _| Scrubbing {
        at: Duration::ZERO,
        sought: Vec::new(),
    });
    cx.update(|window, _| window.activate_window());
    settle(cx);
    (view, cx)
}

#[gpui::test]
fn a_press_seeks_and_a_drag_past_the_end_stops_there(cx: &mut TestAppContext) {
    let (view, cx) = scrubbing(cx);
    let quarter = across("scrubber-rail", 0.25, cx);
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
fn the_arrows_step_five_seconds_and_end_goes_to_the_end(cx: &mut TestAppContext) {
    let (view, cx) = scrubbing(cx);
    let tenth = across("scrubber-rail", 0.1, cx);
    cx.simulate_click(tenth, Modifiers::none());
    settle(cx);
    let start = view.read_with(cx, |view, _| view.at);
    press("right", cx);
    assert_eq!(view.read_with(cx, |view, _| view.at), start + SECOND * 5);
    press("end", cx);
    assert_eq!(view.read_with(cx, |view, _| view.at), SECOND * 100);
}

#[gpui::test]
fn the_pointer_shows_its_time_over_the_track(cx: &mut TestAppContext) {
    let (_, cx) = scrubbing(cx);
    assert!(cx.debug_bounds("time-tip").is_none());
    let half = across("scrubber-rail", 0.5, cx);
    cx.simulate_mouse_move(half, None, Modifiers::none());
    settle(cx);
    let tip = cx.debug_bounds("time-tip").expect("a tip over the track");
    assert!(
        (tip.center().x - half.x).abs() < px(1.0),
        "centered on the pointer: {tip:?}"
    );
}

#[gpui::test]
fn the_tip_stays_over_the_track_at_its_ends(cx: &mut TestAppContext) {
    let (_, cx) = scrubbing(cx);
    for share in [0.0, 1.0] {
        let at = across("scrubber-rail", share, cx);
        cx.simulate_mouse_move(at, None, Modifiers::none());
        settle(cx);
        settle(cx);
        let tip = cx.debug_bounds("time-tip").expect("a tip over the track");
        let rail = cx.debug_bounds("scrubber-rail").expect("the rail draws");
        let inside = tip.left() >= rail.left() - px(0.5) && tip.right() <= rail.right() + px(0.5);
        assert!(inside, "at {share}: {tip:?} over {rail:?}");
    }
}

/// A strip of four frames over a hundred seconds with a trim, and what it was told.
struct Trimming {
    path: PathBuf,
    at: Duration,
    trim: (Duration, Duration),
}

impl Render for Trimming {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (seek, cut) = (cx.entity(), cx.entity());
        let frames: Vec<PathBuf> = (0..4).map(|_| self.path.clone()).collect();
        div().w(px(400.0)).child(
            VideoThumbnailStrip::new("strip", frames, SECOND * 100, self.at)
                .trim(self.trim.0, self.trim.1)
                .on_seek(move |time, _, cx| {
                    seek.update(cx, |view, cx| {
                        view.at = time;
                        cx.notify();
                    })
                })
                .on_trim(move |trim, _, cx| {
                    cut.update(cx, |view, cx| {
                        view.trim = trim;
                        cx.notify();
                    })
                }),
        )
    }
}

fn trimming(cx: &mut TestAppContext) -> (Entity<Trimming>, &mut VisualTestContext) {
    setup(cx);
    let path = picture("strip", 160, 100);
    let (view, cx) = cx.add_window_view(|_, _| Trimming {
        path,
        at: Duration::ZERO,
        trim: (SECOND * 10, SECOND * 50),
    });
    cx.update(|window, _| window.activate_window());
    settle(cx);
    (view, cx)
}

#[gpui::test]
fn a_press_seeks_and_a_handle_drag_moves_its_end_to_the_length(cx: &mut TestAppContext) {
    let (view, cx) = trimming(cx);
    let press_at = across("video-strip", 0.3, cx);
    cx.simulate_click(press_at, Modifiers::none());
    settle(cx);
    let at = view.read_with(cx, |view, _| view.at);
    assert!(at.abs_diff(SECOND * 30) < SECOND, "{at:?}");
    let out = across("video-strip", 0.5, cx);
    let none = Modifiers::none();
    cx.simulate_mouse_down(out, MouseButton::Left, none);
    cx.simulate_mouse_move(out + point(px(10.0), px(0.0)), MouseButton::Left, none);
    cx.simulate_mouse_move(out + point(px(900.0), px(0.0)), MouseButton::Left, none);
    cx.simulate_mouse_up(out + point(px(900.0), px(0.0)), MouseButton::Left, none);
    settle(cx);
    assert_eq!(
        view.read_with(cx, |view, _| view.trim),
        (SECOND * 10, SECOND * 100)
    );
}

#[gpui::test]
fn command_o_leaves_the_trim_to_the_app(cx: &mut TestAppContext) {
    let (view, cx) = trimming(cx);
    let press_at = across("video-strip", 0.3, cx);
    cx.simulate_click(press_at, Modifiers::none());
    settle(cx);
    press("cmd-o", cx);
    assert_eq!(
        view.read_with(cx, |view, _| view.trim),
        (SECOND * 10, SECOND * 50)
    );
}

#[gpui::test]
fn i_and_o_set_the_trim_at_the_playhead(cx: &mut TestAppContext) {
    let (view, cx) = trimming(cx);
    let press_at = across("video-strip", 0.3, cx);
    cx.simulate_click(press_at, Modifiers::none());
    settle(cx);
    let at = view.read_with(cx, |view, _| view.at);
    press("i", cx);
    assert_eq!(view.read_with(cx, |view, _| view.trim), (at, SECOND * 50));
    press("right", cx);
    press("o", cx);
    assert_eq!(
        view.read_with(cx, |view, _| view.trim),
        (at, at + SECOND * 25)
    );
}

/// A subtitle editor over two cues, its playhead, and the cues it sent.
struct Captioning {
    cues: Vec<Cue>,
    at: Duration,
    width: f32,
}

impl Render for Captioning {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let owner = cx.entity();
        div()
            .w(px(self.width))
            .debug_selector(|| "subtitle-host".into())
            .child(
                SubtitleEditor::new("subs", self.cues.clone(), self.at).on_change(
                    move |cues, _, cx| {
                        owner.update(cx, |view, cx| {
                            view.cues = cues.to_vec();
                            cx.notify();
                        })
                    },
                ),
            )
    }
}

fn captioning(cx: &mut TestAppContext) -> (Entity<Captioning>, &mut VisualTestContext) {
    captioning_at(420.0, "b", cx)
}

fn captioning_at<'a>(
    width: f32,
    words: &'static str,
    cx: &'a mut TestAppContext,
) -> (Entity<Captioning>, &'a mut VisualTestContext) {
    setup(cx);
    let cue = |key: &str, text: &str, start: u64, end: u64| Cue {
        key: key.to_string().into(),
        start: SECOND * start as u32,
        end: SECOND * end as u32,
        text: text.to_string().into(),
    };
    let (view, cx) = cx.add_window_view(move |_, _| Captioning {
        cues: vec![cue("a", "a", 1, 3), cue("b", words, 5, 8)],
        at: SECOND * 6,
        width,
    });
    cx.update(|window, _| window.activate_window());
    settle(cx);
    (view, cx)
}

/// Tabs to the `nth` stop and types over it.
fn retype(nth: usize, text: &str, cx: &mut VisualTestContext) {
    cx.update(|window, cx| {
        window.blur(cx);
        for _ in 0..nth {
            window.focus_next(cx);
        }
    });
    press("enter", cx);
    cx.simulate_input(text);
    press("enter", cx);
}

#[gpui::test]
fn a_start_typed_in_place_moves_its_cue_into_order(cx: &mut TestAppContext) {
    let (view, cx) = captioning(cx);
    retype(5, "00:00.500", cx);
    let keys: Vec<String> = view.read_with(cx, |view, _| {
        view.cues.iter().map(|cue| cue.key.to_string()).collect()
    });
    assert_eq!(keys, ["b", "a"]);
}

#[gpui::test]
fn a_time_that_does_not_read_keeps_the_cue_and_says_why(cx: &mut TestAppContext) {
    let (view, cx) = captioning(cx);
    let before = view.read_with(cx, |view, _| view.cues.clone());
    retype(1, "1:x", cx);
    assert_eq!(view.read_with(cx, |view, _| view.cues.clone()), before);
    assert!(cx.debug_bounds("subtitle-complaint").is_some());
}

#[gpui::test]
fn long_words_give_way_inside_a_narrow_editor(cx: &mut TestAppContext) {
    let long = "The shadow walks the length of the hall while the morning comes in";
    let (_, cx) = captioning_at(280.0, long, cx);
    let host = cx.debug_bounds("subtitle-host").expect("the editor draws");
    let words = cx
        .debug_bounds("inline-edit subs-b-words")
        .expect("the words draw");
    assert!(words.right() <= host.right(), "{words:?} in {host:?}");
}
