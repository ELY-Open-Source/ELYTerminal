use std::rc::Rc;

use gpui::{
    AnyElement, App, ElementId, FontWeight, IntoElement, ParentElement, RenderOnce, SharedString,
    Styled, Window, div, prelude::*,
};

use super::{
    card::label_chip,
    marks::{AssigneePicker, IssueIdBadge, StatusSelect},
    work::{Person, Priority, Task},
};
use crate::{
    buttons::{ButtonVariant, IconButton},
    documents::Checklist,
    forms::{Choice, DatePicker, InlineEdit, OnValue, Run, Select},
    primitives::{Icon, IconName},
    theme::{ActiveTheme, ControlSize, IconSize, TextSize},
};

type OnTask = Rc<dyn Fn(&Task, &mut Window, &mut App)>;
type OnStep = Rc<dyn Fn(usize, bool, &mut Window, &mut App)>;
type Edit = Rc<dyn Fn(&dyn Fn(&mut Task), &mut Window, &mut App)>;

/// A task laid out in full: its status and issue with a way to close, its title to edit in place, its status, priority, assignee, due day and labels, its notes, and its steps. Each edit hands the whole task on.
#[derive(IntoElement)]
pub struct TaskDetailPanel {
    id: ElementId,
    task: Task,
    people: Vec<Person>,
    notes: Option<SharedString>,
    steps: Vec<(SharedString, bool)>,
    on_change: Option<OnTask>,
    on_close: Option<Run>,
    on_step: Option<OnStep>,
    on_add_step: Option<OnValue>,
}

impl TaskDetailPanel {
    /// `people` are those it can go to.
    pub fn new(
        id: impl Into<ElementId>,
        task: Task,
        people: impl IntoIterator<Item = Person>,
    ) -> Self {
        Self {
            id: id.into(),
            task,
            people: people.into_iter().collect(),
            notes: None,
            steps: Vec::new(),
            on_change: None,
            on_close: None,
            on_step: None,
            on_add_step: None,
        }
    }

    pub fn notes(mut self, notes: impl Into<SharedString>) -> Self {
        self.notes = Some(notes.into());
        self
    }

    /// Its steps, each done or not.
    pub fn steps(
        mut self,
        steps: impl IntoIterator<Item = (impl Into<SharedString>, bool)>,
    ) -> Self {
        self.steps = steps
            .into_iter()
            .map(|(step, done)| (step.into(), done))
            .collect();
        self
    }

