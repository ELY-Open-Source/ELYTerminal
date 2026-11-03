use std::rc::Rc;

use gpui::{
    AnyElement, App, ElementId, InteractiveElement, IntoElement, MouseButton, ParentElement,
    RenderOnce, SharedString, Styled, Window, div,
};
use jiff::{Timestamp, civil::Date};

use super::{
    marks::{IssueIdBadge, PriorityIndicator},
    work::{Status, Task, due_words},
};
use crate::{
    data_display::Avatar,
    forms::{Checkbox, OnFlag, OnValue},
    lists::{ListItem, SelectableList},
    primitives::{Icon, IconName},
    theme::{ActiveTheme, AvatarSize, IconSize, TextSize},
    typography::{format, fresh},
};

type OnTaskToggle = Rc<dyn Fn(&SharedString, bool, &mut Window, &mut App)>;

/// Today by the clock, in the system's zone.
pub(super) fn clock_today(user: &str) -> Date {
    Timestamp::now().to_zoned(format::system_zone(user)).date()
}

/// A task in a row: its box, its priority and its issue's name, its title, then when it is due and who has it. Closed, the title goes quiet and struck through; past due and still open, the day reads in red.
#[derive(IntoElement)]
pub struct TaskItem {
    id: ElementId,
    task: Task,
    today: Option<Date>,
    on_toggle: Option<OnFlag>,
}

impl TaskItem {
    pub fn new(id: impl Into<ElementId>, task: Task) -> Self {
        Self {
            id: id.into(),
            task,
            today: None,
            on_toggle: None,
        }
    }

    /// The day due dates count from; the clock's otherwise.
    pub fn today(mut self, day: Date) -> Self {
        self.today = Some(day);
        self
    }

    /// A box that ticks the task done; it gets whether the box is now ticked. Without it, the status mark leads.
    pub fn on_toggle(mut self, handler: impl Fn(bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_toggle = Some(Rc::new(handler));
        self
    }

    fn item(self, today: Date, cx: &App) -> ListItem {
        let theme = cx.theme();
        let colors = &theme.colors;
        let task = self.task;
        let lead: AnyElement = match self.on_toggle {
            Some(on_toggle) => {
                let (key, named) = (task.key.clone(), task.key.clone());
                div()
                    .debug_selector(move || format!("task-box {named}"))
                    .flex_none()
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .on_key_down(|event, _, cx| {
                        let key = event.keystroke.key.as_str();
                        if matches!(key, "space" | "enter") && !event.keystroke.modifiers.modified()
                        {
                            cx.stop_propagation();
                        }
                    })
                    .child(
                        Checkbox::new((self.id.clone(), "done"), task.status == Status::Done)
                            .on_change(move |done, window, cx| {
                                log::info!("task {key}: {}", if done { "done" } else { "open" });
                                on_toggle(done, window, cx)
                            }),
                    )
                    .into_any_element()
            }
            None => Icon::new(task.status.icon())
                .size(IconSize::Sm)
                .color(colors.fg_muted)
                .into_any_element(),
        };
        let due = task.due.map(|due| {
            let late = task.overdue(today);
            div()
                .text_size(theme.text_size(TextSize::Sm))
                .text_color(if late { colors.danger } else { colors.fg_muted })
                .child(due_words(due, today))
        });
        let assignee = match task.assignee {
            Some(person) => Avatar::new((self.id.clone(), "assignee"), person.name)
                .size(AvatarSize::Xs)
                .into_any_element(),
            None => div()
                .size(theme.avatar_size(AvatarSize::Xs))
                .flex()
                .items_center()
                .justify_center()
                .child(
                    Icon::new(IconName::User)
                        .size(IconSize::Xs)
                        .color(colors.fg_subtle),
                )
                .into_any_element(),
        };
        let lead = div()
            .flex()
            .items_center()
            .gap_2()
            .child(lead)
            .child(PriorityIndicator::new(
                (self.id.clone(), "priority"),
                task.priority,
            ))
            .children(task.issue.map(IssueIdBadge::new));
        let trail = div()
            .flex_none()
            .flex()
            .items_center()
            .gap_2()
            .children(due)
            .child(assignee);
        let closed = task.status.closed();
        ListItem::new(self.id, task.title)
            .leading(lead)
            .struck(closed)
            .quiet(closed)
            .trailing(trail)
    }
}

impl RenderOnce for TaskItem {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let today = match self.today {
            Some(day) => day,
            None => {
                fresh((self.id.clone(), "clock"), window, cx);
                clock_today("task item")
            }
        };
        self.item(today, cx)
    }
}

/// Tasks in rows: arrows move between them, Enter or a double press opens one, and each box ticks its task done.
#[derive(IntoElement)]
pub struct TaskList {
    id: ElementId,
    tasks: Vec<Task>,
    today: Option<Date>,
    on_open: Option<OnValue>,
    on_toggle: Option<OnTaskToggle>,
}

impl TaskList {
    pub fn new(id: impl Into<ElementId>, tasks: impl IntoIterator<Item = Task>) -> Self {
        let tasks: Vec<Task> = tasks.into_iter().collect();
        for (ix, task) in tasks.iter().enumerate() {
            let twice = tasks[..ix].iter().any(|other| other.key == task.key);
            assert!(!twice, "task {} twice", task.key);
        }
        Self {
            id: id.into(),
            tasks,
            today: None,
            on_open: None,
            on_toggle: None,
        }
    }

    /// The day due dates count from; the clock's otherwise.
    pub fn today(mut self, day: Date) -> Self {
        self.today = Some(day);
        self
    }

    /// Gets the key of the task opened.
    pub fn on_open(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_open = Some(Rc::new(handler));
        self
    }

    /// Gets a task's key and whether its box is now ticked.
    pub fn on_toggle(
        mut self,
        handler: impl Fn(&SharedString, bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_toggle = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for TaskList {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        if self.tasks.is_empty() {
            return div()
                .debug_selector(|| "task-none".into())
                .px_3()
                .py_4()
                .text_size(theme.text_size(TextSize::Sm))
                .text_color(theme.colors.fg_muted)
                .child("No tasks.")
                .into_any_element();
        }
        let today = match self.today {
            Some(day) => day,
            None => {
                fresh((self.id.clone(), "clock"), window, cx);
                clock_today("task list")
            }
        };
        let list = self.tasks.into_iter().fold(
            SelectableList::new((self.id.clone(), "tasks")),
            |list, task| {
                let key = task.key.clone();
                let item = TaskItem::new((self.id.clone(), format!("task-{key}")), task);
                let item = match self.on_toggle.clone() {
                    Some(toggle) => {
                        let ticked = key.clone();
                        item.on_toggle(move |done, window, cx| toggle(&ticked, done, window, cx))
                    }
                    None => item,
                };
                list.row(key, item.item(today, cx))
            },
        );
        let (id, on_open) = (self.id.clone(), self.on_open);
        div()
            .debug_selector(|| "task-list".into())
            .child(list.on_activate(move |key, window, cx| {
                log::info!("task list {id:?}: open {key}");
                if let Some(on_open) = &on_open {
                    on_open(key, window, cx);
                }
            }))
            .into_any_element()
    }
}
