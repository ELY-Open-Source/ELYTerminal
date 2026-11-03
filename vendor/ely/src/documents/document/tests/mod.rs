use std::{cell::RefCell, rc::Rc};

use gpui::{Entity, Modifiers, MouseButton, TestAppContext, VisualTestContext, point, px};

use super::{AT_ONCE, DocumentEvent, MarkdownDocument, edit::Landing, visual};
use crate::documents::MarkdownMode;

const TEXT: &str = "# Lift\n\nBlend toward white.\n\n- one\n- two\n";

pub(super) fn open<'a>(
    text: &str,
    mode: MarkdownMode,
    cx: &'a mut TestAppContext,
) -> (Entity<MarkdownDocument>, &'a mut VisualTestContext) {
    cx.update(crate::init_for_tests);
    let text = text.to_string();
    let (document, cx) =
        cx.add_window_view(move |window, cx| MarkdownDocument::new(text, mode, window, cx));
    cx.update(|window, _| window.activate_window());
    cx.run_until_parked();
    (document, cx)
}

/// Presses block `ix` where it is drawn.
pub(super) fn press(ix: usize, modifiers: Modifiers, cx: &mut VisualTestContext) {
    let bounds = cx
        .debug_bounds(Box::leak(format!("markdown-block-{ix}").into_boxed_str()))
        .expect("the block is drawn");
    let at = point(bounds.left() + px(4.0), bounds.top() + px(4.0));
    cx.simulate_mouse_down(at, MouseButton::Left, modifiers);
    cx.simulate_mouse_up(at, MouseButton::Left, modifiers);
    cx.run_until_parked();
}

/// Frames with their next-frame callbacks, so scrolls land.
pub(super) fn frames(cx: &mut VisualTestContext) {
    for _ in 0..4 {
        cx.run_until_parked();
        cx.update(|window, _| window.refresh());
        cx.run_until_parked();
        cx.update(|window, cx| window.simulate_next_frame(cx));
    }
    cx.run_until_parked();
}

pub(super) fn text(document: &Entity<MarkdownDocument>, cx: &mut VisualTestContext) -> String {
    document.read_with(cx, |document, cx| document.text(cx))
}

#[gpui::test]
fn a_block_typed_in_is_the_text_at_once_and_one_undo_step(cx: &mut TestAppContext) {
    let (document, cx) = open(TEXT, MarkdownMode::Visual, cx);
    let changes = Rc::new(RefCell::new(0));
    let counted = changes.clone();
    cx.update(|_, cx| {
        cx.subscribe(&document, move |_, event: &DocumentEvent, _| {
            if *event == DocumentEvent::Changed {
                *counted.borrow_mut() += 1;
            }
        })
        .detach()
    });
    press(1, Modifiers::none(), cx);
    cx.simulate_input(" Again.");
    assert_eq!(
        text(&document, cx),
        "# Lift\n\nBlend toward white. Again.\n\n- one\n- two\n",
        "typing shows in the text before it is written back"
    );
    assert!(*changes.borrow() > 0, "each key tells the owner");
    cx.simulate_keystrokes("escape");
    let editor = document.read_with(cx, |document, _| document.editor().clone());
    assert_eq!(
        editor.read_with(cx, |editor, _| editor.text().to_string()),
        text(&document, cx),
        "written back"
    );
    cx.dispatch_action(visual::Undo);
    assert_eq!(text(&document, cx), TEXT, "one step undoes the block");
    cx.dispatch_action(visual::Redo);
    assert!(text(&document, cx).contains("Again."));
}

#[gpui::test]
fn the_same_text_and_history_cross_modes(cx: &mut TestAppContext) {
    let (document, cx) = open(TEXT, MarkdownMode::Source, cx);
    let editor = document.read_with(cx, |document, _| document.editor().clone());
    editor.update(cx, |editor, cx| {
        editor.edit([(0..0, "Intro\n\n".to_string())], cx)
    });
    document.update_in(cx, |document, window, cx| {
        document.set_mode(MarkdownMode::Visual, window, cx)
    });
    cx.run_until_parked();
    assert_eq!(text(&document, cx), format!("Intro\n\n{TEXT}"));
    cx.dispatch_action(visual::Undo);
    assert_eq!(
        text(&document, cx),
        TEXT,
        "the source edit undoes in visual"
    );
}

