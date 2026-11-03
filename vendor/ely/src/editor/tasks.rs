use std::{rc::Rc, time::Duration};

use gpui::{
    App, ElementId, FontWeight, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    SharedString, StatefulInteractiveElement, Styled, Window, div, prelude::*,
};

use crate::{
    buttons::{ButtonVariant, IconButton},
    motion::Spinner,
    primitives::{Disclosure, Icon, IconName},
    theme::{ActiveTheme, ControlSize, IconSize, Radius, TextSize},
    typography::{Ellipsis, format},
};

/// Where a task's last run stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskState {
    Idle,
    Running,
    Passed(Duration),
    Failed(i32),
}

/// A task to run: its name, group, command, and where and with what it runs.
#[derive(Clone, Debug, PartialEq)]
pub struct Task {
    pub name: SharedString,
    pub group: SharedString,
    pub command: SharedString,
    pub cwd: SharedString,
    pub env: Vec<(SharedString, SharedString)>,
    pub state: TaskState,
}

type OnIndex = Rc<dyn Fn(usize, &mut Window, &mut App)>;
type OnOpen = Rc<dyn Fn(Option<usize>, &mut Window, &mut App)>;

/// Tasks by group, each with how its last run went and a run or stop button; one opens to show its configuration.
#[derive(IntoElement)]
pub struct TaskRunner {
    id: ElementId,
    tasks: Vec<Task>,
    open: Option<usize>,
    on_run: Option<OnIndex>,
    on_stop: Option<OnIndex>,
    on_open: Option<OnOpen>,
}

impl TaskRunner {
    pub fn new(id: impl Into<ElementId>, tasks: impl IntoIterator<Item = Task>) -> Self {
        Self {
            id: id.into(),
            tasks: tasks.into_iter().collect(),
            open: None,
            on_run: None,
            on_stop: None,
            on_open: None,
        }
    }

    /// The task whose run configuration shows.
    pub fn open(mut self, task: Option<usize>) -> Self {
        self.open = task;
        self
    }

    pub fn on_run(mut self, handler: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_run = Some(Rc::new(handler));
        self
    }

    pub fn on_stop(mut self, handler: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_stop = Some(Rc::new(handler));
        self
    }

    /// Gets the task to open, or none to close it.
    pub fn on_open(
        mut self,
        handler: impl Fn(Option<usize>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_open = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for TaskRunner {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let mut groups: Vec<SharedString> = Vec::new();
        for task in &self.tasks {
            if !groups.contains(&task.group) {
                groups.push(task.group.clone());
            }
        }
        let mut rows = Vec::new();
        for group in groups {
            rows.push(
                div()
                    .px_2()
                    .pt_2()
                    .text_size(theme.text_size(TextSize::Xs))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(colors.fg_subtle)
                    .child(group.to_uppercase())
                    .into_any_element(),
            );
            for (ix, task) in self
                .tasks
                .iter()
                .enumerate()
                .filter(|(_, task)| task.group == group)
            {
                let running = task.state == TaskState::Running;
                let status = match task.state {
                    TaskState::Idle => Icon::new(IconName::Circle)
                        .size(IconSize::Sm)
                        .color(colors.fg_subtle)
                        .into_any_element(),
                    TaskState::Running => Spinner::new((self.id.clone(), format!("spinner-{ix}")))
                        .size(IconSize::Sm)
                        .into_any_element(),
                    TaskState::Passed(_) => Icon::new(IconName::CircleCheck)
                        .size(IconSize::Sm)
                        .color(colors.success)
                        .into_any_element(),
                    TaskState::Failed(_) => Icon::new(IconName::CircleX)
                        .size(IconSize::Sm)
                        .color(colors.danger)
                        .into_any_element(),
                };
                let outcome = match task.state {
                    TaskState::Idle => SharedString::default(),
                    TaskState::Running => "Running".into(),
                    TaskState::Passed(took) => {
                        format::duration(took.as_secs().max(1), format::DurationStyle::Compact)
                            .into()
                    }
                    TaskState::Failed(code) => format!("Exit {code}").into(),
                };
                let (action, icon, words) = if running {
                    (self.on_stop.clone(), IconName::CircleStop, "Stop")
                } else {
                    (self.on_run.clone(), IconName::Play, "Run")
                };
                let opened = self.open == Some(ix);
                let toggle = self.on_open.clone();
                rows.push(
                    div()
                        .id((self.id.clone(), format!("task-{ix}")))
                        .flex()
                        .items_center()
                        .gap_2()
                        .px_2()
                        .py_1()
                        .rounded(theme.radius(Radius::Md))
                        .hover(|row| row.bg(colors.hover))
                        .cursor_pointer()
                        .when_some(toggle, |row, toggle| {
                            row.on_click(move |_, window, cx| {
                                toggle((!opened).then_some(ix), window, cx)
                            })
                        })
                        .child(Disclosure::new(
                            (self.id.clone(), format!("open-{ix}")),
                            opened,
                        ))
                        .child(status)
                        .child(
                            div()
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(colors.fg)
                                .child(task.name.clone()),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .font_family(theme.mono_family.clone())
                                .text_size(theme.text_size(TextSize::Xs))
                                .text_color(colors.fg_subtle)
                                .child(Ellipsis::new(task.command.clone())),
                        )
                        .child(
                            div()
                                .text_size(theme.text_size(TextSize::Xs))
                                .text_color(if matches!(task.state, TaskState::Failed(_)) {
                                    colors.danger
                                } else {
                                    colors.fg_muted
                                })
                                .child(outcome),
                        )
                        .child(
                            IconButton::new((self.id.clone(), format!("run-{ix}")), icon)
                                .variant(ButtonVariant::Ghost)
                                .size(ControlSize::Sm)
                                .tooltip(words)
                                .when_some(action, |button, action| {
                                    button.on_click(move |_, window, cx| {
                                        log::info!("task runner: {words} {ix}");
                                        action(ix, window, cx)
                                    })
                                }),
                        )
                        .into_any_element(),
                );
                if opened {
                    let field = |label: &'static str, value: SharedString| {
                        div()
                            .flex()
                            .gap_3()
                            .child(
                                div()
                                    .w(theme.label_width() * 0.75)
                                    .text_color(colors.fg_subtle)
                                    .child(label),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .font_family(theme.mono_family.clone())
                                    .text_color(colors.fg)
                                    .child(value),
                            )
                    };
                    let env = task
                        .env
                        .iter()
                        .map(|(key, value)| format!("{key}={value}"))
                        .collect::<Vec<_>>()
                        .join("  ");
                    rows.push(
                        div()
                            .ml_8()
                            .mb_1()
                            .p_3()
                            .flex()
                            .flex_col()
                            .gap_1p5()
                            .rounded(theme.radius(Radius::Md))
                            .bg(colors.sunken)
                            .text_size(theme.text_size(TextSize::Xs))
                            .child(field("Command", task.command.clone()))
                            .child(field("Directory", task.cwd.clone()))
                            .child(field(
                                "Environment",
                                if env.is_empty() {
                                    "—".into()
                                } else {
                                    env.into()
                                },
                            ))
                            .into_any_element(),
                    );
                }
            }
        }
        div()
            .flex()
            .flex_col()
            .gap_0p5()
            .text_size(theme.text_size(TextSize::Sm))
            .children(rows)
    }
}
