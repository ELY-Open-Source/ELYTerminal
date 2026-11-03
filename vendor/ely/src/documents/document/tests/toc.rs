use gpui::{Entity, Modifiers, MouseButton, TestAppContext, VisualTestContext, point, px};

use super::{MarkdownDocument, MarkdownMode, frames, open, type_in};

const LISTED: &str = "[toc]\n\n# A\n\ntext\n\n## B\n";

/// Presses entry `ix` of the drawn heading list.
fn press_entry(ix: usize, modifiers: Modifiers, cx: &mut VisualTestContext) {
    let bounds = cx
        .debug_bounds(Box::leak(format!("markdown-toc-{ix}").into_boxed_str()))
        .expect("the entry is drawn");
    let at = point(bounds.left() + px(2.0), bounds.top() + px(2.0));
    cx.simulate_mouse_down(at, MouseButton::Left, modifiers);
    frames(cx);
    cx.simulate_mouse_up(at, MouseButton::Left, modifiers);
    cx.run_until_parked();
}

#[gpui::test]
fn a_heading_written_above_with_the_same_title_is_not_the_pressed_one(cx: &mut TestAppContext) {
    let text = "before\n\n[toc]\n\n# A\n\ntext\n\n## B\n";
    let (document, cx) = open(text, MarkdownMode::Visual, cx);
    type_in(&document, Some(0), Some("## B"), cx);
    frames(cx);
    press_entry(1, Modifiers::secondary_key(), cx);
    let (caret, now) =
        document.read_with(cx, |document, cx| (document.caret(cx), document.text(cx)));
    assert_eq!(now, "## B\n\n[toc]\n\n# A\n\ntext\n\n## B\n");
    assert_eq!(caret, (8, 0));
}

#[gpui::test]
fn a_twin_typed_above_while_the_text_waits_is_not_the_pressed_one(cx: &mut TestAppContext) {
    let (document, cx) = open(LISTED, MarkdownMode::Split, cx);
    document.update(cx, |document, cx| {
        document.editor.update(cx, |editor, cx| {
            editor.edit([(0..0, "## B\n\n".to_string())], cx);
        });
    });
    cx.run_until_parked();
    assert!(!document.read_with(cx, |document, cx| document.read_now(cx)));
    press_entry(1, Modifiers::none(), cx);
    assert_eq!(
        document.read_with(cx, |document, cx| document.caret(cx)),
        (8, 0)
    );
}

const TWINS: &str = "[toc]\n\n## B\n\ntext\n\n## B\n";

/// Deletes the first twin while the text waits.
fn delete_first_twin(document: &Entity<MarkdownDocument>, cx: &mut VisualTestContext) {
    document.update(cx, |document, cx| {
        document.editor.update(cx, |editor, cx| {
            editor.edit([(7..13, String::new())], cx);
        });
    });
    cx.run_until_parked();
}

#[gpui::test]
fn a_deleted_twin_leaves_the_other_found(cx: &mut TestAppContext) {
    let (document, cx) = open(TWINS, MarkdownMode::Split, cx);
    delete_first_twin(&document, cx);
    press_entry(1, Modifiers::none(), cx);
    assert_eq!(
        document.read_with(cx, |document, cx| document.caret(cx)),
        (4, 0)
    );
}

#[gpui::test]
fn a_press_on_a_deleted_twin_moves_nothing(cx: &mut TestAppContext) {
    let (document, cx) = open(TWINS, MarkdownMode::Split, cx);
    delete_first_twin(&document, cx);
    let before = document.read_with(cx, |document, cx| document.caret(cx));
    press_entry(0, Modifiers::none(), cx);
    assert_eq!(
        document.read_with(cx, |document, cx| document.caret(cx)),
        before
    );
}

#[gpui::test]
fn a_press_in_preview_reveals_the_heading(cx: &mut TestAppContext) {
    let (document, cx) = open(LISTED, MarkdownMode::Preview, cx);
    press_entry(1, Modifiers::none(), cx);
    let caret = document.read_with(cx, |document, cx| document.caret(cx));
    assert_eq!(caret, (6, 0));
}

