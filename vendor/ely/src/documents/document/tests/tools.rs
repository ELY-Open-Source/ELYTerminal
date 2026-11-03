use gpui::{Modifiers, TestAppContext};

use super::{MarkdownMode, open, opened, press, text};

#[gpui::test]
fn enter_in_source_goes_on_with_the_list(cx: &mut TestAppContext) {
    let (document, cx) = open("- a", MarkdownMode::Source, cx);
    document.update(cx, |document, cx| document.set_caret(0, 3, cx));
    let editor = document.read_with(cx, |document, _| document.editor().clone());
    let focus = editor.read_with(cx, gpui::Focusable::focus_handle);
    cx.update(|window, cx| window.focus(&focus, cx));
    cx.simulate_keystrokes("enter");
    cx.simulate_input("b");
    assert_eq!(text(&document, cx), "- a\n- b");
    cx.simulate_keystrokes("enter enter");
    assert_eq!(
        text(&document, cx),
        "- a\n- b\n",
        "an empty item ends the list"
    );
}

#[gpui::test]
fn enter_in_a_block_goes_on_with_the_list(cx: &mut TestAppContext) {
    let (document, cx) = open("1. a\n", MarkdownMode::Visual, cx);
    press(0, Modifiers::none(), cx);
    cx.simulate_keystrokes("enter");
    cx.simulate_input("b");
    assert_eq!(opened(&document, cx), "1. a\n2. b");
}

#[gpui::test]
fn tab_shifts_a_block_line_in_and_out(cx: &mut TestAppContext) {
    let (document, cx) = open("- a\n- b\n", MarkdownMode::Visual, cx);
    press(0, Modifiers::none(), cx);
    cx.simulate_keystrokes("tab");
    assert_eq!(opened(&document, cx), "- a\n    - b");
    cx.simulate_keystrokes("shift-tab");
    assert_eq!(opened(&document, cx), "- a\n- b");
}

#[gpui::test]
fn a_table_edit_is_one_undo_step_at_the_caret(cx: &mut TestAppContext) {
    let (document, cx) = open("| A |\n| - |\n| 1 |\n", MarkdownMode::Source, cx);
    document.update(cx, |document, cx| document.set_caret(2, 2, cx));
    let edited = document.update(cx, |document, cx| {
        document.edit_table(crate::documents::TableEdit::RowBelow, cx)
    });
    assert_eq!(edited, Ok(()));
    cx.run_until_parked();
    assert_eq!(
        text(&document, cx),
        "| A    |\n| ---- |\n| 1    |\n|      |\n"
    );
    let editor = document.read_with(cx, |document, _| document.editor().clone());
    editor.update(cx, |editor, cx| editor.undo(cx));
    assert_eq!(text(&document, cx), "| A |\n| - |\n| 1 |\n");
}

#[gpui::test]
fn a_new_table_goes_after_the_caret_line(cx: &mut TestAppContext) {
    let (document, cx) = open("Intro\nNext\n", MarkdownMode::Visual, cx);
    document.update(cx, |document, cx| {
        document.insert_table(cx).expect("inserted")
    });
    assert_eq!(
        text(&document, cx),
        "Intro\n\n|      |      |      |\n| ---- | ---- | ---- |\n|      |      |      |\n|      |      |      |\n\nNext\n"
    );
}

#[gpui::test]
fn a_new_table_after_end_keeps_crlf(cx: &mut TestAppContext) {
    let (document, cx) = open("Intro\r\nNext\r\n", MarkdownMode::Source, cx);
    let editor = document.read_with(cx, |document, _| document.editor().clone());
    let focus = editor.read_with(cx, gpui::Focusable::focus_handle);
    cx.update(|window, cx| window.focus(&focus, cx));
    cx.simulate_keystrokes("end");
    let caret = editor.read_with(cx, |editor, _| editor.primary().range().start);
    assert_eq!(caret, 6, "End lands between CR and LF");
    document.update(cx, |document, cx| {
        document.insert_table(cx).expect("inserted")
    });
    assert_eq!(
        text(&document, cx),
        "Intro\r\n\r\n|      |      |      |\r\n| ---- | ---- | ---- |\r\n|      |      |      |\r\n|      |      |      |\r\n\r\nNext\r\n"
    );
}

#[gpui::test]
fn a_match_in_visual_opens_its_block_selected(cx: &mut TestAppContext) {
    let (document, cx) = open("# A\n\nfind me here\n", MarkdownMode::Visual, cx);
    let at = "# A\n\nfind me here\n".find("me").expect("the word");
    document.update_in(cx, |document, window, cx| {
        document.show_match(at..at + 2, window, cx)
    });
    let selected = document.read_with(cx, |document, cx| {
        let input = document
            .open
            .as_ref()
            .expect("a block opens")
            .input
            .read(cx);
        input.text()[input.selection()].to_string()
    });
    assert_eq!(selected, "me");
}

