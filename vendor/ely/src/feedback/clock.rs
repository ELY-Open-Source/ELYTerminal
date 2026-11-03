use std::{rc::Rc, time::Duration};

use gpui::{
    App, ElementId, IntoElement, ParentElement, RenderOnce, Styled, Task, Window, div, prelude::*,
};
use web_time::Instant;

use crate::{
    forms::Run,
    theme::{ActiveTheme, TextSize},
    typography::AnimatedNumber,
};

const SECOND: Duration = Duration::from_secs(1);

/// A face's anchor, which way it runs, and the task that wakes it each second.
struct Clock {
    anchor: Instant,
    up: bool,
    on_done: Option<Run>,
    _tick: Option<Task<()>>,
}

impl Clock {
    /// Whole seconds shown at `now`: time since the anchor, or time left rounded up.
    fn shown(&self, now: Instant) -> u64 {
        if self.up {
            now.saturating_duration_since(self.anchor).as_secs()
        } else {
            let left = self.anchor.saturating_duration_since(now);
            left.as_secs() + u64::from(left.subsec_nanos() > 0)
        }
    }

    /// Time until the shown second changes; `None` once a countdown is done.
    fn wait(&self, now: Instant) -> Option<Duration> {
        if self.up {
            let past = now.saturating_duration_since(self.anchor);
            return Some(SECOND - Duration::from_nanos(u64::from(past.subsec_nanos())));
        }
        let left = self.anchor.saturating_duration_since(now);
        match (left.is_zero(), left.subsec_nanos()) {
            (true, _) => None,
            (false, 0) => Some(SECOND),
            (false, nanos) => Some(Duration::from_nanos(u64::from(nanos))),
        }
    }
}

/// Keeps a face's clock, waking its view each time the shown second changes. Returns that second.
fn tick(
    id: &ElementId,
    anchor: Instant,
    up: bool,
    on_done: Option<Run>,
    window: &mut Window,
    cx: &mut App,
) -> u64 {
    let state = window.use_keyed_state((id.clone(), "clock"), cx, |_, _| Clock {
        anchor,
        up,
        on_done: None,
        _tick: None,
    });
    state.update(cx, |clock, _| {
        if clock.anchor != anchor {
            clock.anchor = anchor;
            clock._tick = None;
        }
        clock.on_done = on_done;
    });
    if state.read(cx)._tick.is_none() {
        let (view, weak, id) = (window.current_view(), state.downgrade(), id.clone());
        let task = window.spawn(cx, async move |cx| {
            loop {
                let wait =
                    weak.read_with(cx, |clock, cx| clock.wait(cx.background_executor().now()));
                let Ok(wait) = wait else { break };
                if let Some(wait) = wait {
                    cx.background_executor().timer(wait).await;
                }
                let woke = cx.update(|_, cx| {
                    cx.notify(view);
                    weak.read_with(cx, |clock, cx| {
                        let over =
                            !clock.up && clock.wait(cx.background_executor().now()).is_none();
                        (over, clock.on_done.clone().filter(|_| over))
                    })
                });
                let Ok(Ok((over, done))) = woke else { break };
                if let Some(done) = done {
                    log::info!("countdown {id:?}: done");
                    if let Err(error) = cx.update(|window, cx| done(window, cx)) {
                        log::error!("countdown {id:?}: its window closed first: {error:#}");
                    }
                }
                if over {
                    break;
                }
            }
        });
        state.update(cx, |clock, _| clock._tick = Some(task));
    }
    state.read(cx).shown(cx.background_executor().now())
}

/// Days, hours, minutes and seconds in `seconds`.
fn parts(seconds: u64) -> (u64, u64, u64, u64) {
    (
        seconds / 86_400,
        seconds / 3_600 % 24,
        seconds / 60 % 60,
        seconds % 60,
    )
}

