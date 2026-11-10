//! Every command a key runs: its name in the settings, its words, its keys, and the keymap built from them.

use std::collections::BTreeMap;

use anyhow::{Context as _, bail, ensure};
use ely_gpui_component::forms::{Copy, Paste, SelectAll};
use gpui::{
    Action, App, DummyKeyboardMapper, Global, KeyBinding, KeyBindingContextPredicate, Keystroke,
    NoAction,
};

use crate::{
    menus::{self, Bigger, Minimize, Quit, ResetSize, Smaller},
    settings::{
        SettingsStore,
        window::{self, CloseSettings, OpenSettings},
    },
    term::input::{self, Clear, Find, FindNewer, NextPrompt, PreviousPrompt},
    workspace::{
        self, CloseSession, LastSession, NewSession, NextSession, PreviousSession, SelectSession,
        TogglePanel, ToggleSessions,
    },
};

/// The sessions a key shows by their place.
pub const PLACES: usize = 8;

/// What gpui names a modifier pressed alone.
pub const MODIFIERS: [&str; 5] = ["shift", "control", "alt", "platform", "function"];

/// Ely's focus keys, which the terminal hands its program: bound deeper than any command, so none takes them.
const FOCUS_KEYS: [&str; 4] = ["tab", "shift-tab", "f6", "shift-f6"];

/// The keys the user chose, by command; a command it leaves out keeps its own.
pub type Shortcuts = BTreeMap<String, Vec<String>>;

/// A command, with the keys it has until the settings name others.
pub struct Command {
    /// Its name in the settings.
    pub name: String,
    /// Its group's words and its own.
    pub group: &'static str,
    pub label: &'static str,
    /// The session's number its words name.
    pub number: Option<usize>,
    context: Option<&'static str>,
    own: Vec<&'static str>,
    action: Box<dyn Action>,
}

impl Command {
    /// Its keys: the settings', else its own.
    pub fn keys(&self, shortcuts: &Shortcuts) -> Vec<String> {
        match shortcuts.get(&self.name) {
            Some(keys) => keys.clone(),
            None => self.own.iter().map(|key| key.to_string()).collect(),
        }
    }

    /// Whether a key of both would run one of them only: they share a window.
    fn meets(&self, other: &Self) -> bool {
        let settings = |command: &Self| command.context == Some(window::CONTEXT);
        self.context.is_none() || other.context.is_none() || settings(self) == settings(other)
    }
}

