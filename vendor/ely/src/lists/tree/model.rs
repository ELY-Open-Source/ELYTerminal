use std::collections::HashSet;

use gpui::SharedString;

use crate::{data_display::Tone, forms::CheckState, primitives::IconName};

/// What a node holds under it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Children {
    Leaf,
    Known(Vec<TreeNode>),
    /// Children still to come; opening the node asks the owner for them.
    Pending,
}

/// One node of a tree: its key, its label, an icon, and its children.
#[derive(Clone, Debug, PartialEq)]
pub struct TreeNode {
    pub(crate) key: SharedString,
    pub(crate) label: SharedString,
    pub(crate) icon: Option<IconName>,
    pub(crate) note: Option<SharedString>,
    pub(crate) tone: Option<Tone>,
    pub(crate) children: Children,
}

impl TreeNode {
    /// A leaf, until it gains children.
    pub fn new(key: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            key: key.into(),
            label: label.into(),
            icon: None,
            note: None,
            tone: None,
            children: Children::Leaf,
        }
    }

    /// Quiet text at the row's end, such as a status letter.
    pub fn note(mut self, note: impl Into<SharedString>) -> Self {
        self.note = Some(note.into());
        self
    }

    /// Tints the label and the note.
    pub fn tone(mut self, tone: impl Into<Tone>) -> Self {
        self.tone = Some(tone.into());
        self
    }

    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn child(mut self, node: TreeNode) -> Self {
        match &mut self.children {
            Children::Known(children) => children.push(node),
            _ => self.children = Children::Known(vec![node]),
        }
        self
    }

    /// Makes it a node that opens, even with no children yet.
    pub fn children(mut self, nodes: impl IntoIterator<Item = TreeNode>) -> Self {
        let mut known = match self.children {
            Children::Known(known) => known,
            _ => Vec::new(),
        };
        known.extend(nodes);
        self.children = Children::Known(known);
        self
    }

    /// Children still to come: opening it runs the tree's `on_load` until the owner supplies them.
    pub fn pending(mut self) -> Self {
        self.children = Children::Pending;
        self
    }

    pub(crate) fn opens(&self) -> bool {
        self.children != Children::Leaf
    }

    /// The leaf keys at and under this node.
    pub(crate) fn leaves(&self) -> Vec<SharedString> {
        match &self.children {
            Children::Known(children) => children.iter().flat_map(TreeNode::leaves).collect(),
            _ => vec![self.key.clone()],
        }
    }

    /// Whether `key` is this node or under it.
    pub(crate) fn holds(&self, key: &SharedString) -> bool {
        self.key == *key
            || matches!(&self.children, Children::Known(children) if children.iter().any(|child| child.holds(key)))
    }
}

/// What a row shows: a node, or the wait for a pending node's children.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Shown {
    Node {
        key: SharedString,
        label: SharedString,
        icon: Option<IconName>,
        note: Option<SharedString>,
        tone: Option<Tone>,
        opens: bool,
        open: bool,
    },
    Loading,
}

/// One visible row: what it shows, how deep it sits, and its parent's row.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Row {
    pub shown: Shown,
    pub depth: usize,
    pub parent: Option<usize>,
}

impl Row {
    pub(crate) fn key(&self) -> Option<&SharedString> {
        match &self.shown {
            Shown::Node { key, .. } => Some(key),
            Shown::Loading => None,
        }
    }
}

/// The rows in view, depth first, through the open nodes.
pub(crate) fn rows(nodes: &[TreeNode], open: &HashSet<SharedString>) -> Vec<Row> {
    fn walk(
        nodes: &[TreeNode],
        open: &HashSet<SharedString>,
        depth: usize,
        parent: Option<usize>,
        out: &mut Vec<Row>,
    ) {
        for node in nodes {
            let here = out.len();
            let is_open = node.opens() && open.contains(&node.key);
            out.push(Row {
                shown: Shown::Node {
                    key: node.key.clone(),
                    label: node.label.clone(),
                    icon: node.icon,
                    note: node.note.clone(),
                    tone: node.tone,
                    opens: node.opens(),
                    open: is_open,
                },
                depth,
                parent,
            });
            match (&node.children, is_open) {
                (Children::Known(children), true) => {
                    walk(children, open, depth + 1, Some(here), out)
                }
                (Children::Pending, true) => out.push(Row {
                    shown: Shown::Loading,
                    depth: depth + 1,
                    parent: Some(here),
                }),
                _ => {}
            }
        }
    }
    let mut out = Vec::new();
    walk(nodes, open, 0, None, &mut out);
    out
}

