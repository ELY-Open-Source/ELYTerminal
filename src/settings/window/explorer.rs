use ely_gpui_component::{buttons::Button, i18n::text, settings::SettingsRow, theme::ControlSize};
use gpui::{App, Entity, SharedString};

use super::SettingsWindow;
use crate::explorer::{self, CLASSES, Entry};

/// What Explorer's menu opens, and a button to change it.
pub(super) fn row(this: &Entity<SettingsWindow>, cx: &App) -> SettingsRow {
    let word = |key: &str| text(cx, key, &[]);
    let title = SettingsRow::new(word("et.explorer.title"));
    let exe = match std::env::current_exe() {
        Ok(exe) => exe,
        Err(error) => {
            return title.description(format!("explorer: no path to this program: {error}"));
        }
    };
    let entry = match explorer::read(CLASSES, &exe) {
        Ok(entry) => entry,
        Err(error) => return title.description(format!("{error:#}")),
    };
    let (status, key, add) = match &entry {
        Entry::Ours => (word("et.explorer.added"), "et.explorer.remove", false),
        Entry::Absent => (word("et.explorer.absent"), "et.explorer.add", true),
        Entry::Other(command) => (
            text(cx, "et.explorer.other", &[("command", command)]),
            "et.explorer.replace",
            true,
        ),
    };
    let label: SharedString = word("et.explorer.open");
    let this = this.clone();
    let button = Button::new("explorer-menu", word(key))
        .size(ControlSize::Sm)
        .on_click(move |_, _, cx| {
            let done = if add {
                explorer::add(CLASSES, &exe, &label)
            } else {
                explorer::remove(CLASSES)
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
