use super::*;

fn run(len: usize, color: gpui::Hsla) -> TextRun {
    TextRun {
        len,
        color,
        ..Default::default()
    }
}

fn outer(left: f32, right: f32) -> Bounds<Pixels> {
    Bounds::from_corners(point(px(left), px(-5.)), point(px(right), px(30.)))
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

#[test]
fn clips_share_device_pixel_edges_and_preserve_vertical_overhang() {
    let area = outer(-2., 35.);
    let edge = px(0.25) + px(9.25);
    let first = mask(area, None, Some(edge), 1.5).bounds;
    let last = mask(area, Some(edge), None, 1.5).bounds;
    assert_eq!(first.right(), last.left());
    assert!((f32::from(first.right()) * 1.5).fract().abs() < 0.00001);
    assert_eq!(first.left(), area.left());
    assert_eq!(last.right(), area.right());
    assert_eq!(first.top(), area.top());
    assert_eq!(last.bottom(), area.bottom());
}

#[test]
fn clips_outside_the_viewport_have_no_negative_widths() {
    let area = outer(15., 25.);
    let before = mask(area, None, Some(px(10.)), 2.).bounds;
    let after = mask(area, Some(px(30.)), None, 2.).bounds;
    assert_eq!(before.size.width, px(0.));
    assert_eq!(after.size.width, px(0.));
    assert_eq!(mask(area, None, None, 2.).bounds, area);
}