#[gpui::test]
fn visual_reveals_on_a_secondary_press_and_edits_on_a_plain_one(cx: &mut TestAppContext) {
    let (document, cx) = open(LISTED, MarkdownMode::Visual, cx);
    press_entry(1, Modifiers::secondary_key(), cx);
    let (caret, open) = document.read_with(cx, |document, cx| {
        (document.caret(cx), document.open.is_some())
    });
    assert_eq!((caret, open), ((6, 0), false));
    press_entry(0, Modifiers::none(), cx);
    assert!(document.read_with(cx, |document, _| document.open.is_some()));
}

#[gpui::test]
fn a_press_after_an_earlier_block_grew_finds_the_heading(cx: &mut TestAppContext) {
    let text = "before\n\n[toc]\n\n# A\n\ntext\n\n## B\n";
    let (document, cx) = open(text, MarkdownMode::Visual, cx);
    type_in(&document, Some(0), Some("before\nextra\nextra"), cx);
    frames(cx);
    press_entry(1, Modifiers::secondary_key(), cx);
    frames(cx);
    assert_eq!(
        document.read_with(cx, |document, cx| document.caret(cx)),
        (10, 0)
    );
}

#[gpui::test]
fn a_press_while_the_text_waits_to_be_read_finds_the_heading(cx: &mut TestAppContext) {
    let (document, cx) = open(LISTED, MarkdownMode::Split, cx);
    document.update(cx, |document, cx| {
        document.editor.update(cx, |editor, cx| {
            editor.edit([(0..0, "inserted\n".repeat(20))], cx);
        });
    });
    cx.run_until_parked();
    assert!(!document.read_with(cx, |document, cx| document.read_now(cx)));
    press_entry(1, Modifiers::none(), cx);
    frames(cx);
    assert_eq!(
        document.read_with(cx, |document, cx| document.caret(cx)),
        (26, 0)
    );
}

/// Makes `edits` while the text waits, caret at the top.
fn edit_waiting<'a>(
    text: &str,
    edits: Vec<(std::ops::Range<usize>, String)>,
    cx: &'a mut TestAppContext,
) -> (Entity<MarkdownDocument>, &'a mut VisualTestContext) {
    let (document, cx) = open(text, MarkdownMode::Split, cx);
    document.update(cx, |document, cx| {
        document
            .editor
            .update(cx, |editor, cx| editor.edit(edits, cx));
        document.set_caret(0, 0, cx);
    });
    frames(cx);
    assert!(!document.read_with(cx, |document, cx| document.read_now(cx)));
    (document, cx)
}

fn caret(document: &Entity<MarkdownDocument>, cx: &mut VisualTestContext) -> (usize, usize) {
    document.read_with(cx, |document, cx| document.caret(cx))
}

#[gpui::test]
fn a_twin_written_right_at_the_heading_goes_before_it(cx: &mut TestAppContext) {
    let edits = vec![(7..7, "## B\n\n".to_string())];
    let (document, cx) = edit_waiting("[toc]\n\n## B\n\ntext\n", edits, cx);
    press_entry(0, Modifiers::none(), cx);
    assert_eq!(caret(&document, cx), (4, 0));
}

#[gpui::test]
fn adjacent_twins_keep_who_was_deleted(cx: &mut TestAppContext) {
    let twins = "[toc]\n\n## B\n\n## B\n\ntext\n";
    let (document, cx) = edit_waiting(twins, vec![(7..13, String::new())], cx);
    press_entry(0, Modifiers::none(), cx);
    assert_eq!(
        caret(&document, cx),
        (0, 0),
        "the deleted one moves nothing"
    );
    press_entry(1, Modifiers::none(), cx);
    assert_eq!(caret(&document, cx), (2, 0), "the kept one moved up");
}

#[gpui::test]
fn a_heading_between_two_edits_stays_found(cx: &mut TestAppContext) {
    let text = "before\n\n[toc]\n\n# A\n\nafter\n";
    let edits = vec![
        (0..0, "start\n".to_string()),
        (text.len()..text.len(), "end\n".to_string()),
    ];
    let (document, cx) = edit_waiting(text, edits, cx);
    press_entry(0, Modifiers::none(), cx);
    assert_eq!(caret(&document, cx), (5, 0));
}
