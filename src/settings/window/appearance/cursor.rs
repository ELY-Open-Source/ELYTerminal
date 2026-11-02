use ely_gpui_component::{
    i18n::text,
    settings::{SettingsRow, SettingsSection},
    theme::{ActiveTheme, Radius},
};
use gpui::{
    App, Context, Div, FontWeight, ParentElement, Styled, Window, div, prelude::*, px, relative,
};

use super::super::{SettingsWindow, choice, switch};
use crate::{
    settings::{CursorStyle, Settings, SettingsStore},
    themes::{self, TerminalTheme},
};

/// Each cursor style, its key and its name's key.
const CURSORS: [(CursorStyle, &str, &str); 4] = [
    (CursorStyle::Block, "block", "et.cursor.block"),
    (CursorStyle::Bar, "bar", "et.cursor.bar"),
    (CursorStyle::Underline, "underline", "et.cursor.underline"),
    (CursorStyle::Hollow, "hollow", "et.cursor.hollow"),
];

impl SettingsWindow {
    /// Style, blink, glide and a preview that blinks with the terminal.
    pub(super) fn cursor_section(&self, window: &Window, cx: &Context<Self>) -> SettingsSection {
        let settings = SettingsStore::get(cx);
        let this = cx.entity();
        let word = |key: &str| text(cx, key, &[]);
        let style = choice(
            &this,
            "cursor_style",
            &CURSORS,
            settings.cursor_style,
            |settings, style| settings.cursor_style = style,
            cx,
        );
        let active = themes::active(settings, window.appearance(), cx);
        let lit = self.lit || !settings.cursor_blink;
        let preview = prompt(settings, &active, lit, cx);
        SettingsSection::new(word("et.appearance.cursor"))
            .row(SettingsRow::new(word("et.appearance.style")).control(style))
            .row(
                SettingsRow::new(word("et.appearance.blink")).control(switch(
                    &this,
                    "cursor_blink",
                    settings.cursor_blink,
                    |settings, on| settings.cursor_blink = on,
                )),
            )
            .row(
                SettingsRow::new(word("et.appearance.smooth")).control(switch(
                    &this,
                    "cursor_smooth",
                    settings.cursor_smooth,
                    |settings, on| settings.cursor_smooth = on,
                )),
            )
            .row(SettingsRow::new(word("et.appearance.preview")).control(preview))
    }
}

/// A prompt in the terminal's theme and font, its cursor drawn as the terminal draws it.
fn prompt(settings: &Settings, theme: &TerminalTheme, lit: bool, cx: &App) -> Div {
    let rule = (px(settings.font_size) / 14.).max(px(1.));
    let mark = div().absolute().bg(theme.cursor);
    let cursor = match settings.cursor_style {
        CursorStyle::Block => mark.inset_0(),
        CursorStyle::Bar => mark.top_0().bottom_0().left_0().w(rule * 2.),
        CursorStyle::Underline => mark.left_0().right_0().bottom_0().h(rule * 2.),
        CursorStyle::Hollow => div()
            .absolute()
            .inset_0()
            .border_1()
            .border_color(theme.cursor),
    };
    let chrome = cx.theme();
    div()
        .flex()
        .p_2()
        .rounded(chrome.radius(Radius::Sm))
        .border_1()
        .border_color(chrome.colors.border)
        .bg(theme.background)
        .text_color(theme.foreground)
        .font_family(settings.font_family.clone())
        .font_weight(FontWeight(settings.font_weight))
        .text_size(px(settings.font_size))
        .line_height(relative(settings.line_height))
        .child(div().text_color(theme.ansi[2]).child("$"))
        .child(" ls")
        .child(
            div()
                .relative()
                .child(" ")
                .when(lit, |cell| cell.child(cursor)),
        )
}
