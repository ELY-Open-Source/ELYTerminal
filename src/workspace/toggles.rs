use ely_gpui_component::{
    buttons::{ButtonVariant, IconButton},
    i18n::text,
    primitives::IconName,
    theme::ControlSize,
};
use gpui::{App, Context, Entity, SharedString};

use super::Workspace;
use crate::settings::{PanelTab, Settings, SettingsStore};

impl Workspace {
    /// Shows or hides a side column, as the settings keep it.
    pub(super) fn toggle(&mut self, flip: fn(&mut Settings), cx: &mut Context<Self>) {
        match SettingsStore::update(cx, flip) {
            Ok(()) => {
                let settings = SettingsStore::get(cx);
                let shown = |on: bool| if on { "shown" } else { "hidden" };
                log::info!(
                    "app: sessions {}, right column {}",
                    shown(settings.show_sessions),
                    shown(settings.show_panel)
                );
            }
            Err(error) => log::error!("app: the columns stay: {error:#}"),
        }
    }
}

/// Shows Git or Files in the right column, as the settings keep it.
pub(super) fn show_tab(value: &SharedString, cx: &mut App) {
    let panel = match value.as_ref() {
        "git" => PanelTab::Git,
        "files" => PanelTab::Files,
        other => unreachable!("app: no {other} tab"),
    };
    if SettingsStore::get(cx).panel == panel {
        return;
    }
    match SettingsStore::update(cx, |settings| settings.panel = panel) {
        Ok(()) => log::info!("app: the right column shows {panel:?}"),
        Err(error) => log::error!("app: the right column stays: {error:#}"),
    }
}

/// A header button that hides or shows a column, saved.
pub(super) fn column_toggle(
    workspace: &Entity<Workspace>,
    (id, icon): (&'static str, IconName),
    (shown, [hide, show]): (bool, [&str; 2]),
    flip: fn(&mut Settings),
    cx: &App,
) -> IconButton {
    let workspace = workspace.clone();
    IconButton::new(id, icon)
        .variant(ButtonVariant::Ghost)
        .size(ControlSize::Sm)
        .tooltip(text(cx, if shown { hide } else { show }, &[]))
        .on_click(move |_, _, cx| workspace.update(cx, |this, cx| this.toggle(flip, cx)))
}
