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

#[test]
fn shaping_ignores_colors_but_paint_keeps_them() {
    let original = vec![
        TextRun {
            len: "中".len(),
            font: gpui::font("JetBrains Mono"),
            color: gpui::white(),
            ..Default::default()
        },
        TextRun {
            len: 2,
            font: gpui::font("JetBrainsMono Nerd Font Mono").bold().italic(),
            color: gpui::black(),
            ..Default::default()
        },
    ];
    let runs = original.clone();
    let shaped = shaping_runs(&runs);
    assert_eq!(shaped.len(), runs.len());
    for (shaped, source) in shaped.iter().zip(&runs) {
        let mut expected = source.clone();
        expected.color = gpui::black();
        assert_eq!(*shaped, expected);
    }
    assert_eq!(runs, original);
    assert_eq!(color_at(&runs, 0), gpui::white());
    assert_eq!(color_at(&runs, 2), gpui::white());
    assert_eq!(color_at(&runs, 3), gpui::black());
    assert_eq!(color_at(&runs, 4), gpui::black());
    assert!(shaping_runs(&[]).is_empty());
}