#[gpui::test]
fn every_replacement_is_one_undo_step(cx: &mut TestAppContext) {
    let (document, cx) = open("a b a", MarkdownMode::Visual, cx);
    document.update(cx, |document, cx| {
        document.replace(vec![(0..1, "x".into()), (4..5, "x".into())], cx)
    });
    assert_eq!(text(&document, cx), "x b x");
    let editor = document.read_with(cx, |document, _| document.editor().clone());
    editor.update(cx, |editor, cx| editor.undo(cx));
    assert_eq!(text(&document, cx), "a b a");
}

const FENCED: &str = "```text\n- a\n```\n";

#[gpui::test]
fn enter_in_source_code_is_a_plain_line_break(cx: &mut TestAppContext) {
    let (document, cx) = open(FENCED, MarkdownMode::Source, cx);
    document.update(cx, |document, cx| document.set_caret(1, 3, cx));
    let editor = document.read_with(cx, |document, _| document.editor().clone());
    let focus = editor.read_with(cx, gpui::Focusable::focus_handle);
    cx.update(|window, cx| window.focus(&focus, cx));
    cx.simulate_keystrokes("enter");
    assert_eq!(text(&document, cx), "```text\n- a\n\n```\n");
}

#[gpui::test]
fn enter_in_an_open_code_block_is_a_plain_line_break(cx: &mut TestAppContext) {
    let (document, cx) = open(FENCED, MarkdownMode::Visual, cx);
    press(0, Modifiers::none(), cx);
    document.update(cx, |document, cx| {
        let input = document.open.as_ref().expect("open").input.clone();
        input.update(cx, |input, cx| input.select(11..11, cx));
    });
    cx.simulate_keystrokes("enter");
    assert_eq!(opened(&document, cx), "```text\n- a\n\n```");
}

#[gpui::test]
fn a_match_shown_after_a_replacement_opens_selected(cx: &mut TestAppContext) {
    let (document, cx) = open("a b a\n", MarkdownMode::Visual, cx);
    document.update_in(cx, |document, window, cx| {
        document.replace(vec![(0..1, "x".into())], cx);
        document.show_match(4..5, window, cx);
    });
    cx.run_until_parked();
    cx.simulate_input("z");
    assert_eq!(text(&document, cx), "x b z\n");
}

#[gpui::test]
fn preview_takes_no_edits(cx: &mut TestAppContext) {
    let (document, cx) = open("| A |\n| - |\n| 1 |\n", MarkdownMode::Preview, cx);
    let refused = document.update(cx, |document, cx| {
        (
            document.replace(vec![(2..3, "B".into())], cx),
            document.edit_table(crate::documents::TableEdit::RowBelow, cx),
            document.insert_table(cx),
        )
    });
    let preview = Err(crate::documents::EditRefusal::Preview);
    assert_eq!(refused, (false, preview, preview));
    assert_eq!(text(&document, cx), "| A |\n| - |\n| 1 |\n");
}

#[gpui::test]
fn a_quoted_list_goes_on_in_source_and_in_a_block(cx: &mut TestAppContext) {
    let (document, cx) = open("> - a\n", MarkdownMode::Visual, cx);
    press(0, Modifiers::none(), cx);
    cx.simulate_keystrokes("enter");
    assert_eq!(opened(&document, cx), "> - a\n> - ");
    cx.simulate_keystrokes("escape");
    document.update_in(cx, |document, window, cx| {
        document.set_mode(MarkdownMode::Source, window, cx);
        document.set_text("> - a", cx);
        document.set_caret(0, 5, cx);
    });
    cx.run_until_parked();
    cx.simulate_keystrokes("enter");
    assert_eq!(text(&document, cx), "> - a\n> - ");
}

/// A text long enough to be read off the window.
fn long() -> String {
    "para\n\n".repeat(super::super::AT_ONCE / 6 + 1)
}

#[gpui::test]
fn a_waiting_match_lapses_when_the_text_changes(cx: &mut TestAppContext) {
    let (document, cx) = open("x\n", MarkdownMode::Visual, cx);
    document.update_in(cx, |document, window, cx| {
        document.set_text(long(), cx);
        document.show_match(0..1, window, cx);
    });
    document.update(cx, |document, cx| {
        document.set_text(format!("y{}", long()), cx)
    });
    super::frames(cx);
    let opened = document.read_with(cx, |document, _| document.open.is_some());
    assert!(!opened, "the old match does not land on the new text");
}

