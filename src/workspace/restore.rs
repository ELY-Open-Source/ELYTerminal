//! The sessions saved as they settle and opened again at the next start.

use std::time::Duration;

use gpui::{App, Axis, Context, EntityId, Global, Subscription, Task, Window};

use super::{Workspace, groups::Group};
use crate::{
    sessions::{
        Layout, Program,
        saved::{self, Saved, SavedGroup, SavedLayout, SavedSession},
    },
    settings::{SettingsStore, save},
    term::shell::Start,
};

/// How long the sessions rest before they are saved.
const SETTLE: Duration = Duration::from_millis(500);

/// Whether this run keeps its sessions: not one given a folder, a program or a handed console.
pub struct Keeps(pub bool);

impl Global for Keeps {}

/// The sessions last saved, a save waiting for them to rest, and the quit that saves them at once.
pub struct Kept {
    last: Option<Saved>,
    saving: Option<Task<()>>,
    _quit: Subscription,
}

impl Kept {
    /// Nothing kept yet; the app's quit saves what the workspace holds.
    pub fn at_quit(cx: &mut Context<Workspace>) -> Self {
        let quit = cx.on_app_quit(|this, cx| {
            this.flush(cx);
            async {}
        });
        Self {
            last: None,
            saving: None,
            _quit: quit,
        }
    }
}

/// `layout`'s panes as saved; a pane `-e` or Windows started is left out, a split losing a part becomes the other.
fn layout_saved(
    layout: &Layout,
    pane: &impl Fn(EntityId) -> Option<SavedLayout>,
) -> Option<SavedLayout> {
    match layout {
        Layout::Pane(id) => pane(*id),
        Layout::Split(axis, share, first, second) => {
            match (layout_saved(first, pane), layout_saved(second, pane)) {
                (Some(first), Some(second)) => Some(SavedLayout::Split {
                    down: *axis == Axis::Vertical,
                    share: *share,
                    first: Box::new(first),
                    second: Box::new(second),
                }),
                (one, other) => one.or(other),
            }
        }
    }
}

/// `saved` with its panes' terminals `ids`, in their order; a pane that did not start is left out.
fn layout_of(
    saved: &SavedLayout,
    ids: &mut impl Iterator<Item = Option<EntityId>>,
) -> Option<Layout> {
    match saved {
        SavedLayout::Pane { .. } => ids.next().flatten().map(Layout::Pane),
        SavedLayout::Split {
            down,
            share,
            first,
            second,
        } => {
            let axis = if *down {
                Axis::Vertical
            } else {
                Axis::Horizontal
            };
            match (layout_of(first, ids), layout_of(second, ids)) {
                (Some(first), Some(second)) => Some(Layout::Split(
                    axis,
                    *share,
                    Box::new(first),
                    Box::new(second),
                )),
                (one, other) => one.or(other),
            }
        }
    }
}

impl Workspace {
    /// What the next start would open of this window's sessions.
    fn snapshot(&self, cx: &App) -> Saved {
        let mut sessions = Vec::new();
        let mut shown = 0;
        for (index, session) in self.sessions.iter().enumerate() {
            // A session Windows handed over is its launcher's, its splits too.
            if session
                .panes
                .iter()
                .any(|pane| pane.terminal.read(cx).handed())
            {
                continue;
            }
            let pane = |id: EntityId| {
                let pane = session.panes.iter().find(|pane| pane.id() == id)?;
                (!pane.program.written).then(|| SavedLayout::Pane {
                    shell: pane.program.shell.clone(),
                    args: pane.program.args.clone(),
                    folder: pane.folder.clone(),
                })
            };
            let Some(layout) = layout_saved(&session.layout, &pane) else {
                continue;
            };
            if index == self.active {
                shown = sessions.len();
            }
            let order = session.layout.order();
            let kept: Vec<EntityId> = (order.iter().copied())
                .filter(|id| pane(*id).is_some())
                .collect();
            let focused = kept
                .iter()
                .position(|id| *id == session.pane().id())
                .unwrap_or(0);
            sessions.push(SavedSession {
                name: session.name.clone(),
                group: session.group,
                focused,
                layout,
            });
        }
        let groups = (self.groups.iter())
            .filter(|group| {
                sessions
                    .iter()
                    .any(|session| session.group == Some(group.id))
            })
            .map(|group| SavedGroup {
                id: group.id,
                name: group.name.clone(),
            })
            .collect();
        Saved {
            sessions,
            groups,
            shown,
        }
    }

    /// Whether this run keeps its sessions and the settings ask for it.
    fn keeps(cx: &App) -> bool {
        let keeps = cx.try_global::<Keeps>().is_some_and(|keeps| keeps.0);
        keeps && SettingsStore::get(cx).restore_sessions
    }

