use std::rc::Rc;

use gpui::{
    AnyElement, App, ElementId, FontWeight, InteractiveElement, IntoElement, MouseButton,
    ParentElement, RenderOnce, SharedString, StatefulInteractiveElement, Styled, Window, div,
    prelude::*, transparent_black,
};

use crate::{
    buttons::{Button, ButtonVariant},
    collab::Decision,
    forms::OnFlag,
    git::{DiffStat, DiffViewer, stat},
    primitives::{Disclosure, FocusRing, Icon, IconName, file_icon},
    theme::{ActiveTheme, ControlSize, IconSize, Radius, TextSize},
    typography::Ellipsis,
};

/// Where a proposed change stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChangeState {
    Proposed,
    Accepted,
    Rejected,
}

/// A change an agent proposes to one file: its path, the text before and after, and where it stands.
#[derive(Clone, Debug, PartialEq)]
pub struct FileChange {
    pub path: SharedString,
    pub old: SharedString,
    pub new: SharedString,
    pub state: ChangeState,
}

type OnDecision = Rc<dyn Fn(Decision, &mut Window, &mut App)>;

/// Reject and Accept while the change is proposed; after, a quiet word for how it ended.
fn verdict(id: &ElementId, state: ChangeState, on_decide: Option<OnFlag>, cx: &App) -> AnyElement {
    let colors = &cx.theme().colors;
    let said = |icon: IconName, word: &'static str, color| {
        div()
            .flex()
            .items_center()
            .gap_1()
            .text_color(color)
            .child(Icon::new(icon).size(IconSize::Xs).color(color))
            .child(word)
            .into_any_element()
    };
    match (state, on_decide) {
        (ChangeState::Proposed, Some(decide)) => {
            let (reject, accept) = (decide.clone(), decide);
            div()
                .flex()
                .items_center()
                .gap_1()
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .child(
                    Button::new((id.clone(), "reject"), "Reject")
                        .variant(ButtonVariant::Ghost)
                        .size(ControlSize::Sm)
                        .on_click(move |_, window, cx| reject(false, window, cx)),
                )
                .child(
                    Button::new((id.clone(), "accept"), "Accept")
                        .variant(ButtonVariant::Secondary)
                        .size(ControlSize::Sm)
                        .on_click(move |_, window, cx| accept(true, window, cx)),
                )
                .into_any_element()
        }
        (ChangeState::Proposed, None) => said(IconName::Clock, "Proposed", colors.fg_subtle),
        (ChangeState::Accepted, _) => said(IconName::Check, "Accepted", colors.success),
        (ChangeState::Rejected, _) => said(IconName::X, "Rejected", colors.fg_subtle),
    }
}

/// One file an agent would change: its path and how much moves, Reject or Accept while proposed; opened, the diff.
#[derive(IntoElement)]
pub struct FileChangeCard {
    id: ElementId,
    change: FileChange,
    on_decide: Option<OnFlag>,
}

impl FileChangeCard {
    pub fn new(id: impl Into<ElementId>, change: FileChange) -> Self {
        Self {
            id: id.into(),
            change,
            on_decide: None,
        }
    }

