use std::rc::Rc;

use gpui::{
    App, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce, SharedString,
    StatefulInteractiveElement, Styled, Window, div, prelude::*, relative,
};

use super::work::{IssueId, Person, Priority, Status};
use crate::{
    forms::{Choice, Select},
    primitives::{Icon, IconName, Tooltip},
    theme::{ActiveTheme, ControlSize, IconSize, TextSize},
    typography::tabular,
};

type OnStatus = Rc<dyn Fn(Status, &mut Window, &mut App)>;
type OnAssignee = Rc<dyn Fn(Option<&Person>, &mut Window, &mut App)>;

/// A priority at a glance: three bars, as many lit as it is high, or a red mark for urgent. Its words show in a tip, or beside it when labeled.
#[derive(IntoElement)]
pub struct PriorityIndicator {
    id: ElementId,
    priority: Priority,
    labeled: bool,
}

impl PriorityIndicator {
    pub fn new(id: impl Into<ElementId>, priority: Priority) -> Self {
        Self {
            id: id.into(),
            priority,
            labeled: false,
        }
    }

    /// Its words beside it.
    pub fn labeled(mut self) -> Self {
        self.labeled = true;
        self
    }
}

impl RenderOnce for PriorityIndicator {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = &theme.colors;
        let mark = match self.priority {
            Priority::Urgent => Icon::new(IconName::CircleAlert)
                .size(IconSize::Sm)
                .color(colors.danger)
                .into_any_element(),
            priority => div()
                .size(theme.icon_size(IconSize::Sm))
                .py_0p5()
                .flex()
                .items_end()
                .justify_center()
                .gap_0p5()
                .children([0.5, 0.75, 1.0].into_iter().enumerate().map(|(ix, tall)| {
                    let lit = ix < priority.bars();
                    div().w_0p5().h(relative(tall)).rounded_full().bg(if lit {
                        colors.fg_muted
                    } else {
                        colors.border_strong
                    })
                }))
                .into_any_element(),
        };
        let words = self.priority.words();
        div()
            .id(self.id)
            .flex_none()
            .flex()
            .items_center()
            .gap_1p5()
            .child(mark)
            .map(|row| match self.labeled {
                true => row.child(
                    div()
                        .text_size(theme.text_size(TextSize::Sm))
                        .text_color(colors.fg_muted)
                        .child(words),
                ),
                false => row.tooltip(Tooltip::text(words)),
            })
    }
}

/// An issue's name, as ENG-123, quiet and in even figures.
#[derive(IntoElement)]
pub struct IssueIdBadge {
    issue: IssueId,
}

impl IssueIdBadge {
    pub fn new(issue: IssueId) -> Self {
        Self { issue }
    }
}

impl RenderOnce for IssueIdBadge {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        tabular(div())
            .flex_none()
            .text_size(theme.text_size(TextSize::Sm))
            .text_color(theme.colors.fg_muted)
            .child(self.issue.to_string())
    }
}

/// Where a task stands, picked from its workflow, each status with its mark.
#[derive(IntoElement)]
pub struct StatusSelect {
    id: ElementId,
    status: Status,
    size: ControlSize,
    on_change: Option<OnStatus>,
}

impl StatusSelect {
    pub fn new(id: impl Into<ElementId>, status: Status) -> Self {
        Self {
            id: id.into(),
            status,
            size: ControlSize::default(),
            on_change: None,
        }
    }

    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    pub fn on_change(mut self, handler: impl Fn(Status, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for StatusSelect {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let choices =
            Status::ALL.map(|status| Choice::new(status.key(), status.words()).icon(status.icon()));
        let (id, on_change) = (self.id.clone(), self.on_change);
        Select::new(self.id, choices)
            .selected(self.status.key())
            .size(self.size)
            .on_change(move |key: &SharedString, window, cx| {
                let status = Status::of_key(key);
                log::info!("status select {id:?}: {}", status.words());
                if let Some(on_change) = &on_change {
                    on_change(status, window, cx);
                }
            })
    }
}

/// Who has a task, picked from people by name, or no one.
#[derive(IntoElement)]
pub struct AssigneePicker {
    id: ElementId,
    people: Vec<Person>,
    assignee: Option<SharedString>,
    size: ControlSize,
    on_change: Option<OnAssignee>,
}

impl AssigneePicker {
    pub fn new(id: impl Into<ElementId>, people: impl IntoIterator<Item = Person>) -> Self {
        let people: Vec<Person> = people.into_iter().collect();
        for (ix, person) in people.iter().enumerate() {
            let twice = people[..ix].iter().any(|other| other.key == person.key);
            assert!(!twice, "person {} twice", person.key);
        }
        Self {
            id: id.into(),
            people,
            assignee: None,
            size: ControlSize::default(),
            on_change: None,
        }
    }

    /// Who has it now, by key.
    pub fn assignee(mut self, key: impl Into<SharedString>) -> Self {
        self.assignee = Some(key.into());
        self
    }

    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    /// Gets the person picked, or none for no one.
    pub fn on_change(
        mut self,
        handler: impl Fn(Option<&Person>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for AssigneePicker {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        if let Some(key) = &self.assignee {
            let known = self.people.iter().any(|person| person.key == *key);
            if !known {
                log::error!(
                    "assignee picker {:?}: no person {key}; nobody shown",
                    self.id
                );
            }
        }
        let nobody = Choice::new("", "No assignee").icon(IconName::User);
        let choices: Vec<Choice> = std::iter::once(nobody)
            .chain(self.people.iter().map(|person| {
                Choice::new(person.key.clone(), person.name.clone()).avatar(person.name.clone())
            }))
            .collect();
        let (id, people, on_change) = (self.id.clone(), self.people, self.on_change);
        Select::new(self.id, choices)
            .selected(self.assignee.unwrap_or_default())
            .size(self.size)
            .on_change(move |key: &SharedString, window, cx| {
                let person = people.iter().find(|person| person.key == *key);
                log::info!(
                    "assignee picker {id:?}: {}",
                    person.map_or("no one", |person| person.name.as_ref())
                );
                if let Some(on_change) = &on_change {
                    on_change(person, window, cx);
                }
            })
    }
}
