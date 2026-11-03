use std::rc::Rc;

use gpui::{
    AnyElement, App, ElementId, Entity, FontWeight, InteractiveElement, IntoElement, ParentElement,
    RenderOnce, Role, SharedString, StatefulInteractiveElement, Styled, Window, div, prelude::*,
};

use super::badges::GitStatusBadge;
use crate::{
    buttons::{Button, ButtonVariant, IconButton},
    forms::{Checkbox, Input, Submit, TextInput},
    i18n,
    lists::GitStatus,
    primitives::{FocusRing, IconName},
    theme::{ActiveTheme, ControlSize, Radius, TextSize},
    typography::{Ellipsis, tabular},
};

/// A changed file: path, status, and lines changed.
#[derive(Clone, Debug, PartialEq)]
pub struct Changed {
    pub path: SharedString,
    pub status: GitStatus,
    /// Added and removed lines; None when not counted.
    pub lines: Option<(usize, usize)>,
}

/// What a changes list asks for a file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChangeAction {
    Stage,
    Unstage,
    Discard,
    Open,
}

type OnAction = Rc<dyn Fn(&SharedString, ChangeSection, ChangeAction, &mut Window, &mut App)>;
type OnAll = Rc<dyn Fn(bool, &mut Window, &mut App)>;

/// The part of the work tree listing a file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChangeSection {
    Conflicted,
    Staged,
    Unstaged,
    Untracked,
    Ignored,
}

impl ChangeSection {
    fn key(self) -> &'static str {
        match self {
            Self::Conflicted => "git.changes.conflicted",
            Self::Staged => "git.changes.staged",
            Self::Unstaged => "git.changes.unstaged",
            Self::Untracked => "git.changes.untracked",
            Self::Ignored => "git.changes.ignored",
        }
    }
}

/// Work tree files in five sections, each with its actions.
#[derive(IntoElement)]
pub struct ChangesList {
    id: ElementId,
    conflicted: Vec<Changed>,
    staged: Vec<Changed>,
    unstaged: Vec<Changed>,
    untracked: Vec<Changed>,
    ignored: Vec<Changed>,
    on_action: Option<OnAction>,
    on_all: Option<OnAll>,
    selected: Option<(ChangeSection, SharedString)>,
}

impl ChangesList {
    pub fn new(
        id: impl Into<ElementId>,
        staged: impl IntoIterator<Item = Changed>,
        unstaged: impl IntoIterator<Item = Changed>,
    ) -> Self {
        Self {
            id: id.into(),
            conflicted: Vec::new(),
            staged: staged.into_iter().collect(),
            unstaged: unstaged.into_iter().collect(),
            untracked: Vec::new(),
            ignored: Vec::new(),
            on_action: None,
            on_all: None,
            selected: None,
        }
    }

    /// Files both sides changed; listed first, opened only.
    pub fn conflicted(mut self, files: impl IntoIterator<Item = Changed>) -> Self {
        self.conflicted = files.into_iter().collect();
        self
    }

    /// Untracked files, staged one by one.
    pub fn untracked(mut self, files: impl IntoIterator<Item = Changed>) -> Self {
        self.untracked = files.into_iter().collect();
        self
    }

    /// Files Git ignores; listed last, opened only.
    pub fn ignored(mut self, files: impl IntoIterator<Item = Changed>) -> Self {
        self.ignored = files.into_iter().collect();
        self
    }

    /// The file shown as current, in its section.
    pub fn selected(mut self, section: ChangeSection, path: impl Into<SharedString>) -> Self {
        self.selected = Some((section, path.into()));
        self
    }

