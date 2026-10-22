use ely_gpui_component::{
    buttons::{Button, SegmentedControl},
    feedback::InlineMessage,
    forms::{Copy, NumberInput, Paste, PathInput, SelectAll, Switch},
    primitives::Severity,
    settings::{KeyboardShortcutsList, SettingsRow, SettingsSection, Shortcut},
    theme::{ActiveTheme, ControlSize},
};
use gpui::{
    Action, App, Context, Div, Entity, KeyContext, ParentElement, SharedString, Styled, div,
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

/// Each way Option types Alt, its key and its name.
const OPTIONS: [(OptionAsAlt, &str, &str); 4] = [
    (OptionAsAlt::Off, "off", "Off"),
    (OptionAsAlt::Left, "left", "Left"),
    (OptionAsAlt::Right, "right", "Right"),
    (OptionAsAlt::Both, "both", "Both"),
];

/// The names of the sessions a digit shows.
const PLACES: [&str; 8] = [
    "Session 1",
    "Session 2",
    "Session 3",
    "Session 4",
    "Session 5",
    "Session 6",
    "Session 7",
    "Session 8",
];

impl SettingsWindow {
    /// Shell, start in, scrollback, Option as Alt and natural text editing on macOS, copy on select, Ely and updates, then the shortcuts.
    pub(super) fn terminal(&self, cx: &Context<Self>) -> Div {
        let settings = SettingsStore::get(cx);
        let this = cx.entity();
        let shell = div()
            .w(cx.theme().prose_width())
            .flex()
            .flex_col()
            .gap_1()
            .child(PathInput::new("settings-shell", &self.shell))
            .children((self.refused.clone()).map(|why| InlineMessage::new(Severity::Danger, why)));
        let start = match settings.start_in {
            StartIn::Home => "home",
            StartIn::Session => "session",
        };
        let start = SegmentedControl::new("settings-start-in", start)
            .segment("home", "Home", None)
            .segment("session", "Session's folder", None)
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
        let mut section = SettingsSection::new("Terminal")
            .row(SettingsRow::new("Shell").control(shell))
            .row(SettingsRow::new("Start in").control(start))
            .row(SettingsRow::new("Scrollback lines").control(lines));
        // Off macOS, Alt is always Alt.
        if cfg!(target_os = "macos") {
            let option = choice(
                &this,
                "option_as_alt",
                &OPTIONS,
                settings.option_as_alt,
                |settings, option| settings.option_as_alt = option,
            );
            let natural = switch(
                &this,
                "natural_text_editing",
                settings.natural_text_editing,
                |settings, on| settings.natural_text_editing = on,
            );
            section = section
                .row(SettingsRow::new("Option as Alt").control(option))
                .row(
                    SettingsRow::new("Natural text editing")
                        .description("Option and Command edit by word and line")
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
            .row(SettingsRow::new("Copy on select").control(copy))
            .row(
                SettingsRow::new("Drag sessions to reorder")
                    .description("A drag moves a row to another place in the list")
                    .control(drag),
            )
            .row(
                SettingsRow::new("Session menu")
                    .description("A right press duplicates the row or closes it")
                    .control(menu),
            )
            .row(
                SettingsRow::new("Ely")
                    .description("Peeks over the window, her face as your sessions are")
                    .control(ely),
            )
            .row(
                SettingsRow::new("Check for updates automatically")
                    .description("At start and once a day")
                    .control(auto_check(&this, settings.check_updates)),
            )
            .row(updates_row(cx));
        let keys = SettingsSection::new("Shortcuts")
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
    let status: SharedString = match found {
        Updates::Unchecked => format!("ELYTerminal {VERSION}").into(),
        Updates::Checking => "Checking for a newer release…".into(),
        Updates::Latest => format!("ELYTerminal {VERSION} is the latest").into(),
        Updates::Newer { version, .. } => {
            format!("ELYTerminal {version} is out; this is {VERSION}").into()
        }
        Updates::Failed(why) => format!("The check failed: {why}").into(),
    };
    let page = match found {
        Updates::Newer { page, .. } => Some(page.clone()),
        _ => None,
    };
    let check = Button::new("check-updates", "Check now")
        .size(ControlSize::Sm)
        .loading(*found == Updates::Checking)
        .on_click(|_, _, cx| updates::check(false, cx));
    let open = page.map(|page| {
        Button::new("open-update", "Download")
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
    SettingsRow::new("Updates")
        .description(status)
        .control(control)
}

/// Where new sessions start, as the settings keep it.
fn start_in(this: &Entity<SettingsWindow>, value: &SharedString, cx: &mut App) {
    let start = match value.as_ref() {
        "home" => StartIn::Home,
        "session" => StartIn::Session,
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
fn listed() -> Vec<(&'static str, &'static str, Box<dyn Action>)> {
    let mut listed = vec![
        ("Sessions", "New session", NewSession.boxed_clone()),
        ("Sessions", "Close session", CloseSession.boxed_clone()),
        ("Sessions", "Next session", NextSession.boxed_clone()),
        (
            "Sessions",
            "Previous session",
            PreviousSession.boxed_clone(),
        ),
    ];
    let places = PLACES.iter().enumerate();
    listed.extend(places.map(|(ix, name)| ("Sessions", *name, SelectSession(ix).boxed_clone())));
    listed.extend([
        ("Sessions", "Last session", LastSession.boxed_clone()),
        ("Edit", "Copy", Copy.boxed_clone()),
        ("Edit", "Paste", Paste.boxed_clone()),
        ("Edit", "Select all", SelectAll.boxed_clone()),
        ("Edit", "Find", Find.boxed_clone()),
        ("View", "Previous prompt", PreviousPrompt.boxed_clone()),
        ("View", "Next prompt", NextPrompt.boxed_clone()),
        ("View", "Toggle sessions", ToggleSessions.boxed_clone()),
        ("View", "Toggle right panel", TogglePanel.boxed_clone()),
        ("View", "Bigger", Bigger.boxed_clone()),
        ("View", "Smaller", Smaller.boxed_clone()),
        ("View", "Reset size", ResetSize.boxed_clone()),
        ("App", "Settings", OpenSettings.boxed_clone()),
        ("App", "Minimize", Minimize.boxed_clone()),
        ("App", "Quit", Quit.boxed_clone()),
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
    for (group, name, action) in listed() {
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
                    group: group.into(),
                    name: name.into(),
                    keys: strokes.join(" ").into(),
                });
            }
        }
    }
    shortcuts
}
