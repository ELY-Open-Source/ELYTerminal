use std::time::Duration;

use gpui::{
    App, ElementId, FocusHandle, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    Styled, Task, Window, div,
};
use web_time::Instant;

use crate::{
    buttons::{Button, ButtonVariant},
    primitives::tab_stop,
    theme::{ActiveTheme, TextSize},
    typography::tabular,
};

const STEP: Duration = Duration::from_millis(100);

/// Time run while started and held while paused, the laps taken, and the task that wakes the face while it runs.
#[derive(Default)]
struct Run {
    held: Duration,
    since: Option<Instant>,
    laps: Vec<Duration>,
    tick: Option<Task<()>>,
}

impl Run {
    fn elapsed(&self, now: Instant) -> Duration {
        self.held
            + self
                .since
                .map_or(Duration::ZERO, |since| now.saturating_duration_since(since))
    }

    fn start(&mut self, now: Instant) {
        self.since.get_or_insert(now);
    }

    fn pause(&mut self, now: Instant) {
        if let Some(since) = self.since.take() {
            self.held += now.saturating_duration_since(since);
        }
        self.tick = None;
    }

    /// Marks the time so far; only while running.
    fn lap(&mut self, now: Instant) {
        if self.since.is_some() {
            self.laps.push(self.elapsed(now));
        }
    }

    fn reset(&mut self) {
        *self = Run::default();
    }
}

/// Minutes, seconds and tenths, as 01:02.3; hours ahead of them once there are any.
pub(crate) fn tenths(elapsed: Duration) -> String {
    let tenths = elapsed.as_millis() / 100;
    let (hours, minutes, seconds, tenth) = (
        tenths / 36_000,
        tenths / 600 % 60,
        tenths / 10 % 60,
        tenths % 10,
    );
    match hours {
        0 => format!("{minutes:02}:{seconds:02}.{tenth}"),
        _ => format!("{hours}:{minutes:02}:{seconds:02}.{tenth}"),
    }
}

/// Time that runs while started and holds while paused, in tenths, with Start or Pause under it beside Lap, which reads Reset once paused, and the laps below, newest first: each lap's own time and the total at its end. It reads the executor's clock.
#[derive(IntoElement)]
pub struct Stopwatch {
    id: ElementId,
}

impl Stopwatch {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self { id: id.into() }
    }
}

impl RenderOnce for Stopwatch {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id.clone();
        let run = window.use_keyed_state((id.clone(), "run"), cx, |_, _| Run::default());
        let toggle = tab_stop((id.clone(), "toggle").into(), true, window, cx);
        let now = cx.background_executor().now();
        let (running, elapsed) = (run.read(cx).since.is_some(), run.read(cx).elapsed(now));
        let side = tab_stop(
            (id.clone(), "side").into(),
            running || !elapsed.is_zero(),
            window,
            cx,
        );
        if running && run.read(cx).tick.is_none() {
            let (view, weak) = (window.current_view(), run.downgrade());
            let task = window.spawn(cx, async move |cx| {
                loop {
                    cx.background_executor().timer(STEP).await;
                    let woke = cx.update(|_, cx| {
                        cx.notify(view);
                        weak.read_with(cx, |run, _| run.since.is_some())
                    });
                    if !matches!(woke, Ok(Ok(true))) {
                        break;
                    }
                }
            });
            run.update(cx, |run, _| run.tick = Some(task));
        }
        let laps = run.read(cx).laps.clone();
        let theme = cx.theme();
        let shown = tenths(elapsed);
        let act = |name: &'static str, edit: fn(&mut Run, Instant), then: Option<&FocusHandle>| {
            let (run, then) = (run.clone(), then.cloned());
            move |_: &gpui::ClickEvent, window: &mut Window, cx: &mut App| {
                let now = cx.background_executor().now();
                run.update(cx, |run, cx| {
                    edit(run, now);
                    log::info!("stopwatch: {name} at {}", tenths(run.elapsed(now)));
                    cx.notify();
                });
                if let Some(then) = &then {
                    window.focus(then, cx);
                }
            }
        };
        let rows = laps.iter().enumerate().rev().map(|(ix, total)| {
            let before = ix.checked_sub(1).map_or(Duration::ZERO, |last| laps[last]);
            let own = tenths(*total - before);
            div()
                .debug_selector(|| format!("lap-{}-{own}-{}", ix + 1, tenths(*total)))
                .flex()
                .gap_3()
                .py_1()
                .border_b_1()
                .border_color(theme.colors.border)
                .text_color(theme.colors.fg_muted)
                .child(div().flex_1().child(format!("Lap {}", ix + 1)))
                .child(
                    tabular(div())
                        .flex_none()
                        .text_color(theme.colors.fg)
                        .child(own.clone()),
                )
                .child(tabular(div()).flex_none().child(tenths(*total)))
        });
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                tabular(div())
                    .debug_selector(|| format!("stopwatch-{shown}"))
                    .text_size(theme.text_size(TextSize::Xxl))
                    .text_color(theme.colors.fg)
                    .child(shown.clone()),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_2()
                    .child(
                        Button::new(
                            (id.clone(), "start"),
                            if running { "Pause" } else { "Start" },
                        )
                        .variant(if running {
                            ButtonVariant::Secondary
                        } else {
                            ButtonVariant::Primary
                        })
                        .focus_handle(&toggle)
                        .on_click(match running {
                            true => act("pause", Run::pause, None),
                            false => act("start", Run::start, None),
                        }),
                    )
                    .child(
                        Button::new(
                            (id.clone(), "side"),
                            if running || elapsed.is_zero() {
                                "Lap"
                            } else {
                                "Reset"
                            },
                        )
                        .focus_handle(&side)
                        .disabled(!running && elapsed.is_zero())
                        .on_click(match running {
                            true => act("lap", Run::lap, None),
                            false => act("reset", |run, _| run.reset(), Some(&toggle)),
                        }),
                    ),
            )
            .child(div().flex().flex_col().children(rows))
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use web_time::Instant;

    use super::{Run, tenths};

    #[test]
    fn a_readout_shows_minutes_seconds_and_tenths() {
        assert_eq!(tenths(Duration::ZERO), "00:00.0");
        assert_eq!(tenths(Duration::from_millis(62_345)), "01:02.3");
        assert_eq!(tenths(Duration::from_millis(3_723_450)), "1:02:03.4");
        assert_eq!(tenths(Duration::from_millis(99)), "00:00.0", "tenths floor");
    }

    #[test]
    fn a_run_holds_while_paused_and_laps_only_while_running() {
        let start = Instant::now();
        let at = |ms: u64| start + Duration::from_millis(ms);
        let mut run = Run::default();
        run.start(at(0));
        run.lap(at(1_200));
        run.pause(at(2_000));
        assert_eq!(
            run.elapsed(at(9_000)),
            Duration::from_millis(2_000),
            "paused, it holds"
        );
        run.lap(at(9_000));
        assert_eq!(
            run.laps,
            [Duration::from_millis(1_200)],
            "no lap while paused"
        );
        run.start(at(10_000));
        run.start(at(10_500));
        assert_eq!(
            run.elapsed(at(11_000)),
            Duration::from_millis(3_000),
            "a second start keeps the first"
        );
        run.reset();
        assert_eq!(
            (run.elapsed(at(12_000)), run.laps.len()),
            (Duration::ZERO, 0)
        );
    }
}
