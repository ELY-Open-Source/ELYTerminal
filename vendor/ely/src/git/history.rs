use std::rc::Rc;

use gpui::{
    App, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce, Role, SharedString,
    StatefulInteractiveElement, Styled, Window, div, prelude::*, uniform_list,
};

use super::{badges::DiffStat, commits::Commit};
use crate::{
    primitives::FocusRing,
    theme::{ActiveTheme, Radius, TextSize},
    typography::Ellipsis,
};

type OnPick = Rc<dyn Fn(&SharedString, &mut Window, &mut App)>;
/// A commit with lines added and removed, if counted.
pub type Counted = (Commit, Option<(usize, usize)>);

/// A file's commits newest first; draws rows in view.
#[derive(IntoElement)]
pub struct FileHistory {
    id: ElementId,
    commits: Rc<Vec<Counted>>,
    selected: Option<SharedString>,
    on_pick: Option<OnPick>,
}

impl FileHistory {
    /// Each commit with lines added and removed, if counted.
    pub fn new(id: impl Into<ElementId>, commits: Rc<Vec<Counted>>) -> Self {
        Self {
            id: id.into(),
            commits,
            selected: None,
            on_pick: None,
        }
    }

    pub fn selected(mut self, id: impl Into<SharedString>) -> Self {
        self.selected = Some(id.into());
        self
    }

    pub fn on_pick(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_pick = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for FileHistory {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let Self {
            id,
            commits,
            selected,
            on_pick,
        } = self;
        let last = commits.len().saturating_sub(1);
        uniform_list((id.clone(), "rows"), commits.len(), move |range, _, cx| {
            let theme = cx.theme();
            let colors = theme.colors.clone();
            range
                .map(|ix| {
                    let (commit, lines) = &commits[ix];
                    let picked = selected.as_ref() == Some(&commit.id);
                    let (pick, picked_id) = (on_pick.clone(), commit.id.clone());
                    let short: String = commit.id.chars().take(7).collect();
                    div()
                        .id((id.clone(), format!("commit-{ix}")))
                        .w_full()
                        .role(Role::ListItem)
                        .aria_label(commit.subject.clone())
                        .aria_selected(picked)
                        .tab_index(0)
                        .border_1()
                        .border_color(gpui::transparent_black())
                        .focus_ring(cx)
                        .flex()
                        .gap_3()
                        .px_2()
                        .text_size(theme.text_size(TextSize::Sm))
                        .rounded(theme.radius(Radius::Sm))
                        .when(picked, |row| row.bg(colors.active))
                        .when(!picked, |row| row.hover(|row| row.bg(colors.hover)))
                        .when_some(pick, |row, pick| {
                            row.cursor_pointer()
                                .on_click(move |_, window, cx| pick(&picked_id, window, cx))
                        })
                        .child(
                            div()
                                .flex_none()
                                .flex()
                                .flex_col()
                                .items_center()
                                .child(div().w_0().h_2().when(ix > 0, |line| {
                                    line.border_l_1().border_color(colors.border)
                                }))
                                .child(
                                    div()
                                        .size_2()
                                        .rounded_full()
                                        .border_1()
                                        .border_color(if picked {
                                            colors.focus
                                        } else {
                                            colors.border_strong
                                        })
                                        .bg(if ix == 0 { colors.focus } else { colors.bg }),
                                )
                                .child(div().w_0().flex_1().when(ix < last, |line| {
                                    line.border_l_1().border_color(colors.border)
                                })),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .py_1p5()
                                .flex()
                                .flex_col()
                                .gap_0p5()
                                .child(
                                    div()
                                        .text_color(colors.fg)
                                        .child(Ellipsis::new(commit.subject.clone())),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap_2()
                                        .text_size(theme.text_size(TextSize::Xs))
                                        .text_color(colors.fg_subtle)
                                        .child(commit.author.clone())
                                        .child(commit.when.clone())
                                        .child(
                                            div()
                                                .font_family(theme.mono_family.clone())
                                                .child(short),
                                        ),
                                ),
                        )
                        .children(lines.map(|(added, removed)| {
                            div()
                                .flex_none()
                                .py_1p5()
                                .child(DiffStat::new(added, removed))
                        }))
                })
                .collect()
        })
        .size_full()
    }
}
