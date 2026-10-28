use std::ops::Range;

use alacritty_terminal::{grid::Scroll, term::TermMode};
use ely_gpui_component::forms::{Copy, Paste, SelectAll};
use gpui::{
    App, Bounds, Context, EntityInputHandler, KeyBinding, KeyDownEvent, KeyUpEvent, Keystroke,
    NoAction, Pixels, Point, UTF16Selection, Window, actions,
};

use super::{
    keys::{self, Action, Key, Kitty},
    view::TerminalView,
};

actions!(elyterminal, [Find, FindNewer, PreviousPrompt, NextPrompt]);

/// The key context a terminal's root sets.
pub const CONTEXT: &str = "Terminal";

/// The key context of a terminal's find box.
pub const FIND: &str = "TerminalFind";

/// The terminal's keys; Ely's focus keys reach its listeners.
pub fn bind_keys(cx: &mut App) {
    let (copy, paste, all, find) = if cfg!(target_os = "macos") {
        ("cmd-c", "cmd-v", "cmd-a", "cmd-f")
    } else {
        (
            "ctrl-shift-c",
            "ctrl-shift-v",
            "ctrl-shift-a",
            "ctrl-shift-f",
        )
    };
    // Ghostty's jumps: Cmd with or without Shift on macOS, Ctrl-Shift elsewhere.
    let (up, down): (&[&str], &[&str]) = if cfg!(target_os = "macos") {
        (&["cmd-up", "cmd-shift-up"], &["cmd-down", "cmd-shift-down"])
    } else {
        (&["ctrl-shift-up"], &["ctrl-shift-down"])
    };
    cx.bind_keys(
        up.iter()
            .map(|key| KeyBinding::new(key, PreviousPrompt, Some(CONTEXT))),
    );
    cx.bind_keys(
        down.iter()
            .map(|key| KeyBinding::new(key, NextPrompt, Some(CONTEXT))),
    );
    cx.bind_keys([
        KeyBinding::new(copy, Copy, Some(CONTEXT)),
        KeyBinding::new(paste, Paste, Some(CONTEXT)),
        KeyBinding::new(all, SelectAll, Some(CONTEXT)),
        KeyBinding::new(find, Find, Some(CONTEXT)),
        KeyBinding::new(
            "shift-enter",
            FindNewer,
            Some(&format!("{FIND} > ElyInput")),
        ),
    ]);
    cx.bind_keys(
        ["tab", "shift-tab", "f6", "shift-f6"]
            .map(|key| KeyBinding::new(key, NoAction, Some(CONTEXT))),
    );
    // Windows pastes on Ctrl-V too, as Windows Terminal does.
    if cfg!(windows) {
        cx.bind_keys([KeyBinding::new("ctrl-v", Paste, Some(CONTEXT))]);
    }
}

impl TerminalView {
    /// Bytes from the keyboard: back to the newest line, no selection, then to the program, which sees done and error records.
    pub(super) fn input(&mut self, bytes: Vec<u8>, cx: &mut Context<Self>) {
        let mut term = self.pty.term.lock();
        term.scroll_display(Scroll::Bottom);
        if term.selection.take().is_some() {
            log::debug!("terminal {}: input clears the selection", self.pty.pid);
        }
        drop(term);
        self.pty.write(bytes);
        if self.records.seen() {
            self.restate(cx);
        }
        cx.notify();
    }

    pub(super) fn key_down(
        &mut self,
        event: &KeyDownEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if event.prefer_character_input {
            log::debug!(
                "terminal {}: {:?} types through the layout, a dead key or a character its modifiers make",
                self.pty.pid,
                event.keystroke
            );
            return;
        }
        let facts = self.facts(&event.keystroke);
        let action = if event.is_held || facts.repeat {
            Action::Repeat
        } else {
            Action::Press
        };
        let id = held(&event.keystroke, facts);
        let prior = self.held.get(&id).copied();
        let went = self.send(&event.keystroke, action, facts, prior, cx);
        self.held.insert(id, went.unwrap_or(Went::Bytes));
        if went.is_some() {
            cx.stop_propagation();
        }
    }

    /// A release goes out only for a press this terminal saw: none for one a binding took.
    pub(super) fn key_up(&mut self, event: &KeyUpEvent, _: &mut Window, cx: &mut Context<Self>) {
        let facts = self.facts(&event.keystroke);
        let Some(went) = self.held.remove(&held(&event.keystroke, facts)) else {
            return;
        };
        if self
            .send(&event.keystroke, Action::Release, facts, Some(went), cx)
            .is_some()
        {
            cx.stop_propagation();
        }
    }

    /// Presses and lets go of `name` without modifiers, as the keyboard would.
    pub(super) fn tap_key(&mut self, name: &str, cx: &mut Context<Self>) {
        let keystroke = Keystroke {
            modifiers: Default::default(),
            key: name.into(),
            key_char: None,
        };
        let facts = Facts::default();
        let went = self.send(&keystroke, Action::Press, facts, None, cx);
        self.send(
            &keystroke,
            Action::Release,
            facts,
            Some(went.unwrap_or(Went::Bytes)),
            cx,
        );
    }