#[gpui::test]
fn a_mode_switch_writes_the_open_block_and_changes_nothing_else(cx: &mut TestAppContext) {
    let (document, cx) = open(TEXT, MarkdownMode::Visual, cx);
    press(0, Modifiers::none(), cx);
    document.update_in(cx, |document, window, cx| {
        document.set_mode(MarkdownMode::Preview, window, cx)
    });
    assert_eq!(
        text(&document, cx),
        TEXT,
        "an unchanged block writes nothing"
    );
    let undone = document.read_with(cx, |document, cx| {
        document.editor().read(cx).history.can_undo()
    });
    assert!(!undone, "no step was recorded");
}

#[gpui::test]
fn preview_and_a_secondary_press_open_no_block(cx: &mut TestAppContext) {
    let (document, cx) = open(TEXT, MarkdownMode::Preview, cx);
    press(1, Modifiers::none(), cx);
    assert!(document.read_with(cx, |document, _| document.open.is_none()));
    document.update_in(cx, |document, window, cx| {
        document.set_mode(MarkdownMode::Visual, window, cx)
    });
    cx.run_until_parked();
    press(1, Modifiers::secondary_key(), cx);
    assert!(
        document.read_with(cx, |document, _| document.open.is_none()),
        "Cmd or Ctrl leaves the press to links"
    );
}

#[gpui::test]
fn down_at_a_blocks_last_line_opens_the_next(cx: &mut TestAppContext) {
    let (document, cx) = open(TEXT, MarkdownMode::Visual, cx);
    press(0, Modifiers::none(), cx);
    cx.simulate_keystrokes("down");
    let opened = document.read_with(cx, |document, cx| {
        let open = document.open.as_ref().expect("a block is open");
        document.text(cx)[open.range.clone()].to_string()
    });
    assert_eq!(opened, "Blend toward white.");
}

#[gpui::test]
fn reset_starts_undo_afresh(cx: &mut TestAppContext) {
    let (document, cx) = open(TEXT, MarkdownMode::Visual, cx);
    document.update(cx, |document, cx| document.set_text("changed", cx));
    document.update(cx, |document, cx| document.reset("from disk", cx));
    cx.dispatch_action(visual::Undo);
    assert_eq!(text(&document, cx), "from disk");
}

#[gpui::test]
fn a_long_text_reads_off_the_main_thread(cx: &mut TestAppContext) {
    let long = "A paragraph of prose.\n\n".repeat(AT_ONCE / 20);
    let (document, cx) = open(&long, MarkdownMode::Visual, cx);
    let (read, blocks) = document.read_with(cx, |document, cx| {
        (document.read_now(cx), document.parsed.blocks.len())
    });
    assert!(read, "read once parked");
    assert_eq!(blocks, AT_ONCE / 20);
    document.update(cx, |document, cx| document.set_text("# Short\n", cx));
    assert!(
        document.read_with(cx, |document, cx| document.read_now(cx)),
        "a short text reads at once"
    );
}

#[gpui::test]
fn a_press_past_the_end_adds_a_block(cx: &mut TestAppContext) {
    let (document, cx) = open("# Lift", MarkdownMode::Visual, cx);
    let end = cx.debug_bounds("markdown-block-0").expect("drawn").bottom() + px(12.0);
    cx.simulate_mouse_down(point(px(40.0), end), MouseButton::Left, Modifiers::none());
    cx.run_until_parked();
    cx.simulate_input("More.");
    assert_eq!(text(&document, cx), "# Lift\n\nMore.");
    cx.simulate_keystrokes("escape");
    let editor = document.read_with(cx, |document, _| document.editor().clone());
    let written = editor.read_with(cx, |editor, _| editor.text().to_string());
    assert_eq!(written, "# Lift\n\nMore.");
}

