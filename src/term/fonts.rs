//! The terminal's font, cell and metrics.

use gpui::{
    Font, FontFallbacks, FontFeatures, FontId, FontStyle, FontWeight, Pixels, TextSystem, Window,
    px, size,
};

use super::{ink::Ink, paint::Look};
use crate::settings::Settings;

/// The bundled Nerd Font that draws icons any font lacks; gpui loads no font without an `m`.
pub const ICONS: &str = "JetBrainsMono Nerd Font Mono";

/// The bundled code font, the default.
pub const MONO: &str = "JetBrains Mono";

/// `font` as gpui loads it; gpui falls back to another font without a word, and loads none without an `m`.
pub fn load_font(font: &Font, system: &TextSystem) -> anyhow::Result<FontId> {
    let id = system.resolve_font(font);
    let loaded = system.get_font_for_id(id);
    anyhow::ensure!(
        loaded.is_some_and(|loaded| loaded.family == font.family),
        "the terminal cannot load {:?}",
        font.family
    );
    Ok(id)
}

/// Font, cell and metrics at `points` on this window.
pub(super) fn look(
    settings: &Settings,
    points: f32,
    ink: &Ink,
    window: &Window,
) -> anyhow::Result<Look> {
    let font = Font {
        family: settings.font_family.clone().into(),
        features: if settings.ligatures {
            FontFeatures::default()
        } else {
            FontFeatures::disable_ligatures()
        },
        fallbacks: (!settings.font_fallbacks.is_empty())
            .then(|| FontFallbacks::from_fonts(settings.font_fallbacks.clone())),
        weight: FontWeight(settings.font_weight),
        style: FontStyle::Normal,
    };
    let text = px(points);
    let system = window.text_system();
    let id = load_font(&font, system)?;
    let advance = system
        .advance(id, text, 'M')
        .map_err(|error| anyhow::anyhow!("{} cannot measure M: {error}", settings.font_family))?;
    let scale = window.scale_factor();
    let snap = |length: Pixels| px((f32::from(length) * scale).round() / scale);
    let cell = size(
        snap(advance.width).max(px(1.)),
        snap(text * settings.line_height),
    );
    let (ascent, descent) = (system.ascent(id, text), system.descent(id, text).abs());
    let icons = Font {
        family: ICONS.into(),
        features: FontFeatures::default(),
        ..font.clone()
    };
    Ok(Look {
        font_id: id,
        icons,
        baseline: (cell.height - ascent - descent) / 2. + ascent,
        descent,
        rule: snap((text / 14.).max(px(1.))),
        size: text,
        cell,
        font,
        cursor: ink.cursor,
        cursor_text: ink.cursor_text,
        fg: ink.fg,
        bg: ink.bg,
    })
}