    /// Sends what `keystroke` means to the program: none when it is text the input method types, else how it went; `prior` is how a held key's press went.
    fn send(
        &mut self,
        keystroke: &Keystroke,
        action: Action,
        facts: Facts,
        // Linux sends every key one way.
        #[cfg_attr(not(any(windows, target_os = "macos")), expect(unused_variables))] prior: Option<
            Went,
        >,
        cx: &mut Context<Self>,
    ) -> Option<Went> {
        let modifiers = keystroke.modifiers;
        let alt = modifiers.alt && Self::alt_is_meta(facts, cx);
        let text = match &keystroke.key_char {
            Some(_) if alt && cfg!(target_os = "macos") => "",
            Some(text) => text.as_str(),
            None => "",
        };
        let key = Key {
            name: &keystroke.key,
            text,
            shift: modifiers.shift,
            ctrl: modifiers.control,
            alt,
            platform: modifiers.platform,
            action,
            unshifted: facts.unshifted,
        };
        #[cfg(target_os = "macos")]
        {
            let natural = keys::natural(&key, modifiers.alt)
                .filter(|_| crate::settings::SettingsStore::get(cx).natural_text_editing);
            if edits(action, prior, natural.is_some()) {
                if let Some(bytes) = natural.filter(|_| action != Action::Release) {
                    log::trace!("terminal {}: {key:?} sends {bytes:?}", self.pty.pid);
                    self.input(bytes.to_vec(), cx);
                }
                return Some(Went::Editing);
            }
        }
        let mode = *self.pty.term.lock().mode();
        let kitty = Kitty {
            disambiguate: mode.contains(TermMode::DISAMBIGUATE_ESC_CODES),
            events: mode.contains(TermMode::REPORT_EVENT_TYPES),
            alternates: mode.contains(TermMode::REPORT_ALTERNATE_KEYS),
            all: mode.contains(TermMode::REPORT_ALL_KEYS_AS_ESC),
            text: mode.contains(TermMode::REPORT_ASSOCIATED_TEXT),
        };
        #[cfg(windows)]
        if self.win32
            && records(
                kitty,
                action,
                keys::types(&key),
                prior == Some(Went::Record),
            )
        {
            match super::win32::record(keystroke, action) {
                Some(bytes) => {
                    log::trace!("terminal {}: {key:?} sends {bytes:?}", self.pty.pid);
                    self.input(bytes, cx);
                    return Some(Went::Record);
                }
                None => log::debug!(
                    "terminal {}: no virtual key for {:?}, so it goes as bytes",
                    self.pty.pid,
                    keystroke.key
                ),
            }
        }
        let bytes = keys::encode(&key, mode.contains(TermMode::APP_CURSOR), kitty)?;
        log::trace!("terminal {}: {key:?} sends {bytes:?}", self.pty.pid);
        self.input(bytes, cx);
        Some(Went::Bytes)
    }

    /// AppKit's facts of the key; logged and none when unreadable.
    #[cfg(target_os = "macos")]
    fn facts(&self, keystroke: &Keystroke) -> Facts {
        super::mac::key_facts(&keystroke.key, keystroke.modifiers.shift).unwrap_or_else(|error| {
            log::error!("terminal {}: {error:#}", self.pty.pid);
            Facts {
                left_option: false,
                right_option: false,
                repeat: false,
                unshifted: None,
                code: None,
            }
        })
    }

    /// Windows' layout names the key gpui named by what Shift made of it.
    #[cfg(windows)]
    fn facts(&self, keystroke: &Keystroke) -> Facts {
        let unshifted = if keystroke.modifiers.shift {
            None
        } else {
            super::win32::unshifted(&keystroke.key)
        };
        Facts {
            repeat: false,
            unshifted,
            code: super::win32::virtual_key(&keystroke.key).map(|(key, _)| key),
        }
    }

    /// gpui keeps no key code on Linux, so a shifted symbol stays as gpui names it.
    #[cfg(not(any(target_os = "macos", windows)))]
    fn facts(&self, _: &Keystroke) -> Facts {
        Facts::default()
    }

    /// Whether a held Alt is a modifier; on macOS only the Option keys the settings name.
    #[cfg(target_os = "macos")]
    fn alt_is_meta(facts: Facts, cx: &App) -> bool {
        use crate::settings::{OptionAsAlt, SettingsStore};
        match SettingsStore::get(cx).option_as_alt {
            OptionAsAlt::Off => false,
            OptionAsAlt::Both => true,
            OptionAsAlt::Left => facts.left_option,
            OptionAsAlt::Right => facts.right_option,
        }
    }

    #[cfg(not(target_os = "macos"))]
    fn alt_is_meta(_: Facts, _: &App) -> bool {
        true
    }
}

#[cfg(target_os = "macos")]
type Facts = super::mac::KeyFacts;

/// Other platforms tell gpui all it needs.
#[cfg(not(target_os = "macos"))]
#[derive(Clone, Copy, Debug, Default)]
struct Facts {
    repeat: bool,
    unshifted: Option<char>,
    /// The key's virtual key, which Shift does not change; none off Windows.
    code: Option<u16>,
}

