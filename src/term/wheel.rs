//! Local lines or mouse reports, from gpui's units per system.

use gpui::{Pixels, ScrollDelta, ScrollWheelEvent};

/// Lines per gpui line: macOS's notch scrolls 3, as Ghostty's.
const NOTCH: f32 = if cfg!(target_os = "macos") { 3. } else { 1. };
/// Windows' one screen at a time, in lines a notch.
const PAGE: f32 = u32::MAX as f32;

/// Whole local lines or Windows reporting notches; `rest` keeps parts.
pub fn lines(
    rest: &mut f32,
    event: &ScrollWheelEvent,
    height: Pixels,
    rows: usize,
    reporting: bool,
    settings: impl FnOnce() -> Option<(f32, f32)>,
) -> Option<i32> {
    let page = rows as f32;
    let travel = match event.delta {
        ScrollDelta::Pixels(delta) => delta.y / height,
        // gpui gives Shift's notch sideways on Windows and Linux.
        ScrollDelta::Lines(delta) if event.modifiers.shift && delta.y == 0. && delta.x != 0. => {
            let (lines, chars) = settings()?;
            if lines == PAGE {
                delta.x / chars * page
            } else {
                delta.x * lines / chars * NOTCH
            }
        }
        ScrollDelta::Lines(delta) if delta.y == 0. => 0.,
        ScrollDelta::Lines(delta) => {
            let (lines, _) = settings()?;
            if cfg!(windows) && reporting {
                if lines == 0. { 0. } else { delta.y / lines }
            } else if lines == PAGE {
                // A paged notch's parts scroll their share of a screen.
                delta.y / PAGE * page
            } else {
                delta.y * NOTCH
            }
        }
    };
    whole(rest, travel, page, event)
}

/// Whole notches, a trackpad's lines among them, for the font's size; `rest` keeps parts.
pub fn notches(
    rest: &mut f32,
    event: &ScrollWheelEvent,
    height: Pixels,
    settings: impl FnOnce() -> Option<(f32, f32)>,
) -> Option<i32> {
    let travel = match event.delta {
        ScrollDelta::Pixels(delta) => delta.y / height,
        ScrollDelta::Lines(delta) => {
            let lines = if cfg!(windows) {
                settings()?.0
            } else if cfg!(target_os = "linux") {
                LINUX
            } else {
                1.
            };
            if lines == 0. { 0. } else { delta.y / lines }
        }
    };
    whole(rest, travel, ZOOM, event)
}

/// gpui's lines a Linux notch.
const LINUX: f32 = 3.;
/// Notches one event zooms at most.
const ZOOM: f32 = 10.;

/// `travel` added to `rest`, within `most`; the whole part taken out.
fn whole(rest: &mut f32, travel: f32, most: f32, event: &ScrollWheelEvent) -> Option<i32> {
    if travel.is_nan() {
        log::error!("wheel: {:?} moves no number of lines", event.delta);
        return None;
    }
    *rest = (*rest + travel).clamp(-most, most);
    // A notch's f32 parts can sum short of the line.
    if (*rest - rest.round()).abs() < 1e-3 {
        *rest = rest.round();
    }
    let lines = rest.trunc();
    *rest -= lines;
    Some(lines as i32)
}

/// Windows' wheel lines and characters, read when gpui reads them.
#[cfg(windows)]
static KEPT: std::sync::OnceLock<(f32, f32)> = std::sync::OnceLock::new();

/// Reads Windows' wheel settings as the main window opens.
#[cfg(windows)]
pub fn keep() -> anyhow::Result<()> {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        SPI_GETWHEELSCROLLCHARS, SPI_GETWHEELSCROLLLINES, SystemParametersInfoW,
    };
    let setting = |action, name| {
        let mut value = 0u32;
        let read = unsafe { SystemParametersInfoW(action, 0, (&raw mut value).cast(), 0) } != 0;
        anyhow::ensure!(read, "wheel: Windows' {name} do not read");
        Ok(value as f32)
    };
    let kept = (
        setting(SPI_GETWHEELSCROLLLINES, "lines a notch")?,
        setting(SPI_GETWHEELSCROLLCHARS, "characters a sideways notch")?,
    );
    anyhow::ensure!(
        KEPT.set(kept).is_ok(),
        "wheel: a second window keeps Windows' settings"
    );
    log::info!(
        "wheel: Windows sets {} lines a notch and {} characters a sideways one",
        kept.0,
        kept.1
    );
    Ok(())
}

