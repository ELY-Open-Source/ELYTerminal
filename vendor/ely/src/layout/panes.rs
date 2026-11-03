use std::rc::Rc;

use anyhow::Context as _;
use gpui::{
    AnyElement, App, Axis, Context, InteractiveElement, IntoElement, MouseButton, ParentElement,
    Render, Styled, Window, div,
};
use serde::{Deserialize, Serialize};

use super::{SplitPane, split::check_weights};
use crate::{
    buttons::IconButton,
    primitives::IconName,
    theme::{ActiveTheme, ControlSize, TextSize},
};

pub type PaneId = u64;

/// Split tree. The live model and its saved form are the same data.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum PaneLayout {
    Pane(PaneId),
    Split {
        axis: Axis,
        sizes: Vec<f32>,
        children: Vec<PaneLayout>,
    },
}

impl PaneLayout {
    /// Checks ids and splits; returns the next free pane id.
    pub fn validate(&self) -> anyhow::Result<PaneId> {
        let mut ids = Vec::new();
        self.panes(&mut ids);
        let mut unique = ids.clone();
        unique.sort_unstable();
        unique.dedup();
        anyhow::ensure!(unique.len() == ids.len(), "pane ids repeat: {ids:?}");
        check_sizes(self)?;
        let last = *unique.last().expect("a layout holds a pane");
        last.checked_add(1)
            .with_context(|| format!("no pane id follows {last}"))
    }

    fn panes(&self, out: &mut Vec<PaneId>) {
        match self {
            PaneLayout::Pane(id) => out.push(*id),
            PaneLayout::Split { children, .. } => {
                children.iter().for_each(|child| child.panes(out))
            }
        }
    }

    /// Inserts `new` beside `target`; true if `target` was found.
    fn split(&mut self, target: PaneId, axis: Axis, new: PaneId) -> bool {
        match self {
            PaneLayout::Pane(id) if *id == target => {
                *self = PaneLayout::Split {
                    axis,
                    sizes: vec![0.5, 0.5],
                    children: vec![PaneLayout::Pane(target), PaneLayout::Pane(new)],
                };
                true
            }
            PaneLayout::Pane(_) => false,
            PaneLayout::Split {
                axis: own,
                sizes,
                children,
            } => {
                let direct = children
                    .iter()
                    .position(|child| *child == PaneLayout::Pane(target));
                match direct {
                    Some(ix) if *own == axis => {
                        let total: f32 = sizes.iter().sum();
                        sizes.iter_mut().for_each(|size| *size /= total);
                        let half = sizes[ix] / 2.0;
                        sizes[ix] = half;
                        sizes.insert(ix + 1, half);
                        children.insert(ix + 1, PaneLayout::Pane(new));
                        true
                    }
                    _ => children
                        .iter_mut()
                        .any(|child| child.split(target, axis, new)),
                }
            }
        }
    }

    /// Removes `target`, folding splits left with one child.
    fn close(&mut self, target: PaneId) -> bool {
        let PaneLayout::Split {
            sizes, children, ..
        } = self
        else {
            return false;
        };
        if let Some(ix) = children
            .iter()
            .position(|child| *child == PaneLayout::Pane(target))
        {
            children.remove(ix);
            sizes.remove(ix);
            let rest: f32 = sizes.iter().sum();
            if rest > 0.0 {
                sizes.iter_mut().for_each(|size| *size /= rest);
            } else {
                sizes.iter_mut().for_each(|size| *size = 1.0);
            }
        } else if !children.iter_mut().any(|child| child.close(target)) {
            return false;
        }
        if children.len() == 1 {
            *self = children.remove(0);
        }
        true
    }

    /// Sets the split at `path`; false if the tree no longer has it.
    fn set_sizes(&mut self, path: &[usize], shares: &[f32]) -> bool {
        let PaneLayout::Split {
            sizes, children, ..
        } = self
        else {
            return false;
        };
        match path.split_first() {
            None if shares.len() == sizes.len() => {
                *sizes = shares.to_vec();
                true
            }
            None => false,
            Some((ix, rest)) => children
                .get_mut(*ix)
                .is_some_and(|child| child.set_sizes(rest, shares)),
        }
    }
}

type Content = Rc<dyn Fn(PaneId, &mut Window, &mut App) -> AnyElement>;

/// Editor-style split panes: split right or down, close, focus.
pub struct PaneGroup {
    root: PaneLayout,
    focused: PaneId,
    next: PaneId,
    content: Content,
}

impl PaneGroup {
    pub fn new(content: impl Fn(PaneId, &mut Window, &mut App) -> AnyElement + 'static) -> Self {
        Self {
            root: PaneLayout::Pane(1),
            focused: 1,
            next: 2,
            content: Rc::new(content),
        }
    }

    pub fn focused(&self) -> PaneId {
        self.focused
    }