/// What a key does on row `at`.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Move {
    To(usize),
    Open(SharedString),
    Close(SharedString),
}

/// Up, Down, Home and End walk the rows past any still loading; Right opens a node or enters it, Left closes it or goes to its parent.
pub(crate) fn step(rows: &[Row], at: usize, key: &str) -> Option<Move> {
    let row = rows.get(at)?;
    let node = |ix: &usize| rows[*ix].shown != Shown::Loading;
    let (opens, open, node_key) = match &row.shown {
        Shown::Node {
            key, opens, open, ..
        } => (*opens, *open, Some(key.clone())),
        Shown::Loading => (false, false, None),
    };
    match key {
        "down" => (at + 1..rows.len()).find(node).map(Move::To),
        "up" => (0..at).rev().find(node).map(Move::To),
        "home" => (0..rows.len()).find(node).map(Move::To),
        "end" => (0..rows.len()).rev().find(node).map(Move::To),
        "right" if opens && !open => node_key.map(Move::Open),
        "right" if open => Some(at + 1)
            .filter(|next| rows.get(*next).is_some_and(|child| child.depth > row.depth))
            .filter(node)
            .map(Move::To),
        "left" if open => node_key.map(Move::Close),
        "left" => row.parent.map(Move::To),
        _ => None,
    }
}

/// A node's box: on when all its leaves are, mixed when some are.
pub(crate) fn check_state(node: &TreeNode, checked: &[SharedString]) -> CheckState {
    let leaves = node.leaves();
    let on = leaves.iter().filter(|leaf| checked.contains(leaf)).count();
    match on {
        0 => CheckState::Off,
        all if all == leaves.len() => CheckState::On,
        _ => CheckState::Mixed,
    }
}

/// The checked leaves after pressing a node's box: a full node empties, any other fills.
pub(crate) fn toggled(node: &TreeNode, checked: &[SharedString]) -> Vec<SharedString> {
    let leaves = node.leaves();
    let fill = check_state(node, checked) != CheckState::On;
    let mut next: Vec<SharedString> = checked
        .iter()
        .filter(|key| !leaves.contains(key))
        .cloned()
        .collect();
    if fill {
        next.extend(leaves);
    }
    next
}

/// Finds a node by key.
pub(crate) fn find<'a>(nodes: &'a [TreeNode], key: &SharedString) -> Option<&'a TreeNode> {
    nodes.iter().find_map(|node| {
        if node.key == *key {
            return Some(node);
        }
        match &node.children {
            Children::Known(children) => find(children, key),
            _ => None,
        }
    })
}

/// Where a dragged node lands against a row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DropAt {
    Before,
    Inside,
    After,
}

/// The place a pointer `share` of the way down a row picks: the edges for siblings, the middle of a node that opens for inside.
pub(crate) fn place_at(share: f32, opens: bool) -> DropAt {
    match (share, opens) {
        (share, true) if (0.25..0.75).contains(&share) => DropAt::Inside,
        (share, _) if share < 0.5 => DropAt::Before,
        _ => DropAt::After,
    }
}