/// Every command, in the order Settings lists them.
pub fn commands() -> Vec<Command> {
    let mac = cfg!(target_os = "macos");
    let on = |mac_keys: &[&'static str], other: &[&'static str]| {
        if mac { mac_keys } else { other }.to_vec()
    };
    let command = |name: &str, group, label, context, own, action: Box<dyn Action>| Command {
        name: name.to_string(),
        group,
        label,
        number: None,
        context,
        own,
        action,
    };
    let (main, term, find) = (
        Some(workspace::CONTEXT),
        Some(input::CONTEXT),
        Some(input::FIND_INPUT),
    );
    let (sessions, edit, view, app) = (
        "et.keys.sessions",
        "et.keys.edit",
        "et.keys.view",
        "et.keys.app",
    );
    let digits = on(
        &[
            "cmd-1", "cmd-2", "cmd-3", "cmd-4", "cmd-5", "cmd-6", "cmd-7", "cmd-8", "cmd-9",
        ],
        &[
            "alt-1", "alt-2", "alt-3", "alt-4", "alt-5", "alt-6", "alt-7", "alt-8", "alt-9",
        ],
    );
    // Windows' own Insert keys, as Windows Terminal binds them.
    let (copy, paste) = if cfg!(windows) {
        (
            vec!["ctrl-shift-c", "ctrl-insert"],
            vec!["ctrl-shift-v", "ctrl-v", "shift-insert"],
        )
    } else {
        (
            on(&["cmd-c"], &["ctrl-shift-c"]),
            on(&["cmd-v"], &["ctrl-shift-v"]),
        )
    };
    let mut all = vec![
        command(
            "new_session",
            sessions,
            "et.session.new",
            main,
            on(&["cmd-t"], &["ctrl-shift-t"]),
            Box::new(NewSession),
        ),
        command(
            "close_session",
            sessions,
            "et.session.close",
            main,
            on(&["cmd-w"], &["ctrl-shift-w"]),
            Box::new(CloseSession),
        ),
        command(
            "next_session",
            sessions,
            "et.keys.next_session",
            main,
            vec!["ctrl-tab"],
            Box::new(NextSession),
        ),
        command(
            "previous_session",
            sessions,
            "et.keys.previous_session",
            main,
            vec!["ctrl-shift-tab"],
            Box::new(PreviousSession),
        ),
    ];
    all.extend((0..PLACES).map(|place| Command {
        number: Some(place + 1),
        ..command(
            &format!("session_{}", place + 1),
            sessions,
            "et.keys.session",
            main,
            vec![digits[place]],
            Box::new(SelectSession(place)),
        )
    }));
    all.extend([
        command(
            "last_session",
            sessions,
            "et.keys.last_session",
            main,
            vec![digits[PLACES]],
            Box::new(LastSession),
        ),
        command("copy", edit, "et.keys.copy", term, copy, Box::new(Copy)),
        command("paste", edit, "et.keys.paste", term, paste, Box::new(Paste)),
        command(
            "select_all",
            edit,
            "et.keys.select_all",
            term,
            on(&["cmd-a"], &["ctrl-shift-a"]),
            Box::new(SelectAll),
        ),
        command(
            "find",
            edit,
            "et.keys.find",
            term,
            on(&["cmd-f"], &["ctrl-shift-f"]),
            Box::new(Find),
        ),
        command(
            "find_newer",
            edit,
            "et.keys.find_newer",
            find,
            vec!["shift-enter"],
            Box::new(FindNewer),
        ),
        command(
            "clear",
            edit,
            "et.keys.clear",
            term,
            on(&["cmd-k"], &["ctrl-shift-k"]),
            Box::new(Clear),
        ),
        command(
            "previous_prompt",
            view,
            "et.keys.previous_prompt",
            term,
            on(&["cmd-up", "cmd-shift-up"], &["ctrl-shift-up"]),
            Box::new(PreviousPrompt),
        ),
        command(
            "next_prompt",
            view,
            "et.keys.next_prompt",
            term,
            on(&["cmd-down", "cmd-shift-down"], &["ctrl-shift-down"]),
            Box::new(NextPrompt),
        ),
        command(
            "toggle_sessions",
            view,
            "et.keys.toggle_sessions",
            main,
            on(&["cmd-b"], &["ctrl-shift-b"]),
            Box::new(ToggleSessions),
        ),
        command(
            "toggle_panel",
            view,
            "et.keys.toggle_panel",
            main,
            on(&["cmd-alt-b"], &["ctrl-shift-alt-b"]),
            Box::new(TogglePanel),
        ),
        command(
            "bigger",
            view,
            "et.keys.bigger",
            None,
            on(&["cmd-=", "cmd-+"], &["ctrl-=", "ctrl-+"]),
            Box::new(Bigger),
        ),
        command(
            "smaller",
            view,
            "et.keys.smaller",
            None,
            on(&["cmd--"], &["ctrl--"]),
            Box::new(Smaller),
        ),
        command(
            "reset_size",
            view,
            "et.keys.reset_size",
            None,
            on(&["cmd-0"], &["ctrl-0"]),
            Box::new(ResetSize),
        ),
        command(
            "settings",
            app,
            "et.settings.title",
            None,
            on(&["cmd-,"], &["ctrl-,"]),
            Box::new(OpenSettings),
        ),
        command(
            "close_settings",
            app,
            "et.keys.close_settings",
            Some(window::CONTEXT),
            on(&["cmd-w"], &["ctrl-shift-w"]),
            Box::new(CloseSettings),
        ),
        command(
            "minimize",
            app,
            "et.keys.minimize",
            None,
            on(&["cmd-m"], &[]),
            Box::new(Minimize),
        ),
        command(
            "quit",
            app,
            "et.keys.quit",
            None,
            on(&["cmd-q"], &["ctrl-shift-q"]),
            Box::new(Quit),
        ),
    ]);
    all
}

/// Refuses a command no table names, a key that does not read, and a key two commands of one window hold.
pub fn check(shortcuts: &Shortcuts) -> anyhow::Result<()> {
    let commands = commands();
    for name in shortcuts.keys() {
        ensure!(
            commands.iter().any(|command| command.name == *name),
            "shortcuts names {name}, which is no command"
        );
    }
    let mut held: Vec<(String, &Command)> = Vec::new();
    for command in &commands {
        for key in command.keys(shortcuts) {
            let stroke = Keystroke::parse(&key)
                .with_context(|| format!("shortcuts: {}'s {key:?} is no key", command.name))?;
            // gpui reads any word as a key's name.
            let named = !stroke.key.is_empty()
                && !stroke.key.contains(char::is_whitespace)
                && !MODIFIERS.contains(&stroke.key.as_str());
            ensure!(named, "shortcuts: {}'s {key:?} names no key", command.name);
            let stroke = stroke.unparse();
            ensure!(
                !FOCUS_KEYS.contains(&stroke.as_str()),
                "shortcuts: {key} moves the focus and goes to the terminal's program, so {} cannot take it",
                command.name
            );
            let taken = held
                .iter()
                .find(|(other, holder)| *other == stroke && holder.meets(command));
            if let Some((_, holder)) = taken {
                bail!(
                    "shortcuts: {key} runs {} already, so {} cannot take it",
                    holder.name,
                    command.name
                );
            }
            held.push((stroke, command));
        }
    }
    Ok(())
}

