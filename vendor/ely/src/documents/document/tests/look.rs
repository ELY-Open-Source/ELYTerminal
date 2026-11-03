use gpui::{TestAppContext, px, rems};

use super::{MarkdownMode, frames, open};
use crate::documents::DocumentLook;
use crate::documents::document::edit::Landing;

/// Sixty short paragraphs, each its own block.
fn long() -> String {
    (0..60).map(|n| format!("Paragraph {n}.\n\n")).collect()
}

#[gpui::test]
fn the_measure_narrows_the_column(cx: &mut TestAppContext) {
    let (document, cx) = open("A paragraph.\n", MarkdownMode::Visual, cx);
    let look = DocumentLook {
        measure: Some(rems(20.)),
        ..DocumentLook::default()
    };
    document.update(cx, |document, cx| document.set_look(look, cx));
    frames(cx);
    let block = cx.debug_bounds("markdown-block-0").expect("drawn");
    assert!(block.size.width <= px(320.0), "{:?}", block.size);
}

#[gpui::test]
fn typewriter_keeps_the_open_block_mid_view(cx: &mut TestAppContext) {
    let (document, cx) = open(&long(), MarkdownMode::Visual, cx);
    let look = DocumentLook {
        typewriter: true,
        ..DocumentLook::default()
    };
    document.update_in(cx, |document, window, cx| {
        document.set_look(look, cx);
        document.open(Some(40), Landing::End, window, cx);
        document.list.scroll_to_reveal_item(40);
    });
    frames(cx);
    let (block, view) = document.read_with(cx, |document, _| {
        (
            document.list.bounds_for_item(40),
            document.list.viewport_bounds(),
        )
    });
    let block = block.expect("block 40 is laid out");
    let off = (block.center().y - view.center().y).abs();
    assert!(off < px(4.0), "{off:?} from the middle");
}

#[gpui::test]
fn typewriter_keeps_the_source_caret_mid_view(cx: &mut TestAppContext) {
    let (document, cx) = open(&long(), MarkdownMode::Source, cx);
    let look = DocumentLook {
        typewriter: true,
        ..DocumentLook::default()
    };
    document.update(cx, |document, cx| {
        document.set_look(look, cx);
        document.set_caret(90, 0, cx);
    });
    frames(cx);
    let before = document.read_with(cx, |document, cx| document.top_line(cx));
    let editor = document.read_with(cx, |document, _| document.editor().clone());
    let focus = editor.read_with(cx, gpui::Focusable::focus_handle);
    cx.update(|window, cx| window.focus(&focus, cx));
    cx.simulate_input(
        "









",
    );
    frames(cx);
    let after = document.read_with(cx, |document, cx| document.top_line(cx));
    assert_eq!(after, before + 10, "the view follows the caret");
}

#[gpui::test]
fn the_scale_sizes_source_lines(cx: &mut TestAppContext) {
    let (document, cx) = open("a\n", MarkdownMode::Source, cx);
    let editor = document.read_with(cx, |document, _| document.editor().clone());
    frames(cx);
    let plain = editor.read_with(cx, |editor, _| editor.metrics.line);
    let look = DocumentLook {
        scale: 2.0,
        ..DocumentLook::default()
    };
    document.update(cx, |document, cx| document.set_look(look, cx));
    frames(cx);
    let large = editor.read_with(cx, |editor, _| editor.metrics.line);
    assert!(large > plain * 1.8, "{plain:?} to {large:?}");
}

#[gpui::test]
fn focus_counts_the_open_block_as_the_readers(cx: &mut TestAppContext) {
    let (document, cx) = open(&long(), MarkdownMode::Visual, cx);
    document.update_in(cx, |document, window, cx| {
        document.open(Some(3), Landing::End, window, cx)
    });
    assert_eq!(
        document.read_with(cx, |document, cx| document.current_block(cx)),
        3
    );
}

fn typewriter() -> DocumentLook {
    DocumentLook {
        typewriter: true,
        ..DocumentLook::default()
    }
}