/// The lines and characters a notch, as `keep` read them.
#[cfg(windows)]
pub fn settings() -> Option<(f32, f32)> {
    let kept = KEPT.get().copied();
    if kept.is_none() {
        log::error!("wheel: Windows' settings were not kept as the window opened");
    }
    kept
}

/// Off Windows a notch keeps gpui's own lines.
#[cfg(not(windows))]
pub fn settings() -> Option<(f32, f32)> {
    Some((1., 1.))
}

#[cfg(test)]
mod tests {
    use gpui::{Modifiers, point, px};

    use super::*;

    fn wheel(delta: ScrollDelta, shift: bool) -> ScrollWheelEvent {
        let modifiers = Modifiers {
            shift,
            ..Modifiers::default()
        };
        ScrollWheelEvent {
            delta,
            modifiers,
            ..ScrollWheelEvent::default()
        }
    }

    #[test]
    fn ctrl_s_wheel_zooms_a_point_a_notch_and_a_trackpad_a_point_a_line() {
        let notch = if cfg!(target_os = "macos") { 1. } else { 3. };
        let settings = || Some((3., 3.));
        let mut rest = 0.;
        let up = wheel(ScrollDelta::Lines(point(0., notch)), false);
        assert_eq!(notches(&mut rest, &up, px(16.), settings), Some(1));
        let half = wheel(ScrollDelta::Lines(point(0., -notch / 2.)), false);
        assert_eq!(notches(&mut rest, &half, px(16.), settings), Some(0));
        assert_eq!(notches(&mut rest, &half, px(16.), settings), Some(-1));
        let pixels = wheel(ScrollDelta::Pixels(point(px(0.), px(40.))), false);
        assert_eq!(notches(&mut rest, &pixels, px(16.), settings), Some(2));
        assert_eq!(notches(&mut rest, &pixels, px(16.), settings), Some(3));
        let fling = wheel(ScrollDelta::Pixels(point(px(0.), px(16000.))), false);
        assert_eq!(notches(&mut rest, &fling, px(16.), settings), Some(10));
        if cfg!(windows) {
            assert_eq!(notches(&mut rest, &up, px(16.), || Some((0., 0.))), Some(0));
        }
    }

    /// gpui's lines a notch: Windows' setting, Linux's 3, macOS's one.
    #[test]
    fn a_notch_scrolls_as_the_system_asks_and_an_event_a_screen_at_most() {
        let mut rest = 0.;
        let mut scroll = |delta, shift, sideways: (f32, f32)| {
            lines(&mut rest, &wheel(delta, shift), px(14.4), 24, false, || {
                Some(sideways)
            })
        };
        let notch = |y: f32| ScrollDelta::Lines(point(0., y));
        let one = if cfg!(target_os = "macos") { 1. } else { 3. };
        assert_eq!(
            scroll(notch(one), false, (1., 1.)),
            Some(3),
            "at 125%, 14.4 points a line"
        );
        assert_eq!(scroll(notch(1.), false, (1., 1.)), Some(NOTCH as i32));
        assert_eq!(
            scroll(notch(-PAGE), false, (PAGE, 3.)),
            Some(-24),
            "Windows' one screen at a time"
        );
        assert_eq!(
            scroll(notch(100.), false, (100., 3.)),
            Some(24),
            "a screen at most"
        );
        assert_eq!(
            scroll(notch(-1.), false, (1., 1.)),
            Some(-NOTCH as i32),
            "no part stays"
        );
        let shifted = ScrollDelta::Lines(point(3., 0.));
        let at_seven = Some((7. * NOTCH) as i32);
        assert_eq!(
            scroll(shifted, true, (7., 3.)),
            at_seven,
            "Shift's 3 characters, 7 lines"
        );
        assert_eq!(
            scroll(shifted, false, (7., 3.)),
            Some(0),
            "a sideways wheel alone"
        );
        assert_eq!(scroll(shifted, true, (0., 0.)), None, "no number");
        let mut moved = 0;
        for _ in 0..10 {
            moved += scroll(notch(12. / 120. * 7.), false, (1., 1.)).unwrap();
        }
        assert_eq!(
            moved,
            (7. * NOTCH) as i32,
            "ten messages of a tenth of a notch"
        );
        let mut moved = 0;
        for _ in 0..10 {
            let shifted = ScrollDelta::Lines(point(12. / 120. * 7., 0.));
            moved += scroll(shifted, true, (3., 7.)).unwrap();
        }
        assert_eq!(
            moved,
            (3. * NOTCH) as i32,
            "under Shift, 3 lines and 7 characters"
        );
        assert_eq!(
            scroll(notch(one), false, (1., 1.)),
            Some(3),
            "leaves the wheel"
        );
        let part = ScrollDelta::Pixels(point(px(0.), px(10.)));
        assert_eq!(scroll(part, false, (1., 1.)), Some(0));
        assert_eq!(scroll(part, false, (1., 1.)), Some(1));
    }

