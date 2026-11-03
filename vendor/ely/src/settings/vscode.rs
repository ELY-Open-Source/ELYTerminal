use anyhow::{Context, Result, bail};
use gpui::Hsla;
use serde_json::Value;

use crate::{
    forms::parse_hex,
    theme::{Mode, Palette},
};

/// A VS Code color theme, read: its name and mode, the palette it gives, and the tokens it named and those kept from Ely's palette of that mode.
#[derive(Clone, Debug)]
pub struct VsCodeTheme {
    pub name: Option<String>,
    pub mode: Mode,
    pub colors: Palette,
    pub named: Vec<&'static str>,
    pub kept: Vec<&'static str>,
}

/// Each palette token and the VS Code colors that give it, first found first.
const COLORS: &[(&str, &[&str])] = &[
    ("bg", &["editor.background"]),
    (
        "surface",
        &["sideBar.background", "editorWidget.background"],
    ),
    (
        "sunken",
        &["input.background", "editorGroupHeader.tabsBackground"],
    ),
    (
        "overlay",
        &["editorWidget.background", "quickInput.background"],
    ),
    (
        "hover",
        &["list.hoverBackground", "toolbar.hoverBackground"],
    ),
    (
        "active",
        &[
            "list.activeSelectionBackground",
            "list.inactiveSelectionBackground",
        ],
    ),
    (
        "border",
        &["panel.border", "sideBar.border", "editorGroup.border"],
    ),
    ("border_strong", &["input.border", "contrastBorder"]),
    ("fg", &["editor.foreground", "foreground"]),
    ("fg_muted", &["descriptionForeground"]),
    ("fg_subtle", &["editorLineNumber.foreground"]),
    ("fg_disabled", &["disabledForeground"]),
    ("accent", &["button.background"]),
    ("accent_hover", &["button.hoverBackground"]),
    ("on_accent", &["button.foreground"]),
    ("focus", &["focusBorder"]),
    ("link", &["textLink.foreground"]),
    ("selection", &["editor.selectionBackground"]),
    (
        "success",
        &[
            "gitDecoration.addedResourceForeground",
            "terminal.ansiGreen",
        ],
    ),
    (
        "warning",
        &["editorWarning.foreground", "list.warningForeground"],
    ),
    ("danger", &["errorForeground", "editorError.foreground"]),
    ("info", &["editorInfo.foreground"]),
    ("tooltip_bg", &["editorHoverWidget.background"]),
    ("tooltip_fg", &["editorHoverWidget.foreground"]),
    ("shadow", &["widget.shadow"]),
];

/// Each syntax token and the TextMate scopes that color it, first found first.
const SCOPES: &[(&str, &[&str])] = &[
    ("keyword", &["keyword", "storage.type", "storage"]),
    ("string", &["string"]),
    ("number", &["constant.numeric"]),
    ("comment", &["comment"]),
    ("function", &["entity.name.function", "support.function"]),
    (
        "type_name",
        &["entity.name.type", "support.type", "entity.name.class"],
    ),
    ("constant", &["constant.language", "constant"]),
    (
        "property",
        &["variable.other.property", "support.type.property-name"],
    ),
    ("tag", &["entity.name.tag"]),
    ("attribute", &["entity.other.attribute-name"]),
    ("operator", &["keyword.operator"]),
    ("punctuation", &["punctuation"]),
    ("variable", &["variable"]),
];

const ANSI: [&str; 16] = [
    "terminal.ansiBlack",
    "terminal.ansiRed",
    "terminal.ansiGreen",
    "terminal.ansiYellow",
    "terminal.ansiBlue",
    "terminal.ansiMagenta",
    "terminal.ansiCyan",
    "terminal.ansiWhite",
    "terminal.ansiBrightBlack",
    "terminal.ansiBrightRed",
    "terminal.ansiBrightGreen",
    "terminal.ansiBrightYellow",
    "terminal.ansiBrightBlue",
    "terminal.ansiBrightMagenta",
    "terminal.ansiBrightCyan",
    "terminal.ansiBrightWhite",
];

/// The place past a comment that starts at `ix`, or `ix` when none does.
fn past_comment(chars: &[char], ix: usize) -> usize {
    match (chars.get(ix), chars.get(ix + 1)) {
        (Some('/'), Some('/')) => (ix..chars.len())
            .find(|at| chars[*at] == '\n')
            .unwrap_or(chars.len()),
        (Some('/'), Some('*')) => (ix + 2..chars.len())
            .find(|at| chars[*at] == '*' && chars.get(at + 1) == Some(&'/'))
            .map_or(chars.len(), |at| at + 2),
        _ => ix,
    }
}