/// Whether moving `dragged` against `target` keeps the tree whole: a node cannot land in itself or under itself.
pub(crate) fn can_drop(nodes: &[TreeNode], dragged: &SharedString, target: &SharedString) -> bool {
    find(nodes, dragged).is_some_and(|node| !node.holds(target))
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use gpui::SharedString;

    use super::*;

    fn tree() -> Vec<TreeNode> {
        vec![
            TreeNode::new("src", "src").children([
                TreeNode::new("lib", "lib.rs"),
                TreeNode::new("ui", "ui").children([
                    TreeNode::new("button", "button.rs"),
                    TreeNode::new("menu", "menu.rs"),
                ]),
            ]),
            TreeNode::new("docs", "docs").pending(),
            TreeNode::new("readme", "README.md"),
        ]
    }

    fn keys(list: &[&'static str]) -> Vec<SharedString> {
        list.iter().map(|key| SharedString::from(*key)).collect()
    }

    fn open(list: &[&'static str]) -> HashSet<SharedString> {
        keys(list).into_iter().collect()
    }

    #[test]
    fn an_empty_folder_still_opens() {
        assert!(TreeNode::new("empty", "empty").children([]).opens());
        assert!(!TreeNode::new("file", "file").opens());
    }

    #[test]
    fn rows_walk_only_through_open_nodes() {
        let nodes = tree();
        let shown: Vec<_> = rows(&nodes, &open(&["src", "docs"]))
            .iter()
            .map(|row| (row.key().cloned(), row.depth, row.parent))
            .collect();
        assert_eq!(
            shown,
            [
                (Some("src".into()), 0, None),
                (Some("lib".into()), 1, Some(0)),
                (Some("ui".into()), 1, Some(0)),
                (Some("docs".into()), 0, None),
                (None, 1, Some(3)),
                (Some("readme".into()), 0, None),
            ]
        );
    }

    #[test]
    fn keys_walk_open_close_and_climb() {
        let nodes = tree();
        let shown = rows(&nodes, &open(&["src"]));
        assert_eq!(
            step(&shown, 0, "right"),
            Some(Move::To(1)),
            "an open node enters its first child"
        );
        assert_eq!(step(&shown, 2, "right"), Some(Move::Open("ui".into())));
        assert_eq!(step(&shown, 0, "left"), Some(Move::Close("src".into())));
        assert_eq!(
            step(&shown, 1, "left"),
            Some(Move::To(0)),
            "a child climbs to its parent"
        );
        assert_eq!(step(&shown, 1, "right"), None, "a leaf has nowhere to go");
        assert_eq!(step(&shown, 4, "down"), None);
        assert_eq!(step(&shown, 3, "end"), Some(Move::To(4)));
    }

    #[test]
    fn keys_pass_over_a_loading_row() {
        let nodes = tree();
        let shown = rows(&nodes, &open(&["docs"]));
        assert_eq!(
            step(&shown, 1, "down"),
            Some(Move::To(3)),
            "down skips the loading row"
        );
        assert_eq!(step(&shown, 3, "up"), Some(Move::To(1)));
        assert_eq!(
            step(&shown, 1, "right"),
            None,
            "nothing to enter while it loads"
        );
        let waiting = rows(&[TreeNode::new("docs", "docs").pending()], &open(&["docs"]));
        assert_eq!(
            step(&waiting, 0, "end"),
            Some(Move::To(0)),
            "end stops on the last node"
        );
    }

    #[test]
    fn boxes_mix_and_fill_by_their_leaves() {
        let nodes = tree();
        let src = &nodes[0];
        assert_eq!(check_state(src, &keys(&["lib"])), CheckState::Mixed);
        assert_eq!(
            check_state(src, &keys(&["lib", "button", "menu"])),
            CheckState::On
        );
        assert_eq!(check_state(src, &keys(&["readme"])), CheckState::Off);
        assert_eq!(
            toggled(src, &keys(&["lib", "readme"])),
            keys(&["readme", "lib", "button", "menu"])
        );
        assert_eq!(
            toggled(src, &keys(&["lib", "button", "menu", "readme"])),
            keys(&["readme"])
        );
    }

    #[test]
    fn a_drop_picks_its_place_and_refuses_its_own_subtree() {
        assert_eq!(place_at(0.1, true), DropAt::Before);
        assert_eq!(place_at(0.5, true), DropAt::Inside);
        assert_eq!(place_at(0.9, true), DropAt::After);
        assert_eq!(place_at(0.4, false), DropAt::Before);
        assert_eq!(place_at(0.6, false), DropAt::After);
        let nodes = tree();
        assert!(!can_drop(&nodes, &"src".into(), &"button".into()));
        assert!(!can_drop(&nodes, &"ui".into(), &"ui".into()));
        assert!(can_drop(&nodes, &"button".into(), &"readme".into()));
    }
}
