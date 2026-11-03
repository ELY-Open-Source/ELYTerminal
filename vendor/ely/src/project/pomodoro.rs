use std::{rc::Rc, time::Duration};

use gpui::{
    App, ElementId, Entity, IntoElement, ParentElement, RenderOnce, Styled, Task, Window, div,
};
use web_time::Instant;

use crate::{
    buttons::{Button, ButtonVariant, IconButton},
    motion::ProgressRing,
    primitives::IconName,
    theme::{ActiveTheme, ControlSize, TextSize},
    typography::{format, tabular},
};

type OnPhase = Rc<dyn Fn(Phase, &mut Window, &mut App)>;

/// What a pomodoro is doing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Focus,
    ShortBreak,
    LongBreak,
}

impl Phase {
    pub fn words(self) -> &'static str {
        match self {
            Phase::Focus => "Focus",
            Phase::ShortBreak => "Short break",
            Phase::LongBreak => "Long break",
        }
    }
}

/// How long each phase runs, and the focus rounds before a long break.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Lengths {
    pub focus: Duration,
    pub short: Duration,
    pub long: Duration,
    pub rounds: u8,
}

/// Where a pomodoro stands: its phase, the round it is on, time banked before a pause, and when it last started.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Cycle {
    pub phase: Phase,
    pub round: u8,
    pub banked: Duration,
    pub since: Option<Instant>,
}

impl Cycle {
    const START: Cycle = Cycle {
        phase: Phase::Focus,
        round: 1,
        banked: Duration::ZERO,
        since: None,
    };

    pub(crate) fn length(&self, lengths: &Lengths) -> Duration {
        match self.phase {
            Phase::Focus => lengths.focus,
            Phase::ShortBreak => lengths.short,
            Phase::LongBreak => lengths.long,
        }
    }

    /// Time gone in this phase by `now`.
    pub(crate) fn gone(&self, now: Instant) -> Duration {
        self.banked
            + self
                .since
                .map_or(Duration::ZERO, |since| now.duration_since(since))
    }

    /// The phase after this one, waiting: a break after focus, long after the last round, and focus after a break.
    pub(crate) fn next(self, lengths: &Lengths) -> Cycle {
        let (phase, round) = match self.phase {
            Phase::Focus if self.round >= lengths.rounds => (Phase::LongBreak, self.round),
            Phase::Focus => (Phase::ShortBreak, self.round),
            Phase::ShortBreak => (Phase::Focus, self.round + 1),
            Phase::LongBreak => (Phase::Focus, 1),
        };
        Cycle {
            phase,
            round,
            ..Cycle::START
        }
    }
}

/// A focus timer in pomodoros: focus for a spell, a short break, and after the last round a long one. A ring counts each phase down around the time left, dots count the rounds, and a phase that ends waits for Start.
#[derive(IntoElement)]
pub struct PomodoroTimer {
    id: ElementId,
    lengths: Lengths,
    on_phase: Option<OnPhase>,
}

impl PomodoroTimer {
    /// Twenty-five minutes of focus, breaks of five and fifteen, four rounds.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            lengths: Lengths {
                focus: Duration::from_secs(25 * 60),
                short: Duration::from_secs(5 * 60),
                long: Duration::from_secs(15 * 60),
                rounds: 4,
            },
            on_phase: None,
        }
    }

    /// Minutes of focus, of a short break and of a long one.
    pub fn minutes(mut self, focus: u64, short: u64, long: u64) -> Self {
        assert!(
            focus > 0 && short > 0 && long > 0,
            "each phase takes a minute or more"
        );
        self.lengths.focus = Duration::from_secs(focus * 60);
        self.lengths.short = Duration::from_secs(short * 60);
        self.lengths.long = Duration::from_secs(long * 60);
        self
    }

    /// Focus rounds before a long break.
    pub fn rounds(mut self, rounds: u8) -> Self {
        assert!(rounds > 0, "a pomodoro needs a round");
        self.lengths.rounds = rounds;
        self
    }

    /// Gets each phase as it comes, whether it ran out or was skipped.
    pub fn on_phase(mut self, handler: impl Fn(Phase, &mut Window, &mut App) + 'static) -> Self {
        self.on_phase = Some(Rc::new(handler));
        self
    }
}

/// Moves `cycle` to its next phase and tells the owner once the frame is drawn.
fn advance(
    cycle: &Entity<Cycle>,
    lengths: Lengths,
    on_phase: Option<OnPhase>,
    window: &mut Window,
    cx: &mut App,
) {
    let next = cycle.update(cx, |cycle, cx| {
        *cycle = cycle.next(&lengths);
        cx.notify();
        *cycle
    });
    log::info!(
        "pomodoro: {} begins, round {}",
        next.phase.words(),
        next.round
    );
    if let Some(on_phase) = on_phase {
        window.defer(cx, move |window, cx| on_phase(next.phase, window, cx));
    }
}