    /// Saves what the workspace holds at once, as the window closes or the app quits.
    pub(super) fn flush(&mut self, cx: &mut Context<Self>) {
        if !Self::keeps(cx) {
            return;
        }
        let now = self.snapshot(cx);
        self.kept.saving = None;
        match saved::path().and_then(|path| save(&path, &now, "sessions")) {
            Ok(()) => self.kept.last = Some(now),
            Err(error) => log::error!("{error:#}"),
        }
    }

    /// Saves the sessions once they rest after a change, while this run keeps them and the settings ask.
    pub(super) fn keep(&mut self, cx: &mut Context<Self>) {
        if !Self::keeps(cx) {
            return;
        }
        let now = self.snapshot(cx);
        if self.kept.last.as_ref() == Some(&now) {
            return;
        }
        self.kept.last = Some(now.clone());
        self.kept.saving = Some(cx.spawn(async move |_, cx| {
            cx.background_executor().timer(SETTLE).await;
            let written = cx
                .background_executor()
                .spawn(async move { save(&saved::path()?, &now, "sessions") })
                .await;
            if let Err(error) = written {
                log::error!("{error:#}");
            }
        }));
    }

    /// Opens the rest of `saved`, whose first session's first pane this window opened with.
    pub fn restore(&mut self, saved: Saved, window: &mut Window, cx: &mut Context<Self>) {
        self.groups = (saved.groups.iter())
            .map(|group| Group {
                id: group.id,
                name: group.name.clone(),
                editing: None,
            })
            .collect();
        let mut opened = Vec::new();
        for (index, session) in saved.sessions.iter().enumerate() {
            let mut ids = Vec::new();
            for (place, (shell, args, folder)) in session.layout.panes().into_iter().enumerate() {
                if index == 0 && place == 0 {
                    ids.push(Some(self.sessions[0].panes[0].id()));
                    continue;
                }
                let program = Program {
                    shell: shell.to_string(),
                    args: args.to_vec(),
                    written: false,
                };
                let started = match self.started(&program, Start::In(folder.to_path_buf()), cx) {
                    Ok(started) => started,
                    Err(error) => {
                        log::error!(
                            "sessions: a pane of session {} does not start: {error:#}",
                            index + 1
                        );
                        ids.push(None);
                        continue;
                    }
                };
                if ids.iter().all(Option::is_none) {
                    self.add(started, program, window, cx);
                    ids.push(Some(self.sessions[self.sessions.len() - 1].panes[0].id()));
                } else {
                    let pane = self.pane(started, program, window, cx);
                    ids.push(Some(pane.id()));
                    let last = self.sessions.len() - 1;
                    self.sessions[last].panes.push(pane);
                }
            }
            let Some(layout) = layout_of(&session.layout, &mut ids.iter().copied()) else {
                continue;
            };
            let last = self.sessions.len() - 1;
            let restored = &mut self.sessions[last];
            let focused = ids.get(session.focused).copied().flatten();
            restored.focused = (restored.panes.iter())
                .position(|pane| Some(pane.id()) == focused)
                .unwrap_or(0);
            restored.layout = layout;
            restored.name = session.name.clone();
            restored.group = session.group;
            opened.push((index, restored.id));
        }
        log::info!(
            "sessions: {} of {} open again",
            opened.len(),
            saved.sessions.len()
        );
        let shown = (opened.iter())
            .find(|(index, _)| *index == saved.shown)
            .or(opened.first())
            .map(|(_, id)| *id);
        self.arrange();
        let index = (self.sessions.iter())
            .position(|session| Some(session.id) == shown)
            .unwrap_or(0);
        self.select(index, window, cx);
    }
}

#[cfg(test)]
mod tests {
    use gpui::EntityId;

    use super::*;

    #[test]
    fn a_layout_saved_opens_as_it_was_less_the_panes_that_did_not_start() {
        let id = |n: u64| EntityId::from(1 << 32 | n);
        let pane = |n: u64| SavedLayout::Pane {
            shell: format!("sh{n}"),
            args: Vec::new(),
            folder: "/".into(),
        };
        let layout = Layout::Split(
            Axis::Horizontal,
            0.3,
            Box::new(Layout::Pane(id(1))),
            Box::new(Layout::Split(
                Axis::Vertical,
                0.6,
                Box::new(Layout::Pane(id(2))),
                Box::new(Layout::Pane(id(3))),
            )),
        );
        let named = |of: EntityId| (of != id(2)).then(|| pane(of.as_u64() & 0xff));
        let saved = layout_saved(&layout, &named).expect("two panes stay");
        assert_eq!(saved.panes().len(), 2, "-e's pane is left out");
        let back =
            layout_of(&saved, &mut [Some(id(1)), Some(id(3))].into_iter()).expect("both start");
        let want = Layout::Split(
            Axis::Horizontal,
            0.3,
            Box::new(Layout::Pane(id(1))),
            Box::new(Layout::Pane(id(3))),
        );
        assert_eq!(back, want);
        let one = layout_of(&saved, &mut [None, Some(id(3))].into_iter());
        assert_eq!(one, Some(Layout::Pane(id(3))), "a pane that did not start");
    }
}
