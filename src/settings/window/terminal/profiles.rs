//! Settings › Terminal's profiles: each a name, a command line and a folder, as Windows Terminal's.

use std::path::PathBuf;

use ely_gpui_component::{
    buttons::{Button, ButtonVariant},
    feedback::InlineMessage,
    forms::{Input, InputEvent, PathInput, TextInput},
    i18n::text,
    primitives::{IconName, Severity},
    settings::{SettingsRow, SettingsSection},
    theme::{ActiveTheme, ControlSize},
};
use gpui::{
    App, AppContext, Context, Entity, ParentElement, SharedString, Styled, Subscription, Window,
    div,
};

use super::super::{SettingsWindow, set};
use crate::{
    settings::{Profile, SettingsStore, command_line},
    term::shell,
};

/// A profile's fields, and why the last edit did not save.
pub struct Fields {
    name: Entity<TextInput>,
    command: Entity<TextInput>,
    folder: Entity<TextInput>,
    refused: Option<SharedString>,
    _typed: [Subscription; 3],
}

/// Fields for each profile the settings hold, each saving as it changes.
pub fn fields(window: &mut Window, cx: &mut Context<SettingsWindow>) -> Vec<Fields> {
    let profiles = SettingsStore::get(cx).profiles.clone();
    let mut input = |value: String, placeholder: &str, cx: &mut Context<SettingsWindow>| {
        let placeholder = text(cx, placeholder, &[]);
        cx.new(|cx| {
            let mut input = TextInput::new(window, cx).placeholder(placeholder);
            input.set_text(value, cx);
            input
        })
    };
    let mut made = Vec::new();
    for (at, profile) in profiles.into_iter().enumerate() {
        let name = input(profile.name, "et.profile.name", cx);
        let command = input(profile.command, "et.profile.command", cx);
        let folder = input(profile.folder, "et.profile.folder", cx);
        let _typed = [&name, &command, &folder].map(|input| watch(input, at, cx));
        let refused = None;
        made.push(Fields {
            name,
            command,
            folder,
            refused,
            _typed,
        });
    }
    made
}

/// Saves profile `at` as its three fields show it when `input` changes, or keeps why not.
fn watch(input: &Entity<TextInput>, at: usize, cx: &mut Context<SettingsWindow>) -> Subscription {
    cx.subscribe(input, move |this, _, event: &InputEvent, cx| {
        if !matches!(event, InputEvent::Changed) {
            return;
        }
        let fields = &this.profiles[at];
        let read = |input: &Entity<TextInput>| input.read(cx).text().to_string();
        let profile = Profile {
            name: read(&fields.name),
            command: read(&fields.command),
            folder: read(&fields.folder),
        };
        let shown = format!("{profile:?}");
        let saved = SettingsStore::update(cx, |settings| settings.profiles[at] = profile);
        this.profiles[at].refused = match saved {
            Ok(()) => {
                log::info!("settings: profile {at} is {shown}");
                None
            }
            Err(error) => {
                log::debug!("settings: profile {at} stays: {error:#}");
                Some(format!("{error:#}").into())
            }
        };
        cx.notify();
    })
}

impl SettingsWindow {
    /// Each profile's fields and Remove, then Add profile.
    pub(super) fn profiles_section(&self, cx: &Context<Self>) -> SettingsSection {
        let this = cx.entity();
        let word = |key: &str| text(cx, key, &[]);
        let profiles = &SettingsStore::get(cx).profiles;
        let mut section =
            SettingsSection::new(word("et.profiles")).description(word("et.profiles_hint"));
        for (at, (profile, fields)) in profiles.iter().zip(&self.profiles).enumerate() {
            let remove = Button::new(("profile-remove", at), word("et.appearance.remove"))
                .variant(ButtonVariant::Ghost)
                .size(ControlSize::Sm)
                .on_click({
                    let this = this.clone();
                    move |_, window, cx| remove(&this, at, window, cx)
                });
            let folder = PathInput::new(("profile-folder", at), &fields.folder)
                .directories()
                .on_choose(chosen(fields.folder.clone()));
            let refused = fields.refused.clone();
            let control = div()
                .w(cx.theme().prose_width())
                .flex()
                .flex_col()
                .gap_1()
                .child(Input::new(&fields.name))
                .child(Input::new(&fields.command))
                .child(folder)
                .children(refused.map(|why| InlineMessage::new(Severity::Danger, why)))
                .child(div().flex().child(remove));
            section = section.row(SettingsRow::new(profile.name.clone()).control(control));
        }
        let add = Button::new("profile-add", word("et.profile.add"))
            .icon(IconName::Plus)
            .variant(ButtonVariant::Ghost)
            .size(ControlSize::Sm)
            .on_click(move |_, window, cx| add(&this, window, cx));
        section.row(div().flex().child(add))
    }
}

/// Browse's pick into the field, which saves it; a pick not in UTF-8 stays out, logged.
fn chosen(input: Entity<TextInput>) -> impl Fn(PathBuf, &mut Window, &mut App) + 'static {
    move |path, _, cx| match path.to_str() {
        Some(folder) => input.update(cx, |input, cx| input.set_text(folder, cx)),
        None => log::error!("settings: {} is not in UTF-8", path.display()),
    }
}

/// A profile of the login shell, named after the first number free.
fn add(this: &Entity<SettingsWindow>, window: &mut Window, cx: &mut App) {
    let command = match shell::default_shell().and_then(|program| command_line(&program)) {
        Ok(command) => command,
        Err(error) => return log::error!("settings: no profile added: {error:#}"),
    };
    let profiles = &SettingsStore::get(cx).profiles;
    let name = (1..)
        .map(|n| text(cx, "et.profile.new", &[("number", &n.to_string())]).to_string())
        .find(|name| profiles.iter().all(|profile| &profile.name != name))
        .expect("a free number");
    let folder = String::new();
    let profile = Profile {
        name,
        command,
        folder,
    };
    let change = format!("profile {:?} added", profile.name);
    if set(this, change, cx, |settings| settings.profiles.push(profile)) {
        this.update(cx, |this, cx| this.profiles = fields(window, cx));
    }
}

/// Profile `at` gone, the fields made again for those left.
fn remove(this: &Entity<SettingsWindow>, at: usize, window: &mut Window, cx: &mut App) {
    let change = format!("profile {at} removed");
    if set(this, change, cx, |settings| {
        settings.profiles.remove(at);
    }) {
        this.update(cx, |this, cx| this.profiles = fields(window, cx));
    }
}
