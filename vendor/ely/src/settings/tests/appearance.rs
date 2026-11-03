use gpui::{
    AnyElement, App, Entity, IntoElement, ParentElement, Styled, TestAppContext, Window, div, hsla,
    px,
};

use super::{Desk, desk, said, say, tab, tap};
use crate::settings::{AccentColorPicker, FontSizeControl};

fn accent(_: &mut Window, _: &mut App, owner: Entity<Desk>) -> AnyElement {
    let (red, blue) = (hsla(0.0, 0.6, 0.5, 1.0), hsla(0.6, 0.6, 0.5, 1.0));
    AccentColorPicker::new(
        "accent",
        red,
        [("Red", red), ("Blue", blue)],
        move |color, _, cx| say(&owner, format!("blue {}", color == blue), cx),
    )
    .into_any_element()
}

/// Stops: the presets, Red then Blue, then the custom well.
#[gpui::test]
fn a_preset_sets_the_accent(cx: &mut TestAppContext) {
    let (host, cx) = desk(accent, cx);
    tab(2, cx);
    tap("space", cx);
    assert_eq!(said(&host, cx), ["blue true"]);
}

fn default_size(_: &mut Window, _: &mut App, owner: Entity<Desk>) -> AnyElement {
    FontSizeControl::new("size", 14.0, 14.0, (12.0, 24.0), move |size, _, cx| {
        say(&owner, format!("size {size}"), cx)
    })
    .into_any_element()
}

/// At the default size the way back rests, so the second stop wraps to the slider.
#[gpui::test]
fn the_way_back_rests_at_the_default_size(cx: &mut TestAppContext) {
    let (host, cx) = desk(default_size, cx);
    tab(2, cx);
    tap("space", cx);
    assert!(said(&host, cx).is_empty());
}

fn many_presets(_: &mut Window, _: &mut App, _: Entity<Desk>) -> AnyElement {
    let presets: Vec<(String, gpui::Hsla)> = (0..12)
        .map(|ix| (format!("hue {ix}"), hsla(ix as f32 / 12.0, 0.5, 0.5, 1.0)))
        .collect();
    div()
        .w(px(280.0))
        .child(AccentColorPicker::new(
            "accent",
            presets[0].1,
            presets,
            |_, _, _| {},
        ))
        .into_any_element()
}

/// Twelve swatches do not fit in one 280px row, so they fold under each other inside the box.
#[gpui::test]
fn many_presets_fold_inside_a_narrow_box(cx: &mut TestAppContext) {
    let (_, cx) = desk(many_presets, cx);
    let picker = cx.debug_bounds("accent-color-picker").expect("the picker");
    assert!(
        picker.size.width <= px(280.0),
        "no wider than its box: {picker:?}"
    );
    assert!(
        picker.size.height > px(40.0),
        "its swatches fold to a second row: {picker:?}"
    );
}

/// A size saved past the range, a section and a code palette no longer offered.
fn drifted(_: &mut Window, _: &mut App, _: Entity<Desk>) -> AnyElement {
    let sections = [("a", "Appearance", crate::primitives::IconName::Palette)];
    div()
        .w(px(480.0))
        .child(FontSizeControl::new(
            "size",
            40.0,
            9.0,
            (12.0, 24.0),
            |_, _, _| {},
        ))
        .child(crate::settings::SettingsLayout::new(
            "layout",
            sections,
            "gone",
            |_, _, _| {},
        ))
        .child(crate::settings::SyntaxThemePicker::new(
            "syntax",
            crate::theme::syntax_themes(),
            "Gone",
            |_, _, _| {},
        ))
        .into_any_element()
}

#[gpui::test]
fn settings_that_drifted_draw(cx: &mut TestAppContext) {
    let _ = desk(drifted, cx);
}
