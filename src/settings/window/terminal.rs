use std::path::PathBuf;

use ely_gpui_component::{
    buttons::{Button, SegmentedControl},
    feedback::InlineMessage,
    forms::{Copy, NumberInput, Paste, PathInput, SelectAll, Switch},
    i18n::text,
    primitives::Severity,
    settings::{KeyboardShortcutsList, SettingsRow, SettingsSection, Shortcut},
    theme::{ActiveTheme, ControlSize},
};
use gpui::{
    Action, App, Context, Div, Entity, KeyContext, ParentElement, SharedString, Styled, Window, div,
};

use super::{OpenSettings, SettingsWindow, choice, set, switch};
use crate::{
    menus::{Bigger, Minimize, Quit, ResetSize, Smaller},
    settings::{OptionAsAlt, SCROLLBACK, SettingsStore, StartIn},
    term::input::{self, Find, NextPrompt, PreviousPrompt},
    updates::{self, Updates, VERSION},
    workspace::{
        self, CloseSession, LastSession, NewSession, NextSession, PreviousSession, SelectSession,
        TogglePanel, ToggleSessions,
    },
};

/// Each way Option types Alt, its key and name's key.
const OPTIONS: [(OptionAsAlt, &str, &str); 4] = [
    (OptionAsAlt::Off, "off", "et.option.off"),
    (OptionAsAlt::Left, "left", "et.option.left"),
    (OptionAsAlt::Right, "right", "et.option.right"),
    (OptionAsAlt::Both, "both", "et.option.both"),
];

/// The sessions a digit shows, Cmd-1 to Cmd-8.
const PLACES: usize = 8;

impl SettingsWindow {
    /// Shell, start in, scrollback, Option as Alt and natural text editing on macOS, copy on select, Ely and updates, then the shortcuts.
    pub(super) fn terminal(&self, cx: &Context<Self>) -> Div {
        let settings = SettingsStore::get(cx);
        let this = cx.entity();
        let word = |key: &str| text(cx, key, &[]);
        let shell = PathInput::new("settings-shell", &self.shell).on_choose(chosen(&this, "shell"));
        let shell = path_field(shell, self.refused.clone(), cx);
        let folder = PathInput::new("settings-start-folder", &self.folder)
            .directories()
            .on_choose(chosen(&this, "start_folder"));
        let folder = path_field(folder, self.refused_folder.clone(), cx);
        let start = match settings.start_in {
            StartIn::Home => "home",
            StartIn::Session => "session",
            StartIn::Folder => "folder",
        };
        let start = SegmentedControl::new("settings-start-in", start)
            .segment("home", word("et.start.home"), None)
            .segment("session", word("et.start.session"), None)
            .segment("folder", word("et.start.folder"), None)
            .size(ControlSize::Sm)
            .on_change({
                let this = this.clone();
                move |value, _, cx| start_in(&this, value, cx)
            });
        let lines = NumberInput::new(("scrollback", self.drafts), f64::from(settings.scrollback))
            .range(f64::from(*SCROLLBACK.start()), f64::from(*SCROLLBACK.end()))
            .step(1000.)
            .size(ControlSize::Sm)
            .on_commit({
                let this = this.clone();
                move |lines, _, cx| {
                    let lines = lines as u32;
                    set(&this, format!("scrollback is {lines}"), cx, |settings| {
                        settings.scrollback = lines
                    });
                }
            });
        let mut section = SettingsSection::new(word("et.settings.terminal"))
            .row(SettingsRow::new(word("et.terminal.shell")).control(shell))
            .row(SettingsRow::new(word("et.terminal.start_in")).control(start))
            .row(
                SettingsRow::new(word("et.terminal.start_folder"))
                    .description(word("et.terminal.start_folder_hint"))
                    .control(folder),
            )
            .row(SettingsRow::new(word("et.terminal.scrollback")).control(lines));
        // Off macOS, Alt is always Alt.
        if cfg!(target_os = "macos") {
            let option = choice(
                &this,
                "option_as_alt",
                &OPTIONS,
                settings.option_as_alt,
                |settings, option| settings.option_as_alt = option,
                cx,
            );
            let natural = switch(
                &this,
                "natural_text_editing",
                settings.natural_text_editing,
                |settings, on| settings.natural_text_editing = on,
            );
            section = section
                .row(SettingsRow::new(word("et.terminal.option_as_alt")).control(option))
                .row(
                    SettingsRow::new(word("et.terminal.natural"))
                        .description(word("et.terminal.natural_hint"))
                        .control(natural),
                );
        }
        let copy = switch(
            &this,
            "copy_on_select",
            settings.copy_on_select,
            |settings, on| settings.copy_on_select = on,
        );
        let ely = switch(&this, "ely", settings.ely, |settings, on| settings.ely = on);
        let drag = switch(
            &this,
            "drag_sessions",
            settings.drag_sessions,
            |settings, on| settings.drag_sessions = on,
        );
        let menu = switch(
            &this,
            "session_menu",
            settings.session_menu,
            |settings, on| settings.session_menu = on,
        );
        let section = section
            .row(SettingsRow::new(word("et.terminal.copy_on_select")).control(copy))
            .row(
                SettingsRow::new(word("et.terminal.drag_sessions"))
                    .description(word("et.terminal.drag_sessions_hint"))
                    .control(drag),
            )
            .row(
                SettingsRow::new(word("et.terminal.session_menu"))
                    .description(word("et.terminal.session_menu_hint"))
                    .control(menu),
            )
            .row(
                SettingsRow::new(word("et.terminal.ely"))
                    .description(word("et.terminal.ely_hint"))
                    .control(ely),
            )
            .row(
                SettingsRow::new(word("et.terminal.auto_updates"))
                    .description(word("et.terminal.auto_updates_hint"))
                    .control(auto_check(&this, settings.check_updates)),
            )
            .row(updates_row(cx));
        let keys = SettingsSection::new(word("et.terminal.shortcuts"))
            .row(KeyboardShortcutsList::new(shortcuts(cx), &self.search));
        div().flex().flex_col().gap_6().child(section).child(keys)
    }
}

