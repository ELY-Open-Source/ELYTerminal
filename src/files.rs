use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use ely_gpui_component::{
    data_display::Tone,
    lists::{GitStatus, Tree, TreeNode},
    menus::{ContextMenu, Menu},
    primitives::IconName,
};
use gpui::{
    AppContext, Context, EventEmitter, IntoElement, ParentElement, Pixels, Point, Render,
    SharedString, Styled, Task, Window,
};

use crate::{
    sessions,
    settings::{PanelTab, SettingsStore},
};

mod list;
mod marks;
mod menu;
mod remote;

#[cfg(test)]
use list::Entry;
use list::{Listing, list};
pub use remote::Remote;

use marks::{Marked, mark_key, remark};

/// How often Files reads its open folders again.
const POLL: Duration = Duration::from_secs(1);

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

/// New session here's folder: itself, or a file's parent.
pub(super) fn here_folder(path: &Path, folder: bool) -> PathBuf {
    if folder {
        path.to_path_buf()
    } else {
        path.parent()
            .expect("a listed file lies in a folder")
            .to_path_buf()
    }
}

/// New session here: the folder it starts in.
pub struct Here(pub PathBuf);

/// A right press waiting to open Files' menu.
struct MenuAsk {
    n: u64,
    /// The press's point, until a render hands it on.
    at: Option<Point<Pixels>>,
    path: PathBuf,
    here: PathBuf,
}

/// The Files panel: the shown session's folder as a tree, Git's marks on names.
pub struct Files {
    root: PathBuf,
    /// Counts roots, so each root starts a fresh tree.
    generation: usize,
    listings: HashMap<PathBuf, Listing>,
    /// The folders the tree shows open, the root among them.
    open: HashSet<PathBuf>,
    marks: HashMap<PathBuf, GitStatus>,
    /// Folders holding a mark.
    dirty: HashSet<PathBuf>,
    /// Git's changes as last marked, by path.
    changes: Arc<Vec<(PathBuf, GitStatus)>>,
    /// Why Git last failed, while it fails.
    failing: Option<String>,
    reading: Task<()>,
    /// A read under way, so a poll waits for it.
    busy: bool,
    checking: Task<()>,
    /// A Git check running, and whether another follows it.
    marking: (bool, bool),
    polling: Task<()>,
    menu: Option<MenuAsk>,
    /// The host the root lies on, listed through its ssh.
    remote: Option<Arc<Remote>>,
}

impl EventEmitter<Here> for Files {}

impl Files {
    pub fn new(root: PathBuf) -> Self {
        Self {
            open: HashSet::from([root.clone()]),
            root,
            generation: 0,
            listings: HashMap::new(),
            marks: HashMap::new(),
            dirty: HashSet::new(),
            changes: Arc::default(),
            failing: None,
            reading: Task::ready(()),
            busy: false,
            checking: Task::ready(()),
            marking: (false, false),
            polling: Task::ready(()),
            menu: None,
            remote: None,
        }
    }

    /// Reads open folders and Git's marks each second, shown.
    pub fn poll(&mut self, cx: &mut Context<Self>) {
        self.polling = cx.spawn(async move |files, cx| {
            loop {
                cx.background_executor().timer(POLL).await;
                if files.update(cx, |files, cx| files.tick(cx)).is_err() {
                    break;
                }
            }
        });
    }

    fn tick(&mut self, cx: &mut Context<Self>) {
        let settings = SettingsStore::get(cx);
        let showing = settings.show_panel && settings.panel == PanelTab::Files;
        if !showing {
            return;
        }
        if !self.busy {
            self.read(cx);
        }
        // A change deep in a closed folder moves no listing.
        if !self.marking.0 {
            self.check(cx);
        }
    }

    /// Open folders whose folders above are open too.
    fn shown(&self) -> Vec<PathBuf> {
        let within = |folder: &&PathBuf| {
            let up = folder.ancestors().skip(1);
            up.take_while(|above| above.starts_with(&self.root))
                .all(|above| self.open.contains(above))
        };
        let shown = self.open.iter().filter(within).cloned();
        // The root, closed too, so one gone and back shows.
        let root = std::iter::once(self.root.clone());
        root.chain(shown.filter(|folder| *folder != self.root))
            .collect()
    }

