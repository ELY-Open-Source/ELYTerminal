//! The interface's family and size, through Ely's theme.

use ely_gpui_component::theme::{ActiveTheme, Theme};
use gpui::{App, Div, Styled, rems};

use crate::settings::SettingsStore;

/// Ely's base text, which `interface_size` scales.
const BASE: f32 = 13.0;

/// gpui's name for the system's interface font.
const SYSTEM: &str = ".SystemUIFont";

/// Sets Ely's family and scale from the settings.
pub fn init(cx: &mut App) {
    let mut worn = None;
    let mut wear = move |cx: &mut App| {
        let settings = SettingsStore::get(cx);
        let size = settings.interface_size;
        let family = match settings.interface_font.as_str() {
            "" => SYSTEM.to_string(),
            family => family.to_string(),
        };
        if worn.as_ref() != Some(&(size, family.clone())) {
            log::info!("app: the interface's text is {family} at {size} px");
            worn = Some((size, family.clone()));
            Theme::update(cx, |theme| {
                theme.font_scale = f32::from(size) / BASE;
                theme.font_family = family.into();
            });
        }
    };
    wear(cx);
    cx.observe_global::<SettingsStore>(wear).detach();
}

/// A window's root in Ely's family and scale.
pub fn dress(root: Div, cx: &App) -> Div {
    let theme = cx.theme();
    // gpui's 1 rem, scaled, for text of no size.
    root.text_size(rems(theme.font_scale))
        .font_family(theme.font_family.clone())
}
