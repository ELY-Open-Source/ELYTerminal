use std::{
    path::{MAIN_SEPARATOR, Path, PathBuf},
    time::Instant,
};

use ely_gpui_component::{
    buttons::{ButtonVariant, IconButton},
    forms::{Input, TextInput},
    i18n::{I18n, text},
    lists::ListItem,
    menus::ContextMenu,
    primitives::{Icon, IconName, Tooltip},
    theme::{ActiveTheme, ControlSize, IconSize},
};
use gpui::{
    AnyElement, App, Entity, IntoElement, MouseButton, SharedString, Subscription, div, prelude::*,
    rems,
};

use crate::{
    settings::SettingsStore,
    term::{Kind, Record, Records, State, TerminalView},
    workspace::{TITLE, Workspace},
};

mod menu;
pub mod orb;

use orb::Look;

/// A program on its own terminal, as the sessions column lists it.
pub struct Session {
    pub terminal: Entity<TerminalView>,
    /// The program's title; empty while it names none.
    pub title: String,
    /// The name the user gave it; empty leaves the title to its program.
    pub name: String,
    /// The field its row shows while the user renames it.
    pub editing: Option<(Entity<TextInput>, Subscription)>,
    /// The folder it reports, else the one it started in.
    pub folder: PathBuf,
    /// The program it runs, as its path; empty for the login shell.
    pub shell: String,
    /// Its shell has integration, or it reported a folder anyway.
    pub reports: bool,
    /// A bell rang while it was hidden.
    pub bell: bool,
    /// It started waiting on the user while hidden.
    pub waiting: bool,
    /// The look its mark shows, and since when.
    pub look: Option<(Look, Instant)>,
    pub _events: Subscription,
}

impl Session {
    /// What it is called: the user's name, else its program's title, else the app's name.
    pub fn named(&self) -> &str {
        [self.name.as_str(), self.title.as_str()]
            .into_iter()
            .find(|title| !title.is_empty())
            .unwrap_or(TITLE)
    }

    /// The look its mark takes from its terminal now.
    pub fn looks(&self, cx: &App) -> Option<Look> {
        let view = self.terminal.read(cx);
        let failed = view.ended().is_some_and(|code| code != 0);
        look(view.records().shown().map(|(_, record)| record), failed)
    }

    /// Notes the look its mark takes `now`; true while the mark moves.
    pub fn note(&mut self, now: Instant, cx: &App) -> bool {
        let shows = self.looks(cx);
        if self.look.map(|(shown, _)| shown) != shows {
            self.look = shows.map(|shows| (shows, now));
        }
        self.look
            .is_some_and(|(look, since)| orb::moves(look, now.saturating_duration_since(since)))
    }
}

/// The most records hover lists, and the most characters it shows of each part.
const LISTED: usize = 10;
const SHORT: usize = 120;

/// `folder` as a row shows it: the home folder as `~`, on one line.
pub fn shown(folder: &Path, home: Option<&Path>) -> String {
    let shown = match home.and_then(|home| folder.strip_prefix(home).ok()) {
        Some(rest) if rest.as_os_str().is_empty() => "~".into(),
        Some(rest) => format!("~{MAIN_SEPARATOR}{}", rest.display()),
        None => folder.display().to_string(),
    };
    one_line(&shown)
}

/// `text` on one line, as gpui shapes labels: each control character as its picture, `␊` for a newline.
pub fn one_line(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            '\0'..='\x1f' => char::from_u32(0x2400 + c as u32).expect("a control picture"),
            '\x7f' => '\u{2421}',
            c => c,
        })
        .collect()
}

/// The session shown once session `closed` of `left + 1` is gone, `active` having been shown.
pub fn after_close(active: usize, closed: usize, left: usize) -> usize {
    if closed < active || active == left {
        active - 1
    } else {
        active
    }
}