    /// `on_decide` gets true to accept, false to reject.
    pub fn on_decide(mut self, handler: impl Fn(bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_decide = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for FileChangeCard {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = window.use_keyed_state((self.id.clone(), "open"), cx, |_, _| false);
        let folds = window.use_keyed_state((self.id.clone(), "folds"), cx, |_, _| Vec::new());
        let (open, unfolded) = (*state.read(cx), folds.read(cx).clone());
        let (added, removed) = stat(&self.change.old, &self.change.new);
        let path = self.change.path.clone();
        let decide = self.on_decide.map(|decide| -> OnFlag {
            Rc::new(move |accepted, window, cx| {
                log::info!(
                    "file change {path}: {}",
                    if accepted { "accepted" } else { "rejected" }
                );
                decide(accepted, window, cx)
            })
        });
        let theme = cx.theme();
        let colors = theme.colors.clone();
        div()
            .rounded(theme.radius(Radius::Md))
            .border_1()
            .border_color(colors.border)
            .bg(colors.surface)
            .text_size(theme.text_size(TextSize::Sm))
            .child(
                div()
                    .id((self.id.clone(), "header"))
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap_2()
                    .px_2()
                    .py_1p5()
                    .rounded(theme.radius(Radius::Md))
                    .border_1()
                    .border_color(transparent_black())
                    .tab_index(0)
                    .focus_ring(cx)
                    .cursor_pointer()
                    .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
                    .on_click(move |_, _, cx| {
                        state.update(cx, |open, cx| {
                            *open = !*open;
                            log::info!("file change: open {open}");
                            cx.notify();
                        })
                    })
                    .child(Disclosure::new((self.id.clone(), "chevron"), open).size(IconSize::Sm))
                    .child(
                        Icon::new(file_icon(&self.change.path, cx))
                            .size(IconSize::Sm)
                            .color(colors.fg_muted),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w(theme.label_width())
                            .font_family(theme.mono_family.clone())
                            .child(Ellipsis::new(self.change.path.clone())),
                    )
                    .child(
                        div()
                            .flex_none()
                            .ml_auto()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(DiffStat::new(added, removed))
                            .child(verdict(&self.id, self.change.state, decide, cx)),
                    ),
            )
            .when(open, |card| {
                card.child(
                    div()
                        .h(theme.list_max_height())
                        .border_t_1()
                        .border_color(colors.border)
                        .child(
                            DiffViewer::new(
                                (self.id.clone(), "diff"),
                                self.change.path,
                                &self.change.old,
                                &self.change.new,
                            )
                            .headless()
                            .open(unfolded)
                            .on_open(move |ix, _, cx| {
                                folds.update(cx, |folds: &mut Vec<usize>, cx| {
                                    folds.push(ix);
                                    cx.notify();
                                })
                            }),
                        ),
                )
            })
    }
}

/// Every file an agent would change, reviewed together: how many and how much, all at once or a card each.
#[derive(IntoElement)]
pub struct MultiFileDiffReview {
    id: ElementId,
    changes: Vec<FileChange>,
    on_decide: Option<OnDecision>,
}

impl MultiFileDiffReview {
    pub fn new(id: impl Into<ElementId>, changes: impl IntoIterator<Item = FileChange>) -> Self {
        Self {
            id: id.into(),
            changes: changes.into_iter().collect(),
            on_decide: None,
        }
    }

    /// `on_decide` gets one file's verdict, by index, or all of them.
    pub fn on_decide(
        mut self,
        handler: impl Fn(Decision, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_decide = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for MultiFileDiffReview {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let (added, removed) = self
            .changes
            .iter()
            .fold((0, 0), |(added, removed), change| {
                let (more, less) = stat(&change.old, &change.new);
                (added + more, removed + less)
            });
        let open = self
            .changes
            .iter()
            .filter(|change| change.state == ChangeState::Proposed)
            .count();
        let count = self.changes.len();
        let all = self.on_decide.clone().filter(|_| open > 0).map(|decide| {
            let (reject, accept) = (decide.clone(), decide);
            div()
                .flex_none()
                .flex()
                .items_center()
                .gap_1()
                .child(
                    Button::new((self.id.clone(), "reject-all"), "Reject all")
                        .variant(ButtonVariant::Ghost)
                        .size(ControlSize::Sm)
                        .on_click(move |_, window, cx| {
                            log::info!("file changes: reject all");
                            reject(Decision::RejectAll, window, cx)
                        }),
                )
                .child(
                    Button::new((self.id.clone(), "accept-all"), "Accept all")
                        .variant(ButtonVariant::Primary)
                        .size(ControlSize::Sm)
                        .on_click(move |_, window, cx| {
                            log::info!("file changes: accept all");
                            accept(Decision::AcceptAll, window, cx)
                        }),
                )
        });
        div()
            .flex()
            .flex_col()
            .gap_2()
            .text_size(theme.text_size(TextSize::Sm))
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .justify_between()
                    .gap_y_2()
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .items_center()
                            .gap_x_3()
                            .gap_y_1()
                            .child(
                                div()
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(format!("{count} files changed")),
                            )
                            .child(DiffStat::new(added, removed))
                            .child(
                                div()
                                    .text_color(colors.fg_subtle)
                                    .child(format!("{} of {count} reviewed", count - open)),
                            ),
                    )
                    .children(all),
            )
            .children(self.changes.into_iter().enumerate().map(|(ix, change)| {
                let card = FileChangeCard::new((self.id.clone(), format!("file-{ix}")), change);
                match self.on_decide.clone() {
                    Some(decide) => card.on_decide(move |accepted, window, cx| {
                        let decision = if accepted {
                            Decision::Accept(ix)
                        } else {
                            Decision::Reject(ix)
                        };
                        decide(decision, window, cx)
                    }),
                    None => card,
                }
            }))
    }
}
