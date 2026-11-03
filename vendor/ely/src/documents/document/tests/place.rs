use gpui::{Entity, TestAppContext, VisualTestContext, px};

use super::{AT_ONCE, MarkdownDocument, MarkdownMode, TEXT, frames, open, text};

/// The view's top line once no ask waits.
fn landed(document: &Entity<MarkdownDocument>, cx: &mut VisualTestContext) -> usize {
    frames(cx);
    document.read_with(cx, |document, cx| {
        assert!(document.top.is_none(), "the ask finished");
        document.top_line(cx)
    })
}

/// A hundred paragraphs: line `2n` is the `n`th.
fn paragraphs() -> String {
    (0..100).map(|n| format!("Paragraph {n}\n\n")).collect()
}

/// A fenced block of `lines` lines, then a paragraph.
fn fenced(lines: usize) -> String {
    let body: String = (0..lines).map(|n| format!("line {n}\n")).collect();
    format!("```\n{body}```\n\nAfter.\n")
}

#[gpui::test]
fn caret_and_view_set_apart_and_carry_across_modes(cx: &mut TestAppContext) {
    let long = (0..60).map(|n| format!("Line {n}\n\n")).collect::<String>();
    let (document, cx) = open(&long, MarkdownMode::Source, cx);
    document.update(cx, |document, cx| document.set_caret(4, 3, cx));
    document.update_in(cx, |document, window, cx| {
        document.set_top_line(40, window, cx)
    });
    frames(cx);
    let source = document.read_with(cx, |document, cx| document.top_line(cx));
    assert_eq!(source, 40, "the source scrolled");
    let caret = document.read_with(cx, |document, cx| document.caret(cx));
    assert_eq!(caret, (4, 3), "the view moved alone");
    document.update_in(cx, |document, window, cx| {
        document.set_mode(MarkdownMode::Visual, window, cx)
    });
    frames(cx);
    let (caret, top) = document.read_with(cx, |document, cx| {
        (document.caret(cx), document.top_line(cx))
    });
    assert_eq!(caret, (4, 3), "one caret for every mode");
    assert_eq!(top, 40, "the view stays at its line");
}

#[gpui::test]
fn a_top_line_inside_a_tall_block_survives_modes(cx: &mut TestAppContext) {
    let (document, cx) = open(&fenced(100), MarkdownMode::Source, cx);
    document.update_in(cx, |document, window, cx| {
        document.set_top_line(40, window, cx)
    });
    frames(cx);
    document.update_in(cx, |document, window, cx| {
        document.set_mode(MarkdownMode::Visual, window, cx)
    });
    assert_eq!(landed(&document, cx), 40);
}

#[gpui::test]
fn visual_to_preview_keeps_the_offset_in_a_block(cx: &mut TestAppContext) {
    let (document, cx) = open(&fenced(100), MarkdownMode::Visual, cx);
    document.update(cx, |document, cx| {
        document.list.scroll_to(gpui::ListOffset {
            item_ix: 0,
            offset_in_item: px(500.0),
        });
        cx.notify();
    });
    frames(cx);
    let before = document.read_with(cx, |document, _| document.list.logical_scroll_top());
    document.update_in(cx, |document, window, cx| {
        document.set_mode(MarkdownMode::Preview, window, cx)
    });
    frames(cx);
    let after = document.read_with(cx, |document, _| document.list.logical_scroll_top());
    assert_eq!(
        (after.item_ix, after.offset_in_item),
        (before.item_ix, before.offset_in_item)
    );
    assert_eq!(after.offset_in_item, px(500.0));
}

#[gpui::test]
fn a_top_line_asked_with_new_text_lands_once_read(cx: &mut TestAppContext) {
    let (document, cx) = open("old", MarkdownMode::Visual, cx);
    let fresh: String = (0..100).map(|n| format!("Paragraph {n}\n\n")).collect();
    document.update_in(cx, |document, window, cx| {
        document.reset(fresh, cx);
        document.set_top_line(40, window, cx);
    });
    assert_eq!(landed(&document, cx), 40);
}

#[gpui::test]
fn a_second_top_line_asked_while_placing_wins(cx: &mut TestAppContext) {
    let fresh: String = (0..100).map(|n| format!("Paragraph {n}\n\n")).collect();
    let (document, cx) = open(&fresh, MarkdownMode::Source, cx);
    document.update_in(cx, |document, window, cx| {
        document.set_mode(MarkdownMode::Visual, window, cx);
        document.set_top_line(60, window, cx);
    });
    assert_eq!(landed(&document, cx), 60);
}

#[gpui::test]
fn a_switch_before_the_frame_keeps_the_ask(cx: &mut TestAppContext) {
    let (document, cx) = open(&paragraphs(), MarkdownMode::Source, cx);
    document.update_in(cx, |document, window, cx| {
        document.set_top_line(40, window, cx);
        document.set_mode(MarkdownMode::Visual, window, cx);
    });
    assert_eq!(landed(&document, cx), 40, "source to visual");
    document.update_in(cx, |document, window, cx| {
        document.set_top_line(60, window, cx);
        document.set_mode(MarkdownMode::Source, window, cx);
    });
    assert_eq!(landed(&document, cx), 60, "visual to source");
}