    pub fn panes(&self) -> Vec<PaneId> {
        let mut out = Vec::new();
        self.root.panes(&mut out);
        out
    }

    /// Opens a new pane beside `pane` and focuses it; `None` if `pane` is gone.
    pub fn split(&mut self, pane: PaneId, axis: Axis, cx: &mut Context<Self>) -> Option<PaneId> {
        let new = self.next;
        if !self.root.split(pane, axis, new) {
            log::error!("pane group: no pane {pane} to split");
            return None;
        }
        self.next = new.checked_add(1).expect("pane ids exhausted");
        self.focused = new;
        log::info!("pane group: split {pane} -> {new} ({axis:?})");
        cx.notify();
        Some(new)
    }

    /// Closes `pane`; the last pane, or one already gone, stays as it is.
    pub fn close(&mut self, pane: PaneId, cx: &mut Context<Self>) {
        if self.panes().len() < 2 {
            log::error!("pane group: pane {pane} is the last; it stays");
            return;
        }
        if !self.root.close(pane) {
            log::error!("pane group: no pane {pane} to close");
            return;
        }
        if self.focused == pane {
            self.focused = self.panes()[0];
        }
        log::info!("pane group: closed {pane}");
        cx.notify();
    }

    pub fn focus(&mut self, pane: PaneId, cx: &mut Context<Self>) {
        if !self.panes().contains(&pane) {
            log::error!("pane group: no pane {pane} to focus");
            return;
        }
        self.focused = pane;
        cx.notify();
    }

    pub fn layout(&self) -> PaneLayout {
        self.root.clone()
    }

    /// Replaces the tree after `PaneLayout::validate` accepts it.
    pub fn restore(&mut self, layout: PaneLayout, cx: &mut Context<Self>) -> anyhow::Result<()> {
        self.next = layout.validate()?;
        let mut ids = Vec::new();
        layout.panes(&mut ids);
        self.focused = ids[0];
        self.root = layout;
        log::info!("pane group: restored {} panes", ids.len());
        cx.notify();
        Ok(())
    }

    fn render_node(
        &self,
        node: &PaneLayout,
        path: Vec<usize>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        match node {
            PaneLayout::Pane(id) => self.render_pane(*id, window, cx),
            PaneLayout::Split {
                axis,
                sizes,
                children,
            } => {
                let min = cx.theme().pane_min().to_pixels(window.rem_size());
                let key = format!("pane-split-{path:?}-{}", children.len());
                let at = path.clone();
                let mut split = SplitPane::new(gpui::SharedString::from(key), *axis, min)
                    .sizes(sizes)
                    .on_resize(cx.listener(move |group: &mut Self, shares: &[f32], _, _| {
                        if !group.root.set_sizes(&at, shares) {
                            log::error!("pane group: no split at {at:?} for {shares:?}");
                        }
                    }));
                for (ix, child) in children.iter().enumerate() {
                    let mut child_path = path.clone();
                    child_path.push(ix);
                    split = split.pane(self.render_node(child, child_path, window, cx));
                }
                split.into_any_element()
            }
        }
    }

    fn render_pane(&self, id: PaneId, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let focused = id == self.focused;
        let lone = self.panes().len() == 1;
        let header = div()
            .flex()
            .items_center()
            .justify_between()
            .h(theme.control_height(ControlSize::Lg))
            .pl_3()
            .pr_1()
            .border_b_1()
            .border_color(theme.colors.border)
            .text_size(theme.text_size(TextSize::Sm))
            .text_color(if focused {
                theme.colors.fg
            } else {
                theme.colors.fg_subtle
            })
            .child(gpui::SharedString::from(format!("Pane {id}")))
            .child(
                div()
                    .flex()
                    .gap_0p5()
                    .child(
                        IconButton::new(("split-right", id), IconName::PanelRight)
                            .size(ControlSize::Sm)
                            .on_click(cx.listener(move |group, _, _, cx| {
                                group.split(id, Axis::Horizontal, cx);
                            })),
                    )
                    .child(
                        IconButton::new(("split-down", id), IconName::PanelBottom)
                            .size(ControlSize::Sm)
                            .on_click(cx.listener(move |group, _, _, cx| {
                                group.split(id, Axis::Vertical, cx);
                            })),
                    )
                    .child(
                        IconButton::new(("close", id), IconName::X)
                            .size(ControlSize::Sm)
                            .disabled(lone)
                            .on_click(cx.listener(move |group, _, _, cx| group.close(id, cx))),
                    ),
            );
        div()
            .id(("pane", id))
            .size_full()
            .flex()
            .flex_col()
            .bg(if focused {
                theme.colors.surface
            } else {
                theme.colors.bg
            })
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |group, _, _, cx| {
                    if group.focused != id {
                        group.focus(id, cx);
                    }
                }),
            )
            .child(header)
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .child((self.content)(id, window, cx)),
            )
            .into_any_element()
    }
}