#[gpui::test]
fn front_matter_opens_as_written(cx: &mut TestAppContext) {
    let (document, cx) = open("---\ntitle: a\n---\n\nBody\n", MarkdownMode::Visual, cx);
    press(0, Modifiers::none(), cx);
    let opened = document.read_with(cx, |document, cx| {
        document
            .open
            .as_ref()
            .expect("open")
            .input
            .read(cx)
            .text()
            .to_string()
    });
    assert_eq!(opened, "---\ntitle: a\n---");
}

#[gpui::test]
fn the_source_colors_markdown(cx: &mut TestAppContext) {
    let (document, cx) = open(TEXT, MarkdownMode::Source, cx);
    let colors = document.read_with(cx, |document, cx| {
        document.editor().read(cx).line_colors("a **b**", cx)
    });
    assert!(
        colors
            .iter()
            .any(|(range, style)| *range == (2..7)
                && style.font_weight == Some(gpui::FontWeight::BOLD)),
        "{colors:?}"
    );
}

#[gpui::test]
fn down_past_the_last_block_keeps_it_open(cx: &mut TestAppContext) {
    let (document, cx) = open("# Lift\n\nLast.", MarkdownMode::Visual, cx);
    press(1, Modifiers::none(), cx);
    cx.simulate_keystrokes("down");
    let opened = document.read_with(cx, |document, cx| {
        let open = document.open.as_ref().expect("still open");
        document.text(cx)[open.range.clone()].to_string()
    });
    assert_eq!(opened, "Last.");
}

/// Opens block `ix` as shown, its field set to `typed`.
pub(super) fn type_in(
    document: &Entity<MarkdownDocument>,
    ix: Option<usize>,
    typed: Option<&str>,
    cx: &mut VisualTestContext,
) {
    document.update_in(cx, |document, window, cx| {
        document.open(ix, Landing::End, window, cx);
        if let (Some(typed), Some(open)) = (typed, &document.open) {
            open.input.update(cx, |input, cx| input.set_text(typed, cx));
        }
    });
    cx.run_until_parked();
}

pub(super) fn opened(document: &Entity<MarkdownDocument>, cx: &mut VisualTestContext) -> String {
    document.read_with(cx, |document, cx| {
        let open = document.open.as_ref().expect("a block is open");
        open.input.read(cx).text().to_string()
    })
}

#[gpui::test]
fn a_press_after_deleting_a_block_opens_the_one_pressed(cx: &mut TestAppContext) {
    let (document, cx) = open(
        "A

B

C",
        MarkdownMode::Visual,
        cx,
    );
    type_in(&document, Some(0), Some(""), cx);
    type_in(&document, Some(2), None, cx);
    assert_eq!(opened(&document, cx), "C");
}

#[gpui::test]
fn a_press_after_splitting_a_block_opens_the_one_pressed(cx: &mut TestAppContext) {
    let (document, cx) = open(
        "A

B

C",
        MarkdownMode::Visual,
        cx,
    );
    type_in(
        &document,
        Some(0),
        Some(
            "A

New",
        ),
        cx,
    );
    type_in(&document, Some(1), None, cx);
    assert_eq!(opened(&document, cx), "B");
}

#[gpui::test]
fn the_caret_survives_an_undo_inside_a_character(cx: &mut TestAppContext) {
    let (document, cx) = open("éabcd", MarkdownMode::Visual, cx);
    type_in(&document, Some(0), Some("x"), cx);
    cx.simulate_keystrokes("escape");
    cx.dispatch_action(visual::Undo);
    assert_eq!(text(&document, cx), "éabcd");
    let caret = document.read_with(cx, |document, cx| document.caret(cx));
    assert_eq!(caret, (0, 0), "the restored caret");
}

#[gpui::test]
fn preview_takes_no_undo_or_redo(cx: &mut TestAppContext) {
    let (document, cx) = open(TEXT, MarkdownMode::Visual, cx);
    document.update(cx, |document, cx| document.set_text("Changed", cx));
    document.update_in(cx, |document, window, cx| {
        document.set_mode(MarkdownMode::Preview, window, cx)
    });
    cx.dispatch_action(visual::Undo);
    assert_eq!(text(&document, cx), "Changed");
}

