//! Settings › Terminal › Shortcuts: each command's keys, a key recorded for one, and its own keys back.

use ely_gpui_component::{
    buttons::{Button, ButtonVariant, IconButton},
    forms::Input,
    i18n::text,
    primitives::IconName,
    settings::{SettingsRow, SettingsSection},
    theme::{ActiveTheme, ControlSize},
    typography::KbdCombo,
};
use gpui::{
    App, Context, Div, Entity, Keystroke, ParentElement, SharedString, Styled, Subscription,
    Window, div,
};

use super::{SettingsWindow, set};
use crate::{
    keymap::{self, Command, Shortcuts},
    settings::SettingsStore,
};

/// The command whose next key is taken, and the hold on every key meanwhile.
pub(super) struct Recording {
    name: String,
    _keys: Subscription,
}

/// A command's words, a session's number among them.
fn title(command: &Command, cx: &App) -> SharedString {
    let number = command.number.map(|number| number.to_string());
    match &number {
        Some(number) => text(cx, command.label, &[("number", number)]),
        None => text(cx, command.label, &[]),
    }
}

/// Whether a command's words or keys hold every word of `query`.
fn kept(title: &str, keys: &[String], query: &str) -> bool {
    let (title, keys) = (title.to_lowercase(), keys.join(" ").to_lowercase());
    query
        .split_whitespace()
        .map(str::to_lowercase)
        .all(|word| title.contains(&word) || keys.contains(&word))
}

impl SettingsWindow {
    /// The search, then a section for each group of commands it keeps.
    pub(super) fn shortcuts(&self, cx: &Context<Self>) -> Div {
        let shortcuts = &SettingsStore::get(cx).shortcuts;
        let query = self.search.read(cx).text().to_string();
        let head = SettingsSection::new(text(cx, "et.terminal.shortcuts", &[]))
            .description(text(cx, "et.keys.hint", &[]))
            .row(Input::new(&self.search));
        let mut page = div().flex().flex_col().gap_6().child(head);
        let commands = keymap::commands();
        for group in commands.chunk_by(|a, b| a.group == b.group) {
            let rows: Vec<_> = group
                .iter()
                .filter(|command| kept(&title(command, cx), &command.keys(shortcuts), &query))
                .map(|command| self.shortcut(command, shortcuts, cx))
                .collect();
            if !rows.is_empty() {
                let section = SettingsSection::new(text(cx, group[0].group, &[]));
                page = page.child(rows.into_iter().fold(section, SettingsSection::row));
            }
        }
        page
    }

    /// A command's row: its keys, Change, its own keys back while the settings hold others, and why a key was refused.
    fn shortcut(
        &self,
        command: &Command,
        shortcuts: &Shortcuts,
        cx: &Context<Self>,
    ) -> SettingsRow {
        let this = cx.entity();
        let name = command.name.clone();
        let recording = self
            .recording
            .as_ref()
            .is_some_and(|recording| recording.name == name);
        let keys = command.keys(shortcuts);
        let shown = div().flex().items_center().gap_2();
        let shown = if recording {
            shown.child(text(cx, "et.keys.press", &[]))
        } else if keys.is_empty() {
            shown
                .text_color(cx.theme().colors.fg_subtle)
                .child(text(cx, "et.keys.none", &[]))
        } else {
            shown.children(keys.iter().map(|key| KbdCombo::new(key)))
        };
        let change = Button::new(
            SharedString::from(format!("change-{name}")),
            text(cx, "et.keys.change", &[]),
        )
        .size(ControlSize::Sm)
        .variant(ButtonVariant::Ghost)
        .on_click({
            let (this, name) = (this.clone(), name.clone());
            move |_, window, cx| {
                this.update(cx, |this, cx| this.record(name.clone(), window, cx));
            }
        });
        let own = shortcuts.contains_key(&name).then(|| {
            IconButton::new(
                SharedString::from(format!("own-{name}")),
                IconName::RotateCcw,
            )
            .size(ControlSize::Sm)
            .variant(ButtonVariant::Ghost)
            .tooltip(text(cx, "et.keys.own", &[]))
            .on_click(move |_, _, cx| {
                let change = format!("{name} takes its own keys again");
                set(&this, change, cx, |settings| {
                    settings.shortcuts.remove(&name);
                });
            })
        });
        let control = div()
            .flex()
            .items_center()
            .gap_2()
            .child(shown)
            .child(change)
            .children(own);
        let row = SettingsRow::new(title(command, cx)).control(control);
        match &self.key_refused {
            Some((refused, why)) if *refused == command.name => row.description(why.clone()),
            _ => row,
        }
    }

    /// Takes the next key for `name`, or lets go when it is the one recording.
    fn record(&mut self, name: String, window: &mut Window, cx: &mut Context<Self>) {
        let again = self
            .recording
            .take()
            .is_some_and(|recording| recording.name == name);
        if again {
            log::info!("settings: {name} keeps its keys");
            return cx.notify();
        }
        log::info!("settings: recording a key for {name}");
        self.key_refused = None;
        // No field holds the focus, so none types the key.
        window.blur(cx);
        let this: Entity<Self> = cx.entity();
        let this = this.downgrade();
        let keys = cx.intercept_keystrokes(move |event, _, cx| {
            cx.stop_propagation();
            let stroke = event.keystroke.clone();
            if let Err(error) = this.update(cx, |this, cx| this.recorded(stroke, cx)) {
                log::debug!("settings: a key came after the window closed: {error:#}");
            }
        });
        self.recording = Some(Recording { name, _keys: keys });
        cx.notify();
    }

    /// The key recorded: Escape keeps the keys, Backspace or Delete leaves none, any other becomes the one key.
    fn recorded(&mut self, stroke: Keystroke, cx: &mut Context<Self>) {
        // A modifier alone waits for its key.
        if keymap::MODIFIERS.contains(&stroke.key.as_str()) {
            return;
        }
        let Some(Recording { name, .. }) = self.recording.take() else {
            return;
        };
        cx.notify();
        let bare = !stroke.modifiers.modified();
        let keys = match stroke.key.as_str() {
            "escape" if bare => return log::info!("settings: {name} keeps its keys"),
            "backspace" | "delete" if bare => Vec::new(),
            _ => vec![stroke.unparse()],
        };
        let saved = SettingsStore::update(cx, |settings| {
            settings.shortcuts.insert(name.clone(), keys.clone());
        });
        match saved {
            Ok(()) => log::info!("settings: {name} runs on {keys:?}"),
            Err(error) => {
                log::error!("settings: refused {keys:?} for {name}: {error:#}");
                self.key_refused = Some((name, format!("{error:#}").into()));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::kept;

    #[test]
    fn the_search_keeps_a_command_by_its_words_or_its_keys() {
        let keys = ["cmd-t".to_string()];
        assert!(kept("New session", &keys, ""));
        assert!(kept("New session", &keys, "new SESS"));
        assert!(kept("New session", &keys, "cmd-t"));
        assert!(!kept("New session", &keys, "new close"));
    }
}
