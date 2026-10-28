use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    ffi::OsString,
    fs, io,
    path::{Path, PathBuf},
};

use ely_gpui_component::{
    data_display::Tone,
    lists::{GitStatus, Tree, TreeNode},
    menus::{ContextMenu, Menu},
    primitives::IconName,
};
use gpui::{
    AppContext, Context, Entity, IntoElement, ParentElement, Pixels, Point, Render, SharedString,
    Styled, Task, Window,
};

use crate::{git, sessions, settings::SettingsStore, workspace::Workspace};

mod menu;

/// A folder's entry: its name, and whether it is a folder, links followed.
#[derive(Debug, PartialEq)]
struct Entry {
    name: OsString,
    folder: bool,
}

/// A folder's entries, or why it could not be read.
type Listing = Result<Vec<Entry>, String>;

/// Reads `folder`: folders first, then files, each by name ignoring case.
fn list(folder: &Path) -> Listing {
    let read = || -> io::Result<Vec<Entry>> {
        fs::read_dir(folder)?
            .map(|entry| {
                let entry = entry?;
                let kind = entry.file_type()?;
                Ok(Entry {
                    folder: kind.is_dir() || (kind.is_symlink() && entry.path().is_dir()),
                    name: entry.file_name(),
                })
            })
            .collect()
    };
    let mut entries = read().map_err(|error| error.to_string())?;
    entries.sort_by_cached_key(|entry| {
        let name = entry.name.to_string_lossy().to_lowercase();
        (!entry.folder, name, entry.name.clone())
    });
    Ok(entries)
}

/// `path` as marks are keyed: on macOS, whose file systems ignore Unicode forms, composed as Git writes it.
fn mark_key(path: &Path) -> Cow<'_, Path> {
    #[cfg(target_os = "macos")]
    if let Some(text) = path.to_str()
        && !unicode_normalization::is_nfc(text)
    {
        use unicode_normalization::UnicodeNormalization;
        return Cow::Owned(text.nfc().collect::<String>().into());
    }
    Cow::Borrowed(path)
}

/// Changes by their keys, and the folders from `root` down that hold one.
fn marks(
    root: &Path,
    changes: Vec<(PathBuf, GitStatus)>,
) -> (HashMap<PathBuf, GitStatus>, HashSet<PathBuf>) {
    let root = mark_key(root);
    let marks: HashMap<PathBuf, GitStatus> = changes
        .into_iter()
        .map(|(path, status)| (mark_key(&path).into_owned(), status))
        .collect();
    let dirty = marks
        .keys()
        .flat_map(|path| {
            let up = path.ancestors().skip(1);
            up.take_while(|folder| folder.starts_with(&root))
        })
        .map(Path::to_path_buf)
        .collect();
    (marks, dirty)
}

/// Git's mark on a node: its letter and tint; a dot on a folder that holds changes.
fn badge(node: TreeNode, mark: Option<GitStatus>, dirty: bool) -> TreeNode {
    let (note, tone) = match mark {
        Some(GitStatus::Modified) => ("M", Tone::Warning),
        Some(GitStatus::Added) => ("A", Tone::Success),
        Some(GitStatus::Deleted) => ("D", Tone::Danger),
        Some(GitStatus::Untracked) => ("U", Tone::Success),
        Some(GitStatus::Renamed) => ("R", Tone::Info),
        Some(GitStatus::Conflicted) => ("!", Tone::Danger),
        None if dirty => ("•", Tone::Warning),
        None => return node,
    };
    node.note(note).tone(tone)
}

/// `path`'s tree key, one to a path: a path not in UTF-8 adds its bytes after a NUL, which no path holds.
fn key(path: &Path) -> SharedString {
    match path.to_str() {
        Some(text) => text.to_string().into(),
        None => {
            let bytes = path.as_os_str().as_encoded_bytes();
            let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
            format!("{}\0{hex}", path.to_string_lossy()).into()
        }
    }
}

/// The folder a new session starts in: a listed folder, or a file's parent.
pub(super) fn here_folder(path: &Path, folder: bool) -> PathBuf {
    if folder {
        path.to_path_buf()
    } else {
        path.parent().unwrap_or(path).to_path_buf()
    }
}

