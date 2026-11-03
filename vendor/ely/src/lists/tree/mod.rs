mod model;
mod row;
mod select;
#[cfg(all(test, feature = "test-support"))]
mod tests;

use std::{collections::HashSet, rc::Rc};

use gpui::{
    App, Div, ElementId, EntityId, InteractiveElement, IntoElement, ParentElement, Pixels, Point,
    RenderOnce, Role, ScrollStrategy, SharedString, StatefulInteractiveElement, StyleRefinement,
    Styled, UniformListScrollHandle, Window, div, uniform_list,
};

pub(crate) use model::{Children, Shown, rows};
pub use model::{DropAt, TreeNode};
pub use select::TreeSelect;

use self::{
    model::{Move, find, step, toggled},
    row::{Rows, draw},
};
use super::select::{Pick, picked};
use crate::{forms::Editing, layout::seeded::use_seeded, primitives::tab_stop};

type OnKeys = Rc<dyn Fn(&[SharedString], &mut Window, &mut App)>;
type OnKey = Rc<dyn Fn(&SharedString, &mut Window, &mut App)>;
type OnRename = Rc<dyn Fn(&SharedString, &SharedString, &mut Window, &mut App)>;
type OnMove = Rc<dyn Fn(&SharedString, &SharedString, DropAt, &mut Window, &mut App)>;
type OnRow = Rc<dyn Fn(usize, &mut Window, &mut App)>;
type OnPick = Rc<dyn Fn(usize, Pick, &mut Window, &mut App)>;
type OnToggle = Rc<dyn Fn(&SharedString, bool, &mut App)>;
type OnContext = Rc<dyn Fn(&SharedString, Point<Pixels>, &mut Window, &mut App)>;

/// The keyboard's row, where a Shift range starts, the scroll that follows, pending nodes already asked for, where a drag would land, and the node being renamed.
#[derive(Default)]
struct Nav {
    cursor: usize,
    anchor: usize,
    scroll: UniformListScrollHandle,
    asked: HashSet<SharedString>,
    drop: Option<(SharedString, DropAt)>,
    renaming: Option<SharedString>,
}

/// A drag of one tree's node, named by its owner. A drop past the tree's rows takes it wherever the app listens for it.
pub struct TreeDrag {
    owner: EntityId,
    key: SharedString,
    label: SharedString,
}

impl TreeDrag {
    /// The dragged node's key.
    pub fn key(&self) -> &SharedString {
        &self.key
    }

    /// The dragged node's label.
    pub fn label(&self) -> &SharedString {
        &self.label
    }
}

/// Nodes that open and close, drawn a row at a time so large trees stay quick. Right and Left open, close and climb; Up and Down walk; Enter opens a node or activates a leaf; F2 renames. With `multiple`, Cmd and Shift widen the pick. With `checked`, boxes check whole branches. Nodes drag before, after or into others. Give it a height.
#[derive(IntoElement)]
pub struct Tree {
    id: ElementId,
    base: Div,
    nodes: Vec<TreeNode>,
    open: Vec<SharedString>,
    selected: Vec<SharedString>,
    multiple: bool,
    checked: Option<Vec<SharedString>>,
    on_select: Option<OnKeys>,
    on_check: Option<OnKeys>,
    on_activate: Option<OnKey>,
    on_load: Option<OnKey>,
    on_rename: Option<OnRename>,
    on_move: Option<OnMove>,
    on_context: Option<OnContext>,
    on_toggle: Option<OnToggle>,
}

impl Tree {
    pub fn new(id: impl Into<ElementId>, nodes: impl IntoIterator<Item = TreeNode>) -> Self {
        Self {
            id: id.into(),
            base: div(),
            nodes: nodes.into_iter().collect(),
            open: Vec::new(),
            selected: Vec::new(),
            multiple: false,
            checked: None,
            on_select: None,
            on_check: None,
            on_activate: None,
            on_load: None,
            on_rename: None,
            on_move: None,
            on_context: None,
            on_toggle: None,
        }
    }

    /// The nodes open at first; a new list from the owner replaces what was opened since.
    pub fn open(mut self, keys: impl IntoIterator<Item = impl Into<SharedString>>) -> Self {
        self.open = keys.into_iter().map(Into::into).collect();
        self
    }

    pub fn selected(mut self, keys: impl IntoIterator<Item = impl Into<SharedString>>) -> Self {
        self.selected = keys.into_iter().map(Into::into).collect();
        self
    }

    pub fn multiple(mut self) -> Self {
        self.multiple = true;
        self
    }