/// The switch for automatic checks; turning it on checks at once.
fn auto_check(this: &Entity<SettingsWindow>, on: bool) -> Switch {
    let this = this.clone();
    Switch::new("check_updates", on).on_change(move |on, _, cx| {
        let change = format!("check_updates is {on}");
        if set(&this, change, cx, |settings| settings.check_updates = on) && on {
            updates::check(false, cx);
        }
    })
}

/// What the last check found, a button that checks now, and one that opens a newer release's page.
fn updates_row(cx: &App) -> SettingsRow {
    let found = cx.global::<Updates>();
    let status = match found {
        Updates::Unchecked => text(cx, "et.updates.unchecked", &[("version", VERSION)]),
        Updates::Checking => text(cx, "et.updates.checking", &[]),
        Updates::Latest => text(cx, "et.updates.latest", &[("version", VERSION)]),
        Updates::Newer { version, .. } => text(
            cx,
            "et.updates.newer",
            &[("version", version), ("current", VERSION)],
        ),
        Updates::Failed(why) => text(cx, "et.updates.failed", &[("why", why)]),
    };
    let page = match found {
        Updates::Newer { page, .. } => Some(page.clone()),
        _ => None,
    };
    let check = Button::new("check-updates", text(cx, "et.updates.check", &[]))
        .size(ControlSize::Sm)
        .loading(*found == Updates::Checking)
        .on_click(|_, _, cx| updates::check(false, cx));
    let open = page.map(|page| {
        Button::new("open-update", text(cx, "et.updates.download", &[]))
            .primary()
            .size(ControlSize::Sm)
            .on_click(move |_, _, cx| {
                log::info!("updates: Settings opens {page}");
                cx.open_url(&page);
            })
    });
    let control = div()
        .flex()
        .items_center()
        .gap_2()
        .children(open)
        .child(check);
    SettingsRow::new(text(cx, "et.updates.title", &[]))
        .description(status)
        .control(control)
}

/// Hands Browse's pick for `field` to the window.
fn chosen(
    this: &Entity<SettingsWindow>,
    field: &'static str,
) -> impl Fn(PathBuf, &mut Window, &mut App) + 'static {
    let this = this.clone();
    move |path, _, cx| this.update(cx, |this, cx| this.chosen(field, path, cx))
}

/// A path field, with why its text is not saved.
fn path_field(path: PathInput, refused: Option<SharedString>, cx: &App) -> Div {
    let refused = refused.map(|why| InlineMessage::new(Severity::Danger, why));
    let field = div().w(cx.theme().prose_width()).flex().flex_col().gap_1();
    field.child(path).children(refused)
}