#[gpui::test]
fn a_newer_match_takes_the_place_of_a_waiting_one(cx: &mut TestAppContext) {
    let (document, cx) = open("x\n", MarkdownMode::Visual, cx);
    document.update_in(cx, |document, window, cx| {
        document.set_text(long(), cx);
        document.show_match(0..1, window, cx);
    });
    document.update(cx, |document, cx| document.set_text("a b a\n", cx));
    cx.run_until_parked();
    document.update_in(cx, |document, window, cx| {
        document.show_match(4..5, window, cx)
    });
    super::frames(cx);
    cx.simulate_input("z");
    assert_eq!(text(&document, cx), "a b z\n");
}

#[gpui::test]
fn an_empty_task_ends_the_list(cx: &mut TestAppContext) {
    let (document, cx) = open("- [x] done", MarkdownMode::Source, cx);
    document.update(cx, |document, cx| document.set_caret(0, 10, cx));
    let editor = document.read_with(cx, |document, _| document.editor().clone());
    let focus = editor.read_with(cx, gpui::Focusable::focus_handle);
    cx.update(|window, cx| window.focus(&focus, cx));
    cx.simulate_keystrokes("enter");
    cx.run_until_parked();
    cx.simulate_keystrokes("enter");
    assert_eq!(text(&document, cx), "- [x] done\n");
}

#[gpui::test]
fn shift_tab_takes_a_tab_indent_out(cx: &mut TestAppContext) {
    let (document, cx) = open("- a\n\t- b\n", MarkdownMode::Visual, cx);
    press(0, Modifiers::none(), cx);
    cx.simulate_keystrokes("shift-tab");
    assert_eq!(opened(&document, cx), "- a\n- b");
}

#[gpui::test]
fn shift_tab_takes_spaces_then_a_tab_out(cx: &mut TestAppContext) {
    let (document, cx) = open("- a\n  \t- b\n", MarkdownMode::Visual, cx);
    press(0, Modifiers::none(), cx);
    cx.simulate_keystrokes("shift-tab");
    assert_eq!(opened(&document, cx), "- a\n- b");
}

#[gpui::test]
fn a_table_on_an_indented_blank_line_is_a_table(cx: &mut TestAppContext) {
    let (document, cx) = open(
        "    
",
        MarkdownMode::Source,
        cx,
    );
    document.update(cx, |document, cx| {
        document.set_caret(0, 4, cx);
        document.insert_table(cx).expect("inserted");
    });
    let column = document.update(cx, |document, cx| {
        document.edit_table(crate::documents::TableEdit::ColumnRight, cx)
    });
    assert_eq!(column, Ok(()), "{}", text(&document, cx));
    cx.run_until_parked();
    assert!(text(&document, cx).starts_with("| "));
}

const SMALL: &str = "| A | B |\n| - | - |\n| 1 | 2 |\n";

#[gpui::test]
fn a_table_move_reads_apart_as_one_undo_step(cx: &mut TestAppContext) {
    let (document, cx) = open(SMALL, MarkdownMode::Source, cx);
    document.update(cx, |document, cx| document.set_caret(2, 2, cx));
    let moved = document.update(cx, |document, cx| {
        document.edit_table(crate::documents::TableEdit::MoveRowUp, cx)
    });
    assert_eq!(moved, Ok(()));
    cx.run_until_parked();
    assert_eq!(
        text(&document, cx),
        "| 1    | 2    |\n| ---- | ---- |\n| A    | B    |\n"
    );
    let editor = document.read_with(cx, |document, _| document.editor().clone());
    editor.update(cx, |editor, cx| editor.undo(cx));
    assert_eq!(text(&document, cx), SMALL);
}

#[gpui::test]
fn a_table_edit_at_an_edge_refuses_by_event(cx: &mut TestAppContext) {
    let (document, cx) = open(SMALL, MarkdownMode::Source, cx);
    let heard = super::format::refusals(&document, cx);
    document.update(cx, |document, cx| document.set_caret(2, 2, cx));
    document.update(cx, |document, cx| {
        document
            .edit_table(crate::documents::TableEdit::MoveRowDown, cx)
            .expect("taken")
    });
    cx.run_until_parked();
    let edge = crate::documents::EditRefusal::Cells(crate::documents::TableRefusal::Edge);
    assert_eq!(*heard.borrow(), [edge]);
    assert_eq!(text(&document, cx), SMALL);
}

#[gpui::test]
fn tidy_in_cr_text_keeps_what_comes_before(cx: &mut TestAppContext) {
    let source = "Before\r\r| A | B |\r| - | - |\r| 1 | 2 |\r\rAfter\r";
    let (document, cx) = open(source, MarkdownMode::Source, cx);
    let at = source.find('1').expect("a cell");
    let editor = document.read_with(cx, |document, _| document.editor().clone());
    editor.update(cx, |editor, cx| editor.select(std::iter::once(at..at), cx));
    document.update(cx, |document, cx| {
        document
            .edit_table(crate::documents::TableEdit::Tidy, cx)
            .expect("taken")
    });
    cx.run_until_parked();
    assert_eq!(
        text(&document, cx),
        "Before\r\r| A    | B    |\r| ---- | ---- |\r| 1    | 2    |\r\rAfter\r"
    );
}