/// A right press waiting to open Files' menu.
struct MenuAsk {
    n: u64,
    at: Point<Pixels>,
    path: PathBuf,
    here: PathBuf,
}

/// The Files panel: the shown session's folder as a tree, Git's marks on names.
pub struct Files {
    root: PathBuf,
    /// Counts roots, so each root starts a fresh tree.
    generation: usize,
    listings: HashMap<PathBuf, Listing>,
    /// The root and the folders the tree opened.
    wanted: HashSet<PathBuf>,
    marks: HashMap<PathBuf, GitStatus>,
    /// Folders holding a mark.
    dirty: HashSet<PathBuf>,
    reading: Task<()>,
    checking: Task<()>,
    workspace: Option<Entity<Workspace>>,
    menu: Option<MenuAsk>,
}

impl Files {
    pub fn new(root: PathBuf) -> Self {
        Self {
            wanted: HashSet::from([root.clone()]),
            root,
            generation: 0,
            listings: HashMap::new(),
            marks: HashMap::new(),
            dirty: HashSet::new(),
            reading: Task::ready(()),
            checking: Task::ready(()),
            workspace: None,
            menu: None,
        }
    }

    /// The workspace that opens a session from a Files path.
    pub fn with_workspace(mut self, workspace: Entity<Workspace>) -> Self {
        self.workspace = Some(workspace);
        self
    }

    /// Shows `folder`, read afresh with its marks; a new folder starts a new tree.
    pub fn show(&mut self, folder: &Path, cx: &mut Context<Self>) {
        if folder != self.root {
            log::info!("files: the tree shows {}", folder.display());
            let workspace = self.workspace.clone();
            *self = Self {
                generation: self.generation + 1,
                workspace,
                ..Self::new(folder.to_path_buf())
            };
        }
        self.read(cx);
        let root = self.root.clone();
        let changes = cx.background_spawn(async move { git::changes(&root) });
        self.checking = cx.spawn(async move |files, cx| {
            let changes = changes.await;
            files.update(cx, |files, cx| files.mark(changes, cx)).ok();
        });
    }

    fn mark(
        &mut self,
        changes: anyhow::Result<Option<Vec<(PathBuf, GitStatus)>>>,
        cx: &mut Context<Self>,
    ) {
        let changes = match changes {
            Ok(changes) => changes.unwrap_or_default(),
            Err(error) => {
                log::error!("files: no Git marks in {}: {error:#}", self.root.display());
                Vec::new()
            }
        };
        let (marks, dirty) = marks(&self.root, changes);
        if marks != self.marks {
            log::debug!(
                "files: {} Git marks in {}",
                marks.len(),
                self.root.display()
            );
        }
        (self.marks, self.dirty) = (marks, dirty);
        cx.notify();
    }

    /// Reads the root and every folder the tree opened, off the main thread.
    fn read(&mut self, cx: &mut Context<Self>) {
        let folders: Vec<PathBuf> = self.wanted.iter().cloned().collect();
        let read = cx.background_spawn(async move {
            let read = folders.into_iter().map(|folder| (list(&folder), folder));
            read.collect::<Vec<_>>()
        });
        self.reading = cx.spawn(async move |files, cx| {
            let read = read.await;
            files.update(cx, |files, cx| files.store(read, cx)).ok();
        });
    }

    fn store(&mut self, read: Vec<(Listing, PathBuf)>, cx: &mut Context<Self>) {
        let mut changed = false;
        for (listing, folder) in read {
            if self.listings.get(&folder) == Some(&listing) {
                continue;
            }
            match &listing {
                Ok(entries) => {
                    log::debug!("files: {} holds {}", folder.display(), entries.len())
                }
                Err(error) => log::warn!("files: {} is unreadable: {error}", folder.display()),
            }
            self.listings.insert(folder, listing);
            changed = true;
        }
        if changed {
            cx.notify();
        }
    }

    /// The tree opened `folder`, unread so far.
    fn load(&mut self, folder: PathBuf, cx: &mut Context<Self>) {
        if self.wanted.insert(folder) {
            self.read(cx);
        }
    }

