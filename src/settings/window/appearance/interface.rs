use ely_gpui_component::{
    buttons::{Button, ButtonVariant, SegmentedControl},
    forms::FontPicker,
    i18n::text,
    settings::SettingsRow,
    theme::{ActiveTheme, ControlSize},
};
use gpui::{Context, ParentElement, SharedString, Styled, div, prelude::FluentBuilder};

use super::super::{SettingsWindow, set};
use crate::settings::{INTERFACE_SIZE, SettingsStore};

impl SettingsWindow {
    /// The interface's family and size, as ely-mark sets them.
    pub(super) fn interface_rows(&self, cx: &Context<Self>) -> [SettingsRow; 2] {
        let settings = SettingsStore::get(cx);
        let this = cx.entity();
        let word = |key: &str| text(cx, key, &[]);
        let family = settings.interface_font.clone();
        let mut picker = FontPicker::new("interface_font", &self.interface_font).on_change({
            let this = this.clone();
            move |family, window, cx| {
                let family = family.to_string();
                let change = format!("interface_font is {family:?}");
                // A refused pick shows the saved family, the list closed.
                if !set(&this, change, cx, |settings| {
                    settings.interface_font = family
                }) {
                    let saved = SettingsStore::get(cx).interface_font.clone();
                    let input = this.read(cx).interface_font.clone();
                    input.update(cx, |input, cx| input.set_text(saved, cx));
                    window.blur(cx);
                }
            }
        });
        if !family.is_empty() {
            picker = picker.selected(family.clone());
        }
        let system = Button::new("system_font", word("et.appearance.system_font"))
            .variant(ButtonVariant::Ghost)
            .size(ControlSize::Sm)
            .on_click({
                let this = this.clone();
                move |_, _, cx| {
                    let change = "interface_font is the system's".to_string();
                    if set(&this, change, cx, |settings| {
                        settings.interface_font.clear()
                    }) {
                        let input = this.read(cx).interface_font.clone();
                        input.update(cx, |input, cx| input.set_text("", cx));
                    }
                }
            });
        let font = div()
            .w(cx.theme().prose_width())
            .flex()
            .flex_col()
            .gap_1()
            .child(picker)
            .when(!family.is_empty(), |font| {
                font.child(div().flex().child(system))
            });
        let now = SharedString::from(settings.interface_size.to_string());
        let size = INTERFACE_SIZE
            .fold(
                SegmentedControl::new("interface_size", now),
                |control, size| control.segment(size.to_string(), format!("{size} px"), None),
            )
            .size(ControlSize::Sm)
            .on_change(move |key, _, cx| {
                let size: u8 = key.parse().expect("a size the control offers");
                let change = format!("interface_size is {size}");
                set(&this, change, cx, |settings| settings.interface_size = size);
            });
        [
            SettingsRow::new(word("et.appearance.interface_font")).control(font),
            SettingsRow::new(word("et.appearance.interface_size")).control(size),
        ]
    }
}
