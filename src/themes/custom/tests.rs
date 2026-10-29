use gpui::{Hsla, Rgba};

use super::*;
use crate::settings::Settings;

fn rgb(r: f32, g: f32, b: f32) -> Hsla {
    Rgba { r, g, b, a: 1.0 }.into()
}

#[test]
fn a_new_theme_copies_ely_dark_under_the_next_id() {
    let first = CustomTheme::from_ely_dark(&[], "A".into());
    assert_eq!(first.id, "custom-1");
    let base = find("ely-dark").expect("built in");
    let made = first.theme().expect("its colors read");
    assert_eq!(made.background, hex(&text(base.background)).expect("hex"));
    assert_eq!(made.ansi[4], hex(&text(base.ansi[4])).expect("hex"));
    assert!(!made.ely, "the chrome follows its own colors");
    let later = CustomTheme {
        id: "custom-5".into(),
        ..first.clone()
    };
    let next = CustomTheme::from_ely_dark(&[first, later], "B".into());
    assert_eq!(next.id, "custom-6");
}

#[test]
fn each_slot_holds_its_own_color() {
    let mut theme = CustomTheme::from_ely_dark(&[], "A".into());
    theme.set(0, rgb(1.0, 0.0, 0.0));
    theme.set(4, rgb(0.0, 1.0, 0.0));
    theme.set(20, rgb(0.0, 0.0, 1.0));
    assert_eq!(theme.background, "#ff0000");
    assert_eq!(theme.selection, "#00ff00");
    assert_eq!(theme.ansi[15], "#0000ff");
    assert_eq!(theme.slot(20), "#0000ff");
    let made = theme.theme().expect("its colors read");
    assert_eq!(made.selection_background, rgb(0.0, 1.0, 0.0));
}

#[test]
fn a_theme_that_does_not_read_names_itself() {
    let good = CustomTheme::from_ely_dark(&[], "A".into());
    let bad = CustomTheme {
        cursor: "red".into(),
        ..good.clone()
    };
    let error = format!("{:#}", bad.theme().expect_err("red is no hex"));
    assert!(
        error.starts_with("custom theme custom-1, color 2"),
        "{error}"
    );
    let stray = CustomTheme {
        id: "mine".into(),
        ..good.clone()
    };
    assert!(stray.theme().is_err());
    let blank = CustomTheme {
        name: "  ".into(),
        ..good
    };
    assert_eq!(
        blank.theme().expect_err("no name").to_string(),
        "custom theme custom-1 has no name"
    );
}

#[test]
fn a_new_theme_is_worn_where_the_system_looks() {
    let mut settings = Settings {
        theme: "ely-light".into(),
        dark_theme: Some("dracula".into()),
        ..Settings::default()
    };
    wear(&mut settings, "custom-1", true);
    assert_eq!(settings.dark_theme.as_deref(), Some("custom-1"));
    assert_eq!(settings.theme, "ely-light");
    wear(&mut settings, "custom-2", false);
    assert_eq!(settings.theme, "custom-2");
    let mut one = Settings::default();
    wear(&mut one, "custom-3", true);
    assert_eq!((one.theme.as_str(), one.dark_theme), ("custom-3", None));
}

#[test]
fn ely_dark_takes_each_place_a_deleted_theme_held() {
    let mine = CustomTheme::from_ely_dark(&[], "Mine".into());
    let mut settings = Settings {
        theme: "ely-light".into(),
        dark_theme: Some(mine.id.clone()),
        custom_themes: vec![mine.clone()],
        ..Settings::default()
    };
    forget(&mut settings, &mine.id);
    assert!(settings.custom_themes.is_empty());
    assert_eq!(settings.dark_theme.as_deref(), Some("ely-dark"));
    assert_eq!(settings.theme, "ely-light");
    settings.theme = mine.id.clone();
    settings.custom_themes.push(mine.clone());
    forget(&mut settings, &mine.id);
    assert_eq!(settings.theme, "ely-dark");
}