    pub fn on_action(
        mut self,
        handler: impl Fn(&SharedString, ChangeSection, ChangeAction, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_action = Some(Rc::new(handler));
        self
    }

    /// True stages every change; false unstages every staged one.
    pub fn on_all(mut self, handler: impl Fn(bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_all = Some(Rc::new(handler));
        self
    }
}

impl ChangesList {
    fn row(&self, file: &Changed, section: ChangeSection, cx: &App) -> AnyElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let (folder, name) = match file.path.rsplit_once('/') {
            Some((folder, name)) => (folder.to_string(), name.to_string()),
            None => (String::new(), file.path.to_string()),
        };
        let group = SharedString::from(format!("change-{section:?}-{}", file.path));
        let action = |key: &str, icon, words: &str, act: ChangeAction| {
            let words = i18n::text(cx, words, &[]);
            let (on_action, path) = (self.on_action.clone(), file.path.clone());
            IconButton::new(
                (self.id.clone(), format!("{key}-{section:?}-{}", file.path)),
                icon,
            )
            .variant(ButtonVariant::Ghost)
            .size(ControlSize::Sm)
            .tooltip(words)
            .when_some(on_action, |button, on_action| {
                button.on_click(move |_, window, cx| {
                    cx.stop_propagation();
                    on_action(&path, section, act, window, cx)
                })
            })
        };
        let (open, path) = (self.on_action.clone(), file.path.clone());
        let picked = self
            .selected
            .as_ref()
            .is_some_and(|(at, chosen)| *at == section && *chosen == file.path);
        div()
            .id((self.id.clone(), format!("row-{section:?}-{}", file.path)))
            .group(group.clone())
            .role(Role::ListItem)
            .aria_label(file.path.clone())
            .aria_selected(picked)
            .tab_index(0)
            .border_1()
            .border_color(gpui::transparent_black())
            .focus_ring(cx)
            .when(picked, |row| row.bg(colors.selection))
            .flex()
            .items_center()
            .gap_2()
            .px_2()
            .py_1()
            .rounded(theme.radius(Radius::Sm))
            .cursor_pointer()
            .hover(|row| row.bg(colors.hover))
            .when_some(open, |row, open| {
                row.on_click(move |_, window, cx| {
                    open(&path, section, ChangeAction::Open, window, cx)
                })
            })
            .child(GitStatusBadge::new(
                (self.id.clone(), format!("status-{section:?}-{}", file.path)),
                file.status,
            ))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .items_baseline()
                    .gap_2()
                    .child(
                        div()
                            .min_w_0()
                            .text_color(colors.fg)
                            .debug_selector({
                                let path = file.path.clone();
                                move || format!("change-name-{path}")
                            })
                            .when(file.status == GitStatus::Deleted, |name| {
                                name.line_through()
                            })
                            .child(Ellipsis::new(name)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_color(colors.fg_subtle)
                            .child(Ellipsis::new(folder)),
                    ),
            )
            .child(
                div()
                    .flex_none()
                    .flex()
                    .debug_selector({
                        let path = file.path.clone();
                        move || format!("change-actions-{path}")
                    })
                    .child(action(
                        "open",
                        IconName::FileText,
                        "git.changes.open",
                        ChangeAction::Open,
                    ))
                    .when(
                        matches!(section, ChangeSection::Unstaged | ChangeSection::Untracked),
                        |actions| {
                            actions.child(action(
                                "discard",
                                IconName::RotateCcw,
                                "git.changes.discard",
                                ChangeAction::Discard,
                            ))
                        },
                    )
                    .when(section == ChangeSection::Staged, |actions| {
                        actions.child(action(
                            "unstage",
                            IconName::Minus,
                            "git.changes.unstage",
                            ChangeAction::Unstage,
                        ))
                    })
                    .when(
                        matches!(section, ChangeSection::Unstaged | ChangeSection::Untracked),
                        |actions| {
                            actions.child(action(
                                "stage",
                                IconName::Plus,
                                "git.changes.stage",
                                ChangeAction::Stage,
                            ))
                        },
                    ),
            )
            .children(file.lines.map(|(added, removed)| {
                tabular(div().flex_none().text_color(colors.fg_subtle))
                    .child(format!("+{added} −{removed}"))
            }))
            .into_any_element()
    }

    fn section(&self, section: ChangeSection, files: &[Changed], cx: &App) -> Option<AnyElement> {
        if files.is_empty() {
            return None;
        }
        let theme = cx.theme();
        let colors = theme.colors.clone();
        // Only staged and changed files move all at once.
        let all = match section {
            ChangeSection::Staged => Some((IconName::Minus, "git.changes.unstage_all", false)),
            ChangeSection::Unstaged => Some((IconName::Plus, "git.changes.stage_all", true)),
            _ => None,
        };
        let on_all = self.on_all.clone();
        let button = all.map(|(icon, key, stage)| {
            IconButton::new((self.id.clone(), format!("all-{section:?}")), icon)
                .variant(ButtonVariant::Ghost)
                .size(ControlSize::Sm)
                .tooltip(i18n::text(cx, key, &[]))
                .when_some(on_all, |button, on_all| {
                    button.on_click(move |_, window, cx| on_all(stage, window, cx))
                })
        });
        Some(
            div()
                .flex()
                .flex_col()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .px_2()
                        .py_1()
                        .text_size(theme.text_size(TextSize::Xs))
                        .text_color(colors.fg_muted)
                        .child(
                            div()
                                .flex_1()
                                .font_weight(FontWeight::MEDIUM)
                                .child(i18n::text(cx, section.key(), &[])),
                        )
                        .child(tabular(div()).child(files.len().to_string()))
                        .children(button),
                )
                .children(files.iter().map(|file| self.row(file, section, cx)))
                .into_any_element(),
        )
    }
}

