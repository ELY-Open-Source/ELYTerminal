//! Themes the user makes from Ely Dark, kept in the settings.

use anyhow::Context as _;
use gpui::{Hsla, Rgba, SharedString};
use serde::{Deserialize, Serialize};

use super::{TerminalTheme, find, hex};
use crate::settings::Settings;

/// The prefix every custom theme's id carries.
pub const PREFIX: &str = "custom-";

/// The editor's slots: five named colors, then the 16 ANSI colors.
pub const SLOTS: usize = 21;

/// A theme the user made; each color as `#rrggbb`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CustomTheme {
    pub id: String,
    pub name: String,
    pub background: String,
    pub foreground: String,
    pub cursor: String,
    pub cursor_text: String,
    pub selection: String,
    pub ansi: [String; 16],
}

impl CustomTheme {
    /// Ely Dark's colors under a new id after `taken` and `name`.
    pub fn from_ely_dark(taken: &[CustomTheme], name: String) -> Self {
        let base = find("ely-dark").expect("Ely Dark is built in");
        let next = taken
            .iter()
            .filter_map(|theme| theme.id.strip_prefix(PREFIX)?.parse::<u32>().ok())
            .max()
            .map_or(1, |most| most + 1);
        Self {
            id: format!("{PREFIX}{next}"),
            name,
            background: text(base.background),
            foreground: text(base.foreground),
            cursor: text(base.cursor),
            cursor_text: text(base.cursor_text),
            selection: text(base.selection_background),
            ansi: base.ansi.map(text),
        }
    }

    /// Its colors read; an error names the id and the color.
    pub fn theme(&self) -> anyhow::Result<TerminalTheme> {
        anyhow::ensure!(
            self.id.starts_with(PREFIX),
            "custom theme {:?} does not start with {PREFIX}",
            self.id
        );
        anyhow::ensure!(
            !self.name.trim().is_empty(),
            "custom theme {} has no name",
            self.id
        );
        let read = |slot: usize| {
            hex(self.slot(slot)).with_context(|| format!("custom theme {}, color {slot}", self.id))
        };
        let mut ansi = [Hsla::default(); 16];
        for (index, color) in ansi.iter_mut().enumerate() {
            *color = read(5 + index)?;
        }
        Ok(TerminalTheme {
            id: SharedString::from(self.id.clone()),
            name: SharedString::from(self.name.clone()),
            background: read(0)?,
            foreground: read(1)?,
            cursor: read(2)?,
            cursor_text: read(3)?,
            selection_background: read(4)?,
            selection_foreground: None,
            ansi,
            ely: false,
        })
    }

    /// The color in `slot`, in the editor's order.
    pub fn slot(&self, slot: usize) -> &str {
        match slot {
            0 => &self.background,
            1 => &self.foreground,
            2 => &self.cursor,
            3 => &self.cursor_text,
            4 => &self.selection,
            5..SLOTS => &self.ansi[slot - 5],
            _ => panic!("custom theme: no slot {slot}"),
        }
    }

    /// Sets `slot` to `color`.
    pub fn set(&mut self, slot: usize, color: Hsla) {
        let place = match slot {
            0 => &mut self.background,
            1 => &mut self.foreground,
            2 => &mut self.cursor,
            3 => &mut self.cursor_text,
            4 => &mut self.selection,
            5..SLOTS => &mut self.ansi[slot - 5],
            _ => panic!("custom theme: no slot {slot}"),
        };
        *place = text(color);
    }
}

/// Wears theme `id` where the system looks: dark's own theme while dark.
pub fn wear(settings: &mut Settings, id: &str, dark: bool) {
    match &mut settings.dark_theme {
        Some(worn) if dark => *worn = id.to_string(),
        _ => settings.theme = id.to_string(),
    }
}

/// Drops theme `id`; Ely Dark takes each place it held.
pub fn forget(settings: &mut Settings, id: &str) {
    settings.custom_themes.retain(|theme| theme.id != id);
    for place in [Some(&mut settings.theme), settings.dark_theme.as_mut()]
        .into_iter()
        .flatten()
    {
        if place == id {
            *place = "ely-dark".into();
        }
    }
}

/// `color` as `#rrggbb`.
fn text(color: Hsla) -> String {
    let rgba = Rgba::from(color);
    let byte = |channel: f32| (channel.clamp(0.0, 1.0) * 255.0).round() as u8;
    format!(
        "#{:02x}{:02x}{:02x}",
        byte(rgba.r),
        byte(rgba.g),
        byte(rgba.b)
    )
}

#[cfg(test)]
mod tests;
