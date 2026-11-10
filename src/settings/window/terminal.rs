use std::path::PathBuf;

use ely_gpui_component::{
    buttons::{Button, SegmentedControl},
    feedback::InlineMessage,
    forms::{NumberInput, PathInput, Switch},
    i18n::text,
    primitives::Severity,
    settings::{SettingsRow, SettingsSection},
    theme::{ActiveTheme, ControlSize},
};
use gpui::{App, Context, Div, Entity, ParentElement, SharedString, Styled, Window, div};

use super::{SettingsWindow, choice, set, switch};
use crate::{
    settings::{OptionAsAlt, SCROLLBACK, SettingsStore, StartIn},
    updates::{self, Updates, VERSION},
};

pub(super) mod profiles;

/// Each way Option types Alt, its key and name's key.
const OPTIONS: [(OptionAsAlt, &str, &str); 4] = [
    (OptionAsAlt::Off, "off", "et.option.off"),
    (OptionAsAlt::Left, "left", "et.option.left"),
    (OptionAsAlt::Right, "right", "et.option.right"),
    (OptionAsAlt::Both, "both", "et.option.both"),
];

impl SettingsWindow {
    /// The terminal's settings in order, then the shortcuts.
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
        let right = switch(
            &this,
            "right_click_paste",
            settings.right_click_paste,
            |settings, on| settings.right_click_paste = on,
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
        let files_menu = switch(&this, "files_menu", settings.files_menu, |settings, on| {
            settings.files_menu = on
        });
        let section = section
            .row(SettingsRow::new(word("et.terminal.copy_on_select")).control(copy))
            .row(
                SettingsRow::new(word("et.terminal.right_click_paste"))
                    .description(word("et.terminal.right_click_paste_hint"))
                    .control(right),
            )
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
                SettingsRow::new(word("et.terminal.files_menu"))
                    .description(word("et.terminal.files_menu_hint"))
                    .control(files_menu),
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
            .row(updates_row(cx))
            .row(
                SettingsRow::new(word("et.terminal.control"))
                    .description(word("et.terminal.control_hint"))
                    .control(switch(
                        &this,
                        "control",
                        settings.control,
                        |settings, on| settings.control = on,
                    )),
            );
        #[cfg(windows)]
        let section = section.row(super::explorer::row(&cx.entity(), cx));
        let keys = self.shortcuts(cx);
        let profiles = self.profiles_section(cx);
        div()
            .flex()
            .flex_col()
            .gap_6()
            .child(section)
            .child(profiles)
            .child(keys)
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
pub(super) fn chosen(
    this: &Entity<SettingsWindow>,
    field: &'static str,
) -> impl Fn(PathBuf, &mut Window, &mut App) + 'static {
    let this = this.clone();
    move |path, _, cx| this.update(cx, |this, cx| this.chosen(field, path, cx))
}

/// A path field, with why its text is not saved.
pub(super) fn path_field(path: PathInput, refused: Option<SharedString>, cx: &App) -> Div {
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
