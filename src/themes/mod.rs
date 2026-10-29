mod chrome;
mod custom;
#[cfg(target_os = "linux")]
pub mod omarchy;

use std::sync::LazyLock;

use anyhow::Context as _;
use ely_gpui_component::theme::Palette;
use gpui::{App, Global, Hsla, Rgba, SharedString, WindowAppearance};

pub use chrome::{apply, palette};
pub use custom::{CustomTheme, PREFIX, SLOTS, forget, wear};

use crate::settings::Settings;

/// What a terminal paints with. Ely's own themes keep Ely's palette for the chrome.
#[derive(Clone, Debug, PartialEq)]
pub struct TerminalTheme {
    pub id: SharedString,
    pub name: SharedString,
    pub background: Hsla,
    pub foreground: Hsla,
    pub cursor: Hsla,
    pub cursor_text: Hsla,
    pub selection_background: Hsla,
    /// None keeps each cell's own color.
    pub selection_foreground: Option<Hsla>,
    pub ansi: [Hsla; 16],
    pub ely: bool,
}

impl TerminalTheme {
    pub fn is_dark(&self) -> bool {
        self.background.l < 0.5
    }
}

/// The settings' name for Omarchy's theme, on Linux.
pub const OMARCHY: &str = "omarchy";

/// Omarchy's current theme, while ELYTerminal follows it.
pub struct Omarchy(pub TerminalTheme);

impl Global for Omarchy {}

/// Upstream Ghostty files from iTerm2-Color-Schemes: id, shown name, text.
const FILES: [(&str, &str, &str); 12] = [
    (
        "catppuccin-mocha",
        "Catppuccin Mocha",
        include_str!("../../assets/themes/catppuccin-mocha"),
    ),
    (
        "catppuccin-macchiato",
        "Catppuccin Macchiato",
        include_str!("../../assets/themes/catppuccin-macchiato"),
    ),
    (
        "catppuccin-frappe",
        "Catppuccin Frappé",
        include_str!("../../assets/themes/catppuccin-frappe"),
    ),
    (
        "catppuccin-latte",
        "Catppuccin Latte",
        include_str!("../../assets/themes/catppuccin-latte"),
    ),
    (
        "tokyo-night",
        "Tokyo Night",
        include_str!("../../assets/themes/tokyo-night"),
    ),
    (
        "dracula",
        "Dracula",
        include_str!("../../assets/themes/dracula"),
    ),
    ("nord", "Nord", include_str!("../../assets/themes/nord")),
    (
        "gruvbox-dark",
        "Gruvbox Dark",
        include_str!("../../assets/themes/gruvbox-dark"),
    ),
    (
        "rose-pine",
        "Rosé Pine",
        include_str!("../../assets/themes/rose-pine"),
    ),
    (
        "solarized-light",
        "Solarized Light",
        include_str!("../../assets/themes/solarized-light"),
    ),
    (
        "one-light",
        "One Light",
        include_str!("../../assets/themes/one-light"),
    ),
    (
        "kanagawa-wave",
        "Kanagawa Wave",
        include_str!("../../assets/themes/kanagawa-wave"),
    ),
];

static THEMES: LazyLock<Vec<TerminalTheme>> = LazyLock::new(|| {
    let mut themes = vec![
        ely("ely-dark", "Ely Dark", Palette::dark(false)),
        ely("ely-light", "Ely Light", Palette::light(false)),
    ];
    themes.extend(FILES.iter().map(|(id, name, text)| {
        parse(id, name, text).unwrap_or_else(|error| panic!("bundled theme {id}: {error:#}"))
    }));
    themes
});

/// Every built-in theme, Ely's first.
pub fn all() -> &'static [TerminalTheme] {
    &THEMES
}

pub fn find(id: &str) -> Option<&'static TerminalTheme> {
    all().iter().find(|theme| theme.id == id)
}

/// A theme the settings may name: built in, or Omarchy's on Linux.
pub fn known(id: &str) -> bool {
    find(id).is_some() || (cfg!(target_os = "linux") && id == OMARCHY)
}