/// How a press went to the program, which its repeats and release follow.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Went {
    Bytes,
    /// Windows' own record, win32-input-mode.
    #[cfg(windows)]
    Record,
    /// Readline's bytes, natural text editing on macOS.
    #[cfg(target_os = "macos")]
    Editing,
}

/// A key while it is down: its code where the platform gives one, else gpui's name, which Shift may change.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(super) enum Held {
    Code(u16),
    Name(String),
}

/// Whether a key goes as natural editing: a press by its keys, and a repeat or release as its press went, whatever modifiers changed since.
#[cfg(target_os = "macos")]
fn edits(action: Action, prior: Option<Went>, natural: bool) -> bool {
    match (action, prior) {
        (Action::Press, _) | (Action::Repeat, None) => natural,
        (_, prior) => prior == Some(Went::Editing),
    }
}

/// Whether a key goes as Windows' record, as in Windows Terminal: kitty's flags go first, text stays text, and a repeat or release goes as its press did.
#[cfg(any(windows, test))]
fn records(kitty: Kitty, action: Action, types: bool, pressed: bool) -> bool {
    kitty == Kitty::default()
        && match action {
            Action::Press => !types,
            Action::Repeat => pressed || !types,
            Action::Release => pressed,
        }
}

fn held(keystroke: &Keystroke, facts: Facts) -> Held {
    facts
        .code
        .map_or_else(|| Held::Name(keystroke.key.clone()), Held::Code)
}

impl EntityInputHandler for TerminalView {
    /// False while Option as Alt claims the current key's Option.
    #[cfg(target_os = "macos")]
    fn accepts_text_input(&self, _: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some((left_option, right_option)) = super::mac::held_options() else {
            return true;
        };
        let facts = Facts {
            left_option,
            right_option,
            ..Facts::default()
        };
        !Self::alt_is_meta(facts, cx)
    }

    fn text_for_range(
        &mut self,
        _: Range<usize>,
        _: &mut Option<Range<usize>>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<String> {
        None
    }

    fn selected_text_range(
        &mut self,
        _: bool,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        let end = self
            .marked
            .as_ref()
            .map_or(0, |marked| marked.encode_utf16().count());
        Some(UTF16Selection {
            range: end..end,
            reversed: false,
        })
    }

    fn marked_text_range(&self, _: &mut Window, _: &mut Context<Self>) -> Option<Range<usize>> {
        self.marked
            .as_ref()
            .map(|marked| 0..marked.encode_utf16().count())
    }

    fn unmark_text(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        log::debug!("terminal {}: composing ends", self.pty.pid);
        self.marked = None;
        cx.notify();
    }

    fn replace_text_in_range(
        &mut self,
        _: Option<Range<usize>>,
        text: &str,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.marked = None;
        if !text.is_empty() {
            log::trace!("terminal {}: typed {text:?}", self.pty.pid);
            self.input(text.as_bytes().to_vec(), cx);
        }
        cx.notify();
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        _: Option<Range<usize>>,
        text: &str,
        _: Option<Range<usize>>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        log::debug!("terminal {}: composing {text:?}", self.pty.pid);
        self.marked = (!text.is_empty()).then(|| text.to_string());
        cx.notify();
    }

    fn bounds_for_range(
        &mut self,
        _: Range<usize>,
        _: Bounds<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        log::debug!(
            "terminal {}: the input method asks for the caret, {:?}",
            self.pty.pid,
            self.caret
        );
        Some(self.caret)
    }

    fn character_index_for_point(
        &mut self,
        _: Point<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<usize> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_record_press_takes_its_repeats_and_release_with_it() {
        let plain = Kitty::default();
        // Ctrl-Shift-] goes as a record; Ctrl and Shift come up while ] repeats and types.
        assert!(records(plain, Action::Press, false, false));
        assert!(records(plain, Action::Repeat, true, true));
        assert!(records(plain, Action::Release, true, true));
        // A key that types goes as text, and so does its release.
        assert!(!records(plain, Action::Press, true, false));
        assert!(!records(plain, Action::Repeat, true, false));
        assert!(!records(plain, Action::Release, true, false));
        let kitty = Kitty {
            disambiguate: true,
            ..plain
        };
        assert!(
            !records(kitty, Action::Press, false, false),
            "kitty's flags go first"
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn an_editing_press_takes_its_repeats_and_release_with_it() {
        let editing = Some(Went::Editing);
        assert!(edits(Action::Press, None, true));
        assert!(
            !edits(Action::Press, editing, false),
            "a press goes by its own keys"
        );
        // Option came up while Left repeats and lets go: still the editing key's.
        assert!(edits(Action::Repeat, editing, false));
        assert!(edits(Action::Release, editing, false));
        // Option went down while Left was held: Left's own release still goes.
        assert!(!edits(Action::Repeat, Some(Went::Bytes), true));
        assert!(!edits(Action::Release, Some(Went::Bytes), true));
        assert!(
            edits(Action::Repeat, None, true),
            "a repeat of an unseen press"
        );
    }
}
