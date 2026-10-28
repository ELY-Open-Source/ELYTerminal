use super::*;
use gpui::px;

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

fn plain(content: &str) -> Vec<usize> {
    anchors(content, &[run(content.len(), gpui::white())])
}

#[test]
fn a_space_never_holds_the_mark_after_it() {
    let map = plain("👨 🏾X");
    assert_eq!((map[0], map[4], map[5], map[9]), (0, 1, 2, 3));
}

#[test]
fn clusters_break_where_colors_change() {
    let flag = "🇺🇸X";
    let split = [
        run(4, gpui::white()),
        run(4, gpui::black()),
        run(1, gpui::white()),
    ];
    assert_eq!((plain(flag)[0], plain(flag)[4]), (0, 0));
    assert_eq!(anchors(flag, &split)[4], 1);
}

#[test]
fn anchors_count_characters_and_join_clusters() {
    let map = plain("中 aகொא");
    assert_eq!(
        (map[0], map[3], map[4], map[5], map[8], map[11]),
        (0, 1, 2, 3, 3, 5)
    );
}

#[test]
fn right_to_left_glyphs_keep_their_logical_columns() {
    let map = plain("abc אב");
    let shaped = [(2, px(0.)), (0, px(10.))];
    assert_eq!(places(&map, 4, &shaped, px(10.)), [px(50.), px(40.)]);
}

#[test]
fn glyphs_of_one_character_keep_their_offsets() {
    let map = plain("ab");
    let shaped = [(0, px(1.)), (0, px(4.)), (1, px(12.))];
    assert_eq!(places(&map, 0, &shaped, px(10.)), [px(0.), px(3.), px(10.)]);
}

#[test]
fn a_reordered_cluster_keeps_its_shape() {
    let map = plain("xகொ");
    let shaped = [(4, px(0.)), (1, px(14.5)), (4, px(29.))];
    let placed = places(&map, 0, &shaped, px(14.5));
    assert_eq!(placed, [px(14.5), px(29.), px(43.5)]);
}
