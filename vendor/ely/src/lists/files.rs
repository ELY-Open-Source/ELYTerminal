use std::{collections::BTreeMap, rc::Rc};

use gpui::{
    App, Div, ElementId, IntoElement, ParentElement, RenderOnce, SharedString, StyleRefinement,
    Styled, Window, div,
};

use super::{Tree, TreeNode};
use crate::{
    data_display::Tone,
    forms::{SearchInput, TextInput},
    primitives::{IconName, IconTheme, icon_theme},
    theme::ControlSize,
};

/// A file's state in git.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GitStatus {
    Modified,
    Added,
    Deleted,
    Untracked,
    Renamed,
    Conflicted,
}

impl GitStatus {
    pub(crate) fn word(self) -> &'static str {
        match self {
            Self::Modified => "Modified",
            Self::Added => "Added",
            Self::Deleted => "Deleted",
            Self::Untracked => "Untracked",
            Self::Renamed => "Renamed",
            Self::Conflicted => "Conflicted",
        }
    }

    pub(crate) fn letter(self) -> &'static str {
        match self {
            Self::Modified => "M",
            Self::Added => "A",
            Self::Deleted => "D",
            Self::Untracked => "U",
            Self::Renamed => "R",
            Self::Conflicted => "!",
        }
    }

    pub(crate) fn tone(self) -> Tone {
        match self {
            Self::Modified => Tone::Warning,
            Self::Added | Self::Untracked => Tone::Success,
            Self::Deleted | Self::Conflicted => Tone::Danger,
            Self::Renamed => Tone::Info,
        }
    }
}

/// A folder of the tree being built: its folders and its files, each by name.
#[derive(Default)]
pub(crate) struct Folder {
    pub folders: BTreeMap<String, Folder>,
    pub files: Vec<String>,
}

impl Folder {
    /// Paths with `/` between folders, gathered into the folders that hold them.
    pub(crate) fn of<'a>(paths: impl IntoIterator<Item = &'a str>) -> Self {
        let mut root = Folder::default();
        for path in paths {
            let mut parts: Vec<&str> = path.split('/').collect();
            let file = parts.pop().expect("a path has a name").to_string();
            let folder = parts.iter().fold(&mut root, |folder, part| {
                folder.folders.entry(part.to_string()).or_default()
            });
            folder.files.push(file);
        }
        root
    }
}

/// Whether `name` matches `query`, ignoring case; an empty query matches all.
fn matches(name: &str, query: &str) -> bool {
    query.is_empty() || name.to_lowercase().contains(&query.to_lowercase())
}

/// Paths as nodes, folders first, then files, each by name; with a query, only matches and the folders that hold them, which it opens.
fn nodes(
    paths: &[SharedString],
    status: &[(SharedString, GitStatus)],
    query: &str,
    icons: &IconTheme,
) -> (Vec<TreeNode>, Vec<SharedString>) {
    let root = Folder::of(paths.iter().map(|path| path.as_ref()));
    let state = |path: &str| {
        status
            .iter()
            .find(|(known, _)| known.as_ref() == path)
            .map(|(_, state)| *state)
    };
    fn walk(
        folder: &Folder,
        prefix: &str,
        query: &str,
        state: &dyn Fn(&str) -> Option<GitStatus>,
        open: &mut Vec<SharedString>,
        icons: &IconTheme,
    ) -> (Vec<TreeNode>, bool) {
        let mut out = Vec::new();
        let mut changed = false;
        let mut folders: Vec<_> = folder.folders.iter().collect();
        folders.sort_by_key(|(name, _)| name.to_lowercase());
        for (name, inner) in folders {
            let path = format!("{prefix}{name}");
            let (children, inner_changed) =
                walk(inner, &format!("{path}/"), query, state, open, icons);
            if children.is_empty() && !matches(name, query) {
                continue;
            }
            if !query.is_empty() {
                open.push(path.clone().into());
            }
            changed |= inner_changed;
            let node = TreeNode::new(path, name.clone())
                .icon(IconName::Folder)
                .children(children);
            out.push(if inner_changed {
                node.note("•").tone(Tone::Warning)
            } else {
                node
            });
        }
        let mut files: Vec<_> = folder
            .files
            .iter()
            .filter(|name| matches(name, query))
            .collect();
        files.sort_by_key(|name| name.to_lowercase());
        for name in files {
            let path = format!("{prefix}{name}");
            let node = TreeNode::new(path.clone(), name.clone()).icon(icons.file(name));
            out.push(match state(&path) {
                Some(state) => {
                    changed = true;
                    node.note(state.letter()).tone(state.tone())
                }
                None => node,
            });
        }
        (out, changed)
    }
    let mut open = Vec::new();
    let (nodes, _) = walk(&root, "", query, &state, &mut open, icons);
    (nodes, open)
}

