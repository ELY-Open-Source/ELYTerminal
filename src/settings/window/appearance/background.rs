//! The background's rows, one control for each field.

use ely_gpui_component::{
    feedback::InlineMessage,
    forms::{Input, PathInput},
    i18n::text,
    primitives::Severity,
    settings::{SettingsRow, SettingsSection},
    theme::{ActiveTheme, ControlSize},
};
use gpui::{Context, Div, ParentElement, Styled, div};

use super::super::SettingsWindow;
use super::super::terminal::chosen;
use super::{choice, switch};
use crate::settings::{Gradient, Mask, Repeat, Scope, SettingsStore};

/// The repeat choices, their values and their names' keys.
const REPEATS: [(Repeat, &str, &str); 4] = [
    (Repeat::NoRepeat, "no_repeat", "et.background.no_repeat"),
    (Repeat::Repeat, "repeat", "et.background.repeat_all"),
    (Repeat::RepeatX, "repeat_x", "et.background.repeat_x"),
    (Repeat::RepeatY, "repeat_y", "et.background.repeat_y"),
];

/// Where a background paints, its value and its name's key.
const SCOPES: [(Scope, &str, &str); 2] = [
    (Scope::Terminal, "terminal", "et.background.scope_terminal"),
    (Scope::Window, "window", "et.background.scope_window"),
];

/// The background's rows.
pub(super) fn section(this: &SettingsWindow, cx: &Context<SettingsWindow>) -> SettingsSection {
    let settings = SettingsStore::get(cx);
    let word = |key: &str| text(cx, key, &[]);
    let owner = cx.entity();
    // A text field takes no width of its own, so each sits in a box wide enough for its value.
    let input = |key: &str| {
        div()
            .w(cx.theme().label_width())
            .child(Input::new(&this.background_field(key)).size(ControlSize::Sm))
    };
    let image = PathInput::new("settings-background-image", &this.background_field("image"))
        .on_choose(chosen(&owner, "background_image"));
    let refused = this
        .refused_background
        .clone()
        .map(|why| InlineMessage::new(Severity::Danger, why));
    let image = div()
        .w(cx.theme().prose_width())
        .flex()
        .flex_col()
        .gap_1()
        .child(image)
        .children(refused);
    let repeat = choice(
        &owner,
        "background.repeat",
        &REPEATS,
        settings.background.repeat,
        |settings, repeat| settings.background.repeat = repeat,
        cx,
    );
    let scope = choice(
        &owner,
        "background.scope",
        &SCOPES,
        settings.background.scope,
        |settings, scope| settings.background.scope = scope,
        cx,
    );
    let opacity = this.number(
        "background.opacity",
        settings.background.opacity,
        &(0.0..=1.0),
        (0.05, 2),
        |settings, opacity| settings.background.opacity = opacity,
        cx,
    );
    let gradient = switch(
        &owner,
        "background.gradient",
        settings.background.gradient.is_some(),
        |settings, on| {
            settings.background.gradient = on.then(Gradient::default);
        },
    );
    let gradient_angle = this.number(
        "background.gradient.angle",
        settings
            .background
            .gradient
            .as_ref()
            .map_or(0.0, |gradient| gradient.angle),
        &(0.0..=360.0),
        (1.0, 0),
        |settings, angle| {
            if let Some(gradient) = &mut settings.background.gradient {
                gradient.angle = angle;
            }
        },
        cx,
    );
    let mask = switch(
        &owner,
        "background.mask",
        settings.background.mask.is_some(),
        |settings, on| {
            settings.background.mask = on.then(Mask::default);
        },
    );
    let mask_angle = this.number(
        "background.mask.angle",
        settings
            .background
            .mask
            .as_ref()
            .map_or(0.0, |mask| mask.angle),
        &(0.0..=360.0),
        (1.0, 0),
        |settings, angle| {
            if let Some(mask) = &mut settings.background.mask {
                mask.angle = angle;
            }
        },
        cx,
    );
    let stops = gradient_stops(this, cx);
    let stops = SettingsRow::new(word("et.background.stops"))
        .description(word("et.background.stops_hint"))
        .control(stops);
    SettingsSection::new(word("et.appearance.background"))
        .row(
            SettingsRow::new(word("et.background.image"))
                .description(word("et.background.image_hint"))
                .control(image),
        )
        .row(SettingsRow::new(word("et.background.scope")).control(scope))
        .row(SettingsRow::new(word("et.background.color")).control(input("color")))
        .row(SettingsRow::new(word("et.background.size")).control(input("size")))
        .row(SettingsRow::new(word("et.background.position")).control(input("position")))
        .row(SettingsRow::new(word("et.background.repeat")).control(repeat))
        .row(SettingsRow::new(word("et.background.opacity")).control(opacity))
        .row(SettingsRow::new(word("et.background.gradient")).control(gradient))
        .row(SettingsRow::new(word("et.background.angle")).control(gradient_angle))
        .row(SettingsRow::new(word("et.background.from")).control(input("gradient_from")))
        .row(SettingsRow::new(word("et.background.to")).control(input("gradient_to")))
        .row(SettingsRow::new(word("et.background.mask")).control(mask))
        .row(SettingsRow::new(word("et.background.mask_angle")).control(mask_angle))
        .row(stops)
        .row(SettingsRow::new(word("et.background.mask_size")).control(input("mask_size")))
        .row(SettingsRow::new(word("et.background.mask_position")).control(input("mask_position")))
}

/// The mask's stops as one comma-separated field.
fn gradient_stops(this: &SettingsWindow, cx: &Context<SettingsWindow>) -> Div {
    div()
        .w(cx.theme().prose_width())
        .child(Input::new(&this.background_field("mask_stops")).size(ControlSize::Sm))
}
