//! The Weight row: the weights the chosen family ships, named as WinRT names them.

use ely_gpui_component::{
    forms::{Choice, Combobox, TextInput},
    i18n::text,
    theme::{ActiveTheme, ControlSize},
};
use gpui::{App, Context, Div, Entity, ParentElement, SharedString, Styled, Window, div};

use super::{SettingsWindow, set};
use crate::settings::window::named;

/// The row: a list of the family's own weights, and nothing where it ships none.
pub(super) fn weight(
    this: &Entity<SettingsWindow>,
    state: &Entity<TextInput>,
    weights: &[u16],
    now: u16,
    cx: &Context<SettingsWindow>,
) -> Div {
    let choices: Vec<Choice> = weights
        .iter()
        .map(|weight| {
            // The number rides beside the name, so a weight the list does not name
            // still reads as one, and a pick's number is never a mystery.
            let label: SharedString = match named(*weight) {
                Some(key) => format!("{} {weight}", text(cx, key, &[])).into(),
                None => weight.to_string().into(),
            };
            Choice::new(weight.to_string(), label)
        })
        .collect();
    // A weight the family does not ship shows the nearest one it does, so the row
    // never stands empty when a saved weight outlives the family that drew it.
    let picked = nearest(weights, now);

    let chosen = this.clone();
    let combo = Combobox::new(("font_weight", this.entity_id().as_u64()), state, choices)
        .selected(picked.to_string())
        .size(ControlSize::Sm)
        .on_change(move |value, _, cx: &mut App| {
            if let Ok(weight) = value.parse::<u16>() {
                set(
                    &chosen,
                    format!("font_weight is {weight}"),
                    cx,
                    |settings| settings.font_weight = weight,
                );
            }
        });

    div()
        .flex()
        .flex_col()
        .gap_2()
        .w(cx.theme().prose_width())
        .child(combo)
}

/// `now` where `weights` holds it, else the weight there that lies closest.
fn nearest(weights: &[u16], now: u16) -> u16 {
    if weights.contains(&now) {
        return now;
    }
    weights
        .iter()
        .copied()
        .min_by_key(|weight| weight.abs_diff(now))
        .unwrap_or(now)
}

/// The input a weight row starts with, showing the weight it picked.
pub(crate) fn field(
    weights: &[u16],
    now: u16,
    window: &mut Window,
    cx: &mut Context<TextInput>,
) -> TextInput {
    let mut input = TextInput::new(window, cx);
    let picked = nearest(weights, now);
    let label = match named(picked) {
        Some(key) => format!("{} {picked}", text(cx, key, &[])),
        None => picked.to_string(),
    };
    input.set_text(label, cx);
    input
}