    /// The tree opened or closed `folder`; opened, it reads.
    fn toggle(&mut self, folder: PathBuf, open: bool, cx: &mut Context<Self>) {
        log::debug!(
            "files: {} {}",
            folder.display(),
            if open { "opened" } else { "closed" }
        );
        if open {
            self.open.insert(folder);
            self.read(cx);
        } else {
            self.open.remove(&folder);
        }
    }

    /// Shows `folder`, on `remote`'s host when given, read afresh with its marks; a new folder starts a new tree.
    pub fn show(&mut self, folder: &Path, remote: Option<Arc<Remote>>, cx: &mut Context<Self>) {
        let host = |remote: &Option<Arc<Remote>>| remote.as_ref().map(|remote| remote.host.clone());
        if folder != self.root || host(&remote) != host(&self.remote) {
            let on = host(&remote).map_or(String::new(), |host| format!(" on {host}"));
            log::info!("files: the tree shows {}{on}", folder.display());
            let polling = std::mem::replace(&mut self.polling, Task::ready(()));
            *self = Self {
                generation: self.generation + 1,
                polling,
                ..Self::new(folder.to_path_buf())
            };
        }
        self.remote = remote;
        self.read(cx);
        self.check(cx);
    }

    /// The host the tree lies on, when another.
    pub fn remote(&self) -> Option<&Arc<Remote>> {
        self.remote.as_ref()
    }

    /// Reads Git's marks in the background, one at a time.
    fn check(&mut self, cx: &mut Context<Self>) {
        // Git runs on this host alone.
        if self.remote.is_some() {
            return;
        }
        if self.marking.0 {
            self.marking.1 = true;
            return;
        }
        self.marking = (true, false);
        let (root, last) = (self.root.clone(), self.changes.clone());
        let marked = cx.background_spawn(async move { remark(&root, &last) });
        self.checking = cx.spawn(async move |files, cx| {
            let marked = marked.await;
            files.update(cx, |files, cx| files.mark(marked, cx)).ok();
        });
    }

    /// Wears new marks, then runs a check asked meanwhile.
    fn mark(&mut self, marked: Result<Option<Marked>, String>, cx: &mut Context<Self>) {
        let root = self.root.display();
        // A failure keeps the last marks, logged once.
        let marked = match marked {
            Err(why) => {
                if self.failing.as_ref() != Some(&why) {
                    log::error!("files: no Git marks in {root}, the last kept: {why}");
                }
                self.failing = Some(why);
                None
            }
            Ok(marked) => {
                if self.failing.take().is_some() {
                    log::info!("files: Git marks {root} again");
                }
                marked
            }
        };
        if let Some((changes, marks, dirty)) = marked {
            log::debug!(
                "files: {} Git marks in {}",
                marks.len(),
                self.root.display()
            );
            (self.changes, self.marks, self.dirty) = (changes, marks, dirty);
            cx.notify();
        }
        let (_, again) = std::mem::take(&mut self.marking);
        if again {
            self.check(cx);
        }
    }

    /// Reads the root and open folders off the main thread.
    fn read(&mut self, cx: &mut Context<Self>) {
        let (folders, remote) = (self.shown(), self.remote.clone());
        let read = cx.background_spawn(async move {
            let listed = |folder: &Path| match &remote {
                Some(remote) => remote.list(folder),
                None => list(folder),
            };
            let read = folders.into_iter().map(|folder| (listed(&folder), folder));
            read.collect::<Vec<_>>()
        });
        self.busy = true;
        self.reading = cx.spawn(async move |files, cx| {
            let read = read.await;
            files
                .update(cx, |files, cx| {
                    files.busy = false;
                    files.store(read, cx);
                })
                .ok();
        });
    }

