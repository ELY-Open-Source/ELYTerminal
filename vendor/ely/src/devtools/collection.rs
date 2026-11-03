use std::rc::Rc;

use gpui::{App, ElementId, IntoElement, RenderOnce, SharedString, Styled, Window};

use super::request::Method;
use crate::{
    lists::{DropAt, Tree, TreeNode},
    primitives::IconName,
};

/// A folder of saved requests, or a request with its method.
#[derive(Clone, Debug, PartialEq)]
pub enum Saved {
    Folder {
        key: SharedString,
        name: SharedString,
        items: Vec<Saved>,
    },
    Request {
        key: SharedString,
        name: SharedString,
        method: Method,
    },
}

impl Saved {
    pub fn folder(
        key: impl Into<SharedString>,
        name: impl Into<SharedString>,
        items: impl IntoIterator<Item = Saved>,
    ) -> Self {
        Saved::Folder {
            key: key.into(),
            name: name.into(),
            items: items.into_iter().collect(),
        }
    }

    pub fn request(
        key: impl Into<SharedString>,
        name: impl Into<SharedString>,
        method: Method,
    ) -> Self {
        Saved::Request {
            key: key.into(),
            name: name.into(),
            method,
        }
    }

    fn key(&self) -> &SharedString {
        match self {
            Saved::Folder { key, .. } | Saved::Request { key, .. } => key,
        }
    }

    fn node(&self) -> TreeNode {
        match self {
            Saved::Folder { key, name, items } => TreeNode::new(key.clone(), name.clone())
                .icon(IconName::Folder)
                .children(items.iter().map(Saved::node)),
            Saved::Request { key, name, method } => TreeNode::new(key.clone(), name.clone())
                .icon(IconName::Send)
                .note(method.name()),
        }
    }
}

/// Takes `key` out of `items`, wherever it lies.
fn taken(items: &mut Vec<Saved>, key: &str) -> Option<Saved> {
    if let Some(ix) = items.iter().position(|item| item.key() == key) {
        return Some(items.remove(ix));
    }
    items.iter_mut().find_map(|item| match item {
        Saved::Folder { items, .. } => taken(items, key),
        Saved::Request { .. } => None,
    })
}

/// Sets `item` at `at` of `target`, or hands it back when `target` is not in `items`.
fn placed(items: &mut Vec<Saved>, item: Saved, target: &str, at: DropAt) -> Result<(), Saved> {
    if let Some(ix) = items.iter().position(|each| each.key() == target) {
        match (at, &mut items[ix]) {
            (DropAt::Before, _) => items.insert(ix, item),
            (DropAt::After, _) => items.insert(ix + 1, item),
            (DropAt::Inside, Saved::Folder { items, .. }) => items.push(item),
            (DropAt::Inside, Saved::Request { key, .. }) => {
                log::error!("collection: {key} is a request, and nothing goes inside it");
                return Err(item);
            }
        }
        return Ok(());
    }
    let mut item = item;
    for each in items.iter_mut() {
        if let Saved::Folder { items, .. } = each {
            match placed(items, item, target, at) {
                Ok(()) => return Ok(()),
                Err(back) => item = back,
            }
        }
    }
    Err(item)
}

/// `items` with `key` moved to `at` of `target`, as a drop in the tree asks; none, logged, when either left the items.
pub fn moved(items: &[Saved], key: &str, target: &str, at: DropAt) -> Option<Vec<Saved>> {
    let mut items = items.to_vec();
    let Some(item) = taken(&mut items, key) else {
        log::error!("collection: no {key} to move");
        return None;
    };
    if let Err(item) = placed(&mut items, item, target, at) {
        log::error!("collection: no {target} to move {} beside", item.key());
        return None;
    }
    log::info!("collection: moved {key} {at:?} {target}");
    Some(items)
}

type OnKey = Rc<dyn Fn(&SharedString, &mut Window, &mut App)>;
type OnKeys = Rc<dyn Fn(&[SharedString], &mut Window, &mut App)>;
type OnMove = Rc<dyn Fn(&SharedString, &SharedString, DropAt, &mut Window, &mut App)>;