fn check_sizes(node: &PaneLayout) -> anyhow::Result<()> {
    if let PaneLayout::Split {
        sizes, children, ..
    } = node
    {
        anyhow::ensure!(children.len() >= 2, "a split needs two children");
        anyhow::ensure!(
            sizes.len() == children.len(),
            "split sizes {sizes:?} miss children"
        );
        check_weights(sizes)?;
        children.iter().try_for_each(check_sizes)?;
    }
    Ok(())
}

impl Render for PaneGroup {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let root = self.root.clone();
        div()
            .size_full()
            .child(self.render_node(&root, Vec::new(), window, cx))
    }
}

#[cfg(test)]
mod tests {
    use gpui::Axis;

    use super::PaneLayout;

    fn ids(layout: &PaneLayout) -> Vec<u64> {
        let mut out = Vec::new();
        layout.panes(&mut out);
        out
    }

    #[test]
    fn split_nests_or_extends() {
        let mut root = PaneLayout::Pane(1);
        assert!(root.split(1, Axis::Horizontal, 2));
        assert!(root.split(2, Axis::Horizontal, 3));
        let PaneLayout::Split { sizes, .. } = &root else {
            panic!("split")
        };
        assert_eq!(sizes, &[0.5, 0.25, 0.25]);
        assert!(root.split(3, Axis::Vertical, 4));
        assert_eq!(ids(&root), [1, 2, 3, 4]);
        assert!(!root.split(9, Axis::Vertical, 5));
        let mut tiny = PaneLayout::Split {
            axis: Axis::Horizontal,
            sizes: vec![1e-45, 0.0],
            children: vec![PaneLayout::Pane(1), PaneLayout::Pane(2)],
        };
        assert!(tiny.split(1, Axis::Horizontal, 3));
        assert!(tiny.validate().is_ok());
    }

    #[test]
    fn validate_refuses_bad_weights_and_exhausted_ids() {
        let split = |sizes: Vec<f32>| PaneLayout::Split {
            axis: Axis::Horizontal,
            sizes,
            children: vec![PaneLayout::Pane(1), PaneLayout::Pane(2)],
        };
        assert_eq!(split(vec![1.0, 3.0]).validate().unwrap(), 3);
        assert!(split(vec![0.0, 0.0]).validate().is_err());
        assert!(split(vec![-1.0, 2.0]).validate().is_err());
        assert!(split(vec![f32::MAX, f32::MAX]).validate().is_err());
        assert!(PaneLayout::Pane(u64::MAX).validate().is_err());
    }

    #[test]
    fn close_keeps_weights_finite() {
        let three = |sizes: Vec<f32>| PaneLayout::Split {
            axis: Axis::Horizontal,
            sizes,
            children: vec![
                PaneLayout::Pane(1),
                PaneLayout::Pane(2),
                PaneLayout::Pane(3),
            ],
        };
        let sizes = |layout: &PaneLayout| match layout {
            PaneLayout::Split { sizes, .. } => sizes.clone(),
            PaneLayout::Pane(_) => panic!("split"),
        };
        let mut zeros = three(vec![0.0, 0.0, 1.0]);
        assert!(zeros.close(3));
        assert_eq!(sizes(&zeros), [1.0, 1.0]);
        let mut tiny = three(vec![0.0, 0.0, 1e-45]);
        assert!(tiny.close(3));
        assert!(tiny.validate().is_ok());
        let mut huge = three(vec![1e30, 1e30, 1e30]);
        assert!(huge.close(1));
        assert_eq!(sizes(&huge), [0.5, 0.5]);
        let mut kept = three(vec![1.0, 3.0, 4.0]);
        assert!(kept.close(3));
        assert_eq!(sizes(&kept), [0.25, 0.75]);
    }

    #[test]
    fn a_stale_split_path_sets_nothing() {
        let mut root = PaneLayout::Pane(1);
        root.split(1, Axis::Horizontal, 2);
        assert!(!root.set_sizes(&[0], &[0.5, 0.5]), "a leaf");
        assert!(!root.set_sizes(&[4], &[0.5, 0.5]), "past the children");
        assert!(!root.set_sizes(&[], &[1.0]), "a share too few");
        assert!(root.set_sizes(&[], &[0.3, 0.7]));
    }

    #[test]
    fn close_folds_single_children() {
        let mut root = PaneLayout::Pane(1);
        root.split(1, Axis::Horizontal, 2);
        root.split(2, Axis::Vertical, 3);
        assert!(root.close(3));
        assert_eq!(ids(&root), [1, 2]);
        assert!(root.close(2));
        assert_eq!(root, PaneLayout::Pane(1));
    }
}