#[gpui::test]
fn a_new_table_takes_the_texts_line_ending(cx: &mut TestAppContext) {
    let (document, cx) = open("Intro\rNext\r", MarkdownMode::Visual, cx);
    document.update(cx, |document, cx| {
        document.insert_table(cx).expect("inserted")
    });
    assert_eq!(
        text(&document, cx),
        "Intro\r\r|      |      |      |\r| ---- | ---- | ---- |\r|      |      |      |\r|      |      |      |\r\rNext\r"
    );
}

#[gpui::test]
fn tidy_in_an_open_cell_writes_it_and_tidies(cx: &mut TestAppContext) {
    let (document, cx) = open(SMALL, MarkdownMode::Visual, cx);
    press(0, Modifiers::none(), cx);
    assert_eq!(opened(&document, cx), "2");
    document.update(cx, |document, cx| {
        document
            .edit_table(crate::documents::TableEdit::Tidy, cx)
            .expect("taken")
    });
    cx.run_until_parked();
    assert_eq!(
        text(&document, cx),
        "| A    | B    |\n| ---- | ---- |\n| 1    | 2    |\n"
    );
}

const CRLF: &str = "Intro\r\nNext\r\n";

/// A code block in CRLF text after `keys`.
fn code_block_after(keys: &str, cx: &mut TestAppContext) -> String {
    let (document, cx) = open(CRLF, MarkdownMode::Source, cx);
    let editor = document.read_with(cx, |document, _| document.editor().clone());
    let focus = editor.read_with(cx, gpui::Focusable::focus_handle);
    cx.update(|window, cx| window.focus(&focus, cx));
    cx.simulate_keystrokes(keys);
    document.update(cx, |document, cx| {
        document
            .block(crate::documents::BlockCommand::CodeBlock, cx)
            .expect("taken")
    });
    cx.run_until_parked();
    text(&document, cx)
}

const FENCE_AFTER_INTRO: &str = "Intro\r\n\r\n```\r\n\r\n```\r\n\r\nNext\r\n";

#[gpui::test]
fn a_block_after_end_in_crlf_text_reads_the_line(cx: &mut TestAppContext) {
    assert_eq!(code_block_after("end", cx), FENCE_AFTER_INTRO);
}

#[gpui::test]
fn a_selection_from_inside_a_crlf_reads_the_line(cx: &mut TestAppContext) {
    assert_eq!(code_block_after("end shift-right", cx), FENCE_AFTER_INTRO);
}

#[gpui::test]
fn delete_table_takes_the_table_or_refuses_outside(cx: &mut TestAppContext) {
    let source = "before\n\n| A | B |\n| - | - |\n| 1 | 2 |\n\nafter\n";
    let (document, cx) = open(source, MarkdownMode::Source, cx);
    let heard = super::format::refusals(&document, cx);
    let delete = |cx: &mut gpui::VisualTestContext| {
        document.update(cx, |document, cx| {
            document
                .edit_table(crate::documents::TableEdit::Delete, cx)
                .expect("taken")
        });
        cx.run_until_parked();
    };
    document.update(cx, |document, cx| document.set_caret(4, 2, cx));
    delete(cx);
    assert_eq!(text(&document, cx), "before\n\nafter\n");
    let editor = document.read_with(cx, |document, _| document.editor().clone());
    editor.update(cx, |editor, cx| editor.undo(cx));
    assert_eq!(text(&document, cx), source);
    document.update(cx, |document, cx| document.set_caret(0, 2, cx));
    delete(cx);
    let outside = crate::documents::EditRefusal::Cells(crate::documents::TableRefusal::Outside);
    assert_eq!(*heard.borrow(), [outside]);
    assert_eq!(text(&document, cx), source);
}

#[gpui::test]
fn undo_follows_a_table_edit_that_closes_the_block(cx: &mut TestAppContext) {
    let source = "before\n\n| A |\n| - |\n| 1 |\n\nafter\n";
    let (document, cx) = open(source, MarkdownMode::Visual, cx);
    press(1, Modifiers::none(), cx);
    document.update(cx, |document, cx| {
        document
            .edit_table(crate::documents::TableEdit::Delete, cx)
            .expect("taken")
    });
    cx.run_until_parked();
    assert_eq!(text(&document, cx), "before\n\nafter\n");
    let held = cx.update(|window, cx| {
        let handle = gpui::Focusable::focus_handle(document.read(cx), cx);
        handle.contains_focused(window, cx)
    });
    assert!(held, "focus stays in the document");
    cx.simulate_keystrokes("secondary-z");
    assert_eq!(text(&document, cx), source);
}