/// Session `index`'s row: its title and folder, the mark of its status, a bell while one rang unseen, a button that closes it.
pub fn row(
    workspace: &Entity<Workspace>,
    index: usize,
    session: &Session,
    active: bool,
    home: Option<&Path>,
    now: Instant,
    cx: &App,
) -> impl IntoElement {
    let terminal = session.terminal.read(cx);
    let pid = terminal.pid() as usize;
    let (select, close) = (workspace.clone(), workspace.clone());
    let editing = session.editing.is_some();
    let item = ListItem::new(("session", pid), session.named().to_string())
        .description(shown(&session.folder, home))
        .selected(active)
        .strong(session.bell || session.waiting)
        // A press in the field it renames with stays the field's.
        .on_click(
            move |event, window, cx| match (editing, event.click_count()) {
                (true, _) => {}
                (false, 2..) => select.update(cx, |this, cx| this.rename(index, window, cx)),
                (false, _) => select.update(cx, |this, cx| this.select(index, window, cx)),
            },
        )
        .trailing(
            IconButton::new(("close-session", pid), IconName::X)
                .variant(ButtonVariant::Ghost)
                .size(ControlSize::Sm)
                .tooltip(text(cx, "et.session.close", &[]))
                .on_click(move |_, window, cx| {
                    cx.stop_propagation();
                    close.update(cx, |this, cx| this.close(index, window, cx))
                }),
        );
    let item = match &session.editing {
        Some((input, _)) => item.editor(editor(workspace, session, input)),
        None => item,
    };
    let mark = mark(session, now, cx);
    let bell = session
        .bell
        .then(|| Icon::new(IconName::Bell).color(cx.theme().colors.warning));
    let item = if mark.is_none() && bell.is_none() {
        item
    } else {
        item.leading(
            div()
                .flex()
                .items_center()
                .gap_1()
                .children(mark)
                .children(bell),
        )
    };
    if !SettingsStore::get(cx).session_menu {
        return item.into_any_element();
    }
    let id = session.terminal.entity_id();
    ContextMenu::new(("session-menu", pid), menu::row(workspace, id, cx))
        .w_full()
        .child(item)
        .into_any_element()
}

/// The field a session's row renames it with; Escape drops what it holds.
fn editor(
    workspace: &Entity<Workspace>,
    session: &Session,
    input: &Entity<TextInput>,
) -> impl IntoElement {
    let (workspace, id) = (workspace.clone(), session.terminal.entity_id());
    div()
        // A press selecting text stays the field's: the row would lift it for a drag.
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .on_key_down(move |event, window, cx| {
            if event.keystroke.key == "escape" {
                cx.stop_propagation();
                workspace.update(cx, |this, cx| this.end_rename(id, false, window, cx));
            }
        })
        .child(Input::new(input).size(ControlSize::Sm))
}

/// What a mark shows: the most pressing record's state, else a failed last command.
pub fn look(record: Option<&Record>, failed: bool) -> Option<Look> {
    Some(
        match (record.map(|record| (record.state, record.progress)), failed) {
            (Some((State::Blocked, _)), _) => Look::Waiting,
            (Some((State::Error, _)), _) => Look::Error,
            (Some((State::Working, progress)), _) => Look::Working(progress),
            (Some((State::Done, _)), _) => Look::Done,
            (_, true) => Look::Error,
            (Some((State::Idle, _)), false) => Look::Idle,
            (None, false) => return None,
        },
    )
}