/// A fresh install wears Omarchy's theme unless its file is surely absent.
pub fn default_id() -> &'static str {
    #[cfg(target_os = "linux")]
    if omarchy::path().is_some_and(|path| !matches!(path.try_exists(), Ok(false))) {
        return OMARCHY;
    }
    "catppuccin-mocha"
}

/// The theme the settings choose under the system's appearance.
pub fn chosen(settings: &Settings, appearance: WindowAppearance) -> &str {
    let dark = matches!(
        appearance,
        WindowAppearance::Dark | WindowAppearance::VibrantDark
    );
    match (&settings.dark_theme, dark) {
        (Some(id), true) => id,
        _ => &settings.theme,
    }
}

/// The chosen theme's colors; Omarchy's come from its global, the user's from the settings.
pub fn active(settings: &Settings, appearance: WindowAppearance, cx: &App) -> TerminalTheme {
    let id = chosen(settings, appearance);
    if id == OMARCHY {
        return cx.global::<Omarchy>().0.clone();
    }
    if let Some(custom) = settings.custom_themes.iter().find(|theme| theme.id == id) {
        return custom
            .theme()
            .unwrap_or_else(|error| panic!("validation admits {id}: {error:#}"));
    }
    find(id)
        .unwrap_or_else(|| panic!("settings name theme {id}, which validation admits"))
        .clone()
}

fn ely(id: &'static str, name: &'static str, palette: Palette) -> TerminalTheme {
    TerminalTheme {
        id: SharedString::new_static(id),
        name: SharedString::new_static(name),
        background: palette.bg,
        foreground: palette.fg,
        cursor: palette.focus,
        cursor_text: palette.bg,
        selection_background: palette.selection,
        selection_foreground: None,
        ansi: palette.ansi,
        ely: true,
    }
}

/// Reads Ghostty's `key = value` theme format; every key it needs must be there.
fn parse(id: &'static str, name: &'static str, text: &str) -> anyhow::Result<TerminalTheme> {
    let mut ansi: [Option<Hsla>; 16] = [None; 16];
    let mut keys: Vec<(&str, Hsla)> = Vec::new();
    for line in text.lines().map(str::trim).filter(|line| !line.is_empty()) {
        let (key, value) = line
            .split_once('=')
            .with_context(|| format!("no `=` in `{line}`"))?;
        let (key, value) = (key.trim(), value.trim());
        if key == "palette" {
            let (index, color) = value
                .split_once('=')
                .with_context(|| format!("no index in `{line}`"))?;
            let index: usize = index.trim().parse().with_context(|| format!("`{line}`"))?;
            anyhow::ensure!(index < 16, "palette index {index} past 15");
            ansi[index] = Some(hex(color.trim())?);
        } else {
            keys.push((key, hex(value)?));
        }
    }
    let get = |key: &str| {
        keys.iter()
            .find(|(name, _)| *name == key)
            .map(|(_, color)| *color)
            .with_context(|| format!("no `{key}`"))
    };
    let mut colors = [Hsla::default(); 16];
    for (index, color) in ansi.iter().enumerate() {
        colors[index] = color.with_context(|| format!("no palette {index}"))?;
    }
    Ok(TerminalTheme {
        id: SharedString::new_static(id),
        name: SharedString::new_static(name),
        background: get("background")?,
        foreground: get("foreground")?,
        cursor: get("cursor-color")?,
        cursor_text: get("cursor-text")?,
        selection_background: get("selection-background")?,
        selection_foreground: Some(get("selection-foreground")?),
        ansi: colors,
        ely: false,
    })
}

pub(crate) fn hex(text: &str) -> anyhow::Result<Hsla> {
    let digits = text
        .strip_prefix('#')
        .with_context(|| format!("`{text}` is not #rrggbb"))?;
    anyhow::ensure!(digits.len() == 6, "`{text}` is not #rrggbb");
    let value = u32::from_str_radix(digits, 16).with_context(|| format!("`{text}`"))?;
    let channel = |shift: u32| ((value >> shift) & 0xff) as f32 / 255.0;
    Ok(Rgba {
        r: channel(16),
        g: channel(8),
        b: channel(0),
        a: 1.0,
    }
    .into())
}

#[cfg(test)]
mod tests;
