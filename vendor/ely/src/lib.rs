pub mod account;
pub mod agent;
mod assets;

pub mod buttons;
pub mod calendar;
pub mod canvas;
pub mod charts;
pub mod chat;
pub mod collab;
pub mod dashboard;
pub mod data_display;
pub mod debug;
pub mod devtools;
pub mod documents;
pub mod editor;
pub mod feedback;
pub mod files;
pub mod finance;
pub mod forms;
pub mod generative;
pub mod git;
pub mod i18n;
pub mod interaction;
pub mod layout;
pub mod lists;
pub mod mail;
pub mod maps;
pub mod media;
pub mod menus;
pub mod messaging;
pub mod misc;
pub mod motion;
pub mod navigation;
pub mod onboarding;
pub mod overlays;
pub mod primitives;
pub mod project;
pub mod rendering;
pub mod settings;
pub mod shell;
pub mod tables;
pub mod terminal;
pub mod theme;
pub mod tooling;
pub mod typography;

pub use assets::{Assets, Before};

use anyhow::Context as _;
use gpui::{App, KeyBinding};

use crate::primitives::{FOCUS_CONTEXT, FocusNext, FocusPrev};

/// Loads fonts and the theme, binds Tab and text keys. Call once, first.
pub fn init(cx: &mut App) -> anyhow::Result<()> {
    let missing = assets::missing(cx.asset_source().as_ref());
    if let Some(first) = missing.first() {
        anyhow::bail!(
            "ely: the asset source lacks {} of Ely's files, first {first}; pass `ely_gpui_component::Assets`, or `Assets::before` your own, to `Application::with_assets`",
            missing.len()
        );
    }
    log::info!("assets: the app's source loads all of Ely's files");
    assets::load_fonts(cx).context("ely: embedded fonts failed to register")?;
    setup(cx);
    Ok(())
}

/// `init` without assets or fonts, for app tests that render.
#[cfg(any(test, feature = "test-support"))]
pub fn init_for_tests(cx: &mut App) {
    setup(cx);
}

/// The theme and key bindings.
fn setup(cx: &mut App) {
    theme::Theme::init(cx);
    cx.bind_keys([
        KeyBinding::new("tab", FocusNext, Some(FOCUS_CONTEXT)),
        KeyBinding::new("shift-tab", FocusPrev, Some(FOCUS_CONTEXT)),
        // Out of a field that keeps Tab, such as a document.
        KeyBinding::new("f6", FocusNext, Some(FOCUS_CONTEXT)),
        KeyBinding::new("shift-f6", FocusPrev, Some(FOCUS_CONTEXT)),
    ]);
    forms::bind_keys(cx);
    editor::bind_keys(cx);
    #[cfg(not(target_family = "wasm"))]
    terminal::bind_keys(cx);
    documents::bind_keys(cx);
}

#[cfg(all(test, feature = "test-support"))]
mod tests {
    use gpui::{AppContext, TestAppContext};

    #[gpui::test]
    #[should_panic(expected = "leaked handles")]
    fn a_leaked_entity_fails_its_test(cx: &mut TestAppContext) {
        std::mem::forget(cx.new(|_| ()));
    }
}