#[gpui::test]
fn an_added_block_reports_its_caret_after_the_joint(cx: &mut TestAppContext) {
    for start in ["A", "A\n", "A\n\n"] {
        let (document, cx) = open(start, MarkdownMode::Visual, cx);
        type_in(&document, None, Some("B"), cx);
        let caret = document.read_with(cx, |document, cx| document.caret(cx));
        assert_eq!(caret, (2, 1), "after {start:?}");
    }
}

#[gpui::test]
fn the_keyboard_reaches_and_opens_a_block(cx: &mut TestAppContext) {
    let (document, cx) = open(TEXT, MarkdownMode::Visual, cx);
    cx.update(|window, cx| window.focus_next(cx));
    cx.simulate_keystrokes("enter");
    frames(cx);
    cx.simulate_event(gpui::KeyUpEvent {
        keystroke: gpui::Keystroke::parse("enter").expect("a key"),
    });
    frames(cx);
    assert_eq!(opened(&document, cx), "# Lift");
}

#[gpui::test]
fn a_change_under_an_open_block_closes_it(cx: &mut TestAppContext) {
    let (document, cx) = open(TEXT, MarkdownMode::Visual, cx);
    type_in(&document, Some(2), Some("- three"), cx);
    let editor = document.read_with(cx, |document, _| document.editor().clone());
    editor.update(cx, |editor, cx| editor.edit([(0..0, "x".to_string())], cx));
    cx.run_until_parked();
    assert!(document.read_with(cx, |document, _| document.open.is_none()));
    assert_eq!(text(&document, cx), format!("x{TEXT}"));
}

#[gpui::test]
fn a_press_after_a_write_back_waits_for_the_long_read(cx: &mut TestAppContext) {
    let long = format!("A\n\nB\n\nCCCC\n\n{}", "Prose.\n\n".repeat(AT_ONCE / 8 + 1));
    let (document, cx) = open(&long, MarkdownMode::Visual, cx);
    document.update_in(cx, |document, window, cx| {
        document.open(Some(0), Landing::End, window, cx);
        let input = document.open.as_ref().expect("open").input.clone();
        input.update(cx, |input, cx| input.set_text("AAAA", cx));
        // B is pressed before the new read lands.
        document.open(Some(1), Landing::End, window, cx);
        assert!(document.open.is_none(), "a stale range opens nothing");
    });
    cx.run_until_parked();
    type_in(&document, Some(1), None, cx);
    assert_eq!(opened(&document, cx), "B", "once read, B opens whole");
}

mod attach;
mod block;
mod block_edges;
mod cells;
mod format;
mod history;
mod lend;
mod links;
mod look;
mod open_look;
mod place;
mod point;
mod presses;
mod reading;
mod secondary;
mod toc;
mod tools;

#[gpui::test]
fn typed_source_reads_after_a_pause(cx: &mut TestAppContext) {
    let (document, cx) = open(TEXT, MarkdownMode::Source, cx);
    let editor = document.read_with(cx, |document, _| document.editor().clone());
    editor.update(cx, |editor, cx| editor.edit([(0..0, "x".to_string())], cx));
    cx.run_until_parked();
    assert!(
        !document.read_with(cx, |document, cx| document.read_now(cx)),
        "no read per key"
    );
    cx.executor().advance_clock(super::PAUSE);
    cx.run_until_parked();
    assert!(
        document.read_with(cx, |document, cx| document.read_now(cx)),
        "read after the pause"
    );
}

#[gpui::test]
fn a_text_past_the_largest_read_is_never_read(cx: &mut TestAppContext) {
    let (document, cx) = open("# Old\n", MarkdownMode::Source, cx);
    let long = "a\n".repeat(super::LARGEST_READ / 2 + 1);
    document.update(cx, |document, cx| document.set_text(long, cx));
    cx.executor().advance_clock(super::PAUSE);
    cx.run_until_parked();
    let (blocks, outline) = document.read_with(cx, |document, cx| {
        (document.parsed.blocks.len(), document.outline(cx).len())
    });
    assert_eq!((blocks, outline), (0, 0), "the old read is gone");
}
