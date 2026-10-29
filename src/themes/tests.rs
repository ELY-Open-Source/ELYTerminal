use gpui::{Rgba, WindowAppearance};

use super::*;

fn rgb(hex: u32) -> Hsla {
    let channel = |shift: u32| ((hex >> shift) & 0xff) as f32 / 255.0;
    Rgba {
        r: channel(16),
        g: channel(8),
        b: channel(0),
        a: 1.0,
    }
    .into()
}

#[test]
fn every_bundled_file_parses_with_unique_ids() {
    assert_eq!(all().len(), 14);
    let mut ids: Vec<_> = all().iter().map(|theme| theme.id.clone()).collect();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), 14);
    let folder = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/themes");
    for entry in std::fs::read_dir(folder).expect("themes folder") {
        let name = entry
            .expect("entry")
            .file_name()
            .into_string()
            .expect("utf-8");
        if name != "LICENSE" {
            assert!(find(&name).is_some(), "{name} is bundled but not listed");
        }
    }
}

#[test]
fn catppuccin_mocha_reads_its_upstream_colors() {
    let mocha = find("catppuccin-mocha").expect("mocha");
    assert_eq!(mocha.background, rgb(0x1e1e2e));
    assert_eq!(mocha.foreground, rgb(0xcdd6f4));
    assert_eq!(mocha.cursor, rgb(0xf5e0dc));
    assert_eq!(mocha.ansi[1], rgb(0xf38ba8));
    assert_eq!(mocha.ansi[15], rgb(0xa6adc8));
    assert_eq!(mocha.selection_foreground, Some(rgb(0x1e1e2e)));
}

#[test]
fn light_and_dark_follow_the_background() {
    let light: Vec<_> = all()
        .iter()
        .filter(|theme| !theme.is_dark())
        .map(|theme| theme.id.clone())
        .collect();
    assert_eq!(
        light,
        [
            "ely-light",
            "catppuccin-latte",
            "solarized-light",
            "one-light"
        ]
    );
}

#[test]
fn a_missing_key_or_bad_color_names_it() {
    let text = include_str!("../../assets/themes/nord");
    let cut: String = text
        .lines()
        .filter(|line| !line.starts_with("cursor-text"))
        .collect::<Vec<_>>()
        .join("\n");
    let error = parse("x", "X", &cut).expect_err("no cursor-text");
    assert_eq!(error.to_string(), "no `cursor-text`");
    let cut: String = text
        .lines()
        .filter(|line| !line.starts_with("palette = 7="))
        .collect::<Vec<_>>()
        .join("\n");
    assert_eq!(
        parse("x", "X", &cut).expect_err("no 7").to_string(),
        "no palette 7"
    );
    let bad = text.replace("background = #", "background = ");
    assert!(parse("x", "X", &bad).is_err());
    assert!(parse("x", "X", "palette = 16=#000000").is_err());
}

/// Omarchy 4 renders Tokyo Night so; glass mode appends the last four lines.
#[cfg(target_os = "linux")]
fn tokyo_night() -> String {
    [
        "background = #1a1b26",
        "foreground = #a9b1d6",
        "cursor-color = #c0caf5",
        "selection-background = #292e42",
        "selection-foreground = #c0caf5",
        "",
        "palette = 0=#1a1b26",
        "palette = 1=#f7768e",
        "palette = 2=#9ece6a",
        "palette = 3=#e0af68",
        "palette = 4=#7aa2f7",
        "palette = 5=#ad8ee6",
        "palette = 6=#449dab",
        "palette = 7=#a9b1d6",
        "palette = 8=#414868",
        "palette = 9=#ff7a93",
        "palette = 10=#b9f27c",
        "palette = 11=#ff9e64",
        "palette = 12=#7da6ff",
        "palette = 13=#bb9af7",
        "palette = 14=#0db9d7",
        "palette = 15=#c0caf5",
        "# omarchy-glass begin",
        "background-opacity = 0.65",
        "gtk-custom-css = /usr/share/omarchy/default/gtk/glass.css",
        "# omarchy-glass end",
    ]
    .join("\n")
}

