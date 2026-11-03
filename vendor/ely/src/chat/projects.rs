use std::rc::Rc;

use gpui::{
    AnyElement, App, ElementId, FontWeight, InteractiveElement, IntoElement, MouseButton,
    ParentElement, RenderOnce, SharedString, StatefulInteractiveElement, Styled, Window, div,
    prelude::*, transparent_black,
};
use jiff::Timestamp;

use crate::{
    buttons::{Button, ButtonVariant, IconButton},
    data_display::Meter,
    forms::{CheckState, Checkbox, Choice, OnFlag, OnValue, Pick, RadioGroup, Run},
    overlays::Dialog,
    primitives::{FocusRing, Icon, IconName, file_icon},
    theme::{ActiveTheme, ControlSize, IconSize, Radius, TextSize},
    typography::{Ellipsis, RelativeTime, format::file_size, tabular},
};

type OnKey = OnValue;

/// A project that holds conversations: its key, name, and how many it holds.
#[derive(Clone, Debug, PartialEq)]
pub struct Project {
    pub key: SharedString,
    pub name: SharedString,
    pub count: usize,
}

/// Projects in a column, each with how many conversations it holds, the open one marked; a press opens one.
#[derive(IntoElement)]
pub struct ProjectList {
    id: ElementId,
    projects: Vec<Project>,
    active: Option<SharedString>,
    on_select: Option<OnKey>,
}

impl ProjectList {
    pub fn new(id: impl Into<ElementId>, projects: impl IntoIterator<Item = Project>) -> Self {
        Self {
            id: id.into(),
            projects: projects.into_iter().collect(),
            active: None,
            on_select: None,
        }
    }

    pub fn active(
        mut self,
        active: Option<SharedString>,
        on_select: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.active = active;
        self.on_select = Some(Rc::new(on_select));
        self
    }
}

impl RenderOnce for ProjectList {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        div()
            .flex()
            .flex_col()
            .gap_0p5()
            .children(self.projects.into_iter().map(|project| {
                let active = self.active.as_ref() == Some(&project.key);
                let select = self.on_select.clone();
                let key = project.key.clone();
                div()
                    .id((self.id.clone(), format!("project-{key}")))
                    .flex()
                    .items_center()
                    .gap_2()
                    .h(theme.control_height(ControlSize::Md))
                    .px_2()
                    .rounded(theme.radius(Radius::Md))
                    .border_1()
                    .border_color(transparent_black())
                    .text_size(theme.text_size(TextSize::Sm))
                    .when(active, |row| row.bg(colors.active))
                    .when_some(select, |row, select| {
                        row.tab_index(0)
                            .focus_ring(cx)
                            .cursor_pointer()
                            .when(!active, |row| row.hover(|row| row.bg(colors.hover)))
                            .on_mouse_down(MouseButton::Left, |_, window, _| {
                                window.prevent_default()
                            })
                            .on_click(move |_, window, cx| {
                                log::info!("projects: open {key}");
                                select(&key, window, cx)
                            })
                    })
                    .child(
                        Icon::new(IconName::Folder)
                            .size(IconSize::Sm)
                            .color(colors.fg_muted),
                    )
                    .child(div().flex_1().min_w_0().child(Ellipsis::new(project.name)))
                    .child(
                        tabular(
                            div()
                                .flex_none()
                                .text_size(theme.text_size(TextSize::Xs))
                                .text_color(colors.fg_subtle),
                        )
                        .child(project.count.to_string()),
                    )
            }))
    }
}

/// What a project knows beyond the conversation: its files, how full that knowledge is, a way to add more and to take one off.
#[derive(IntoElement)]
pub struct ProjectKnowledgePanel {
    id: ElementId,
    files: Vec<(SharedString, u64)>,
    capacity: u64,
    on_add: Option<Run>,
    on_remove: Option<Pick>,
}

impl ProjectKnowledgePanel {
    /// `files` by name and size; `capacity` is how much the project holds, in bytes.
    pub fn new(
        id: impl Into<ElementId>,
        files: impl IntoIterator<Item = (impl Into<SharedString>, u64)>,
        capacity: u64,
    ) -> Self {
        assert!(capacity > 0, "a project holds something");
        Self {
            id: id.into(),
            files: files
                .into_iter()
                .map(|(name, bytes)| (name.into(), bytes))
                .collect(),
            capacity,
            on_add: None,
            on_remove: None,
        }
    }

    pub fn on_add(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_add = Some(Rc::new(handler));
        self
    }

    /// Gets the index of the file to take off.
    pub fn on_remove(mut self, handler: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_remove = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for ProjectKnowledgePanel {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let used: u64 = self.files.iter().map(|(_, bytes)| bytes).sum();
        let share = used as f32 / self.capacity as f32;
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_size(theme.text_size(TextSize::Sm))
                            .font_weight(FontWeight::MEDIUM)
                            .child("Project knowledge"),
                    )
                    .children(self.on_add.map(|add| {
                        Button::new((self.id.clone(), "add"), "Add files")
                            .variant(ButtonVariant::Ghost)
                            .size(ControlSize::Sm)
                            .icon(IconName::Plus)
                            .on_click(move |_, window, cx| add(window, cx))
                    })),
            )
            .child(
                Meter::new((self.id.clone(), "used"), "Capacity", share.min(1.0)).detail(format!(
                    "{} of {}",
                    file_size(used, false),
                    file_size(self.capacity, false)
                )),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_0p5()
                    .children(
                        self.files
                            .into_iter()
                            .enumerate()
                            .map(|(ix, (name, bytes))| {
                                let remove = self.on_remove.clone();
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .py_1()
                                    .text_size(theme.text_size(TextSize::Sm))
                                    .child(
                                        Icon::new(file_icon(&name, cx))
                                            .size(IconSize::Sm)
                                            .color(colors.fg_muted),
                                    )
                                    .child(div().flex_1().min_w_0().child(Ellipsis::new(name)))
                                    .child(
                                        tabular(
                                            div()
                                                .flex_none()
                                                .text_size(theme.text_size(TextSize::Xs))
                                                .text_color(colors.fg_subtle),
                                        )
                                        .child(file_size(bytes, false)),
                                    )
                                    .children(remove.map(|remove| {
                                        IconButton::new(
                                            (self.id.clone(), format!("remove-{ix}")),
                                            IconName::X,
                                        )
                                        .variant(ButtonVariant::Ghost)
                                        .size(ControlSize::Sm)
                                        .tooltip("Remove")
                                        .on_click(
                                            move |_, window, cx| {
                                                log::info!("project knowledge: remove {ix}");
                                                remove(ix, window, cx)
                                            },
                                        )
                                    }))
                            }),
                    ),
            )
    }
}

