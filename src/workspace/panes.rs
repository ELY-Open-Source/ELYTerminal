//! Panes inside the shown session: split, closed, stepped through and drawn.

use ely_gpui_component::theme::ActiveTheme;
use gpui::{
    AnyElement, App, AppContext, Axis, Context, EntityId, Focusable, IntoElement, ParentElement,
    Styled, Window, div, prelude::FluentBuilder,
};

use super::Workspace;
use crate::{
    sessions::{Layout, Pane, Program, Session},
    term::{Events, Launch, Pty, TerminalView, shell::Start},
};

impl Workspace {
    /// A pane of `pty`, started from `launch`, whose reports and focus reach the workspace.
    pub(super) fn pane(
        &mut self,
        (pty, events, launch): (Pty, Events, Launch),
        program: Program,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Pane {
        let terminal = cx.new(|cx| TerminalView::new(pty, events, window, cx));
        let id = terminal.entity_id();
        let events = cx.subscribe_in(&terminal, window, move |this, _, event, window, cx| {
            this.session_event(id, event, window, cx)
        });
        let focus = terminal.focus_handle(cx);
        let focused = cx.on_focus(&focus, window, move |this, window, cx| {
            this.pane_focused(id, window, cx)
        });
        let pid = terminal.read(cx).pid();
        log::info!("app: pane {pid} opens in {}", launch.cwd.display());
        Pane {
            terminal,
            title: String::new(),
            folder: launch.cwd,
            program,
            reports: launch.reports,
            _events: [events, focused],
        }
    }

    /// The session and pane whose terminal is `id`.
    pub(super) fn find_pane(&self, id: EntityId) -> Option<(usize, usize)> {
        self.sessions
            .iter()
            .enumerate()
            .find_map(|(index, session)| {
                let pane = session.panes.iter().position(|pane| pane.id() == id)?;
                Some((index, pane))
            })
    }

    /// Splits the shown pane along `axis`: its program again, in its folder.
    pub(super) fn split(&mut self, axis: Axis, window: &mut Window, cx: &mut Context<Self>) {
        let Some(session) = self.sessions.get(self.active) else {
            return;
        };
        let shown = session.pane();
        let (at, program) = (shown.id(), shown.program.clone());
        let started = match self.started(&program, Start::In(shown.folder.clone()), cx) {
            Ok(started) => started,
            Err(error) => return log::error!("app: no split: {error:#}"),
        };
        let pane = self.pane(started, program, window, cx);
        let (id, focus) = (pane.id(), pane.terminal.focus_handle(cx));
        let session = &mut self.sessions[self.active];
        assert!(
            session.layout.split(at, axis, id),
            "the shown pane is laid out"
        );
        session.panes.push(pane);
        session.focused = session.panes.len() - 1;
        log::info!(
            "app: session {} splits {axis:?}, {} panes",
            self.active + 1,
            session.panes.len()
        );
        window.focus(&focus, cx);
        self.show_folder(cx);
        self.retitle(window);
        cx.notify();
    }

    /// Closes pane `pane` of session `index`; its last pane closes the session.
    pub(super) fn close_pane(
        &mut self,
        index: usize,
        pane: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let session = &mut self.sessions[index];
        if session.panes.len() == 1 {
            return self.close(index, window, cx);
        }
        let gone = session.panes.remove(pane);
        assert!(session.layout.remove(gone.id()), "a laid out pane");
        // Its textures leave the window's atlas with it.
        gone.terminal
            .update(cx, |view, _| view.set_front(false, window));
        let first = session.layout.order()[0];
        let had_keys = pane == session.focused;
        session.focused = match pane.cmp(&session.focused) {
            std::cmp::Ordering::Less => session.focused - 1,
            std::cmp::Ordering::Greater => session.focused,
            std::cmp::Ordering::Equal => session
                .panes
                .iter()
                .position(|pane| pane.id() == first)
                .expect("the layout's pane"),
        };
        log::info!(
            "app: pane {} of session {} closes, {} left",
            gone.terminal.read(cx).pid(),
            index + 1,
            session.panes.len()
        );
        // Only the focused pane's keys move; a find box keeps its own.
        if index == self.active && had_keys {
            let focus = self.sessions[index].terminal().focus_handle(cx);
            window.focus(&focus, cx);
            self.show_folder(cx);
            self.retitle(window);
        }
        cx.notify();
    }

    /// Gives the keys to the shown session's next pane, or the one before, round the ends.
    pub(super) fn step_pane(&mut self, forward: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(session) = self.sessions.get(self.active) else {
            return;
        };
        let order = session.layout.order();
        let at = order
            .iter()
            .position(|id| *id == session.pane().id())
            .expect("the focused pane is laid out");
        let by = if forward { 1 } else { order.len() - 1 };
        let next = order[(at + by) % order.len()];
        let pane = session
            .panes
            .iter()
            .find(|pane| pane.id() == next)
            .expect("a laid out pane");
        window.focus(&pane.terminal.focus_handle(cx), cx);
        // An inactive window hears no focus.
        self.pane_focused(next, window, cx);
    }

    /// A pane's terminal or find box took the keys, or a step gave them.
    pub(super) fn pane_focused(
        &mut self,
        id: EntityId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some((index, pane)) = self.find_pane(id) else {
            return;
        };
        let session = &mut self.sessions[index];
        if index != self.active || session.focused == pane {
            return;
        }
        session.focused = pane;
        log::debug!("app: pane {} of session {} focused", pane + 1, index + 1);
        self.show_folder(cx);
        self.retitle(window);
        cx.notify();
    }

    /// Files and Git show the shown pane's folder.
    pub(super) fn show_folder(&mut self, cx: &mut Context<Self>) {
        let folder = self.sessions[self.active].pane().folder.clone();
        self.files.update(cx, |files, cx| files.show(&folder, cx));
        self.git.update(cx, |git, cx| git.show(&folder, cx));
    }
}

/// The session's panes as its layout splits them; with several, the focused one's border stands out.
pub(super) fn draw(layout: &Layout, session: &Session, cx: &App) -> AnyElement {
    let colors = &cx.theme().colors;
    let many = session.panes.len() > 1;
    match layout {
        Layout::Pane(id) => {
            let pane = session
                .panes
                .iter()
                .find(|pane| pane.id() == *id)
                .expect("a laid out pane");
            let edge = if session.pane().id() == *id {
                colors.border_strong
            } else {
                colors.border
            };
            div()
                .relative()
                .size_full()
                .flex_1()
                .min_w_0()
                .min_h_0()
                .when(many, |pane| pane.border_1().border_color(edge))
                .child(pane.terminal.clone())
                .into_any_element()
        }
        Layout::Split(axis, first, second) => div()
            .flex()
            .size_full()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .when(*axis == Axis::Horizontal, |split| split.flex_row())
            .when(*axis == Axis::Vertical, |split| split.flex_col())
            .child(draw(first, session, cx))
            .child(draw(second, session, cx))
            .into_any_element(),
    }
}