/// Whether what comes next, past space and comments, closes an object or a list.
fn closes(chars: &[char], mut ix: usize) -> bool {
    loop {
        match chars.get(ix) {
            Some(ch) if ch.is_whitespace() => ix += 1,
            Some('/') if past_comment(chars, ix) != ix => ix = past_comment(chars, ix),
            Some('}' | ']') => return true,
            _ => return false,
        }
    }
}

/// JSON with comments and trailing commas, as VS Code writes it, made plain.
fn plain(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut ix = 0;
    while let Some(&ch) = chars.get(ix) {
        match ch {
            '"' => {
                let end = (ix + 1..chars.len())
                    .scan(false, |escaped, at| {
                        let end = !*escaped && chars[at] == '"';
                        *escaped = !*escaped && chars[at] == '\\';
                        Some((at, end))
                    })
                    .find(|(_, end)| *end)
                    .map_or(chars.len(), |(at, _)| at + 1);
                out.extend(&chars[ix..end]);
                ix = end;
            }
            '/' if past_comment(&chars, ix) != ix => ix = past_comment(&chars, ix),
            ',' if closes(&chars, ix + 1) => ix += 1,
            _ => {
                out.push(ch);
                ix += 1;
            }
        }
    }
    out
}

fn color(value: &Value, what: &str) -> Result<Hsla> {
    let text = value
        .as_str()
        .with_context(|| format!("{what} is not a color"))?;
    let rgba = parse_hex(text).map_err(|error| anyhow::anyhow!("{what}: {error}"))?;
    Ok(rgba.into())
}

/// The foreground the theme's token rules give `scope`, if a rule names it.
fn scoped(rules: &[Value], scope: &str) -> Result<Option<Hsla>> {
    for rule in rules {
        let names: Vec<&str> = match rule.get("scope") {
            Some(Value::String(names)) => names.split(',').map(str::trim).collect(),
            Some(Value::Array(names)) => names.iter().filter_map(Value::as_str).collect(),
            _ => continue,
        };
        let Some(foreground) = rule.pointer("/settings/foreground") else {
            continue;
        };
        if names.contains(&scope) {
            return color(foreground, scope).map(Some);
        }
    }
    Ok(None)
}

/// Reads a VS Code color theme; fails on text that is not JSON, a type that is neither light nor dark, or a color that does not read.
pub fn read_vscode_theme(text: &str) -> Result<VsCodeTheme> {
    let json: Value = serde_json::from_str(&plain(text)).context("the theme is not JSON")?;
    let name = json.get("name").and_then(Value::as_str).map(str::to_string);
    let mode = match json.get("type").and_then(Value::as_str) {
        Some("dark" | "hc-black") => Mode::Dark,
        Some("light" | "hc-light") => Mode::Light,
        other => bail!("the theme's type {other:?} is neither light nor dark"),
    };
    let mut colors = match mode {
        Mode::Light => Palette::light(false),
        Mode::Dark => Palette::dark(false),
    };
    let given = json
        .get("colors")
        .and_then(Value::as_object)
        .context("the theme has no colors")?;
    let (mut named, mut kept) = (Vec::new(), Vec::new());
    for (token, keys) in COLORS {
        match keys
            .iter()
            .find_map(|key| given.get(*key).map(|value| (key, value)))
        {
            Some((key, value)) => {
                *colors.token_mut(token) = color(value, key)?;
                named.push(*token);
            }
            None => kept.push(*token),
        }
    }
    for (ix, key) in ANSI.iter().enumerate() {
        if let Some(value) = given.get(*key) {
            colors.ansi[ix] = color(value, key)?;
        }
    }
    let rules: &[Value] = match json.get("tokenColors") {
        Some(Value::Array(rules)) => rules,
        Some(_) => bail!("the theme's tokenColors is not a list"),
        None => &[],
    };
    for (token, scopes) in SCOPES {
        let mut found = None;
        for scope in *scopes {
            found = scoped(rules, scope)?;
            if found.is_some() {
                break;
            }
        }
        match found {
            Some(ink) => {
                *colors.syntax.token_mut(token) = ink;
                named.push(*token);
            }
            None => kept.push(*token),
        }
    }
    log::info!(
        "vscode theme {name:?}: {} named, {} kept",
        named.len(),
        kept.len()
    );
    Ok(VsCodeTheme {
        name,
        mode,
        colors,
        named,
        kept,
    })
}