type OnOpen = Rc<dyn Fn(&SharedString, &mut Window, &mut App)>;

/// A project's files as a tree: folders first, an icon for each kind, git status by letter and tint, a dot on folders with changes, and a filter that keeps matches and opens the folders that hold them. Enter or a double press opens a file. Give it a height.
#[derive(IntoElement)]
pub struct FileTree {
    id: ElementId,
    base: Div,
    paths: Vec<SharedString>,
    status: Vec<(SharedString, GitStatus)>,
    on_open: Option<OnOpen>,
}

impl FileTree {
    /// Files by their path from the project's root, with `/` between folders.
    pub fn new(
        id: impl Into<ElementId>,
        paths: impl IntoIterator<Item = impl Into<SharedString>>,
    ) -> Self {
        Self {
            id: id.into(),
            base: div(),
            paths: paths.into_iter().map(Into::into).collect(),
            status: Vec::new(),
            on_open: None,
        }
    }

    pub fn status(mut self, path: impl Into<SharedString>, status: GitStatus) -> Self {
        self.status.push((path.into(), status));
        self
    }

    pub fn on_open(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_open = Some(Rc::new(handler));
        self
    }
}

impl Styled for FileTree {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for FileTree {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let filter = window.use_keyed_state((self.id.clone(), "filter"), cx, |window, cx| {
            TextInput::new(window, cx)
        });
        let picked = window.use_keyed_state((self.id.clone(), "picked"), cx, |_, _| {
            Vec::<SharedString>::new()
        });
        let query = filter.read(cx).text().trim().to_string();
        let (nodes, open) = nodes(&self.paths, &self.status, &query, icon_theme(cx));
        let top: Vec<SharedString> = nodes
            .iter()
            .filter(|node| node.opens())
            .map(|node| node.key.clone())
            .collect();
        let (id, on_open, store) = (self.id.clone(), self.on_open, picked.clone());
        let tree = Tree::new((self.id.clone(), "tree"), nodes)
            .open(if query.is_empty() { top } else { open })
            .selected(picked.read(cx).clone())
            .on_select(move |keys, _, cx| {
                store.update(cx, |picked, cx| {
                    *picked = keys.to_vec();
                    cx.notify();
                })
            })
            .on_activate(move |path, window, cx| {
                log::info!("file tree {id:?}: opened {path}");
                if let Some(on_open) = &on_open {
                    on_open(path, window, cx);
                }
            })
            .flex_1()
            .min_h_0();
        self.base
            .flex()
            .flex_col()
            .gap_2()
            .child(SearchInput::new((self.id, "search"), &filter).size(ControlSize::Sm))
            .child(tree)
    }
}

#[cfg(test)]
mod tests {
    use gpui::SharedString;

    use super::{GitStatus, IconTheme, nodes};
    use crate::lists::tree::{Shown, rows};

    fn paths() -> Vec<SharedString> {
        [
            "src/main.rs",
            "README.md",
            "src/ui/Button.rs",
            "assets/logo.svg",
            "src/app.rs",
            "Cargo.toml",
        ]
        .map(SharedString::from)
        .to_vec()
    }

    /// Labels in row order with everything open, each with its note.
    fn shown(query: &str) -> (Vec<String>, Vec<SharedString>) {
        let status = [
            ("src/ui/Button.rs".into(), GitStatus::Modified),
            ("README.md".into(), GitStatus::Added),
        ];
        let (nodes, open) = nodes(&paths(), &status, query, &IconTheme::default());
        let every = ["assets", "src", "src/ui"]
            .map(SharedString::from)
            .into_iter()
            .collect();
        let labels = rows(&nodes, &every)
            .into_iter()
            .filter_map(|row| match row.shown {
                Shown::Node { label, note, .. } => Some(format!(
                    "{label}{}",
                    note.map(|note| format!(" {note}")).unwrap_or_default()
                )),
                Shown::Loading => None,
            })
            .collect();
        (labels, open)
    }

    #[test]
    fn folders_come_first_then_files_by_name_with_their_status() {
        let (labels, open) = shown("");
        assert_eq!(
            labels,
            [
                "assets",
                "logo.svg",
                "src •",
                "ui •",
                "Button.rs M",
                "app.rs",
                "main.rs",
                "Cargo.toml",
                "README.md A"
            ]
        );
        assert!(open.is_empty());
    }

    #[test]
    fn a_filter_keeps_matches_and_opens_their_folders() {
        let (labels, open) = shown("butt");
        assert_eq!(labels, ["src •", "ui •", "Button.rs M"]);
        assert_eq!(open, ["src/ui", "src"].map(SharedString::from));
    }
}
