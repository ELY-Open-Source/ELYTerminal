use gpui::{
    Context, IntoElement, ParentElement, Render, Rgba, Styled, TestAppContext, VisualTestContext,
    Window, div, px,
};

use crate::{
    forms::hex,
    theme::{ControlSize, Density, Palette, Theme},
    tooling::{A11yChecker, DesignTokenViewer},
};

/// gpui takes a static selector, so a formatted one is leaked.
fn shown(selector: &str, cx: &mut VisualTestContext) -> bool {
    cx.debug_bounds(Box::leak(selector.into())).is_some()
}

struct Tokens;

impl Render for Tokens {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().w(px(900.0)).child(DesignTokenViewer::new())
    }
}

/// Each token shows its name as code reads it and its value in the mode shown; a changed theme shows through.
#[gpui::test]
fn the_viewer_names_each_token_with_its_value(cx: &mut TestAppContext) {
    cx.update(Theme::init);
    let (_, cx) = cx.add_window_view(|_, _| Tokens);
    cx.run_until_parked();
    let light = Palette::light(false);
    let keyword = format!(
        "token-colors.syntax.keyword-{}",
        hex(Rgba::from(light.syntax.keyword))
    );
    let blue = format!(
        "token-colors.hue(0) Blue-{}",
        hex(Rgba::from(light.chart[0]))
    );
    let measures: [(&str, &[&str], &[u32]); 7] = [
        (
            "TextSize::",
            &["Xs", "Sm", "Base", "Md", "Lg", "Xl", "Xxl", "Display"],
            &[11, 12, 13, 14, 16, 20, 24, 32],
        ),
        ("Radius::", &["Sm", "Md", "Lg", "Xl"], &[4, 6, 8, 12]),
        ("ControlSize::", &["Sm", "Md", "Lg"], &[24, 28, 32]),
        (
            "IconSize::",
            &["Xs", "Sm", "Md", "Lg", "Xl", "Xxl"],
            &[12, 14, 16, 20, 24, 32],
        ),
        (
            "AvatarSize::",
            &["Xs", "Sm", "Md", "Lg", "Xl"],
            &[20, 24, 32, 40, 56],
        ),
        ("ContainerSize::", &["Sm", "Md", "Lg"], &[640, 960, 1200]),
        (
            "gap_",
            &["1", "2", "3", "4", "6", "8"],
            &[4, 8, 12, 16, 24, 32],
        ),
    ];
    for (kind, names, sizes) in measures {
        for (name, size) in names.iter().zip(sizes) {
            let selector = format!("token-{kind}{name}-{size}");
            assert!(shown(&selector, cx), "{selector}");
        }
    }
    for level in ["Raised", "Floating", "Modal"] {
        assert!(shown(&format!("token-Elevation::{level}"), cx), "{level}");
    }
    for (ix, name) in [(0, "black"), (15, "bright white")] {
        let code = hex(Rgba::from(light.ansi[ix]));
        let selector = format!("token-colors.ansi[{ix}] {name}-{code}");
        assert!(shown(&selector, cx), "{selector}");
    }
    for selector in [
        "token-colors.fg_subtle-#696764",
        &keyword,
        &blue,
        "token-TextSize::Sm-12",
        "token-Radius::Md-6",
        "token-ControlSize::Md-28",
        "token-IconSize::Md-16",
        "token-gap_2-8",
        "token-Elevation::Modal",
    ] {
        assert!(shown(selector, cx), "{selector}");
    }
    cx.update(|_, cx| {
        Theme::update(cx, |theme| {
            theme.colors = Palette::dark(false);
            theme.density = Density::Compact;
        })
    });
    cx.run_until_parked();
    assert!(
        shown("token-colors.fg_subtle-#acaaa8", cx),
        "the dark palette"
    );
    let compact = cx.update(|window, cx| {
        use crate::theme::ActiveTheme;
        cx.theme()
            .control_height(ControlSize::Md)
            .to_pixels(window.rem_size())
    });
    assert_eq!(compact, px(24.0));
    assert!(shown("token-ControlSize::Md-24", cx), "compact density");
}

struct Audit;

impl Render for Audit {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().w(px(600.0)).child(A11yChecker::new())
    }
}

/// Ely's palette passes; a color moved under its need fails, counted, and lists first.
#[gpui::test]
fn the_checker_counts_failures_first(cx: &mut TestAppContext) {
    cx.update(Theme::init);
    let (_, cx) = cx.add_window_view(|_, _| Audit);
    cx.run_until_parked();
    assert!(shown("contrast-summary-81-0", cx), "every pair passes");
    assert!(shown("contrast-fg-bg-AAA", cx));
    assert!(
        shown("contrast-fg_subtle-active-AA", cx),
        "a selected row's detail"
    );
    assert!(
        shown("contrast-syntax.comment-hover-AA", cx),
        "a comment on the current line"
    );
    assert!(shown("contrast-focus-surface-AA", cx));
    assert!(shown("contrast-chart Blue-bg-AA", cx));
    cx.update(|_, cx| Theme::update(cx, |theme| theme.colors.fg_subtle = theme.colors.bg));
    cx.run_until_parked();
    assert!(
        shown("contrast-summary-81-6", cx),
        "six surfaces under fg_subtle"
    );
    let failing = cx
        .debug_bounds("contrast-fg_subtle-hover-Fails")
        .expect("a failing row");
    let passing = cx
        .debug_bounds("contrast-fg-bg-AAA")
        .expect("a passing row");
    assert!(failing.top() < passing.top(), "failures list first");
}