#[cfg(target_os = "linux")]
#[test]
fn omarchys_ghostty_conf_reads_without_its_glass() {
    let rendered = tokyo_night();
    let theme = omarchy::colors(&rendered).expect("Omarchy's file reads");
    assert_eq!(theme.id, OMARCHY);
    assert_eq!(theme.background, rgb(0x1a1b26));
    assert_eq!(theme.cursor, rgb(0xc0caf5));
    assert_eq!(theme.cursor_text, theme.background, "Ghostty's default");
    assert_eq!(theme.ansi[8], rgb(0x414868));
    let cut = rendered.replace("palette = 7=#a9b1d6\n", "");
    assert_eq!(
        omarchy::colors(&cut).expect_err("no 7").to_string(),
        "no palette 7"
    );
}

#[cfg(target_os = "linux")]
#[test]
fn omarchys_file_is_worn_again_once_it_reads() {
    use std::{fs, os::unix::fs::PermissionsExt};
    let folder = std::env::temp_dir().join(format!("elyterminal-omarchy-{}", std::process::id()));
    if folder.exists() {
        fs::remove_dir_all(&folder).expect("a stale test folder goes");
    }
    fs::create_dir_all(&folder).expect("a test folder");
    let path = folder.join("ghostty.conf");
    let text = tokyo_night();
    fs::write(&path, &text).expect("a theme");
    assert!(omarchy::check(&path, &text).expect("reads").is_none());
    fs::set_permissions(&path, fs::Permissions::from_mode(0o000)).expect("000");
    // Root reads any mode.
    if unsafe { libc::geteuid() } != 0 {
        let error = omarchy::check(&path, "").expect_err("unreadable");
        assert!(
            format!("{error:#}").contains("Permission denied"),
            "{error:#}"
        );
    }
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).expect("644");
    let (shown, theme) = omarchy::check(&path, "").expect("reads").expect("a theme");
    assert_eq!((shown, theme.background), (text, rgb(0x1a1b26)));
    fs::remove_dir_all(&folder).expect("the test folder goes");
    assert!(
        omarchy::check(&path, "")
            .expect("gone mid-switch")
            .is_none()
    );
}

#[test]
fn the_dark_theme_wins_only_in_dark_mode() {
    let settings = Settings {
        theme: "catppuccin-latte".into(),
        dark_theme: Some("catppuccin-mocha".into()),
        ..Settings::default()
    };
    assert_eq!(
        chosen(&settings, WindowAppearance::Light),
        "catppuccin-latte"
    );
    assert_eq!(
        chosen(&settings, WindowAppearance::VibrantDark),
        "catppuccin-mocha"
    );
    let single = Settings {
        dark_theme: None,
        ..settings
    };
    assert_eq!(chosen(&single, WindowAppearance::Dark), "catppuccin-latte");
}

#[test]
fn the_chrome_wears_the_theme() {
    let dracula = find("dracula").expect("dracula");
    let chrome = palette(dracula, 1.0);
    assert_eq!(chrome.bg, dracula.background);
    assert_eq!(chrome.fg, dracula.foreground);
    assert_eq!(chrome.ansi, dracula.ansi);
    assert_eq!(chrome.danger, dracula.ansi[1]);
    let ely = palette(find("ely-dark").expect("ely"), 1.0);
    assert_eq!(ely.bg, Palette::dark(false).bg, "Ely's own colors");
}

#[test]
fn the_glass_lets_through_what_the_opacity_leaves() {
    for id in ["dracula", "ely-light"] {
        let theme = find(id).expect("a theme");
        let chrome = palette(theme, 0.4);
        assert_eq!(chrome.glass, chrome.bg.alpha(0.4), "{id}");
        assert_eq!(
            chrome.bg.a, 1.0,
            "{id}: the ground under dialogs stays solid"
        );
        assert_eq!(palette(theme, 1.0).glass, chrome.bg, "{id}");
    }
}
