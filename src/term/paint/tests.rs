use super::*;
use crate::term::frame::{Row, Underline};

#[test]
fn spans_count_columns_not_bytes() {
    let style = Style {
        fg: gpui::black(),
        bg: None,
        bold: false,
        italic: false,
        underline: Underline::None,
        underline_color: gpui::black(),
        strike: false,
    };
    let bold = Style {
        bold: true,
        ..style
    };
    let row = Row {
        text: "中 ab".into(),
        runs: vec![(4, style), (2, bold)],
        alone: Vec::new(),
    };
    let spans: Vec<_> = spans(&row).into_iter().map(|(at, n, _)| (at, n)).collect();
    assert_eq!(spans, [(0, 2), (2, 2)]);
}

#[test]
fn icons_live_in_the_private_use_areas() {
    assert!(private('\u{E0B0}'));
    assert!(private('\u{F0035}'));
    assert!(!private('a'));
    assert!(!private('中'));
}