impl RenderOnce for PomodoroTimer {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let lengths = self.lengths;
        let cycle = window.use_keyed_state((self.id.clone(), "cycle"), cx, |_, _| Cycle::START);
        let now = cx.background_executor().now();
        if cycle.read(cx).since.is_some()
            && cycle.read(cx).gone(now) >= cycle.read(cx).length(&lengths)
        {
            advance(&cycle, lengths, self.on_phase.clone(), window, cx);
        }
        let current = *cycle.read(cx);
        if current.since.is_some() {
            window.use_keyed_state((self.id.clone(), "tick"), cx, |window, cx| -> Task<()> {
                cx.spawn_in(window, async move |ticker, cx| {
                    loop {
                        cx.background_executor().timer(Duration::from_secs(1)).await;
                        let ticked = cx.update(|_, cx| ticker.update(cx, |_, cx| cx.notify()));
                        if ticked.and_then(|inner| inner).is_err() {
                            return;
                        }
                    }
                })
            });
        }
        let length = current.length(&lengths);
        let gone = current.gone(now).min(length);
        let left = (length - gone).as_millis().div_ceil(1000) as u64;
        let theme = cx.theme();
        let colors = &theme.colors;
        let face = div()
            .flex()
            .flex_col()
            .items_center()
            .gap_0p5()
            .child(
                tabular(div())
                    .text_size(theme.text_size(TextSize::Xxl))
                    .child(format::duration(left, format::DurationStyle::Clock)),
            )
            .child(
                div()
                    .text_size(theme.text_size(TextSize::Sm))
                    .text_color(colors.fg_muted)
                    .child(current.phase.words()),
            );
        let share = gone.as_secs_f32() / length.as_secs_f32();
        let dots = (1..=lengths.rounds).map(|round| {
            let done =
                round < current.round || (round == current.round && current.phase != Phase::Focus);
            div().size(theme.status_dot()).rounded_full().bg(if done {
                colors.fg_muted
            } else {
                colors.border_strong
            })
        });
        let running = current.since.is_some();
        let toggle = {
            let cycle = cycle.clone();
            Button::new(
                (self.id.clone(), "toggle"),
                if running { "Pause" } else { "Start" },
            )
            .variant(ButtonVariant::Primary)
            .icon(if running {
                IconName::Pause
            } else {
                IconName::Play
            })
            .on_click(move |_, _, cx| {
                let now = cx.background_executor().now();
                cycle.update(cx, |cycle, cx| {
                    match cycle.since.take() {
                        Some(since) => cycle.banked += now.duration_since(since),
                        None => cycle.since = Some(now),
                    }
                    log::info!(
                        "pomodoro: {}",
                        if cycle.since.is_some() {
                            "running"
                        } else {
                            "paused"
                        }
                    );
                    cx.notify();
                })
            })
        };
        let skip = {
            let (cycle, on_phase) = (cycle.clone(), self.on_phase.clone());
            IconButton::new((self.id.clone(), "skip"), IconName::SkipForward)
                .variant(ButtonVariant::Ghost)
                .size(ControlSize::Sm)
                .tooltip("Skip to the next phase")
                .on_click(move |_, window, cx| {
                    advance(&cycle, lengths, on_phase.clone(), window, cx)
                })
        };
        let reset = IconButton::new((self.id.clone(), "reset"), IconName::RotateCcw)
            .variant(ButtonVariant::Ghost)
            .size(ControlSize::Sm)
            .tooltip("Start over")
            .on_click(move |_, _, cx| {
                log::info!("pomodoro: reset");
                cycle.update(cx, |cycle, cx| {
                    *cycle = Cycle::START;
                    cx.notify();
                })
            });
        div()
            .flex()
            .flex_col()
            .items_center()
            .gap_4()
            .child(
                ProgressRing::new((self.id.clone(), "ring"), share)
                    .size(theme.project().pomodoro)
                    .inside(face),
            )
            .child(div().flex().gap_1p5().children(dots))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(reset)
                    .child(toggle)
                    .child(skip),
            )
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use web_time::Instant;

    use super::{Cycle, Lengths, Phase};

    const LENGTHS: Lengths = Lengths {
        focus: Duration::from_secs(1500),
        short: Duration::from_secs(300),
        long: Duration::from_secs(900),
        rounds: 2,
    };

    #[test]
    fn focus_and_breaks_take_turns_and_the_last_round_rests_long() {
        let mut cycle = Cycle::START;
        let mut seen = Vec::new();
        for _ in 0..5 {
            cycle = cycle.next(&LENGTHS);
            seen.push((cycle.phase, cycle.round));
        }
        assert_eq!(
            seen,
            [
                (Phase::ShortBreak, 1),
                (Phase::Focus, 2),
                (Phase::LongBreak, 2),
                (Phase::Focus, 1),
                (Phase::ShortBreak, 1),
            ]
        );
    }

    #[test]
    fn time_gone_adds_what_was_banked_to_the_run_since_start() {
        let start = Instant::now();
        let cycle = Cycle {
            banked: Duration::from_secs(60),
            since: Some(start),
            ..Cycle::START
        };
        assert_eq!(
            cycle.gone(start + Duration::from_secs(30)),
            Duration::from_secs(90)
        );
        assert_eq!(
            Cycle::START.gone(start),
            Duration::ZERO,
            "a stopped cycle has none"
        );
    }
}