    /// A paged notch's messages scroll one screen together.
    #[test]
    fn a_paged_notch_split_into_messages_scrolls_one_screen() {
        for parts in [1, 2, 10, 120] {
            for shift in [false, true] {
                let mut rest = 0.;
                let mut moved = 0;
                for _ in 0..parts {
                    // gpui's arithmetic: the part's delta over 120, times the setting.
                    let share = (120 / parts) as f32 / 120.;
                    let delta = match shift {
                        true => ScrollDelta::Lines(point(share * 3., 0.)),
                        false => ScrollDelta::Lines(point(0., share * PAGE)),
                    };
                    let event = wheel(delta, shift);
                    moved +=
                        lines(&mut rest, &event, px(14.4), 53, false, || Some((PAGE, 3.))).unwrap();
                }
                assert_eq!(moved, 53, "{parts} messages, Shift {shift}");
            }
        }
    }

    #[cfg(windows)]
    fn reports(rest: &mut f32, delta: f32, setting: f32) -> Option<i32> {
        let event = wheel(ScrollDelta::Lines(point(0., delta)), false);
        lines(rest, &event, px(14.4), 24, true, || Some((setting, 3.)))
    }

    #[cfg(windows)]
    #[test]
    fn mouse_reports_count_windows_notches_not_scrollback_lines() {
        for setting in [1., 3., 5., 100., PAGE] {
            let mut rest = 0.;
            assert_eq!(reports(&mut rest, setting, setting), Some(1));
            assert_eq!(reports(&mut rest, -setting, setting), Some(-1));
            assert_eq!(reports(&mut rest, setting * 2., setting), Some(2));
            assert_eq!(rest, 0.);
        }
    }

    #[cfg(windows)]
    #[test]
    fn fractional_windows_notches_accumulate_before_reports_are_counted() {
        for setting in [1., 3., 5., 100., PAGE] {
            for parts in [2, 10, 120] {
                for direction in [-1, 1] {
                    let mut rest = 0.;
                    let delta = (120 / parts) as f32 / 120. * setting * direction as f32;
                    for _ in 1..parts {
                        assert_eq!(reports(&mut rest, delta, setting), Some(0));
                    }
                    assert_eq!(reports(&mut rest, delta, setting), Some(direction));
                    assert_eq!(rest, 0.);
                }
            }
        }
    }

    #[cfg(windows)]
    #[test]
    fn mouse_reporting_keeps_pixels_and_handles_disabled_or_invalid_wheels() {
        let mut rest = 0.;
        assert_eq!(reports(&mut rest, 0., 0.), Some(0));
        assert_eq!(reports(&mut rest, 5., 0.), Some(0));
        assert_eq!(reports(&mut rest, f32::NAN, 5.), None);
        assert_eq!(reports(&mut rest, 5., 5.), Some(1));
        let event = wheel(ScrollDelta::Pixels(point(px(0.), px(10.))), false);
        for expected in [0, 1] {
            assert_eq!(
                lines(&mut rest, &event, px(14.4), 24, true, || {
                    panic!("pixels need no wheel settings")
                }),
                Some(expected)
            );
        }
    }
}