/// A conversation someone shared, to read: who shared it and when, its messages as they stood, and a way to carry it on.
#[derive(IntoElement)]
pub struct SharedConversationView {
    id: ElementId,
    sharer: SharedString,
    at: Timestamp,
    messages: AnyElement,
    on_continue: Option<Run>,
}

impl SharedConversationView {
    /// `messages` show the conversation, such as a `MessageList`.
    pub fn new(
        id: impl Into<ElementId>,
        sharer: impl Into<SharedString>,
        at: Timestamp,
        messages: impl IntoElement,
    ) -> Self {
        Self {
            id: id.into(),
            sharer: sharer.into(),
            at,
            messages: messages.into_any_element(),
            on_continue: None,
        }
    }

    pub fn on_continue(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_continue = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for SharedConversationView {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        div()
            .size_full()
            .flex()
            .flex_col()
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap_2()
                    .px_4()
                    .py_2()
                    .border_b_1()
                    .border_color(colors.border)
                    .bg(colors.surface)
                    .text_size(theme.text_size(TextSize::Sm))
                    .text_color(colors.fg_muted)
                    .child(
                        Icon::new(IconName::Link)
                            .size(IconSize::Sm)
                            .color(colors.fg_subtle),
                    )
                    .child(format!("Shared by {}", self.sharer))
                    .child("·")
                    .child(RelativeTime::new((self.id.clone(), "when"), self.at))
                    .child(div().flex_1())
                    .children(self.on_continue.map(|carry| {
                        Button::new((self.id.clone(), "continue"), "Continue this conversation")
                            .variant(ButtonVariant::Primary)
                            .size(ControlSize::Sm)
                            .on_click(move |_, window, cx| carry(window, cx))
                    })),
            )
            .child(div().flex_1().min_h_0().child(self.messages))
    }
}

const FORMATS: [(&str, &str); 3] = [("markdown", "Markdown"), ("pdf", "PDF"), ("json", "JSON")];

/// Saves a conversation to a file: the format, whether its thinking goes with it, then export or cancel. Render it while open.
#[derive(IntoElement)]
pub struct ConversationExport {
    id: ElementId,
    format: SharedString,
    on_format: OnValue,
    thinking: Option<(bool, OnFlag)>,
    on_export: Run,
    on_close: Run,
}

impl ConversationExport {
    /// `format` is markdown, pdf or json; `on_close` runs on Cancel, Escape or a press outside.
    pub fn new(
        id: impl Into<ElementId>,
        format: impl Into<SharedString>,
        on_format: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
        on_export: impl Fn(&mut Window, &mut App) + 'static,
        on_close: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        let format = format.into();
        assert!(
            FORMATS.iter().any(|(value, _)| *value == format.as_ref()),
            "no export format {format}"
        );
        Self {
            id: id.into(),
            format,
            on_format: Rc::new(on_format),
            thinking: None,
            on_export: Rc::new(on_export),
            on_close: Rc::new(on_close),
        }
    }

    /// Offers to include the thinking behind each answer.
    pub fn thinking(
        mut self,
        included: bool,
        on_change: impl Fn(bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.thinking = Some((included, Rc::new(on_change)));
        self
    }
}

impl RenderOnce for ConversationExport {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let (pick, export, close) = (self.on_format, self.on_export, self.on_close.clone());
        let format = self.format.clone();
        let (cancel, done) = ((self.id.clone(), "cancel"), (self.id.clone(), "export"));
        Dialog::new(self.id.clone(), "Export conversation", move |window, cx| {
            close(window, cx)
        })
        .child(
            div()
                .flex()
                .flex_col()
                .gap_4()
                .child(
                    RadioGroup::new(
                        (self.id.clone(), "format"),
                        FORMATS.map(|(value, label)| Choice::new(value, label)),
                    )
                    .selected(self.format)
                    .on_change(move |next, window, cx| {
                        log::info!("export: format {next}");
                        pick(next, window, cx)
                    }),
                )
                .children(self.thinking.map(|(included, change)| {
                    Checkbox::new((self.id.clone(), "thinking"), CheckState::from(included))
                        .label("Include the thinking behind each answer")
                        .on_change(move |on, window, cx| change(on, window, cx))
                })),
        )
        .action(move |close| {
            Button::new(cancel, "Cancel")
                .variant(ButtonVariant::Secondary)
                .on_click(move |_, window, cx| close(window, cx))
        })
        .action(move |close| {
            Button::new(done, "Export")
                .variant(ButtonVariant::Primary)
                .on_click(move |_, window, cx| {
                    log::info!("export: {format}");
                    export(window, cx);
                    close(window, cx)
                })
        })
    }
}