#[cfg(test)]
mod tests {
    use gpui::{rgb, rgba};

    use super::{plain, read_vscode_theme};
    use crate::theme::{Mode, Palette};

    const THEME: &str = r##"{
        // a theme as VS Code writes it
        "name": "Harbor",
        "type": "dark",
        "colors": {
            "editor.background": "#101820", /* the page */
            "editor.foreground": "#e0e6ee",
            "focusBorder": "#4a90e2aa",
        },
        "tokenColors": [
            { "scope": "comment", "settings": { "foreground": "#6a7680" } },
            { "scope": ["keyword", "storage"], "settings": { "foreground": "#c792ea" } },
            { "scope": "string, string.quoted", "settings": { "foreground": "#9ece6a" } },
        ],
    }"##;

    #[test]
    fn comments_and_trailing_commas_fall_away_and_strings_stay() {
        assert_eq!(
            plain(r#"{"a": "x // y", /* z */ "b": [1, 2,],}"#),
            r#"{"a": "x // y",  "b": [1, 2]}"#
        );
    }

    #[test]
    fn a_high_contrast_theme_reads_as_its_mode() {
        let mode = |kind: &str| {
            read_vscode_theme(&format!(r#"{{"type": "{kind}", "colors": {{}}}}"#))
                .expect("a theme")
                .mode
        };
        assert_eq!(mode("hc-black"), Mode::Dark);
        assert_eq!(mode("hc-light"), Mode::Light);
    }

    #[test]
    fn a_four_digit_color_reads_with_its_alpha() {
        let theme =
            read_vscode_theme(r##"{"type": "dark", "colors": {"widget.shadow": "#0008"}}"##)
                .expect("a theme");
        assert_eq!(theme.colors.shadow, rgba(0x00000088).into());
    }

    #[test]
    fn a_terminal_color_reads_into_its_place() {
        let theme =
            read_vscode_theme(r##"{"type": "dark", "colors": {"terminal.ansiRed": "#ff5555"}}"##)
                .expect("a theme");
        assert_eq!(theme.colors.ansi[1], rgb(0xff5555).into());
        assert_eq!(theme.colors.ansi[2], Palette::dark(false).ansi[2]);
    }

    #[test]
    fn the_first_scope_listed_wins() {
        let theme = read_vscode_theme(
            r##"{"type": "dark", "colors": {}, "tokenColors": [
                {"scope": "storage", "settings": {"foreground": "#111111"}},
                {"scope": "keyword", "settings": {"foreground": "#222222"}}
            ]}"##,
        )
        .expect("a theme");
        assert_eq!(theme.colors.syntax.keyword, rgb(0x222222).into());
    }

    #[test]
    fn a_theme_names_what_it_gives_and_keeps_the_rest() {
        let theme = read_vscode_theme(THEME).expect("a theme");
        assert_eq!(theme.name.as_deref(), Some("Harbor"));
        assert_eq!(theme.mode, Mode::Dark);
        assert_eq!(theme.colors.bg, rgb(0x101820).into());
        assert_eq!(theme.colors.syntax.keyword, rgb(0xc792ea).into());
        assert_eq!(theme.colors.syntax.string, rgb(0x9ece6a).into());
        for named in ["bg", "fg", "focus", "comment", "keyword", "string"] {
            assert!(theme.named.contains(&named), "{named} named");
        }
        assert!(theme.kept.contains(&"surface"));
        assert_eq!(
            theme.colors.surface,
            Palette::dark(false).surface,
            "what it leaves out keeps Ely's dark palette"
        );
    }

    #[test]
    fn a_bad_type_color_or_text_fails() {
        let error = read_vscode_theme(r#"{"type": "sepia", "colors": {}}"#).expect_err("a type");
        assert!(
            error.to_string().contains("neither light nor dark"),
            "{error}"
        );
        assert!(
            read_vscode_theme(r#"{"type": "light", "colors": {"editor.background": "blue"}}"#)
                .is_err()
        );
        assert!(
            read_vscode_theme(r#"{"type": "light", "colors": {}, "tokenColors": {}}"#).is_err()
        );
        assert!(read_vscode_theme("{ not json").is_err());
    }
}