    pub fn on_select(
        mut self,
        handler: impl Fn(&[SharedString], &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_select = Some(Rc::new(handler));
        self
    }

    /// Shows a box on each row; `keys` are the checked leaves.
    pub fn checked(mut self, keys: impl IntoIterator<Item = impl Into<SharedString>>) -> Self {
        self.checked = Some(keys.into_iter().map(Into::into).collect());
        self
    }

    /// Gets the checked leaves after a box is pressed.
    pub fn on_check(
        mut self,
        handler: impl Fn(&[SharedString], &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_check = Some(Rc::new(handler));
        self
    }

    /// Enter or a double press on a leaf.
    pub fn on_activate(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_activate = Some(Rc::new(handler));
        self
    }

    /// A right press on a row: its key and the pointer.
    pub fn on_context(
        mut self,
        handler: impl Fn(&SharedString, Point<Pixels>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_context = Some(Rc::new(handler));
        self
    }

    /// Each node opened or closed: its key, and whether it is open.
    pub fn on_toggle(mut self, handler: impl Fn(&SharedString, bool, &mut App) + 'static) -> Self {
        self.on_toggle = Some(Rc::new(handler));
        self
    }

    /// Asked once for a pending node's children, the first time it opens.
    pub fn on_load(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_load = Some(Rc::new(handler));
        self
    }

    /// Gets a node's key and its new name, after F2.
    pub fn on_rename(
        mut self,
        handler: impl Fn(&SharedString, &SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_rename = Some(Rc::new(handler));
        self
    }

    /// Gets a dragged node, the node it landed against, and where.
    pub fn on_move(
        mut self,
        handler: impl Fn(&SharedString, &SharedString, DropAt, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_move = Some(Rc::new(handler));
        self
    }
}

impl Styled for Tree {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for Tree {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id.clone();
        let focus = tab_stop((id.clone(), "focus").into(), true, window, cx);
        let opened = use_seeded((id.clone(), "open"), self.open, window, cx);
        let nav = window.use_keyed_state((id.clone(), "nav"), cx, |_, _| Nav::default());
        let editing = window.use_keyed_state((id.clone(), "rename"), cx, |_, _| Editing::default());
        if nav.read(cx).drop.is_some() && !cx.has_active_drag() {
            nav.update(cx, |nav, _| nav.drop = None);
        }
        if nav.read(cx).renaming.is_some() && editing.read(cx).field().is_none() {
            nav.update(cx, |nav, _| nav.renaming = None);
            window.focus(&focus, cx);
        }
        let open: HashSet<SharedString> = opened.read(cx).value.iter().cloned().collect();
        let nodes = Rc::new(self.nodes);
        let shown = Rc::new(rows(&nodes, &open));
        for row in shown.iter().filter(|row| row.shown == Shown::Loading) {
            let key = shown[row.parent.expect("a wait has a parent")]
                .key()
                .cloned()
                .expect("a node");
            if nav.read(cx).asked.contains(&key) {
                continue;
            }
            nav.update(cx, |nav, _| nav.asked.insert(key.clone()));
            if let Some(on_load) = self.on_load.clone() {
                log::info!("tree {id:?}: asking for {key}'s children");
                window.defer(cx, move |window, cx| on_load(&key, window, cx));
            }
        }
        let count = shown.len();
        let at = nav.read(cx).cursor.min(count.saturating_sub(1));
        let keys: Rc<[SharedString]> = shown
            .iter()
            .map(|row| row.key().cloned().unwrap_or_default())
            .collect();
        let selected = Rc::new(self.selected);
        let (multiple, checked) = (self.multiple, self.checked.map(Rc::new));
        let toggle: OnToggle = {
            let (opened, id, on_toggle) = (opened.clone(), id.clone(), self.on_toggle);
            Rc::new(move |key, open, cx| {
                log::info!(
                    "tree {id:?}: {} {key}",
                    if open { "opened" } else { "closed" }
                );
                opened.update(cx, |seeded, cx| {
                    seeded.value.retain(|known| known != key);
                    if open {
                        seeded.value.push(key.clone());
                    }
                    cx.notify();
                });
                if let Some(on_toggle) = &on_toggle {
                    on_toggle(key, open, cx);
                }
            })
        };
        let pick: OnPick = {
            let (nav, keys, selected, on_select) =
                (nav.clone(), keys.clone(), selected.clone(), self.on_select);
            Rc::new(move |ix, how, window, cx| {
                if keys[ix].is_empty() {
                    return;
                }
                let how = match (multiple, how) {
                    (true, how) => how,
                    (false, Pick::All) => return,
                    (false, _) => Pick::One,
                };
                let next: Vec<_> = picked(&keys, &selected, nav.read(cx).anchor, ix, how)
                    .into_iter()
                    .filter(|key| !key.is_empty())
                    .collect();
                nav.update(cx, |nav, cx| {
                    nav.cursor = ix;
                    if how != Pick::Range {
                        nav.anchor = ix;
                    }
                    cx.notify();
                });
                if let Some(on_select) = &on_select {
                    on_select(&next, window, cx);
                }
            })
        };
        let check: Option<OnKey> = checked.clone().map(|checked| {
            let (nodes, on_check) = (nodes.clone(), self.on_check);
            Rc::new(
                move |key: &SharedString, window: &mut Window, cx: &mut App| {
                    let node = find(&nodes, key).expect("a checked row is a node");
                    let next = toggled(node, &checked);
                    if let Some(on_check) = &on_check {
                        on_check(&next, window, cx);
                    }
                },
            ) as OnKey
        });
        let rename: OnRow = {
            let (nav, editing, shown, on_rename) =
                (nav.clone(), editing.clone(), shown.clone(), self.on_rename);
            Rc::new(move |ix, window, cx| {
                let (Some(on_rename), Shown::Node { key, label, .. }) =
                    (on_rename.clone(), &shown[ix].shown)
                else {
                    return;
                };
                let key = key.clone();
                nav.update(cx, |nav, _| nav.renaming = Some(key.clone()));
                editing.update(cx, |editing, _| {
                    editing.on_commit = Some(Rc::new(move |name, window, cx| {
                        on_rename(&key, name, window, cx)
                    }))
                });
                Editing::begin(&editing, label.to_string(), window, cx);
            })
        };
        let activate: OnRow = {
            let (shown, toggle, on_activate) = (shown.clone(), toggle.clone(), self.on_activate);
            Rc::new(move |ix, window, cx| match &shown[ix].shown {
                Shown::Node {
                    key,
                    opens: true,
                    open,
                    ..
                } => toggle(key, !open, cx),
                Shown::Node { key, .. } => {
                    if let Some(on_activate) = &on_activate {
                        on_activate(key, window, cx);
                    }
                }
                Shown::Loading => {}
            })
        };
        let scroll = nav.read(cx).scroll.clone();
        let data = Rows {
            id: id.clone(),
            owner: nav.entity_id(),
            nodes: nodes.clone(),
            shown: shown.clone(),
            selected,
            checked,
            current: focus.is_focused(window).then_some(at),
            nav: nav.clone(),
            editing: editing.clone(),
            focus: focus.clone(),
            pick: pick.clone(),
            toggle: toggle.clone(),
            check: check.clone(),
            activate: activate.clone(),
            on_move: self.on_move,
            on_context: self.on_context,
        };
        let list = uniform_list((id.clone(), "rows"), count, move |range, window, cx| {
            draw(&data, range, window, cx)
        })
        .track_scroll(&scroll)
        .size_full();
        self.base
            .id(self.id)
            .role(Role::Tree)
            .track_focus(&focus)
            .on_key_down(move |event, window, cx| {
                if count == 0 || nav.read(cx).renaming.is_some() {
                    return;
                }
                let at = nav.read(cx).cursor.min(count - 1);
                let held = &event.keystroke.modifiers;
                let key = event.keystroke.key.as_str();
                match key {
                    "f2" => rename(at, window, cx),
                    "enter" => activate(at, window, cx),
                    "space" => match (&check, &shown[at].shown) {
                        (Some(check), Shown::Node { key, .. }) => check(key, window, cx),
                        _ => pick(at, Pick::Toggle, window, cx),
                    },
                    "a" if held.platform => pick(at, Pick::All, window, cx),
                    _ => match step(&shown, at, key) {
                        Some(Move::To(to)) => {
                            pick(
                                to,
                                if held.shift { Pick::Range } else { Pick::One },
                                window,
                                cx,
                            );
                            let strategy = if to < at {
                                ScrollStrategy::Top
                            } else {
                                ScrollStrategy::Bottom
                            };
                            nav.read(cx).scroll.scroll_to_item(to, strategy);
                        }
                        Some(Move::Open(key)) => toggle(&key, true, cx),
                        Some(Move::Close(key)) => toggle(&key, false, cx),
                        None => return,
                    },
                }
                cx.stop_propagation();
            })
            .child(list)
    }
}
