use gpui::{Context, EntityId, Focusable, Window};

use super::Workspace;
use crate::term::{State, TerminalEvent};

impl Workspace {
    /// What pane `id`'s terminal reported; only the shown pane moves Files, Git and the title.
    pub(super) fn session_event(
        &mut self,
        id: EntityId,
        event: &TerminalEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some((index, at)) = self.find_pane(id) else {
            return;
        };
        let shown = index == self.active && self.sessions[index].focused == at;
        let session = &mut self.sessions[index];
        let pane = &mut session.panes[at];
        match event {
            TerminalEvent::Title(title) => {
                log::debug!(
                    "app: pane {} of session {} is named {title:?}",
                    at + 1,
                    index + 1
                );
                pane.title.clone_from(title);
            }
            TerminalEvent::Folder(folder) => {
                pane.folder.clone_from(folder);
                pane.reports = true;
                if shown {
                    self.show_folder(cx);
                }
            }
            TerminalEvent::Bell if index != self.active => {
                if !session.bell {
                    log::debug!("app: a bell in hidden session {}", index + 1);
                }
                session.bell = true;
            }
            TerminalEvent::Bell => {}
            // A hidden session that starts waiting marks its row until it is shown.
            TerminalEvent::Shows(Some(State::Blocked)) if index != self.active => {
                log::debug!("app: hidden session {} waits", index + 1);
                session.waiting = true;
            }
            TerminalEvent::Shows(_) | TerminalEvent::Ended(_) => {}
            TerminalEvent::Dragged(key) => {
                if index != self.active {
                    return;
                }
                let terminal = pane.terminal.clone();
                match self.files.read(cx).path(key) {
                    Some(path) => {
                        log::info!("app: {} dragged onto the terminal", path.display());
                        terminal.update(cx, |view, cx| view.type_paths(&[path], cx));
                        window.focus(&terminal.focus_handle(cx), cx);
                    }
                    None => log::warn!("app: a tree drag of {key} types nothing"),
                }
            }
            TerminalEvent::Focused => return self.pane_focused(id, window, cx),
            TerminalEvent::Exited(status) => {
                let pid = pane.terminal.read(cx).pid();
                log::info!("app: pane {pid}'s program ended, {status}");
                self.close_pane(index, at, window, cx);
                return;
            }
        }
        if index == self.active {
            self.retitle(window);
        }
        cx.notify();
    }

    /// A press on notification `tag`: its pane hears of it and shows, unless it asked not to; false when no session here posted it.
    pub fn pressed(&mut self, tag: &str, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let found = self
            .sessions
            .iter()
            .enumerate()
            .find_map(|(index, session)| {
                session.panes.iter().enumerate().find_map(|(at, pane)| {
                    let focus = pane.terminal.update(cx, |view, _| view.pressed(tag))?;
                    Some((index, at, focus))
                })
            });
        let Some((index, at, focus)) = found else {
            return false;
        };
        if focus {
            log::info!(
                "app: notification {tag} shows pane {} of session {}",
                at + 1,
                index + 1
            );
            self.sessions[index].focused = at;
            cx.activate(true);
            window.activate_window();
            self.select(index, window, cx);
        }
        true
    }
}
