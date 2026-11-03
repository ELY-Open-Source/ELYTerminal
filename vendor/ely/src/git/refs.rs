use std::rc::Rc;

use gpui::{
    AnyElement, App, ElementId, FontWeight, InteractiveElement, IntoElement, ParentElement,
    RenderOnce, Role, SharedString, StatefulInteractiveElement, Styled, Window, div, prelude::*,
};

use crate::{
    buttons::{ButtonVariant, IconButton},
    i18n,
    navigation::{Group, Palette, Row, fuzzy, marked, query_field},
    primitives::{FocusRing, Icon, IconName},
    theme::{ActiveTheme, ControlSize, IconSize, Radius, TextSize},
    typography::{Ellipsis, tabular},
};

/// A branch, its upstream distance, and its last commit.
#[derive(Clone, Debug, PartialEq)]
pub struct Branch {
    pub name: SharedString,
    pub remote: bool,
    pub current: bool,
    pub ahead: usize,
    pub behind: usize,
    pub subject: SharedString,
    pub when: SharedString,
}

/// `↑2 ↓1`, or nothing when even.
fn distance(branch: &Branch) -> Option<String> {
    let parts: Vec<String> = [(branch.ahead, '↑'), (branch.behind, '↓')]
        .iter()
        .filter(|(count, _)| *count > 0)
        .map(|(count, arrow)| format!("{arrow}{count}"))
        .collect();
    (!parts.is_empty()).then(|| parts.join(" "))
}

/// Marks a picked row's value as a new branch's name.
const CREATE: &str = "\u{0}create:";

pub(super) type OnName = Rc<dyn Fn(&SharedString, &mut Window, &mut App)>;
pub(super) type Run = Rc<dyn Fn(&mut Window, &mut App)>;

/// Branches as a palette; an unknown query makes one.
#[derive(IntoElement)]
pub struct BranchSelector {
    id: ElementId,
    branches: Vec<Branch>,
    on_pick: Option<OnName>,
    on_create: Option<OnName>,
    on_close: Run,
}

impl BranchSelector {
    pub fn new(
        id: impl Into<ElementId>,
        branches: impl IntoIterator<Item = Branch>,
        on_close: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            branches: branches.into_iter().collect(),
            on_pick: None,
            on_create: None,
            on_close: Rc::new(on_close),
        }
    }

    pub fn on_pick(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_pick = Some(Rc::new(handler));
        self
    }

    pub fn on_create(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_create = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for BranchSelector {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let input = query_field(
            &self.id,
            i18n::text(cx, "palette.branch.placeholder", &[]),
            window,
            cx,
        );
        let query = input.read(cx).text().trim().to_string();
        let colors = cx.theme().colors.clone();
        let row = |branch: &Branch, cx: &App| -> Option<(i32, Row)> {
            let (score, hits) = if query.is_empty() {
                (0, Vec::new())
            } else {
                let fit = fuzzy(&query, &branch.name)?;
                (fit.score, fit.hits)
            };
            Some((
                score,
                Row {
                    value: branch.name.clone(),
                    name: branch.name.clone(),
                    icon: Some(if branch.current {
                        IconName::Check
                    } else {
                        IconName::GitBranch
                    }),
                    label: marked(branch.name.clone(), hits, cx),
                    detail: Some(
                        div()
                            .text_color(colors.fg_subtle)
                            .child(branch.subject.clone())
                            .into_any_element(),
                    ),
                    end: distance(branch).map(|text| {
                        div()
                            .text_color(colors.fg_subtle)
                            .child(text)
                            .into_any_element()
                    }),
                },
            ))
        };
        let group = |title: SharedString, remote: bool, cx: &App| {
            let mut rows: Vec<(i32, Row)> = self
                .branches
                .iter()
                .filter(|branch| branch.remote == remote)
                .filter_map(|branch| row(branch, cx))
                .collect();
            if !query.is_empty() {
                rows.sort_by(|(a, _), (b, _)| b.cmp(a));
            }
            Group {
                title: Some(title),
                rows: rows.into_iter().map(|(_, row)| row).collect(),
            }
        };
        let mut groups = vec![
            group(i18n::text(cx, "palette.branch.local", &[]), false, cx),
            group(i18n::text(cx, "palette.branch.remote", &[]), true, cx),
        ];
        let named = self
            .branches
            .iter()
            .any(|branch| branch.name.as_ref() == query);
        if !query.is_empty() && !named {
            let create = i18n::text(cx, "palette.branch.create", &[("query", &query)]);
            groups.insert(
                0,
                Group {
                    title: None,
                    rows: vec![Row {
                        value: format!("{CREATE}{query}").into(),
                        name: create.clone(),
                        icon: Some(IconName::Plus),
                        label: div().child(create).into_any_element(),
                        detail: None,
                        end: None,
                    }],
                },
            );
        }
        let (pick, create) = (self.on_pick, self.on_create);
        let on_pick: OnName = Rc::new(
            move |value: &SharedString, window: &mut Window, cx: &mut App| match value
                .strip_prefix(CREATE)
            {
                Some(name) => {
                    if let Some(create) = &create {
                        create(&name.to_string().into(), window, cx)
                    }
                }
                None => {
                    if let Some(pick) = &pick {
                        pick(value, window, cx)
                    }
                }
            },
        );
        Palette {
            id: self.id,
            input,
            groups,
            start: 0,
            empty: i18n::text(cx, "palette.branch.empty", &[]),
            on_pick: Some(on_pick),
            on_close: self.on_close,
        }
        .overlay(window, cx)
    }
}

