use std::time::Instant;

use ely_gpui_component::{
    buttons::{ButtonVariant, IconButton, SegmentedControl},
    i18n::text,
    lists::List,
    motion::Reorder,
    primitives::{FocusScope, IconName},
    shell::{ColumnShell, ResizeBorder, ShellColumn},
    theme::ControlSize,
    typography::Caption,
};
use gpui::{
    Action, AnyElement, App, AppContext, Context, Entity, FocusHandle, Focusable,
    InteractiveElement, IntoElement, KeyBinding, ParentElement, Render, StatefulInteractiveElement,
    StyleRefinement, Styled, Subscription, Task, Window, actions, div, prelude::FluentBuilder,
};

use crate::{
    blur::Blur,
    files::Files,
    git::panel::{Acted, GitPanel},
    menus,
    sessions::{self, Session, orb},
    settings::{PanelTab, SettingsStore, window::OpenSettings},
    term::{Events, Launch, Pty, TerminalView},
};

mod events;
mod rename;
mod reorder;
mod session;
mod toggles;
mod widths;

use toggles::{column_toggle, show_tab};
use widths::{Dragged, Side};

actions!(
    elyterminal,
    [
        ToggleSessions,
        TogglePanel,
        NewSession,
        CloseSession,
        NextSession,
        PreviousSession,
        LastSession
    ]
);

/// Shows the session at this place, from 0; past the last one, the key goes on to the program.
#[derive(Clone, Debug, PartialEq, Action)]
#[action(namespace = elyterminal, no_json)]
pub struct SelectSession(pub usize);

/// The key context of the main window's content.
pub const CONTEXT: &str = "Workspace";

/// The window's title while the program names none.
pub const TITLE: &str = "ELYTerminal";

/// Ghostty's keys: Cmd on macOS; off it, Ctrl-Shift for keys a terminal takes, Alt for digits.
pub fn bind_keys(cx: &mut App) {
    let (sessions, panel, new, close, digit) = if cfg!(target_os = "macos") {
        ("cmd-b", "cmd-alt-b", "cmd-t", "cmd-w", "cmd")
    } else {
        (
            "ctrl-shift-b",
            "ctrl-shift-alt-b",
            "ctrl-shift-t",
            "ctrl-shift-w",
            "alt",
        )
    };
    let context = Some(CONTEXT);
    cx.bind_keys([
        KeyBinding::new(sessions, ToggleSessions, context),
        KeyBinding::new(panel, TogglePanel, context),
        KeyBinding::new(new, NewSession, context),
        KeyBinding::new(close, CloseSession, context),
        KeyBinding::new("ctrl-tab", NextSession, context),
        KeyBinding::new("ctrl-shift-tab", PreviousSession, context),
        KeyBinding::new(&format!("{digit}-9"), LastSession, context),
    ]);
    cx.bind_keys(
        (1..=8).map(|n| KeyBinding::new(&format!("{digit}-{n}"), SelectSession(n - 1), context)),
    );
}

/// The main window's content: sessions, the terminal, and Git or Files, in Ely's columns.
pub struct Workspace {
    focus: FocusHandle,
    sessions: Vec<Session>,
    /// The session shown.
    active: usize,
    files: Entity<Files>,
    git: Entity<GitPanel>,
    /// What the window shows of the desktop, and the blur behind it.
    background: Blur,
    /// Column widths a drag shows before the settings hold them.
    dragged: Dragged,
    /// Draws the window at each step of the status marks while one shown moves.
    ticking: Option<Task<()>>,
    _settings: Subscription,
    _acted: Subscription,
}

impl Workspace {
    /// The window's content with its first session, `pty` started from `launch`.
    pub fn new(
        pty: Pty,
        events: Events,
        launch: Launch,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let git = cx.new(|cx| GitPanel::new(launch.cwd.clone(), window, cx));
        // A change the Git panel made shows in Files' marks too.
        let acted = cx.subscribe(&git, |this, _, _: &Acted, cx| {
            let folder = this.sessions[this.active].folder.clone();
            this.files.update(cx, |files, cx| files.show(&folder, cx));
        });
        let mut workspace = Self {
            focus: cx.focus_handle(),
            sessions: Vec::new(),
            active: 0,
            files: cx.new(|_| Files::new(launch.cwd.clone())),
            git,
            background: Blur::new(window, SettingsStore::get(cx)),
            dragged: Dragged::default(),
            ticking: None,
            _settings: cx.observe_global_in::<SettingsStore>(window, |this, window, cx| {
                this.background.follow(window, SettingsStore::get(cx));
                cx.notify();
            }),
            _acted: acted,
        };
        workspace.add(pty, events, launch, window, cx);
        workspace.select(0, window, cx);
        workspace
    }

