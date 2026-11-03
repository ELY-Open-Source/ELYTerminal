use ely_gpui_component::theme::{Mix, Mode, Palette, Syntax, Theme};
use gpui::App;

use super::{TerminalTheme, active};
use crate::settings::SettingsStore;

/// The chrome's palette for a terminal theme, its glass as see-through as the background; Ely's own keep Ely's colors.
pub fn palette(theme: &TerminalTheme, opacity: f32) -> Palette {
    let base = if theme.is_dark() {
        Palette::dark(false)
    } else {
        Palette::light(false)
    };
    if theme.ely {
        return Palette {
            glass: base.bg.alpha(opacity),
            ..base
        };
    }
    let (bg, fg, ansi) = (theme.background, theme.foreground, theme.ansi);
    let tint = |share: f32| bg.mix(&fg, share);
    let wash = |color: gpui::Hsla| bg.mix(&color, 0.18);
    Palette {
        bg,
        surface: bg,
        sunken: tint(0.03),
        overlay: tint(0.07),
        hover: tint(0.1),
        active: tint(0.15),
        border: tint(0.14),
        border_strong: tint(0.26),
        fg,
        fg_muted: tint(0.8),
        fg_subtle: tint(0.62),
        fg_disabled: tint(0.36),
        accent: fg,
        accent_hover: tint(0.85),
        on_accent: bg,
        focus: ansi[4],
        link: ansi[12],
        selection: theme.selection_background.alpha(0.4),
        success: ansi[2],
        warning: ansi[3],
        danger: ansi[1],
        info: ansi[4],
        success_subtle: wash(ansi[2]),
        warning_subtle: wash(ansi[3]),
        danger_subtle: wash(ansi[1]),
        info_subtle: wash(ansi[4]),
        glass: bg.alpha(opacity),
        tooltip_bg: tint(0.2),
        tooltip_fg: fg,
        chart: [
            ansi[4], ansi[6], ansi[3], ansi[5], ansi[2], ansi[1], ansi[12], ansi[14],
        ],
        ansi,
        syntax: Syntax {
            keyword: ansi[5],
            string: ansi[2],
            number: ansi[3],
            comment: tint(0.62),
            function: ansi[4],
            type_name: ansi[6],
            constant: ansi[3],
            property: ansi[1],
            tag: ansi[1],
            attribute: ansi[3],
            operator: tint(0.8),
            punctuation: tint(0.8),
            variable: fg,
        },
        ..base
    }
}

/// Dresses the app in the theme the settings choose for the system's appearance.
pub fn apply(cx: &mut App) {
    let settings = SettingsStore::get(cx);
    let theme = active(settings, cx.window_appearance(), cx);
    // A picture grounds the window; the columns leave it as it is.
    let opacity = if settings.background_image.is_empty() {
        settings.background_opacity
    } else {
        0.0
    };
    let mode = if theme.is_dark() {
        Mode::Dark
    } else {
        Mode::Light
    };
    Theme::set_palette(mode, Some(palette(&theme, opacity)), cx);
    Theme::set_mode_now(mode, cx);
    log::info!("theme: {} in {mode:?}, glass at {opacity}", theme.id);
}
