use std::rc::Rc;

use gpui::{
    App, Div, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    StatefulInteractiveElement, Styled, Window, div, prelude::*,
};
use jiff::civil::Date;

use super::{
    marks::{IssueIdBadge, PriorityIndicator},
    tasks::clock_today,
    work::{Task, due_words},
};
use crate::{
    data_display::{Avatar, color_mark},
    forms::Run,
    mail::Label,
    primitives::{FocusRing, Icon},
    theme::{ActiveTheme, AvatarSize, IconSize, Radius, TextSize},
    typography::fresh,
};

/// A label worn on a task: its hue and its name, quiet.
pub(super) fn label_chip(label: &Label, cx: &App) -> Div {
    let theme = cx.theme();
    let hue = theme
        .colors
        .hue(label.hue, format_args!("label {}", label.name));
    div()
        .flex_none()
        .flex()
        .items_center()
        .gap_1()
        .text_size(theme.text_size(TextSize::Xs))
        .text_color(theme.colors.fg_muted)
        .child(color_mark(hue, cx))
        .child(label.name.clone())
}

/// An issue on a card: its status, name and who has it, its title, then its priority, labels and due day, red when past and still open. With `on_open`, a press, Enter or Space opens it.
#[derive(IntoElement)]
pub struct IssueCard {
    id: ElementId,
    task: Task,
    today: Option<Date>,
    on_open: Option<Run>,
}

impl IssueCard {
    pub fn new(id: impl Into<ElementId>, task: Task) -> Self {
        Self {
            id: id.into(),
            task,
            today: None,
            on_open: None,
        }
    }

    /// The day due dates count from; the clock's otherwise.
    pub fn today(mut self, day: Date) -> Self {
        self.today = Some(day);
        self
    }

    pub fn on_open(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_open = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for IssueCard {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let today = match self.today {
            Some(day) => day,
            None => {
                fresh((self.id.clone(), "clock"), window, cx);
                clock_today("issue card")
            }
        };
        let theme = cx.theme();
        let colors = &theme.colors;
        let task = self.task;
        let key = task.key.clone();
        let late = task.overdue(today);
        let due = task.due.map(|due| {
            div()
                .flex_none()
                .text_size(theme.text_size(TextSize::Xs))
                .text_color(if late { colors.danger } else { colors.fg_muted })
                .child(due_words(due, today))
        });
        let labels = task.labels.iter().map(|label| label_chip(label, cx));
        let top = div()
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
            .children(task.assignee.as_ref().map(|person| {
                Avatar::new((self.id.clone(), "assignee"), person.name.clone()).size(AvatarSize::Xs)
            }));
        let foot = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_x_3()
            .gap_y_1()
            .child(PriorityIndicator::new(
                (self.id.clone(), "priority"),
                task.priority,
            ))
            .children(labels)
            .children(due);
        div()
            .id(self.id)
            .debug_selector(move || format!("issue {key}"))
            .flex()
            .flex_col()
            .gap_2()
            .p_3()
            .rounded(theme.radius(Radius::Md))
            .border_1()
            .border_color(colors.border)
            .bg(colors.surface)
            .child(top)
            .child(
                div()
                    .text_size(theme.text_size(TextSize::Base))
                    .text_color(colors.fg)
                    .child(task.title),
            )
            .child(foot)
            .when_some(self.on_open, |card, open| {
                card.tab_index(0)
                    .focus_ring(cx)
                    .cursor_pointer()
                    .hover(|style| style.border_color(colors.border_strong))
                    .on_click(move |_, window, cx| open(window, cx))
            })
    }
}