    fn add(
        &mut self,
        pty: Pty,
        events: Events,
        launch: Launch,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let terminal = cx.new(|cx| TerminalView::new(pty, events, window, cx));
        let id = terminal.entity_id();
        let events = cx.subscribe_in(&terminal, window, move |this, _, event, window, cx| {
            this.session_event(id, event, window, cx)
        });
        let shell = launch
            .program
            .as_ref()
            .map_or_else(String::new, |(program, _)| program.clone());
        let pid = terminal.read(cx).pid();
        log::info!("app: session {pid} opens in {}", launch.cwd.display());
        self.sessions.push(Session {
            terminal,
            title: String::new(),
            name: String::new(),
            editing: None,
            folder: launch.cwd,
            shell,
            reports: launch.reports,
            bell: false,
            waiting: false,
            look: None,
            _events: events,
        });
    }

    /// Notes each session's mark `now`, and draws the window at each step while one shown moves.
    fn tick(&mut self, now: Instant, cx: &mut Context<Self>) {
        let mut moving = false;
        for session in &mut self.sessions {
            moving |= session.note(now, cx);
        }
        let moving = moving && SettingsStore::get(cx).show_sessions;
        if moving == self.ticking.is_some() {
            return;
        }
        log::debug!("app: status marks {}", if moving { "move" } else { "rest" });
        self.ticking = moving.then(|| {
            cx.spawn(async move |this, cx| {
                loop {
                    cx.background_executor()
                        .timer(orb::wait(Instant::now()))
                        .await;
                    if this.update(cx, |_, cx| cx.notify()).is_err() {
                        break;
                    }
                }
            })
        });
    }

    /// Shows session `index`, clears its bell and its waiting mark, and gives its terminal the keys.
    pub fn select(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if index >= self.sessions.len() {
            return;
        }
        if index != self.active
            && let Some(hidden) = self.sessions.get(self.active)
        {
            hidden.terminal.update(cx, |view, cx| {
                view.let_go(cx);
                view.set_front(false);
            });
        }
        let session = &mut self.sessions[index];
        session.bell = false;
        session.waiting = false;
        session.terminal.update(cx, |view, _| view.set_front(true));
        window.focus(&session.terminal.focus_handle(cx), cx);
        let folder = session.folder.clone();
        self.files.update(cx, |files, cx| files.show(&folder, cx));
        self.git.update(cx, |git, cx| git.show(&folder, cx));
        self.active = index;
        log::debug!("app: session {} shown", index + 1);
        self.retitle(window);
        cx.notify();
    }

    /// Closes session `index`; its program hangs up. The last one closes the window.
    pub fn close(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if index >= self.sessions.len() {
            return;
        }
        let session = self.sessions.remove(index);
        log::info!("app: session {} closes", session.terminal.read(cx).pid());
        if self.sessions.is_empty() {
            log::info!("app: no session left; the window closes");
            window.remove_window();
            return;
        }
        let shown = index == self.active;
        self.active = sessions::after_close(self.active, index, self.sessions.len());
        if shown {
            self.select(self.active, window, cx);
        } else {
            cx.notify();
        }
    }

    /// Its sessions, in their rows' order.
    pub fn sessions(&self) -> &[Session] {
        &self.sessions
    }

    /// What the shown session is called.
    fn title(&self) -> &str {
        self.sessions.get(self.active).map_or(TITLE, Session::named)
    }

    fn retitle(&self, window: &mut Window) {
        let title = self.title();
        if window.window_title() != title {
            log::debug!("app: the title is {title:?}");
            window.set_window_title(title);
        }
    }

    /// Shows the next session, or the one before, round the ends.
    fn step(&mut self, forward: bool, window: &mut Window, cx: &mut Context<Self>) {
        let count = self.sessions.len();
        let by = if forward { 1 } else { count - 1 };
        self.select((self.active + by) % count, window, cx);
    }
}

