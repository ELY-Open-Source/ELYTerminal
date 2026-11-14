use ely_gpui_component::{buttons::Button, i18n::text, settings::SettingsRow, theme::ControlSize};
use gpui::{App, Entity};

use super::SettingsWindow;
use crate::defterm::{self, Default, KEYS};

/// Who opens console programs, and a button to change it.
pub(super) fn row(this: &Entity<SettingsWindow>, cx: &App) -> SettingsRow {
    let word = |key: &str| text(cx, key, &[]);
    let title = SettingsRow::new(word("et.defterm.title"));
    let exe = match std::env::current_exe() {
        Ok(exe) => exe,
        Err(error) => {
            return title.description(format!("defterm: no path to this program: {error}"));
        }
    };
    let found = match defterm::read(&KEYS, &exe) {
        Ok(found) => found,
        Err(error) => return title.description(format!("{error:#}")),
    };
    let (status, key, make) = match &found {
        Default::Ours => (word("et.defterm.ours"), "et.defterm.restore", false),
        Default::Elsewhere => (word("et.defterm.elsewhere"), "et.defterm.make", true),
        Default::Copy(command) => (
            text(cx, "et.defterm.other", &[("command", command)]),
            "et.explorer.replace",
            true,
        ),
    };
    let this = this.clone();
    let button = Button::new("default-terminal", word(key))
        .size(ControlSize::Sm)
        .on_click(move |_, _, cx| {
            let done = if make {
                defterm::make(&KEYS, &exe)
            } else {
                defterm::restore(&KEYS)
            };
            this.update(cx, |window, cx| {
                window.failed = done.err().map(|error| {
                    log::error!("{error:#}");
                    format!("{error:#}").into()
                });
                cx.notify();
            });
        });
    title.description(status).control(button)
}
