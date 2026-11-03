use std::rc::Rc;

use gpui::{App, ElementId, IntoElement, RenderOnce, SharedString, Styled, Window};

use super::{
    gesture::{OnKeys, OnPair},
    shape::Shape,
};
use crate::lists::{DropAt, Tree, TreeNode};

/// The keys in paint order, bottom first, after `dragged` lands `at` `target` in a list drawn top first; none, logged, when either left the list.
pub(crate) fn restacked(
    order: &[SharedString],
    dragged: &SharedString,
    target: &SharedString,
    at: DropAt,
) -> Option<Vec<SharedString>> {
    let mut rest: Vec<SharedString> = order
        .iter()
        .filter(|key| *key != dragged)
        .cloned()
        .collect();
    let under = rest.iter().position(|key| key == target);
    let (true, Some(under)) = (order.contains(dragged), under) else {
        log::error!("layer panel: {dragged} or {target} left the layers; nothing moves");
        return None;
    };
    let ix = match at {
        DropAt::Before => under + 1,
        DropAt::After => under,
        DropAt::Inside => panic!("layer {target} holds no layers"),
    };
    rest.insert(ix, dragged.clone());
    Some(rest)
}

/// The canvas's layers, topmost first. A box shows or hides each, a press selects and Shift or Command adds, F2 renames, and a drag restacks; a locked layer says so. It fills its box, which the host sizes.
#[derive(IntoElement)]
pub struct LayerPanel {
    id: ElementId,
    shapes: Vec<Shape>,
    selected: Vec<SharedString>,
    on_select: Option<OnKeys>,
    on_show: Option<OnKeys>,
    on_rename: Option<OnPair>,
    on_restack: Option<OnKeys>,
}

impl LayerPanel {
    /// `shapes` in paint order, bottom first, as the canvas takes them.
    pub fn new(id: impl Into<ElementId>, shapes: impl IntoIterator<Item = Shape>) -> Self {
        Self {
            id: id.into(),
            shapes: shapes.into_iter().collect(),
            selected: Vec::new(),
            on_select: None,
            on_show: None,
            on_rename: None,
            on_restack: None,
        }
    }

    pub fn selected(mut self, keys: impl IntoIterator<Item = impl Into<SharedString>>) -> Self {
        self.selected = keys.into_iter().map(Into::into).collect();
        self
    }

    pub fn on_select(
        mut self,
        handler: impl Fn(&[SharedString], &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_select = Some(Rc::new(handler));
        self
    }

    /// Gets the keys of the layers shown after a box is pressed.
    pub fn on_show(
        mut self,
        handler: impl Fn(&[SharedString], &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_show = Some(Rc::new(handler));
        self
    }

    pub fn on_rename(
        mut self,
        handler: impl Fn(&SharedString, &SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_rename = Some(Rc::new(handler));
        self
    }

    /// Gets every key in its new paint order, bottom first.
    pub fn on_restack(
        mut self,
        handler: impl Fn(&[SharedString], &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_restack = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for LayerPanel {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let order: Vec<SharedString> = self.shapes.iter().map(|shape| shape.key.clone()).collect();
        let shown: Vec<SharedString> = self
            .shapes
            .iter()
            .filter(|shape| !shape.hidden)
            .map(|shape| shape.key.clone())
            .collect();
        let nodes = self.shapes.iter().rev().map(|shape| {
            let node = TreeNode::new(shape.key.clone(), shape.name.clone()).icon(shape.kind.icon());
            if shape.locked {
                node.note("Locked")
            } else {
                node
            }
        });
        let mut tree = Tree::new(self.id.clone(), nodes)
            .selected(self.selected)
            .multiple()
            .checked(shown)
            .size_full();
        if let Some(on_select) = self.on_select {
            tree = tree.on_select(move |keys, window, cx| on_select(keys, window, cx));
        }
        if let Some(on_show) = self.on_show {
            tree = tree.on_check(move |keys, window, cx| {
                log::info!("layer panel: {} shown", keys.len());
                on_show(keys, window, cx)
            });
        }
        if let Some(on_rename) = self.on_rename {
            tree = tree.on_rename(move |key, name, window, cx| on_rename(key, name, window, cx));
        }
        if let Some(on_restack) = self.on_restack {
            tree = tree.on_move(move |dragged, target, at, window, cx| {
                log::info!("layer panel: {dragged} {at:?} {target}");
                if let Some(order) = restacked(&order, dragged, target, at) {
                    on_restack(&order, window, cx);
                }
            });
        }
        tree
    }
}

#[cfg(test)]
mod tests {
    use gpui::SharedString;

    use super::restacked;
    use crate::lists::DropAt;

    fn keys(names: &[&'static str]) -> Vec<SharedString> {
        names.iter().map(|name| SharedString::from(*name)).collect()
    }

    #[test]
    fn a_layer_dropped_above_another_paints_over_it() {
        let order = keys(&["a", "b", "c"]);
        assert_eq!(
            restacked(&order, &"a".into(), &"c".into(), DropAt::Before),
            Some(keys(&["b", "c", "a"]))
        );
        assert_eq!(
            restacked(&order, &"c".into(), &"a".into(), DropAt::After),
            Some(keys(&["c", "a", "b"]))
        );
        assert_eq!(
            restacked(&order, &"a".into(), &"b".into(), DropAt::After),
            Some(keys(&["a", "b", "c"])),
            "just under b is where a lies"
        );
    }

    #[test]
    #[should_panic(expected = "holds no layers")]
    fn a_layer_cannot_land_inside_another() {
        restacked(&keys(&["a", "b"]), &"a".into(), &"b".into(), DropAt::Inside);
    }
}
