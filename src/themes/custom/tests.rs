use gpui::{Hsla, Rgba};

use super::*;

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