/// The bindings Ely made before ELYTerminal's own.
struct Base(Vec<KeyBinding>);

impl Global for Base {}

/// Keeps Ely's bindings, then binds every command as the settings say.
pub fn init(cx: &mut App) {
    let base = cx.key_bindings().borrow().bindings().cloned().collect();
    cx.set_global(Base(base));
    apply(cx);
    let mut bound = SettingsStore::get(cx).shortcuts.clone();
    cx.observe_global::<SettingsStore>(move |cx| {
        let now = &SettingsStore::get(cx).shortcuts;
        if *now != bound {
            bound = now.clone();
            apply(cx);
        }
    })
    .detach();
}

/// Builds the keymap again from Ely's bindings and the settings, and the menu bar with it.
pub fn apply(cx: &mut App) {
    let shortcuts = SettingsStore::get(cx).shortcuts.clone();
    let mut bindings = cx.global::<Base>().0.clone();
    // Ely's focus keys reach the terminal's listeners.
    bindings.extend(FOCUS_KEYS.map(|key| KeyBinding::new(key, NoAction, Some(input::CONTEXT))));
    for command in commands() {
        let context = command.context.map(|context| {
            KeyBindingContextPredicate::parse(context)
                .expect("a command's context parses")
                .into()
        });
        for key in command.keys(&shortcuts) {
            let action = command.action.boxed_clone();
            let binding = KeyBinding::load(
                &key,
                action,
                context.clone(),
                false,
                None,
                &DummyKeyboardMapper,
            )
            .expect("the settings checked the key");
            bindings.push(binding);
        }
    }
    let count = bindings.len();
    cx.clear_key_bindings();
    cx.bind_keys(bindings);
    cx.set_menus(menus::menus(cx));
    log::info!(
        "keys: {count} bindings, {} commands the settings rebind",
        shortcuts.len()
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chosen(pairs: &[(&str, &[&str])]) -> Shortcuts {
        pairs
            .iter()
            .map(|(name, keys)| {
                let keys = keys.iter().map(|key| key.to_string()).collect();
                (name.to_string(), keys)
            })
            .collect()
    }

    #[test]
    fn the_commands_own_keys_clash_nowhere() {
        check(&Shortcuts::new()).expect("the table holds no clash");
        let commands = commands();
        let mut names: Vec<_> = commands.iter().map(|command| &command.name).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), commands.len(), "each name once");
    }

    #[test]
    fn a_chosen_key_replaces_a_command_s_own() {
        let shortcuts = chosen(&[("new_session", &["ctrl-alt-n"]), ("quit", &[])]);
        check(&shortcuts).expect("a free key and an unbound command");
        let commands = commands();
        let keys = |name: &str| {
            let command = commands.iter().find(|command| command.name == name);
            command.expect("a command").keys(&shortcuts)
        };
        assert_eq!(keys("new_session"), ["ctrl-alt-n"]);
        assert!(keys("quit").is_empty());
        assert!(!keys("copy").is_empty(), "the rest keep their own");
    }

    #[test]
    fn a_key_two_commands_of_one_window_hold_is_refused() {
        let find = commands()
            .into_iter()
            .find(|command| command.name == "find")
            .expect("find");
        let taken = find.keys(&Shortcuts::new())[0].clone();
        let clash = check(&chosen(&[("clear", &[taken.as_str()])])).expect_err("find holds it");
        assert!(format!("{clash:#}").contains("find"), "{clash:#}");
        // The same key in another order of modifiers is the same key.
        check(&chosen(&[
            ("clear", &["shift-ctrl-alt-x"]),
            ("copy", &["alt-ctrl-shift-x"]),
        ]))
        .expect_err("one key spelled twice");
        // Settings and the main window never hold focus together.
        let close = commands()
            .into_iter()
            .find(|command| command.name == "close_session")
            .expect("close_session");
        let shared = close.keys(&Shortcuts::new())[0].clone();
        check(&chosen(&[("close_settings", &[shared.as_str()])])).expect("apart");
        // An app-wide key meets both.
        check(&chosen(&[("quit", &[shared.as_str()])])).expect_err("quit runs everywhere");
    }

    #[test]
    fn what_is_no_command_or_no_key_is_refused() {
        assert!(check(&chosen(&[("fly", &["ctrl-alt-f"])])).is_err());
        for key in [
            "",
            "ctrl-",
            "ctrl-shift",
            "ctrl-a b",
            "hyper-c",
            "f6",
            "shift-tab",
        ] {
            assert!(check(&chosen(&[("copy", &[key])])).is_err(), "{key:?}");
        }
    }
}