    /// Keeps what was read.
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
            self.prune();
            cx.notify();
        }
    }

    /// Drops each folder, open or read before, that is gone.
    fn prune(&mut self) {
        let mut gone: Vec<PathBuf> = (self.open.iter().chain(self.listings.keys()))
            .filter(|folder| self.gone(folder))
            .cloned()
            .collect();
        gone.sort();
        gone.dedup();
        for folder in &gone {
            log::debug!("files: {} is gone", folder.display());
            self.open.remove(folder);
            self.listings.remove(folder);
        }
    }

    /// Whether a listing no longer names `folder` or one above.
    fn gone(&self, folder: &Path) -> bool {
        let mut up = folder.ancestors().take_while(|path| *path != self.root);
        up.any(|path| {
            let (Some(parent), Some(name)) = (path.parent(), path.file_name()) else {
                return false;
            };
            match self.listings.get(parent) {
                Some(Ok(entries)) => !entries.iter().any(|e| e.folder && e.name == name),
                // An unreadable folder, a deleted root too, shows nothing.
                Some(Err(_)) => true,
                None => false,
            }
        })
    }

    /// The tree opened `folder`, unread so far.
    fn load(&mut self, folder: PathBuf, cx: &mut Context<Self>) {
        self.open.insert(folder);
        self.read(cx);
    }

    /// The open folders' keys, so a remounted tree opens them.
    fn open_keys(&self) -> Vec<SharedString> {
        let mut keys: Vec<SharedString> = self.open.iter().map(|folder| key(folder)).collect();
        keys.sort();
        keys
    }

    /// Opens the menu for tree key `of` at the pointer.
    fn ask(&mut self, of: &str, at: Point<Pixels>, cx: &mut Context<Self>) {
        match self.named(of) {
            Some((path, folder)) => {
                let n = self.menu.as_ref().map_or(0, |ask| ask.n).saturating_add(1);
                log::info!("files: menu for {}", path.display());
                self.menu = Some(MenuAsk {
                    n,
                    at: Some(at),
                    here: here_folder(&path, folder),
                    path,
                });
                cx.notify();
            }
            None => log::warn!("files: {of} to open a menu is gone"),
        }
    }

    /// The menu's request, once: a remounted menu would replay it.
    fn request(&mut self) -> Option<(u64, Point<Pixels>)> {
        let ask = self.menu.as_mut()?;
        Some((ask.n, ask.at.take()?))
    }

    /// The path a tree key stands for: the root, or an entry read.
    pub fn path(&self, of: &str) -> Option<PathBuf> {
        self.named(of).map(|(path, _)| path)
    }

    /// A tree key's path, and whether it is a folder.
    fn named(&self, of: &str) -> Option<(PathBuf, bool)> {
        if key(&self.root).as_ref() == of {
            return Some((self.root.clone(), true));
        }
        self.listings.iter().find_map(|(folder, listing)| {
            listing.iter().flatten().find_map(|entry| {
                let path = folder.join(&entry.name);
                (key(&path).as_ref() == of).then_some((path, entry.folder))
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
        let toggled = cx.entity();
        let tree = Tree::new(
            ("files", self.generation),
            [self.folder(&self.root, label, false)],
        )
        .open(self.open_keys())
        .on_load(move |key, _, cx| {
            load.update(cx, |files, cx| match files.path(key) {
                Some(folder) => files.load(folder, cx),
                None => log::warn!("files: {key} to read is gone"),
            })
        })
        .on_toggle(move |key, open, cx| {
            toggled.update(cx, |files, cx| match files.path(key) {
                Some(folder) => files.toggle(folder, open, cx),
                None => log::warn!("files: {key} to open or close is gone"),
            })
        })
        .on_activate(move |key, _, cx| match open.read(cx).path(key) {
            Some(path) if open.read(cx).remote.is_some() => {
                log::info!(
                    "files: {} lies on another host; it opens nowhere here",
                    path.display()
                )
            }
            Some(path) => {
                log::info!("files: opening {}", path.display());
                cx.open_with_system(&path)
            }
            None => log::warn!("files: {key} to open is gone"),
        });
        if !SettingsStore::get(cx).files_menu {
            return tree.size_full().into_any_element();
        }
        let tree = tree
            .on_context(move |key, at, _, cx| asked.update(cx, |files, cx| files.ask(key, at, cx)));
        let items = match &self.menu {
            Some(ask) => {
                let here = self.remote.is_none().then(|| ask.here.clone());
                menu::row(&cx.entity(), ask.path.clone(), here, cx)
            }
            None => Menu::new(),
        };
        let request = self.request();
        ContextMenu::new(("files-menu", self.generation), items)
            .manual(request)
            .size_full()
            .child(tree.size_full())
            .into_any_element()
    }
}

#[cfg(test)]
mod tests;