/// A pickable row: its spoken name, whether picked, the pick.
pub(super) struct Pick {
    pub spoken: SharedString,
    pub selected: bool,
    pub run: Run,
}

/// A row's icon, name, detail, trailing words and hover actions.
pub(super) struct Entry {
    pub icon: IconName,
    pub name: AnyElement,
    pub detail: SharedString,
    pub trailing: Vec<SharedString>,
    pub actions: Vec<AnyElement>,
}

pub(super) fn listed(id: ElementId, entry: Entry, pick: Option<Pick>, cx: &App) -> AnyElement {
    let Entry {
        icon,
        name,
        detail,
        trailing,
        actions,
    } = entry;
    let theme = cx.theme();
    let colors = theme.colors.clone();
    div()
        .id(id)
        .when_some(pick, |row, pick| {
            let run = pick.run;
            row.role(Role::ListItem)
                .aria_label(pick.spoken)
                .aria_selected(pick.selected)
                .tab_index(0)
                .border_1()
                .border_color(gpui::transparent_black())
                .focus_ring(cx)
                .cursor_pointer()
                .when(pick.selected, |row| row.bg(colors.selection))
                .on_click(move |_, window, cx| run(window, cx))
        })
        .flex()
        .items_center()
        .gap_2()
        .px_2()
        .py_1()
        .rounded(theme.radius(Radius::Sm))
        .hover(|row| row.bg(colors.hover))
        .child(Icon::new(icon).size(IconSize::Sm).color(colors.fg_muted))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                // Narrow, the name gives way; actions stay in reach.
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .overflow_hidden()
                                .whitespace_nowrap()
                                .child(name),
                        )
                        .child(div().flex_none().flex().children(actions)),
                )
                .when(!detail.is_empty() || !trailing.is_empty(), |text| {
                    text.child(
                        div()
                            .flex()
                            .gap_2()
                            .text_color(colors.fg_subtle)
                            .child(div().flex_1().min_w_0().child(Ellipsis::new(detail)))
                            .children(
                                trailing
                                    .into_iter()
                                    .map(|words| tabular(div().flex_none()).child(words)),
                            ),
                    )
                }),
        )
        .into_any_element()
}

