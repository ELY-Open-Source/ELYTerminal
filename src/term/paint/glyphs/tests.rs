use super::*;

fn run(len: usize, color: gpui::Hsla) -> TextRun {
    TextRun {
        len,
        color,
        ..Default::default()
    }
}

#[test]
fn shaping_keeps_fonts_and_original_paint_colors() {
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
    assert!(shaping_runs(&[]).is_empty());
}

#[test]
fn different_colors_shape_operators_separately() {
    let runs = [run(1, gpui::white()), run(1, gpui::black())];
    for text in ["!=", "->", "=="] {
        let parts: Vec<_> = sections(text, &runs).collect();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].0, &text[..1]);
        assert_eq!(parts[1].0, &text[1..]);
        assert_eq!(parts[0].1[0].color, gpui::white());
        assert_eq!(parts[1].1[0].color, gpui::black());
    }
}

#[test]
fn same_color_keeps_the_operator_in_one_shaping_call() {
    let runs = [run(1, gpui::white()), run(1, gpui::white())];
    for text in ["!=", "->", "=="] {
        let parts: Vec<_> = sections(text, &runs).collect();
        assert_eq!(parts, [(text, runs.as_slice())]);
    }
}

#[test]
fn sections_preserve_utf8_boundaries_and_font_changes() {
    let runs = [
        run(3, gpui::white()),
        TextRun {
            font: gpui::font("JetBrains Mono").bold(),
            ..run(2, gpui::white())
        },
        run(1, gpui::black()),
    ];
    let parts: Vec<_> = sections("中 ab", &runs).collect();
    assert_eq!(parts.len(), 2);
    assert_eq!(parts[0].0, "中 a");
    assert_eq!(parts[0].0.chars().count(), 3);
    assert_eq!(parts[0].1, &runs[..2]);
    assert_eq!(parts[1].0, "b");
    assert!(sections("", &[]).next().is_none());
}
