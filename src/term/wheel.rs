//! Lines a wheel event scrolls, from gpui's units per system.

use gpui::{Pixels, ScrollDelta, ScrollWheelEvent};

/// Lines per gpui line: macOS's notch scrolls 3, as Ghostty's.
const NOTCH: f32 = if cfg!(target_os = "macos") { 3. } else { 1. };
/// Windows' one screen at a time, in lines a notch.
const PAGE: f32 = u32::MAX as f32;

/// Whole lines `event` moves, `rows` at most; `rest` keeps parts.
pub fn lines(
    rest: &mut f32,
    event: &ScrollWheelEvent,
    height: Pixels,
    rows: usize,
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
            // A paged notch's parts scroll their share of a screen.
            if lines == PAGE {
                delta.y / PAGE * page
            } else {
                delta.y * NOTCH
            }
        }
    };
    if travel.is_nan() {
        log::error!("wheel: {:?} moves no number of lines", event.delta);
        return None;
    }
    *rest = (*rest + travel).clamp(-page, page);
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

    /// gpui's lines a notch: Windows' setting, Linux's 3, macOS's one.
    #[test]
    fn a_notch_scrolls_as_the_system_asks_and_an_event_a_screen_at_most() {
        let mut rest = 0.;
        let mut scroll = |delta, shift, sideways: (f32, f32)| {
            lines(&mut rest, &wheel(delta, shift), px(14.4), 24, || {
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
                    moved += lines(&mut rest, &event, px(14.4), 53, || Some((PAGE, 3.))).unwrap();
                }
                assert_eq!(moved, 53, "{parts} messages, Shift {shift}");
            }
        }
    }
}
