use std::time::Duration;

use gpui::{
    Bounds, Context, IntoElement, Modifiers, MouseButton, ParentElement, Render, Styled,
    TestAppContext, Window, div, point, px, size,
};

use super::{settle, setup};
use crate::media::{
    AudioPlayer, AudioWaveform, Device, DeviceKind, DeviceSelector, Playlist, Scrubber, Track,
    VideoPlayer, VideoThumbnailStrip, played, scrubber::along,
};

const SECOND: Duration = Duration::from_secs(1);

#[test]
fn a_position_past_the_end_sits_at_it() {
    assert_eq!(played(SECOND * 3, SECOND * 10, "test"), SECOND * 3);
    assert_eq!(played(SECOND * 12, SECOND * 10, "test"), SECOND * 10);
}

/// Live positions past their media, and choices gone from their lists.
struct Lagging;

impl Render for Lagging {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let (length, past) = (SECOND * 10, SECOND * 12);
        let track = Track {
            key: "a".into(),
            title: "Tall Windows".into(),
            artist: "Aster Quartet".into(),
            length,
        };
        let device = Device {
            key: "a".into(),
            name: "Device a".into(),
        };
        div()
            .w(px(480.0))
            .child(AudioWaveform::new("wave", [0.5, 0.8], length, past))
            .child(AudioPlayer::new(
                "player",
                "Tall Windows",
                "Aster Quartet",
                length,
                past,
            ))
            .child(VideoPlayer::new("video", None, 1.5, length, past))
            .child(
                Scrubber::new("scrub", length, past)
                    .loaded(past)
                    .chapter(SECOND * 2, "Intro")
                    .chapter(past, "Gone"),
            )
            .child(
                VideoThumbnailStrip::new("strip", ["a.jpg"], length, past).trim(SECOND * 2, past),
            )
            .child(Playlist::new("list", [track]).current("c", false))
            .child(DeviceSelector::new("mics", DeviceKind::Microphone, [device]).chosen("c"))
    }
}

#[gpui::test]
fn live_positions_and_gone_choices_draw(cx: &mut TestAppContext) {
    setup(cx);
    let (_, cx) = cx.add_window_view(|_, _| Lagging);
    settle(cx);
}

#[test]
fn a_rail_with_no_width_has_no_place_along_it() {
    let rail = |width: f32| Bounds::new(point(px(10.0), px(0.0)), size(px(width), px(8.0)));
    assert_eq!(along(rail(0.0), point(px(10.0), px(4.0))), None);
    assert_eq!(along(rail(100.0), point(px(60.0), px(4.0))), Some(0.5));
    assert_eq!(along(rail(100.0), point(px(500.0), px(4.0))), Some(1.0));
}

/// Rails that seek, laid out with no width.
struct Squeezed;

impl Render for Squeezed {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let length = SECOND * 10;
        let seek = |_, _: &mut Window, _: &mut gpui::App| {};
        div()
            .w(px(0.0))
            .child(Scrubber::new("scrub", length, SECOND).on_seek(seek))
            .child(AudioWaveform::new("wave", [0.5, 0.8], length, SECOND).on_seek(seek))
            .child(VideoThumbnailStrip::new("strip", ["a.jpg"], length, SECOND).on_seek(seek))
    }
}

#[gpui::test]
fn the_pointer_over_rails_with_no_width_seeks_nothing(cx: &mut TestAppContext) {
    setup(cx);
    let (_, cx) = cx.add_window_view(|_, _| Squeezed);
    settle(cx);
    for rail in ["scrubber", "audio-waveform", "video-strip"] {
        let bounds = cx.debug_bounds(rail).expect("a rail");
        let at = point(bounds.left(), bounds.center().y);
        cx.simulate_mouse_move(at, None, Modifiers::none());
        settle(cx);
        cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
        cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
    }
    settle(cx);
}
