use ely_gpui_component::{
    buttons::{Button, ButtonVariant},
    forms::FontPicker,
    i18n::text,
    settings::SettingsRow,
    theme::{ActiveTheme, ControlSize},
};
use gpui::{Context, ParentElement, Styled, div, prelude::FluentBuilder};

use super::super::{SettingsWindow, choice, set};
use crate::settings::{Antialias, SettingsStore};

/// Each antialiasing, its key and its name's key.
const ANTIALIAS: [(Antialias, &str, &str); 2] = [
    (Antialias::Grayscale, "grayscale", "et.antialias.grayscale"),
    (Antialias::Subpixel, "subpixel", "et.antialias.subpixel"),
];

impl SettingsWindow {
    /// The family CJK characters draw in; empty leaves them to Family.
    pub(super) fn cjk_row(&self, cx: &Context<Self>) -> SettingsRow {
        let this = cx.entity();
        let word = |key: &str| text(cx, key, &[]);
        let family = SettingsStore::get(cx).cjk_font.clone();
        let mut picker = FontPicker::new("cjk_font", &self.cjk_font).on_change({
            let this = this.clone();
            move |family, window, cx| {
                let family = family.to_string();
                let change = format!("cjk_font is {family:?}");
                // A refused pick shows the saved family, the list closed.
                if !set(&this, change, cx, |settings| settings.cjk_font = family) {
                    let saved = SettingsStore::get(cx).cjk_font.clone();
                    let input = this.read(cx).cjk_font.clone();
                    input.update(cx, |input, cx| input.set_text(saved, cx));
                    window.blur(cx);
                }
            }
        });
        if !family.is_empty() {
            picker = picker.selected(family.clone());
        }
        let clear = Button::new("cjk_clear", word("et.appearance.cjk_clear"))
            .variant(ButtonVariant::Ghost)
            .size(ControlSize::Sm)
            .on_click(move |_, _, cx| {
                let change = "cjk_font is none".to_string();
                if set(&this, change, cx, |settings| settings.cjk_font.clear()) {
                    let input = this.read(cx).cjk_font.clone();
                    input.update(cx, |input, cx| input.set_text("", cx));
                }
            });
        let font = div()
            .w(cx.theme().prose_width())
            .flex()
            .flex_col()
            .gap_1()
            .child(picker)
            .when(!family.is_empty(), |font| {
                font.child(div().flex().child(clear))
            });
        SettingsRow::new(word("et.appearance.cjk_font"))
            .description(word("et.appearance.cjk_font_hint"))
            .control(font)
    }

    /// Grayscale or subpixel glyphs; none on macOS, where gpui draws no subpixel.
    pub(super) fn antialias_row(&self, cx: &Context<Self>) -> Option<SettingsRow> {
        if cfg!(target_os = "macos") {
            return None;
        }
        let now = SettingsStore::get(cx).text_antialias;
        let control = choice(
            &cx.entity(),
            "text_antialias",
            &ANTIALIAS,
            now,
            |settings, antialias| settings.text_antialias = antialias,
            cx,
        );
        let row = SettingsRow::new(text(cx, "et.appearance.antialias", &[])).description(text(
            cx,
            "et.appearance.antialias_hint",
            &[],
        ));
        Some(row.control(control))
    }
}