/// A clock face whose digits roll: minutes and seconds, hours once there are any, days ahead of them.
fn face(id: &ElementId, seconds: u64, size: TextSize, cx: &App) -> impl IntoElement + use<> {
    let theme = cx.theme();
    let (days, hours, minutes, secs) = parts(seconds);
    let number = |part: &'static str, value: u64| {
        AnimatedNumber::new((id.clone(), part), value as f64)
            .pad(2)
            .size(size)
    };
    let colon = || {
        div()
            .text_size(theme.text_size(size))
            .text_color(theme.colors.fg_subtle)
            .child(":")
    };
    div()
        .flex()
        .items_center()
        .gap_0p5()
        .when(days > 0, |face| {
            face.child(AnimatedNumber::new((id.clone(), "d"), days as f64).size(size))
                .child(
                    div()
                        .mr_2()
                        .text_size(theme.text_size(size))
                        .text_color(theme.colors.fg_subtle)
                        .child("d"),
                )
        })
        .when(seconds >= 3_600, |face| {
            face.child(number("h", hours)).child(colon())
        })
        .child(number("m", minutes))
        .child(colon())
        .child(number("s", secs))
}

/// Time left until a moment, its digits rolling as they change. `on_done` runs once at zero.
#[derive(IntoElement)]
pub struct Countdown {
    id: ElementId,
    until: Instant,
    size: TextSize,
    on_done: Option<Run>,
}

impl Countdown {
    pub fn new(id: impl Into<ElementId>, until: Instant) -> Self {
        Self {
            id: id.into(),
            until,
            size: TextSize::Xxl,
            on_done: None,
        }
    }

    pub fn size(mut self, size: TextSize) -> Self {
        self.size = size;
        self
    }

    pub fn on_done(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_done = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for Countdown {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let seconds = tick(&self.id, self.until, false, self.on_done, window, cx);
        face(&self.id, seconds, self.size, cx)
    }
}

/// Time since a moment, counting up with rolling digits; `stopped` freezes it.
#[derive(IntoElement)]
pub struct Timer {
    id: ElementId,
    since: Instant,
    stopped: Option<Instant>,
    size: TextSize,
}

impl Timer {
    pub fn new(id: impl Into<ElementId>, since: Instant) -> Self {
        Self {
            id: id.into(),
            since,
            stopped: None,
            size: TextSize::Xxl,
        }
    }

    /// Shows the time from `since` to `at`, and stops ticking.
    pub fn stopped(mut self, at: Option<Instant>) -> Self {
        self.stopped = at;
        self
    }

    pub fn size(mut self, size: TextSize) -> Self {
        self.size = size;
        self
    }
}

impl RenderOnce for Timer {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let seconds = match self.stopped {
            Some(at) => at.saturating_duration_since(self.since).as_secs(),
            None => tick(&self.id, self.since, true, None, window, cx),
        };
        face(&self.id, seconds, self.size, cx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parts_split_days_hours_minutes_and_seconds() {
        assert_eq!(parts(0), (0, 0, 0, 0));
        assert_eq!(parts(3_661), (0, 1, 1, 1));
        assert_eq!(parts(2 * 86_400 + 4 * 3_600 + 12 * 60 + 9), (2, 4, 12, 9));
    }

    #[test]
    fn a_countdown_shows_the_second_it_is_in_and_wakes_at_the_next() {
        let start = Instant::now();
        let clock = Clock {
            anchor: start + Duration::from_millis(4_300),
            up: false,
            on_done: None,
            _tick: None,
        };
        assert_eq!(clock.shown(start), 5);
        assert_eq!(clock.wait(start), Some(Duration::from_millis(300)));
        assert_eq!(clock.shown(start + Duration::from_millis(4_300)), 0);
        assert_eq!(clock.wait(start + Duration::from_millis(4_300)), None);
    }

    #[test]
    fn a_timer_counts_whole_seconds_and_wakes_on_the_next() {
        let start = Instant::now();
        let clock = Clock {
            anchor: start,
            up: true,
            on_done: None,
            _tick: None,
        };
        let now = start + Duration::from_millis(65_250);
        assert_eq!(clock.shown(now), 65);
        assert_eq!(clock.wait(now), Some(Duration::from_millis(750)));
    }
}
