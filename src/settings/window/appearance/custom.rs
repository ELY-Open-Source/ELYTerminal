//! The card that adds a theme, and the worn custom theme's editor.

use ely_gpui_component::{
    buttons::{Button, ButtonVariant},
    forms::{ColorPicker, ColorSwatch, Input},
    i18n::text,
    primitives::{Icon, IconName},
    settings::SettingsRow,
    theme::{ActiveTheme, ControlSize, Radius},
};
use gpui::{
    App, Context, Entity, InteractiveElement, IntoElement, MouseButton, ParentElement,
    SharedString, StatefulInteractiveElement, Styled, div,
};

use super::super::{SettingsWindow, set};
use crate::{
    settings::SettingsStore,
    themes::{CustomTheme, SLOTS},
};

/// The card after the last theme: Ely Dark's copy, worn at once.
pub(super) fn add_card(this: &Entity<SettingsWindow>, cx: &App) -> impl IntoElement {
    let theme = cx.theme();
    let colors = &theme.colors;
    let this = this.clone();
    div()
        .id("add_theme")
        .size_full()
        .min_h_24()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap_2()
        .p_4()
        .rounded(theme.radius(Radius::Lg))
        .border_1()
        .border_dashed()
        .border_color(colors.border)
        .text_color(colors.fg_muted)
        .cursor_pointer()
        .hover(|card| card.border_color(colors.border_strong))
        .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
        .child(Icon::new(IconName::Plus))
        .child(text(cx, "et.custom.new", &[]))
        .on_click(move |_, _, cx| {
            let taken = SettingsStore::get(cx).custom_themes.clone();
            let made = CustomTheme::from_ely_dark(&taken, String::new());
            let number = made
                .id
                .trim_start_matches(crate::themes::PREFIX)
                .to_string();
            let made = CustomTheme {
                name: text(cx, "et.custom.name_of", &[("number", &number)]).to_string(),
                ..made
            };
            set(
                &this,
                format!("custom theme {} made", made.id),
                cx,
                |settings| {
                    settings.theme = made.id.clone();
                    settings.custom_themes.push(made);
                },
            );
        })
}

impl SettingsWindow {
    /// The name, colors and delete of `worn`, when it is a custom theme.
    pub(super) fn custom_editor(&self, worn: &str, cx: &Context<Self>) -> Vec<SettingsRow> {
        let settings = SettingsStore::get(cx);
        let Some(custom) = settings.custom_themes.iter().find(|theme| theme.id == worn) else {
            return Vec::new();
        };
        let this = cx.entity();
        let word = |key: &str| text(cx, key, &[]);
        let id = custom.id.clone();
        let slot = self.slot;
        let color = |slot: usize| {
            crate::themes::hex(custom.slot(slot)).expect("validation read every color")
        };
        let swatches = (0..SLOTS).map(|at| {
            let this = this.clone();
            ColorSwatch::new(("custom_swatch", at), color(at))
                .size(ControlSize::Sm)
                .selected(at == slot)
                .tooltip(slot_name(at, cx))
                .on_click(move |_, cx| {
                    this.update(cx, |this, cx| {
                        this.slot = at;
                        cx.notify();
                    })
                })
        });
        let picker = ColorPicker::new(
            SharedString::from(format!("custom_picker_{id}_{slot}")),
            color(slot),
        )
        .opaque()
        .on_change({
            let (this, id) = (this.clone(), id.clone());
            move |chosen, _, cx| {
                set(
                    &this,
                    format!("{id} color {slot}"),
                    cx,
                    |settings| match settings.custom_themes.iter_mut().find(|t| t.id == id) {
                        Some(theme) => theme.set(slot, chosen),
                        None => log::error!("settings: {id} went before its color saved"),
                    },
                );
            }
        });
        let width = cx.theme().prose_width();
        let colors = div()
            .w(width)
            .flex()
            .flex_col()
            .gap_3()
            .child(div().flex().flex_wrap().gap_1p5().children(swatches))
            .child(picker);
        let delete = Button::new("delete_theme", word("et.custom.delete"))
            .variant(ButtonVariant::Danger)
            .size(ControlSize::Sm)
            .on_click(move |_, _, cx| {
                set(
                    &this,
                    format!("custom theme {id} deleted"),
                    cx,
                    |settings| {
                        settings.custom_themes.retain(|theme| theme.id != id);
                        if settings.theme == id {
                            settings.theme = "ely-dark".into();
                        }
                        if settings.dark_theme.as_deref() == Some(id.as_str()) {
                            settings.dark_theme = None;
                        }
                    },
                );
            });
        vec![
            SettingsRow::new(word("et.custom.name"))
                .control(div().w(width).child(Input::new(&self.theme_name))),
            SettingsRow::new(word("et.custom.colors"))
                .description(word("et.custom.colors_hint"))
                .control(colors),
            SettingsRow::new(word("et.custom.remove"))
                .description(word("et.custom.remove_hint"))
                .control(delete),
        ]
    }
}

/// A slot's name: five named colors, then ANSI 0 to 15.
fn slot_name(slot: usize, cx: &App) -> SharedString {
    const NAMED: [&str; 5] = [
        "et.custom.background",
        "et.custom.foreground",
        "et.custom.cursor",
        "et.custom.cursor_text",
        "et.custom.selection",
    ];
    match NAMED.get(slot) {
        Some(key) => text(cx, key, &[]),
        None => text(cx, "et.custom.ansi", &[("number", &(slot - 5).to_string())]),
    }
}