pub(super) fn action(
    id: ElementId,
    icon: IconName,
    words: impl Into<SharedString>,
    run: Option<Run>,
) -> AnyElement {
    IconButton::new(id, icon)
        .variant(ButtonVariant::Ghost)
        .size(ControlSize::Sm)
        .tooltip(words)
        .when_some(run, |button, run| {
            button.on_click(move |_, window, cx| run(window, cx))
        })
        .into_any_element()
}

/// Branches with upstream distance and last commit.
#[derive(IntoElement)]
pub struct BranchList {
    id: ElementId,
    branches: Vec<Branch>,
    on_switch: Option<OnName>,
    on_delete: Option<OnName>,
    selected: Option<SharedString>,
    on_pick: Option<OnName>,
}

impl BranchList {
    pub fn new(id: impl Into<ElementId>, branches: impl IntoIterator<Item = Branch>) -> Self {
        Self {
            id: id.into(),
            branches: branches.into_iter().collect(),
            on_switch: None,
            on_delete: None,
            selected: None,
            on_pick: None,
        }
    }

    /// The branch shown as picked.
    pub fn selected(mut self, name: impl Into<SharedString>) -> Self {
        self.selected = Some(name.into());
        self
    }

    /// A press on a row picks its branch.
    pub fn on_pick(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_pick = Some(Rc::new(handler));
        self
    }

    pub fn on_switch(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_switch = Some(Rc::new(handler));
        self
    }

    pub fn on_delete(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_delete = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for BranchList {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let rows: Vec<AnyElement> = self
            .branches
            .iter()
            .map(|branch| {
                let name = div()
                    .when(branch.current, |name| {
                        name.font_weight(FontWeight::SEMIBOLD)
                    })
                    .text_color(colors.fg)
                    .child(branch.name.clone())
                    .into_any_element();
                let bind = |handler: &Option<OnName>| {
                    handler.clone().map(|handler| {
                        let name = branch.name.clone();
                        Rc::new(move |window: &mut Window, cx: &mut App| handler(&name, window, cx))
                            as Run
                    })
                };
                // Only actions the owner handles are drawn.
                let actions: Vec<AnyElement> = if branch.current {
                    Vec::new()
                } else {
                    [
                        (
                            "switch",
                            IconName::ArrowRight,
                            "palette.branch.switch",
                            bind(&self.on_switch),
                        ),
                        (
                            "delete",
                            IconName::Trash2,
                            "palette.branch.delete",
                            bind(&self.on_delete),
                        ),
                    ]
                    .into_iter()
                    .filter_map(|(key, icon, words, run)| {
                        let run = run?;
                        let id = (self.id.clone(), format!("{key}-{}", branch.name)).into();
                        Some(action(id, icon, i18n::text(cx, words, &[]), Some(run)))
                    })
                    .collect()
                };
                let trailing: Vec<SharedString> = distance(branch)
                    .into_iter()
                    .map(SharedString::from)
                    .chain([branch.when.clone()])
                    .collect();
                listed(
                    (self.id.clone(), format!("branch-{}", branch.name)).into(),
                    Entry {
                        icon: if branch.current {
                            IconName::Check
                        } else {
                            IconName::GitBranch
                        },
                        name,
                        detail: branch.subject.clone(),
                        trailing,
                        actions,
                    },
                    bind(&self.on_pick).map(|run| Pick {
                        spoken: branch.name.clone(),
                        selected: self.selected.as_ref() == Some(&branch.name),
                        run,
                    }),
                    cx,
                )
            })
            .collect();
        div()
            .flex()
            .flex_col()
            .text_size(theme.text_size(TextSize::Sm))
            .children(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distance_reads_ahead_then_behind() {
        let branch = |ahead, behind| Branch {
            name: "main".into(),
            remote: false,
            current: true,
            ahead,
            behind,
            subject: "".into(),
            when: "".into(),
        };
        assert_eq!(distance(&branch(2, 1)).as_deref(), Some("↑2 ↓1"));
        assert_eq!(distance(&branch(0, 3)).as_deref(), Some("↓3"));
        assert_eq!(distance(&branch(0, 0)), None);
    }
}