#[gpui::test]
fn new_text_under_an_ask_places_it_again(cx: &mut TestAppContext) {
    let (document, cx) = open(&paragraphs(), MarkdownMode::Visual, cx);
    document.update_in(cx, |document, window, cx| {
        document.set_top_line(40, window, cx);
        document.reset(fenced(100), cx);
        document.set_top_line(40, window, cx);
    });
    assert_eq!(landed(&document, cx), 40);
}

#[gpui::test]
fn an_ask_past_a_shorter_text_lands_on_its_last_line(cx: &mut TestAppContext) {
    let (document, cx) = open("old", MarkdownMode::Visual, cx);
    let long = "Prose.\n\n".repeat(AT_ONCE / 8 + 1);
    document.update_in(cx, |document, window, cx| {
        document.reset(long, cx);
        document.set_top_line(40, window, cx);
        document.reset("short", cx);
    });
    assert_eq!(landed(&document, cx), 0);
}

#[gpui::test]
fn reveal_after_new_text_lands_once_read(cx: &mut TestAppContext) {
    let (document, cx) = open("old", MarkdownMode::Visual, cx);
    document.update_in(cx, |document, window, cx| {
        document.reset(paragraphs(), cx);
        document.reveal(80, window, cx);
    });
    assert_eq!(landed(&document, cx), 80);
    let caret = document.read_with(cx, |document, cx| document.caret(cx));
    assert_eq!(caret, (80, 0));
}

#[gpui::test]
fn reveal_puts_the_caret_on_the_line(cx: &mut TestAppContext) {
    let (document, cx) = open(TEXT, MarkdownMode::Source, cx);
    document.update_in(cx, |document, window, cx| document.reveal(4, window, cx));
    assert_eq!(
        document.read_with(cx, |document, cx| document.caret(cx)),
        (4, 0)
    );
    cx.simulate_input("x");
    assert!(
        text(&document, cx).contains(
            "
x- one"
        ),
        "keys land on the line"
    );
    let outline: Vec<(String, usize)> = document.read_with(cx, |document, cx| {
        document
            .outline(cx)
            .into_iter()
            .map(|(heading, line)| (heading.title, line))
            .collect()
    });
    assert_eq!(outline, [("Lift".to_string(), 0)]);
}

#[gpui::test]
fn a_reset_keeps_the_caret_and_the_top_line(cx: &mut TestAppContext) {
    let lines: String = (0..60).map(|n| format!("Line {n}\n\n")).collect();
    let (document, cx) = open(&lines, MarkdownMode::Source, cx);
    document.update_in(cx, |document, window, cx| {
        document.set_caret(40, 2, cx);
        document.set_top_line(30, window, cx);
    });
    frames(cx);
    let changed: String = (0..60).map(|n| format!("Row {n}\n\n")).collect();
    document.update(cx, |document, cx| document.reset(changed, cx));
    frames(cx);
    let (caret, top) = document.read_with(cx, |document, cx| {
        (document.caret(cx), document.top_line(cx))
    });
    assert_eq!((caret, top), ((40, 2), 30));
}

#[gpui::test]
fn the_selection_reads_in_source_and_in_an_open_block(cx: &mut TestAppContext) {
    let (document, cx) = open("# One\n\nTwo words.\n", MarkdownMode::Source, cx);
    document.update(cx, |document, cx| {
        document
            .editor
            .update(cx, |editor, cx| editor.select(std::iter::once(2..5), cx))
    });
    assert_eq!(
        document.read_with(cx, |document, cx| document.selected(cx)),
        "One"
    );
    for mode in [MarkdownMode::Preview, MarkdownMode::Visual] {
        document.update_in(cx, |document, window, cx| {
            document.set_mode(mode, window, cx)
        });
        frames(cx);
        let selected = document.read_with(cx, |document, cx| document.selected(cx));
        assert_eq!(selected, "", "{mode:?} shows no source selection");
    }
    super::type_in(&document, Some(1), None, cx);
    document.update(cx, |document, cx| {
        let open = document.open.as_ref().expect("a block is open");
        open.input.update(cx, |input, cx| input.select(4..9, cx));
    });
    assert_eq!(
        document.read_with(cx, |document, cx| document.selected(cx)),
        "words"
    );
}

#[gpui::test]
fn a_visual_opening_starts_at_the_top(cx: &mut TestAppContext) {
    let source = format!("alpha beta\n\n{}", "x".repeat(20000));
    let (document, cx) = open(&source, MarkdownMode::Visual, cx);
    cx.run_until_parked();
    assert_eq!(landed(&document, cx), 0);
}