/// How far block `ix` sits from mid-view.
fn off_middle(
    document: &gpui::Entity<super::MarkdownDocument>,
    ix: usize,
    cx: &mut gpui::VisualTestContext,
) -> gpui::Pixels {
    document.read_with(cx, |document, _| {
        let mut block = document.list.bounds_for_item(ix).expect("laid out");
        if ix == 0 {
            block.origin.y += document.room;
            block.size.height -= document.room;
        }
        if ix == document.parsed.blocks.len() {
            block.size.height -= document.room;
        }
        (block.center().y - document.list.viewport_bounds().center().y).abs()
    })
}

#[gpui::test]
fn an_open_block_takes_the_scale(cx: &mut TestAppContext) {
    let (document, cx) = open("A paragraph.\n", MarkdownMode::Visual, cx);
    document.update_in(cx, |document, window, cx| {
        document.open(Some(0), Landing::End, window, cx)
    });
    frames(cx);
    let plain = document
        .read_with(cx, |document, _| document.list.bounds_for_item(0))
        .expect("laid out");
    let large = DocumentLook {
        scale: 2.0,
        ..DocumentLook::default()
    };
    document.update(cx, |document, cx| document.set_look(large, cx));
    frames(cx);
    let scaled = document
        .read_with(cx, |document, _| document.list.bounds_for_item(0))
        .expect("laid out");
    assert!(
        scaled.size.height > plain.size.height * 1.5,
        "{plain:?} to {scaled:?}"
    );
}

#[gpui::test]
fn a_resize_centers_the_block_again(cx: &mut TestAppContext) {
    let (document, cx) = open(&long(), MarkdownMode::Visual, cx);
    document.update_in(cx, |document, window, cx| {
        document.set_look(typewriter(), cx);
        document.open(Some(40), Landing::End, window, cx);
        document.list.scroll_to_reveal_item(40);
    });
    frames(cx);
    cx.simulate_resize(gpui::size(px(1000.0), px(400.0)));
    frames(cx);
    assert!(off_middle(&document, 40, cx) < px(4.0));
}

#[gpui::test]
fn the_first_and_last_blocks_center(cx: &mut TestAppContext) {
    let (document, cx) = open(&long(), MarkdownMode::Visual, cx);
    document.update_in(cx, |document, window, cx| {
        document.set_look(typewriter(), cx);
        document.open(Some(0), Landing::End, window, cx);
    });
    frames(cx);
    let first = off_middle(&document, 0, cx);
    assert!(first < px(4.0), "the first is {first:?} off");
    document.update_in(cx, |document, window, cx| {
        document.open(Some(59), Landing::End, window, cx);
        document.list.scroll_to_reveal_item(59);
    });
    frames(cx);
    assert!(off_middle(&document, 59, cx) < px(4.0), "the last");
}

#[gpui::test]
fn the_last_source_line_centers_and_a_resize_follows(cx: &mut TestAppContext) {
    let (document, cx) = open(&long(), MarkdownMode::Source, cx);
    document.update(cx, |document, cx| {
        document.set_look(typewriter(), cx);
        document.set_caret(119, 0, cx);
    });
    frames(cx);
    let top = document.read_with(cx, |document, cx| document.top_line(cx));
    assert!(
        top > 80,
        "top {top}: the last line is mid-view, not at the bottom"
    );
    cx.simulate_resize(gpui::size(px(1000.0), px(400.0)));
    frames(cx);
    let resized = document.read_with(cx, |document, cx| document.top_line(cx));
    let editor = document.read_with(cx, |document, _| document.editor().clone());
    let (line, height) =
        editor.read_with(cx, |editor, _| (editor.metrics.line, editor.metrics.height));
    let half = (height / line / 2.0).floor() as usize;
    let middle = 119 - half;
    assert!(
        resized.abs_diff(middle) <= 1,
        "top {top} to {resized}; mid-view is {middle}"
    );
}

/// Typewriter source, caret on line 60 of 120.
fn centered_source(
    cx: &mut TestAppContext,
) -> (
    gpui::Entity<super::MarkdownDocument>,
    &mut gpui::VisualTestContext,
) {
    let text: String = (0..120).map(|n| format!("Line {n}.\n")).collect();
    let (document, cx) = open(&text, MarkdownMode::Source, cx);
    document.update(cx, |document, cx| {
        document.set_look(typewriter(), cx);
        document.set_caret(60, 0, cx);
    });
    frames(cx);
    (document, cx)
}

