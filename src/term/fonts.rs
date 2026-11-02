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
    let cjk = (!settings.cjk_font.is_empty()).then(|| Font {
        family: settings.cjk_font.clone().into(),
        ..font.clone()
    });
    Ok(Look {
        font_id: id,
        icons,
        cjk,
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

/// Han, kana, Hangul, Bopomofo, CJK punctuation, vertical, small and fullwidth forms.
pub fn cjk(ch: char) -> bool {
    matches!(
        ch,
        '\u{1100}'..='\u{11FF}'
            | '\u{2E80}'..='\u{2FFF}'
            | '\u{3000}'..='\u{4DBF}'
            | '\u{4E00}'..='\u{9FFF}'
            | '\u{A960}'..='\u{A97F}'
            | '\u{AC00}'..='\u{D7FF}'
            | '\u{F900}'..='\u{FAFF}'
            | '\u{FE10}'..='\u{FE1F}'
            | '\u{FE30}'..='\u{FE6F}'
            | '\u{FF00}'..='\u{FFEF}'
            | '\u{16FE0}'..='\u{16FFF}'
            | '\u{1AFF0}'..='\u{1B16F}'
            | '\u{20000}'..='\u{3FFFF}'
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn cjk_holds_han_kana_hangul_and_fullwidth_alone() {
        let more = [
            '\u{FE11}',
            '\u{FE12}',
            '\u{FE51}',
            '\u{1B000}',
            '\u{1AFF0}',
            '\u{1B11F}',
        ];
        for ch in ['中', '文', 'か', 'カ', '한', 'ㄅ', '。', '，', 'Ａ', '𠀀']
            .into_iter()
            .chain(more)
        {
            assert!(super::cjk(ch), "{ch}");
        }
        for ch in [
            'a', 'M', '─', '█', '\u{E0B0}', 'é', 'Ж', '😀', '\u{FE00}', '\u{FE20}',
        ] {
            assert!(!super::cjk(ch), "{ch}");
        }
    }
}