    /// Opens the menu for tree key `of` at the pointer.
    fn ask(&mut self, of: &str, at: Point<Pixels>, cx: &mut Context<Self>) {
        match self.named(of) {
            Some((path, folder)) => {
                let n = self.menu.as_ref().map_or(0, |ask| ask.n).saturating_add(1);
                log::info!("files: menu for {}", path.display());
                self.menu = Some(MenuAsk {
                    n,
                    at,
                    here: here_folder(&path, folder),
                    path,
                });
                cx.notify();
            }
            None => log::warn!("files: {of} to open a menu is gone"),
        }
    }

    /// The path a tree key stands for: the root, or an entry read.
    fn path(&self, of: &str) -> Option<PathBuf> {
        self.named(of).map(|(path, _)| path)
    }

    /// The path a tree key stands for, and whether the tree listed it as a folder.
    fn named(&self, of: &str) -> Option<(PathBuf, bool)> {
        if key(&self.root).as_ref() == of {
            return Some((self.root.clone(), true));
        }
        self.listings.iter().find_map(|(folder, listing)| {
            listing.iter().flatten().find_map(|entry| {
                let path = folder.join(&entry.name);
                (key(&path).as_ref() == of).then(|| (path, entry.folder))
            })
        })
    }

    /// Folder `path`'s node, its entries once read; `untracked` when an untracked folder holds it.
    fn folder(&self, path: &Path, label: String, untracked: bool) -> TreeNode {
        let own = |path: &Path| self.marks.get(mark_key(path).as_ref()).copied();
        let mark = own(path).or(untracked.then_some(GitStatus::Untracked));
        let inside = mark == Some(GitStatus::Untracked);
        let node = TreeNode::new(key(path), label).icon(IconName::Folder);
        let node = match self.listings.get(path) {
            None => node.pending(),
            Some(Err(_)) => return node.children([]).note("unreadable").tone(Tone::Danger),
            Some(Ok(entries)) => node.children(entries.iter().map(|entry| {
                let path = path.join(&entry.name);
                let label = sessions::one_line(&entry.name.to_string_lossy());
                if entry.folder {
                    return self.folder(&path, label, inside);
                }
                let file = TreeNode::new(key(&path), label).icon(IconName::File);
                badge(
                    file,
                    own(&path).or(inside.then_some(GitStatus::Untracked)),
                    false,
                )
            })),
        };
        badge(node, mark, self.dirty.contains(mark_key(path).as_ref()))
    }
}

impl Render for Files {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let home = dirs::home_dir();
        let label = match self.root.file_name() {
            Some(name) if home.as_deref() != Some(self.root.as_path()) => {
                sessions::one_line(&name.to_string_lossy())
            }
            _ => sessions::shown(&self.root, home.as_deref()),
        };
        let (load, open, asked) = (cx.entity(), cx.entity(), cx.entity());
        let tree = Tree::new(
            ("files", self.generation),
            [self.folder(&self.root, label, false)],
        )
        .open([key(&self.root)])
        .on_load(move |key, _, cx| {
            load.update(cx, |files, cx| match files.path(key) {
                Some(folder) => files.load(folder, cx),
                None => log::warn!("files: {key} to read is gone"),
            })
        })
        .on_activate(move |key, _, cx| match open.read(cx).path(key) {
            Some(path) => {
                log::info!("files: opening {}", path.display());
                cx.open_with_system(&path)
            }
            None => log::warn!("files: {key} to open is gone"),
        });
        let Some(workspace) = self
            .workspace
            .clone()
            .filter(|_| SettingsStore::get(cx).files_menu)
        else {
            return tree.size_full().into_any_element();
        };
        let tree = tree.on_context(move |key, at, _, cx| {
            asked.update(cx, |files, cx| files.ask(key, at, cx))
        });
        let items = match &self.menu {
            Some(ask) => menu::row(&workspace, ask.path.clone(), ask.here.clone(), cx),
            None => Menu::new(),
        };
        let request = self.menu.as_ref().map(|ask| (ask.n, ask.at));
        ContextMenu::new(("files-menu", self.generation), items)
            .manual(request)
            .size_full()
            .child(tree.size_full())
            .into_any_element()
    }
}

#[cfg(test)]
mod tests;