/// The session's mark: its look's dots in its state's color; hover lists the records.
fn mark(session: &Session, now: Instant, cx: &App) -> Option<AnyElement> {
    let (look, since) = session.look?;
    let theme = cx.theme();
    let colors = &theme.colors;
    let color = match look {
        Look::Working(_) => colors.info,
        Look::Waiting => colors.warning,
        Look::Error => colors.danger,
        Look::Done => colors.success,
        Look::Idle => colors.fg_muted,
    };
    let pid = session.terminal.read(cx).pid() as usize;
    // Hover reads the records as they stand, so an open one follows them, and scrolls within the window.
    let watched = session.terminal.downgrade();
    let tip = Tooltip::rich(move |window, cx| {
        let lines = watched.upgrade().map_or_else(Vec::new, |terminal| {
            let view = terminal.read(cx);
            let failed = view.ended().filter(|&code| code != 0);
            hover(view.records(), failed, cx.global::<I18n>())
        });
        let room = window.viewport_size().height - rems(3.).to_pixels(window.rem_size());
        div()
            .id("session-hover")
            .max_h(room)
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .children(lines)
            .into_any_element()
    });
    // Every mark takes a 24-pixel box, so the title stays put as the state changes.
    Some(
        div()
            .id(("session-status", pid))
            .size(theme.icon_size(IconSize::Xl))
            .flex()
            .items_center()
            .justify_center()
            .hoverable_tooltip(tip)
            .child(orb::lattice(look, color, now, since))
            .into_any_element(),
    )
}

/// What hover shows: a line for each record, ten at most, then a failed command's status.
pub fn hover(records: &Records, failed: Option<i32>, words: &I18n) -> Vec<SharedString> {
    let mut lines: Vec<SharedString> = records
        .iter()
        .take(LISTED)
        .map(|(id, record)| line(records, id, record, words).into())
        .collect();
    let more = records.iter().count().saturating_sub(LISTED);
    if more > 0 {
        lines.push(words.text("et.status.more", &[("count", &more.to_string())]));
    }
    if let Some(code) = failed {
        lines.push(words.text("et.status.exited", &[("code", &code.to_string())]));
    }
    lines
}

/// A record as hover and notifications say it: its state, then its app, title and message.
pub fn line(records: &Records, id: &str, record: &Record, words: &I18n) -> String {
    let parts = [
        records.app(id),
        record.title.as_deref(),
        record.msg.as_deref(),
    ];
    let said: Vec<String> = parts.into_iter().flatten().map(plain).collect();
    let state = label(record, words);
    if said.is_empty() {
        state.to_string()
    } else {
        let said = said.join(" · ");
        words
            .text("et.status.line", &[("state", &state), ("said", &said)])
            .to_string()
    }
}

/// A record's state, as hover names it.
fn label(record: &Record, words: &I18n) -> SharedString {
    let key = match (record.state, record.kind, record.progress) {
        (State::Idle, ..) => "et.status.idle",
        (State::Working, _, Some(value)) => {
            return words.text("et.status.working_at", &[("value", &value.to_string())]);
        }
        (State::Working, ..) => "et.status.working",
        (State::Blocked, Some(Kind::Permission), _) => "et.status.permission",
        (State::Blocked, Some(Kind::Question), _) => "et.status.question",
        (State::Blocked, Some(Kind::Auth), _) => "et.status.auth",
        (State::Blocked, None, _) => "et.status.waiting",
        (State::Done, ..) => "et.status.done",
        (State::Error, ..) => "et.status.error",
    };
    words.text(key, &[])
}

/// A program's text as hover shows it: on one line, 120 characters at most, without what could disguise it.
pub fn plain(text: &str) -> String {
    let kept: String = text.chars().filter(|&c| !invisible(c)).collect();
    let line = one_line(&kept);
    match line.char_indices().nth(SHORT) {
        Some((at, _)) => format!("{}…", &line[..at]),
        None => line,
    }
}

/// C1 controls, direction controls and invisible formatting; joiners stay, as scripts and emoji need them.
fn invisible(c: char) -> bool {
    matches!(
        c,
        '\u{80}'..='\u{9f}'
            | '\u{ad}'
            | '\u{61c}'
            | '\u{180e}'
            | '\u{200b}'
            | '\u{200e}'
            | '\u{200f}'
            | '\u{202a}'..='\u{202e}'
            | '\u{2060}'..='\u{2064}'
            | '\u{2066}'..='\u{2069}'
            | '\u{feff}'
            | '\u{fff9}'..='\u{fffb}'
            | '\u{e0000}'..='\u{e007f}'
    )
}

#[cfg(test)]
mod tests;