/// Where new sessions start, as the settings keep it.
fn start_in(this: &Entity<SettingsWindow>, value: &SharedString, cx: &mut App) {
    let start = match value.as_ref() {
        "home" => StartIn::Home,
        "session" => StartIn::Session,
        "folder" => StartIn::Folder,
        other => unreachable!("settings: no start {other}"),
    };
    set(
        this,
        format!("new sessions start in {start:?}"),
        cx,
        |settings| settings.start_in = start,
    );
}

/// The listed commands, in order: group, name and action.
fn listed(cx: &App) -> Vec<(SharedString, SharedString, Box<dyn Action>)> {
    let word = |key: &str| text(cx, key, &[]);
    let sessions = word("et.keys.sessions");
    let mut listed = vec![
        (
            sessions.clone(),
            word("et.session.new"),
            NewSession.boxed_clone(),
        ),
        (
            sessions.clone(),
            word("et.session.close"),
            CloseSession.boxed_clone(),
        ),
        (
            sessions.clone(),
            word("et.keys.next_session"),
            NextSession.boxed_clone(),
        ),
        (
            sessions.clone(),
            word("et.keys.previous_session"),
            PreviousSession.boxed_clone(),
        ),
    ];
    listed.extend((0..PLACES).map(|ix| {
        let name = text(cx, "et.keys.session", &[("number", &(ix + 1).to_string())]);
        (sessions.clone(), name, SelectSession(ix).boxed_clone())
    }));
    let (edit, view, app) = (
        word("et.keys.edit"),
        word("et.keys.view"),
        word("et.keys.app"),
    );
    listed.extend([
        (
            sessions,
            word("et.keys.last_session"),
            LastSession.boxed_clone(),
        ),
        (edit.clone(), word("et.keys.copy"), Copy.boxed_clone()),
        (edit.clone(), word("et.keys.paste"), Paste.boxed_clone()),
        (
            edit.clone(),
            word("et.keys.select_all"),
            SelectAll.boxed_clone(),
        ),
        (edit, word("et.keys.find"), Find.boxed_clone()),
        (
            view.clone(),
            word("et.keys.previous_prompt"),
            PreviousPrompt.boxed_clone(),
        ),
        (
            view.clone(),
            word("et.keys.next_prompt"),
            NextPrompt.boxed_clone(),
        ),
        (
            view.clone(),
            word("et.keys.toggle_sessions"),
            ToggleSessions.boxed_clone(),
        ),
        (
            view.clone(),
            word("et.keys.toggle_panel"),
            TogglePanel.boxed_clone(),
        ),
        (view.clone(), word("et.keys.bigger"), Bigger.boxed_clone()),
        (view.clone(), word("et.keys.smaller"), Smaller.boxed_clone()),
        (view, word("et.keys.reset_size"), ResetSize.boxed_clone()),
        (
            app.clone(),
            word("et.settings.title"),
            OpenSettings.boxed_clone(),
        ),
        (
            app.clone(),
            word("et.keys.minimize"),
            Minimize.boxed_clone(),
        ),
        (app, word("et.keys.quit"), Quit.boxed_clone()),
    ]);
    listed
}

/// Each listed command's keys in the main window's terminal, a line per binding, from the keymap.
fn shortcuts(cx: &App) -> Vec<Shortcut> {
    let stack = [workspace::CONTEXT, input::CONTEXT]
        .map(|name| KeyContext::parse(name).expect("a context name parses"));
    let keymap = cx.key_bindings();
    let keymap = keymap.borrow();
    let mut shortcuts = Vec::new();
    for (group, name, action) in listed(cx) {
        for binding in keymap.bindings_for_action(action.as_ref()) {
            // gpui matches a binding at any depth of the context stack.
            let applies = binding.predicate().is_none_or(|predicate| {
                (1..=stack.len()).any(|depth| predicate.eval(&stack[..depth]))
            });
            if applies {
                let strokes: Vec<String> = binding
                    .keystrokes()
                    .iter()
                    .map(|key| key.unparse())
                    .collect();
                shortcuts.push(Shortcut {
                    group: group.clone(),
                    name: name.clone(),
                    keys: strokes.join(" ").into(),
                });
            }
        }
    }
    shortcuts
}