#[gpui::test]
fn source_lines_below_the_center_draw(cx: &mut TestAppContext) {
    let (_, cx) = centered_source(cx);
    let caret = cx.debug_bounds("editor-line-60").expect("caret line");
    let next = cx.debug_bounds("editor-line-61").expect("the line below");
    assert!(next.top() >= caret.bottom() - px(1.0));
}

#[gpui::test]
fn platform_text_queries_meet_the_drawn_line(cx: &mut TestAppContext) {
    use gpui::EntityInputHandler as _;
    let (document, cx) = centered_source(cx);
    let drawn = cx.debug_bounds("editor-line-60").expect("caret line");
    let editor = document.read_with(cx, |document, _| document.editor().clone());
    let (anchor, line) = editor.update_in(cx, |editor, window, cx| {
        let at = editor.text()[..editor.primary().head]
            .encode_utf16()
            .count();
        let anchor = editor.bounds_for_range(at..at, gpui::Bounds::default(), window, cx);
        let point = gpui::point(editor.metrics.left, drawn.center().y);
        let index = editor.character_index_for_point(point, window, cx);
        let line = index.map(|at| editor.buffer.line_of(editor.byte_range(&(at..at)).start));
        (anchor.expect("anchor"), line)
    });
    assert!(
        (anchor.top() - drawn.top()).abs() < px(2.0),
        "{anchor:?} vs {drawn:?}"
    );
    assert_eq!(line, Some(60));
}

#[gpui::test]
fn switching_to_visual_centers_the_caret_block(cx: &mut TestAppContext) {
    let (document, cx) = open(&long(), MarkdownMode::Source, cx);
    document.update(cx, |document, cx| {
        document.set_look(typewriter(), cx);
        document.set_caret(100, 0, cx);
    });
    frames(cx);
    document.update_in(cx, |document, window, cx| {
        document.set_mode(MarkdownMode::Visual, window, cx)
    });
    frames(cx);
    let off = off_middle(&document, 50, cx);
    assert!(off < px(4.0), "{off:?} from the middle");
}

/// How far line 100 sits from mid-view.
fn source_off(
    document: &gpui::Entity<super::MarkdownDocument>,
    cx: &mut gpui::VisualTestContext,
) -> gpui::Pixels {
    let drawn = cx.debug_bounds("editor-line-100").expect("line drawn");
    let editor = document.read_with(cx, |document, _| document.editor().clone());
    let middle = editor.read_with(cx, |editor, _| {
        editor.metrics.top + editor.metrics.height / 2.0
    });
    (drawn.center().y - middle).abs()
}

/// Typewriter source, caret on line 100.
fn source_at_100(
    cx: &mut TestAppContext,
) -> (
    gpui::Entity<super::MarkdownDocument>,
    &mut gpui::VisualTestContext,
) {
    let (document, cx) = open(&long(), MarkdownMode::Source, cx);
    document.update(cx, |document, cx| {
        document.set_look(typewriter(), cx);
        document.set_caret(100, 0, cx);
    });
    frames(cx);
    (document, cx)
}

#[gpui::test]
fn a_mode_round_trip_centers_the_source_line(cx: &mut TestAppContext) {
    let (document, cx) = source_at_100(cx);
    for mode in [MarkdownMode::Visual, MarkdownMode::Source] {
        document.update_in(cx, |document, window, cx| {
            document.set_mode(mode, window, cx)
        });
        frames(cx);
    }
    let off = source_off(&document, cx);
    assert!(off < px(4.0), "{off:?} from the middle");
}

#[gpui::test]
fn revealing_the_caret_line_keeps_it_centered(cx: &mut TestAppContext) {
    let (document, cx) = source_at_100(cx);
    document.update_in(cx, |document, window, cx| document.reveal(100, window, cx));
    frames(cx);
    let off = source_off(&document, cx);
    assert!(off < px(4.0), "{off:?} from the middle");
}

#[gpui::test]
fn a_small_resize_centers_the_source_line(cx: &mut TestAppContext) {
    let (document, cx) = source_at_100(cx);
    cx.simulate_resize(gpui::size(px(1000.0), px(1070.0)));
    frames(cx);
    let off = source_off(&document, cx);
    assert!(off < px(4.0), "{off:?} from the middle");
}