impl RenderOnce for ChangesList {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let sections = [
            self.section(ChangeSection::Conflicted, &self.conflicted, cx),
            self.section(ChangeSection::Staged, &self.staged, cx),
            self.section(ChangeSection::Unstaged, &self.unstaged, cx),
            self.section(ChangeSection::Untracked, &self.untracked, cx),
            self.section(ChangeSection::Ignored, &self.ignored, cx),
        ];
        div()
            .flex()
            .flex_col()
            .gap_2()
            .text_size(theme.text_size(TextSize::Sm))
            .children(sections.into_iter().flatten())
    }
}

/// A subject's comfortable length, and its most.
const SUBJECT: (usize, usize) = (50, 72);

type Run = Rc<dyn Fn(&mut Window, &mut App)>;

/// A commit message, its counted subject, body, and button.
#[derive(IntoElement)]
pub struct CommitInput {
    id: ElementId,
    subject: Entity<TextInput>,
    body: Entity<TextInput>,
    branch: SharedString,
    staged: usize,
    amend: bool,
    on_amend: Option<OnAll>,
    on_commit: Option<Run>,
}

impl CommitInput {
    pub fn new(
        id: impl Into<ElementId>,
        subject: &Entity<TextInput>,
        body: &Entity<TextInput>,
        branch: impl Into<SharedString>,
        staged: usize,
    ) -> Self {
        Self {
            id: id.into(),
            subject: subject.clone(),
            body: body.clone(),
            branch: branch.into(),
            staged,
            amend: false,
            on_amend: None,
            on_commit: None,
        }
    }

    pub fn amend(mut self, amend: bool) -> Self {
        self.amend = amend;
        self
    }

    pub fn on_amend(mut self, handler: impl Fn(bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_amend = Some(Rc::new(handler));
        self
    }

    pub fn on_commit(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_commit = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for CommitInput {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let length = self.subject.read(cx).text().chars().count();
        let ready = length > 0 && (self.staged > 0 || self.amend);
        let count = match length {
            length if length > SUBJECT.1 => colors.danger,
            length if length > SUBJECT.0 => colors.warning,
            _ => colors.fg_subtle,
        };
        let commit = self.on_commit.filter(|_| ready);
        let submit = commit.clone();
        let label = if self.amend {
            i18n::text(cx, "git.commit.amend_to", &[("branch", &self.branch)])
        } else {
            i18n::text(cx, "git.commit.commit_to", &[("branch", &self.branch)])
        };
        let amend = Checkbox::new((self.id.clone(), "amend"), self.amend).label(i18n::text(
            cx,
            "git.commit.amend",
            &[],
        ));
        let amend = match self.on_amend {
            Some(on_amend) => amend.on_change(move |on, window, cx| on_amend(on, window, cx)),
            None => amend,
        };
        div()
            .id(self.id.clone())
            .flex()
            .flex_col()
            .gap_2()
            .text_size(theme.text_size(TextSize::Sm))
            .capture_action(move |_: &Submit, window, cx| {
                if let Some(submit) = &submit {
                    cx.stop_propagation();
                    submit(window, cx)
                }
            })
            .child(
                Input::new(&self.subject).suffix(
                    tabular(div().text_color(count)).child(format!("{length}/{}", SUBJECT.0)),
                ),
            )
            .child(Input::new(&self.body))
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .debug_selector(|| "commit-footer".into())
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .debug_selector(|| "commit-amend".into())
                            .flex_grow_1()
                            .flex_shrink_0()
                            .max_w_full()
                            .child(amend),
                    )
                    .child(
                        div()
                            .debug_selector(|| "commit-button".into())
                            .flex()
                            .flex_auto()
                            .min_w_0()
                            .max_w_full()
                            .child(
                                Button::new((self.id.clone(), "commit"), label)
                                    .full_width()
                                    .variant(ButtonVariant::Primary)
                                    .size(ControlSize::Sm)
                                    .disabled(!ready)
                                    .when_some(commit, |button, commit| {
                                        button.on_click(move |_, window, cx| commit(window, cx))
                                    }),
                            ),
                    ),
            )
    }
}