    /// Gets the task after each edit.
    pub fn on_change(mut self, handler: impl Fn(&Task, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }

    pub fn on_close(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_close = Some(Rc::new(handler));
        self
    }

    /// Gets a step's place and whether it is now done.
    pub fn on_step(
        mut self,
        handler: impl Fn(usize, bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_step = Some(Rc::new(handler));
        self
    }

    /// A field under the steps; gets a new step's words.
    pub fn on_add_step(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_add_step = Some(Rc::new(handler));
        self
    }
}

/// A property's name over a fixed column and its control, which drops below when narrow.
fn property(name: &'static str, control: AnyElement, cx: &App) -> impl IntoElement {
    let theme = cx.theme();
    div()
        .flex()
        .flex_wrap()
        .items_center()
        .gap_x_3()
        .gap_y_1()
        .child(
            div()
                .w(theme.label_width())
                .flex_none()
                .text_size(theme.text_size(TextSize::Sm))
                .text_color(theme.colors.fg_muted)
                .child(name),
        )
        .child(div().flex_1().min_w(theme.label_width()).child(control))
}

impl RenderOnce for TaskDetailPanel {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id.clone();
        let task = self.task;
        let edit: Edit = {
            let (task, on_change, id) = (task.clone(), self.on_change, id.clone());
            Rc::new(move |apply, window, cx| {
                let mut next = task.clone();
                apply(&mut next);
                log::info!("task panel {id:?}: {} changed", next.key);
                if let Some(on_change) = &on_change {
                    on_change(&next, window, cx);
                }
            })
        };
        let theme = cx.theme();
        let colors = &theme.colors;
        let head = div()
            .flex()
            .items_center()
            .justify_between()
            .gap_2()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        Icon::new(task.status.icon())
                            .size(IconSize::Sm)
                            .color(colors.fg_muted),
                    )
                    .children(task.issue.clone().map(IssueIdBadge::new)),
            )
            .children(self.on_close.map(|close| {
                IconButton::new((id.clone(), "close"), IconName::X)
                    .variant(ButtonVariant::Ghost)
                    .size(ControlSize::Sm)
                    .tooltip("Close")
                    .on_click(move |_, window, cx| close(window, cx))
            }));
        let title = {
            let edit = edit.clone();
            InlineEdit::new((id.clone(), "title"), task.title.clone()).on_commit(
                move |title, window, cx| {
                    let title = SharedString::from(title.trim().to_string());
                    if title.is_empty() {
                        log::info!("task panel: a blank title is not taken");
                        return;
                    }
                    edit(&move |task| task.title = title.clone(), window, cx)
                },
            )
        };
        let status = {
            let edit = edit.clone();
            StatusSelect::new((id.clone(), "status"), task.status)
                .size(ControlSize::Sm)
                .on_change(move |status, window, cx| {
                    edit(&move |task| task.status = status, window, cx)
                })
        };
        let priority = {
            let edit = edit.clone();
            let choices =
                Priority::ALL.map(|priority| Choice::new(priority.key(), priority.words()));
            Select::new((id.clone(), "priority"), choices)
                .selected(task.priority.key())
                .size(ControlSize::Sm)
                .on_change(move |key: &SharedString, window, cx| {
                    let priority = Priority::of_key(key);
                    edit(&move |task| task.priority = priority, window, cx)
                })
        };
        let assignee = {
            let edit = edit.clone();
            let picker = AssigneePicker::new((id.clone(), "assignee"), self.people)
                .size(ControlSize::Sm)
                .on_change(move |person, window, cx| {
                    let person = person.cloned();
                    edit(&move |task| task.assignee = person.clone(), window, cx)
                });
            match &task.assignee {
                Some(person) => picker.assignee(person.key.clone()),
                None => picker,
            }
        };
        let due = {
            let edit = edit.clone();
            DatePicker::new((id.clone(), "due"), task.due)
                .size(ControlSize::Sm)
                .on_change(move |day, window, cx| {
                    edit(&move |task| task.due = Some(day), window, cx)
                })
        };
        let labels = match task.labels.is_empty() {
            true => div()
                .text_size(theme.text_size(TextSize::Sm))
                .text_color(colors.fg_subtle)
                .child("None"),
            false => div()
                .flex()
                .flex_wrap()
                .gap_x_3()
                .gap_y_1()
                .children(task.labels.iter().map(|label| label_chip(label, cx))),
        };
        let steps = (!self.steps.is_empty() || self.on_add_step.is_some()).then(|| {
            let steps = Checklist::new((id.clone(), "steps"), "Subtasks", self.steps);
            let steps = match self.on_step {
                Some(on_step) => {
                    steps.on_toggle(move |ix, done, window, cx| on_step(ix, done, window, cx))
                }
                None => steps,
            };
            match self.on_add_step {
                Some(on_add) => steps.on_add("Add a subtask", move |words, window, cx| {
                    on_add(words, window, cx)
                }),
                None => steps,
            }
        });
        div()
            .debug_selector(|| "task-panel".into())
            .flex()
            .flex_col()
            .gap_5()
            .child(head)
            .child(
                div()
                    .text_size(theme.text_size(TextSize::Lg))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(title),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(property("Status", status.into_any_element(), cx))
                    .child(property("Priority", priority.into_any_element(), cx))
                    .child(property("Assignee", assignee.into_any_element(), cx))
                    .child(property("Due", due.into_any_element(), cx))
                    .child(property("Labels", labels.into_any_element(), cx)),
            )
            .children(self.notes.map(|notes| {
                div()
                    .text_size(theme.text_size(TextSize::Sm))
                    .text_color(colors.fg)
                    .child(notes)
            }))
            .children(steps)
    }
}