/// Saved requests in folders, each with its method. Enter or a double press opens a request; with `on_move`, a drag moves one into another folder or among its own, and `moved` applies it. It fills its box, which the host sizes.
#[derive(IntoElement)]
pub struct CollectionTree {
    id: ElementId,
    items: Vec<Saved>,
    open: Vec<SharedString>,
    selected: Vec<SharedString>,
    on_open: Option<OnKey>,
    on_select: Option<OnKeys>,
    on_move: Option<OnMove>,
}

impl CollectionTree {
    pub fn new(id: impl Into<ElementId>, items: impl IntoIterator<Item = Saved>) -> Self {
        Self {
            id: id.into(),
            items: items.into_iter().collect(),
            open: Vec::new(),
            selected: Vec::new(),
            on_open: None,
            on_select: None,
            on_move: None,
        }
    }

    /// The folders open at first.
    pub fn open(mut self, keys: impl IntoIterator<Item = impl Into<SharedString>>) -> Self {
        self.open = keys.into_iter().map(Into::into).collect();
        self
    }

    pub fn selected(mut self, keys: impl IntoIterator<Item = impl Into<SharedString>>) -> Self {
        self.selected = keys.into_iter().map(Into::into).collect();
        self
    }

    /// Gets the request to open.
    pub fn on_open(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_open = Some(Rc::new(handler));
        self
    }

    /// Runs with the request or folder dragged, the row it landed on, and where on that row.
    pub fn on_move(
        mut self,
        handler: impl Fn(&SharedString, &SharedString, DropAt, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_move = Some(Rc::new(handler));
        self
    }

    pub fn on_select(
        mut self,
        handler: impl Fn(&[SharedString], &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_select = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for CollectionTree {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let mut tree = Tree::new(self.id, self.items.iter().map(Saved::node))
            .open(self.open)
            .selected(self.selected)
            .size_full();
        if let Some(on_open) = self.on_open {
            tree = tree.on_activate(move |key, window, cx| {
                log::info!("collection tree: open {key}");
                on_open(key, window, cx)
            });
        }
        if let Some(on_select) = self.on_select {
            tree = tree.on_select(move |keys, window, cx| on_select(keys, window, cx));
        }
        if let Some(on_move) = self.on_move {
            tree = tree
                .on_move(move |key, target, at, window, cx| on_move(key, target, at, window, cx));
        }
        tree
    }
}

#[cfg(test)]
mod tests {
    use super::{Saved, moved};
    use crate::{devtools::Method, lists::DropAt};

    fn keys(items: &[Saved]) -> Vec<String> {
        items
            .iter()
            .flat_map(|item| match item {
                Saved::Folder { key, items, .. } => {
                    let inner = keys(items).join(",");
                    vec![format!("{key}[{inner}]")]
                }
                Saved::Request { key, .. } => vec![key.to_string()],
            })
            .collect()
    }

    #[test]
    fn a_drop_moves_a_request_beside_a_row_or_into_a_folder() {
        let items = vec![
            Saved::folder(
                "a",
                "A",
                [
                    Saved::request("x", "X", Method::Get),
                    Saved::request("y", "Y", Method::Get),
                ],
            ),
            Saved::folder("b", "B", []),
            Saved::request("z", "Z", Method::Post),
        ];
        assert_eq!(
            keys(&moved(&items, "x", "b", DropAt::Inside).expect("both are in the items")),
            ["a[y]", "b[x]", "z"]
        );
        assert_eq!(
            keys(&moved(&items, "z", "a", DropAt::Inside).expect("both are in the items")),
            ["a[x,y,z]", "b[]"],
            "a drop inside a folder lands last"
        );
        assert_eq!(
            keys(&moved(&items, "z", "x", DropAt::Before).expect("both are in the items")),
            ["a[z,x,y]", "b[]"]
        );
        assert_eq!(
            keys(&moved(&items, "x", "z", DropAt::After).expect("both are in the items")),
            ["a[y]", "b[]", "z", "x"]
        );
        assert_eq!(
            keys(&moved(&items, "b", "a", DropAt::Before).expect("both are in the items")),
            ["b[]", "a[x,y]", "z"]
        );
        assert_eq!(moved(&items, "gone", "a", DropAt::Before), None);
        assert_eq!(moved(&items, "x", "gone", DropAt::Before), None);
        assert_eq!(
            moved(&items, "x", "z", DropAt::Inside),
            None,
            "a request holds nothing"
        );
    }
}