impl Render for Workspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.background.fit(window);
        let now = Instant::now();
        self.tick(now, cx);
        let settings = SettingsStore::get(cx);
        let workspace = cx.entity();
        // It leads the first column or ends the last.
        let left = column_toggle(
            &workspace,
            ("toggle-sessions", IconName::PanelLeft),
            (
                settings.show_sessions,
                ["et.tip.hide_sessions", "et.tip.show_sessions"],
            ),
            |settings| settings.show_sessions = !settings.show_sessions,
            cx,
        );
        let right = column_toggle(
            &workspace,
            ("toggle-panel", IconName::PanelRight),
            (
                settings.show_panel,
                ["et.tip.hide_panel", "et.tip.show_panel"],
            ),
            |settings| settings.show_panel = !settings.show_panel,
            cx,
        );
        // No menu bar off macOS: a gear opens Settings.
        let gear = (!cfg!(target_os = "macos")).then(|| {
            IconButton::new("open-settings", IconName::Settings)
                .variant(ButtonVariant::Ghost)
                .size(ControlSize::Sm)
                .tooltip(text(cx, "et.settings.title", &[]))
                .on_click(|_, window, cx| window.dispatch_action(Box::new(OpenSettings), cx))
        });
        let end = div()
            .flex()
            .items_center()
            .gap_2()
            .children(gear)
            .child(right);
        let mut shell = ColumnShell::new("workspace").glass();
        let shown = self
            .sessions
            .get(self.active)
            .map(|session| session.terminal.clone());
        let mut terminal = ShellColumn::new("terminal")
            .title(self.title().to_string())
            .children(shown);
        if settings.show_sessions {
            let home = dirs::home_dir();
            let rows: Vec<(String, AnyElement)> = self
                .sessions
                .iter()
                .enumerate()
                .map(|(index, session)| {
                    let active = index == self.active;
                    let pid = session.terminal.read(cx).pid();
                    let row =
                        sessions::row(&workspace, index, session, active, home.as_deref(), now, cx);
                    (pid.to_string(), row.into_any_element())
                })
                .collect();
            // Only a drag needs Reorder; List keeps its own gap.
            let list = if settings.drag_sessions {
                let dragged = workspace.clone();
                rows.into_iter()
                    .fold(
                        Reorder::new("session-rows").on_reorder(move |from, to, _, cx| {
                            dragged.update(cx, |this, cx| this.reorder(from, to, cx))
                        }),
                        |list, (key, row)| list.row(key, div().pb_0p5().child(row)),
                    )
                    .into_any_element()
            } else {
                List::new()
                    .children(rows.into_iter().map(|(_, row)| row))
                    .into_any_element()
            };
            let new = IconButton::new("new-session", IconName::Plus)
                .variant(ButtonVariant::Ghost)
                .size(ControlSize::Sm)
                .tooltip(text(cx, "et.session.new", &[]))
                .on_click(move |_, window, cx| {
                    workspace.update(cx, |this, cx| this.new_session(window, cx))
                });
            shell = shell.column(
                self.side_column(Side::Sessions, cx)
                    .leading(left)
                    .action(new)
                    .child(
                        div()
                            .id("sessions")
                            .size_full()
                            .overflow_y_scroll()
                            .p_2()
                            .child(list),
                    ),
            );
        } else {
            terminal = terminal.leading(left);
        }
        if settings.show_panel {
            let tab = settings.panel;
            let chosen = match tab {
                PanelTab::Git => "git",
                PanelTab::Files => "files",
            };
            let switch = SegmentedControl::new("panel-tab", chosen)
                .segment("git", text(cx, "et.panel.git", &[]), None)
                .segment("files", text(cx, "et.panel.files", &[]), None)
                .size(ControlSize::Sm)
                .on_change(|value, _, cx| show_tab(value, cx));
            let quiet = self
                .sessions
                .get(self.active)
                .is_some_and(|session| !session.reports);
            let caption = || {
                Caption::new(text(cx, "et.panel.no_folder", &[]))
                    .px_3()
                    .pt_2()
            };
            // Hidden, not dropped, so each keeps its state.
            let body = |on: bool| {
                div()
                    .flex_1()
                    .min_h_0()
                    .p_2()
                    .when(!on, |body| body.hidden())
            };
            // Git's list draws every row, so terminal output must not redraw it.
            let fill = || StyleRefinement::default().size_full();
            shell = shell.column(terminal).column(
                self.side_column(Side::Panel, cx)
                    .action(end)
                    // Below the title bar, clear of Windows' caption buttons.
                    .child(div().flex().justify_center().px_2().pt_2().child(switch))
                    .children(quiet.then(caption))
                    .child(body(tab == PanelTab::Git).child(self.git.clone().cached(fill())))
                    .child(body(tab == PanelTab::Files).child(self.files.clone())),
            );
        } else {
            shell = shell.column(terminal.action(end));
        }
        // The context sits above the focus root, so its keys work wherever focus is.
        div()
            .size_full()
            .key_context(CONTEXT)
            .map(menus::window_actions)
            .on_action(cx.listener(|this, _: &ToggleSessions, _, cx| {
                this.toggle(
                    |settings| settings.show_sessions = !settings.show_sessions,
                    cx,
                )
            }))
            .on_action(cx.listener(|this, _: &TogglePanel, _, cx| {
                this.toggle(|settings| settings.show_panel = !settings.show_panel, cx)
            }))
            .on_action(cx.listener(|this, _: &NewSession, window, cx| this.new_session(window, cx)))
            .on_action(
                cx.listener(|this, _: &CloseSession, window, cx| {
                    this.close(this.active, window, cx)
                }),
            )
            .on_action(cx.listener(|this, _: &NextSession, window, cx| this.step(true, window, cx)))
            .on_action(
                cx.listener(|this, _: &PreviousSession, window, cx| this.step(false, window, cx)),
            )
            .on_action(cx.listener(|this, _: &LastSession, window, cx| {
                this.select(this.sessions.len().saturating_sub(1), window, cx)
            }))
            .on_action(cx.listener(|this, select: &SelectSession, window, cx| {
                if select.0 < this.sessions.len() {
                    this.select(select.0, window, cx);
                } else {
                    cx.propagate();
                }
            }))
            .child(
                ResizeBorder::new()
                    .when(self.background.see_through(), ResizeBorder::glass)
                    .child(FocusScope::new(&self.focus).root().size_full().child(shell)),
            )
    }
}
