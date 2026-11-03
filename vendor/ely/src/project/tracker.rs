use std::{rc::Rc, time::Duration};

use gpui::{
    App, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce, SharedString,
    Styled, Task, Window, div,
};
use web_time::Instant;

use crate::{
    buttons::{Button, ButtonVariant},
    forms::{Enter, Input, Run, TextInput},
    primitives::{IconName, tab_stop},
    theme::{ActiveTheme, TextSize},
    typography::{Ellipsis, format, tabular},
};

type OnStart = Rc<dyn Fn(&SharedString, Instant, &mut Window, &mut App)>;
type OnStop = Rc<dyn Fn(Instant, &mut Window, &mut App)>;

/// Time spent on a task.
#[derive(Clone, Debug, PartialEq)]
pub struct TimeEntry {
    pub key: SharedString,
    pub task: SharedString,
    pub spent: Duration,
}

impl TimeEntry {
    pub fn new(
        key: impl Into<SharedString>,
        task: impl Into<SharedString>,
        spent: Duration,
    ) -> Self {
        Self {
            key: key.into(),
            task: task.into(),
            spent,
        }
    }
}

/// Time spent as a tracker counts it, in hours and whole minutes: "1h 35m", "40m".
pub(crate) fn spent(duration: Duration) -> String {
    let minutes = duration.as_secs() / 60;
    match minutes / 60 {
        0 => format!("{minutes}m"),
        hours => format!("{hours}h {}m", minutes % 60),
    }
}

/// A timer against a task: a field names it, Start or Enter's release starts it and Stop ends it, the running time counting up; under it, the entries made and their total. The owner keeps what runs and what was spent.
#[derive(IntoElement)]
pub struct TimeTracker {
    id: ElementId,
    entries: Vec<TimeEntry>,
    running: Option<(SharedString, Instant)>,
    on_start: Option<OnStart>,
    on_stop: Option<OnStop>,
}

impl TimeTracker {
    pub fn new(id: impl Into<ElementId>, entries: impl IntoIterator<Item = TimeEntry>) -> Self {
        Self {
            id: id.into(),
            entries: entries.into_iter().collect(),
            running: None,
            on_start: None,
            on_stop: None,
        }
    }

    /// The task running and when it started, by the executor's clock.
    pub fn running(mut self, task: impl Into<SharedString>, since: Instant) -> Self {
        self.running = Some((task.into(), since));
        self
    }

    /// Gets the task named and the moment it starts.
    pub fn on_start(
        mut self,
        handler: impl Fn(&SharedString, Instant, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_start = Some(Rc::new(handler));
        self
    }

    /// Gets the moment the running task stops.
    pub fn on_stop(mut self, handler: impl Fn(Instant, &mut Window, &mut App) + 'static) -> Self {
        self.on_stop = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for TimeTracker {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id.clone();
        let field = window.use_keyed_state((id.clone(), "task"), cx, |window, cx| {
            TextInput::new(window, cx).placeholder("What are you working on?")
        });
        if self.running.is_some() {
            window.use_keyed_state((id.clone(), "tick"), cx, |window, cx| -> Task<()> {
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
        let now = cx.background_executor().now();
        let elapsed = self
            .running
            .as_ref()
            .map_or(Duration::ZERO, |(_, since)| now.duration_since(*since));
        let action = tab_stop((id.clone(), "action").into(), true, window, cx);
        let start: Run = {
            let (id, field, on_start, action) =
                (id.clone(), field.clone(), self.on_start, action.clone());
            Rc::new(move |window, cx| {
                let task = SharedString::from(field.read(cx).text().trim().to_string());
                if task.is_empty() {
                    log::info!("time tracker {id:?}: nothing named to time");
                    return;
                }
                let now = cx.background_executor().now();
                log::info!("time tracker {id:?}: start {task}");
                field.update(cx, |input, cx| input.set_text(String::new(), cx));
                window.focus(&action, cx);
                if let Some(on_start) = &on_start {
                    on_start(&task, now, window, cx);
                }
            })
        };
        let stop: Run = {
            let (id, on_stop, typing) = (
                id.clone(),
                self.on_stop.clone(),
                field.read(cx).focus().clone(),
            );
            Rc::new(move |window, cx| {
                let now = cx.background_executor().now();
                log::info!("time tracker {id:?}: stop");
                window.focus(&typing, cx);
                if let Some(on_stop) = &on_stop {
                    on_stop(now, window, cx);
                }
            })
        };
        let running = self.running.is_some();
        let task = match &self.running {
            Some((task, _)) => div()
                .flex_1()
                .min_w_0()
                .child(Ellipsis::new(task.clone()))
                .into_any_element(),
            None => {
                let typed = start.clone();
                div()
                    .flex_1()
                    .min_w_0()
                    .capture_action(|_: &Enter, _, cx| cx.stop_propagation())
                    .on_key_up(move |event, window, cx| {
                        if event.keystroke.key == "enter" {
                            typed(window, cx)
                        }
                    })
                    .child(Input::new(&field))
                    .into_any_element()
            }
        };
        let action = Button::new(
            (id.clone(), "action"),
            if running { "Stop" } else { "Start" },
        )
        .focus_handle(&action)
        .variant(if running {
            ButtonVariant::default()
        } else {
            ButtonVariant::Primary
        })
        .icon(if running {
            IconName::Square
        } else {
            IconName::Play
        })
        .on_click(move |_, window, cx| match running {
            true => stop(window, cx),
            false => start(window, cx),
        });
        let theme = cx.theme();
        let colors = &theme.colors;
        let clock = tabular(div())
            .flex_none()
            .text_size(theme.text_size(TextSize::Lg))
            .child(format::duration(
                elapsed.as_secs(),
                format::DurationStyle::Clock,
            ));
        let quiet = |text: String| {
            tabular(div())
                .flex_none()
                .text_size(theme.text_size(TextSize::Sm))
                .text_color(colors.fg_muted)
                .child(text)
        };
        let total: Duration = self.entries.iter().map(|entry| entry.spent).sum();
        let rows = self.entries.iter().map(|entry| {
            div()
                .flex()
                .items_center()
                .gap_3()
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .child(Ellipsis::new(entry.task.clone())),
                )
                .child(quiet(spent(entry.spent)))
        });
        div()
            .debug_selector(|| "time-tracker".into())
            .flex()
            .flex_col()
            .gap_4()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(task)
                    .child(clock)
                    .child(action),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .children(rows.collect::<Vec<_>>()),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_3()
                    .pt_2()
                    .border_t_1()
                    .border_color(colors.border)
                    .child(quiet("Total".into()))
                    .child(quiet(spent(total))),
            )
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::spent;

    #[test]
    fn time_spent_reads_in_hours_and_whole_minutes() {
        let words =
            [95 * 60, 40 * 60 + 59, 59, 3 * 3600].map(|secs| spent(Duration::from_secs(secs)));
        assert_eq!(words, ["1h 35m", "40m", "0m", "3h 0m"]);
    }
}
