//! A press in a table opens only its cell (R6.1).

use gpui::{Entity, Modifiers, MouseButton, TestAppContext, VisualTestContext, point, px};

use super::{open, opened, point::heard, text};
use crate::documents::{MarkdownDocument, MarkdownMode};

const TABLE: &str = "| a | **bold** |\n| --- | --- |\n| c |  |\n";

/// Presses cell `row`, `column` of the first table.
fn press_cell(row: usize, column: usize, cx: &mut VisualTestContext) {
    press_cell_with(row, column, MouseButton::Left, cx);
}

fn press_cell_with(row: usize, column: usize, button: MouseButton, cx: &mut VisualTestContext) {
    let name = format!("markdown-cell-0-{row}-{column}");
    let bounds = cx
        .debug_bounds(Box::leak(name.into_boxed_str()))
        .expect("the cell is drawn");
    let at = point(bounds.left() + px(2.0), bounds.top() + px(2.0));
    cx.simulate_mouse_down(at, button, Modifiers::none());
    cx.simulate_mouse_up(at, button, Modifiers::none());
    cx.run_until_parked();
}

/// Presses the open cell's field `x` past its left.
fn press_field(x: f32, button: MouseButton, cx: &mut VisualTestContext) {
    let field = cx
        .debug_bounds("markdown-open-cell")
        .expect("a cell is open");
    let at = point(field.left() + px(x), field.top() + px(10.0));
    cx.simulate_mouse_down(at, button, Modifiers::none());
    cx.simulate_mouse_up(at, button, Modifiers::none());
    cx.run_until_parked();
}

fn caret(document: &Entity<MarkdownDocument>, cx: &mut VisualTestContext) -> usize {
    document.read_with(cx, |document, cx| {
        document
            .open
            .as_ref()
            .expect("open")
            .input
            .read(cx)
            .cursor()
    })
}

/// Where byte `k` of the open cell lies.
fn x_of(document: &Entity<MarkdownDocument>, k: usize, cx: &mut VisualTestContext) -> f32 {
    (0..400)
        .map(|step| step as f32)
        .find(|&x| {
            press_field(x, MouseButton::Left, cx);
            caret(document, cx) == k
        })
        .expect("the byte is drawn")
}

#[gpui::test]
fn a_press_opens_its_cell_and_writes_one_undo_step(cx: &mut TestAppContext) {
    let (document, cx) = open(TABLE, MarkdownMode::Visual, cx);
    press_cell(0, 1, cx);
    assert_eq!(opened(&document, cx), "**bold**");
    assert!(
        cx.debug_bounds("markdown-cell-0-0-0").is_some(),
        "the rest stays drawn"
    );
    press_cell(1, 1, cx);
    assert_eq!(opened(&document, cx), "");
    cx.simulate_input("x|y");
    assert_eq!(
        text(&document, cx),
        "| a | **bold** |\n| --- | --- |\n| c | x\\|y |\n"
    );
    cx.simulate_keystrokes("escape");
    let editor = document.read_with(cx, |document, _| document.editor().clone());
    editor.update(cx, |editor, cx| editor.undo(cx));
    assert_eq!(text(&document, cx), TABLE);
}

#[gpui::test]
fn tab_and_arrows_walk_the_cells(cx: &mut TestAppContext) {
    let (document, cx) = open(TABLE, MarkdownMode::Visual, cx);
    press_cell(0, 0, cx);
    assert_eq!(opened(&document, cx), "a");
    cx.simulate_keystrokes("tab");
    assert_eq!(opened(&document, cx), "**bold**");
    cx.simulate_keystrokes("shift-tab");
    assert_eq!(opened(&document, cx), "a");
    cx.simulate_keystrokes("down");
    assert_eq!(opened(&document, cx), "c");
    cx.simulate_keystrokes("enter");
    assert!(document.read_with(cx, |document, _| document.open.is_none()));
    assert_eq!(text(&document, cx), TABLE);
}

const WORDS: &str = "| alpha | beta |\n| --- | --- |\n| c | d |\n";

#[gpui::test]
fn a_press_in_the_open_cell_moves_its_caret_and_keeps_one_step(cx: &mut TestAppContext) {
    let (document, cx) = open(WORDS, MarkdownMode::Visual, cx);
    press_cell(0, 0, cx);
    assert_eq!(caret(&document, cx), 5);
    cx.simulate_input("X");
    let x = x_of(&document, 1, cx);
    press_field(x, MouseButton::Left, cx);
    assert_eq!(caret(&document, cx), 1, "the cell stays open");
    cx.simulate_input("Y");
    cx.simulate_keystrokes("escape");
    assert_eq!(
        text(&document, cx),
        "| aYlphaX | beta |\n| --- | --- |\n| c | d |\n"
    );
    let editor = document.read_with(cx, |document, _| document.editor().clone());
    editor.update(cx, |editor, cx| editor.undo(cx));
    assert_eq!(text(&document, cx), WORDS);
}

#[gpui::test]
fn a_match_in_a_table_opens_its_cell_selected(cx: &mut TestAppContext) {
    let table = "| needle | B |\n| --- | --- |\n| C | tail |\n";
    let (document, cx) = open(table, MarkdownMode::Visual, cx);
    let at = table.find("needle").expect("the word");
    document.update_in(cx, |document, window, cx| {
        document.show_match(at..at + 6, window, cx)
    });
    assert_eq!(opened(&document, cx), "needle");
    let picked = document.read_with(cx, |document, cx| {
        document
            .open
            .as_ref()
            .expect("open")
            .input
            .read(cx)
            .selection()
    });
    assert_eq!(picked, 0..6);
}

#[gpui::test]
fn a_closing_backslash_keeps_the_table(cx: &mut TestAppContext) {
    let table = "|a|b|\n|---|---|\n|c|d|\n";
    let (document, cx) = open(table, MarkdownMode::Visual, cx);
    press_cell(0, 0, cx);
    cx.simulate_input("\\");
    cx.simulate_keystrokes("escape");
    assert_eq!(text(&document, cx), "|a\\\\|b|\n|---|---|\n|c|d|\n");
    press_cell(0, 1, cx);
    assert_eq!(opened(&document, cx), "b", "still its own cell");
}

#[gpui::test]
fn a_right_press_after_typed_pipes_finds_the_link(cx: &mut TestAppContext) {
    let (document, cx) = open(WORDS, MarkdownMode::Visual, cx);
    let heard = heard(&document, cx);
    press_cell(1, 1, cx);
    cx.simulate_keystrokes("backspace");
    cx.simulate_input("||[go](https://ely.dev)");
    let x = x_of(&document, 3, cx);
    press_field(x, MouseButton::Right, cx);
    let target = heard.borrow().last().cloned().expect("heard");
    assert_eq!(target.link.as_deref(), Some("https://ely.dev"));
}

#[gpui::test]
fn a_right_press_on_a_cell_points_at_it(cx: &mut TestAppContext) {
    let (document, cx) = open(WORDS, MarkdownMode::Visual, cx);
    let heard = heard(&document, cx);
    press_cell_with(1, 1, MouseButton::Right, cx);
    let at = heard.borrow().last().expect("heard").at;
    assert_eq!(at, WORDS.rfind(" d").expect("the cell") + 2);
}

#[gpui::test]
fn an_untouched_backslash_cell_keeps_its_bytes(cx: &mut TestAppContext) {
    let table = "| a\\ | b |\n| --- | --- |\n| c | d |\n";
    let (document, cx) = open(table, MarkdownMode::Visual, cx);
    press_cell(0, 0, cx);
    assert_eq!(opened(&document, cx), "a\\");
    document.update_in(cx, |document, window, cx| {
        document.set_mode(MarkdownMode::Source, window, cx)
    });
    assert_eq!(text(&document, cx), table);
}

const BARE: &str = "alpha | beta\n--- | ---\nc | d\n";

#[gpui::test]
fn tab_leaves_an_edited_first_cell_without_outer_pipes(cx: &mut TestAppContext) {
    let (document, cx) = open(BARE, MarkdownMode::Visual, cx);
    press_cell(0, 0, cx);
    cx.simulate_input("X");
    cx.simulate_keystrokes("tab");
    assert_eq!(opened(&document, cx), "beta");
    assert_eq!(text(&document, cx), "alphaX | beta\n--- | ---\nc | d\n");
}

#[gpui::test]
fn a_neighbour_press_leaves_an_edited_first_cell(cx: &mut TestAppContext) {
    let (document, cx) = open(BARE, MarkdownMode::Visual, cx);
    press_cell(0, 0, cx);
    cx.simulate_input("X");
    press_cell(0, 1, cx);
    assert_eq!(opened(&document, cx), "beta");
}

#[gpui::test]
fn the_open_cells_padding_lands_at_its_end_in_one_step(cx: &mut TestAppContext) {
    let (document, cx) = open(WORDS, MarkdownMode::Visual, cx);
    press_cell(0, 0, cx);
    cx.simulate_input("X");
    let x = x_of(&document, 1, cx);
    press_field(x, MouseButton::Left, cx);
    assert_eq!(caret(&document, cx), 1);
    press_cell(0, 0, cx);
    assert_eq!(caret(&document, cx), 6, "the padding lands at the end");
    cx.simulate_input("Y");
    cx.simulate_keystrokes("escape");
    assert_eq!(
        text(&document, cx),
        "| alphaXY | beta |\n| --- | --- |\n| c | d |\n"
    );
    let editor = document.read_with(cx, |document, _| document.editor().clone());
    editor.update(cx, |editor, cx| editor.undo(cx));
    assert_eq!(text(&document, cx), WORDS);
}

#[gpui::test]
fn a_right_press_on_the_open_cells_padding_points_at_its_end(cx: &mut TestAppContext) {
    let (document, cx) = open(WORDS, MarkdownMode::Visual, cx);
    let heard = heard(&document, cx);
    press_cell(0, 0, cx);
    cx.simulate_input("||");
    press_cell_with(0, 0, MouseButton::Right, cx);
    let target = heard.borrow().last().cloned().expect("heard");
    assert!(target.in_table);
    assert_eq!(
        target.at,
        WORDS.find("alpha").expect("the cell") + "alpha\\|\\|".len()
    );
    assert_eq!(opened(&document, cx), "alpha||", "the cell stays open");
}

#[gpui::test]
fn the_caret_after_a_typed_pipe_reads_the_escaped_cell(cx: &mut TestAppContext) {
    for (typed, column) in [("|中", 5), ("|x", 5)] {
        let (document, cx) = open(WORDS, MarkdownMode::Visual, cx);
        press_cell(0, 0, cx);
        cx.simulate_keystrokes("backspace backspace backspace backspace backspace");
        cx.simulate_input(typed);
        let caret = document.read_with(cx, |document, cx| document.caret(cx));
        assert_eq!(caret, (0, column), "{typed}");
    }
}

#[gpui::test]
fn emptied_bare_edge_cells_keep_the_table(cx: &mut TestAppContext) {
    let cases = [
        (0, 0, 5, "| | beta\n--- | ---\nc | d\n"),
        (0, 1, 4, "alpha | |\n--- | ---\nc | d\n"),
        (1, 0, 1, "alpha | beta\n--- | ---\n| | d\n"),
    ];
    for (row, column, chars, written) in cases {
        let (document, cx) = open(BARE, MarkdownMode::Visual, cx);
        press_cell(row, column, cx);
        for _ in 0..chars {
            cx.simulate_keystrokes("backspace");
        }
        cx.simulate_keystrokes("escape");
        assert_eq!(text(&document, cx), written);
        press_cell(1, 1, cx);
        assert_eq!(opened(&document, cx), "d", "{written:?} is still a table");
    }
}

#[gpui::test]
fn a_right_press_beside_an_edited_cell_points(cx: &mut TestAppContext) {
    let (document, cx) = open(WORDS, MarkdownMode::Visual, cx);
    let heard = heard(&document, cx);
    press_cell(0, 0, cx);
    cx.simulate_input("X");
    press_cell_with(0, 1, MouseButton::Right, cx);
    let at = heard.borrow().last().expect("heard").at;
    let written = "| alphaX | beta |\n| --- | --- |\n| c | d |\n";
    assert_eq!(text(&document, cx), written);
    assert_eq!(at, written.find("beta").expect("the cell") + 4);
}

#[gpui::test]
fn tab_in_a_long_document_opens_the_next_cell_once_read(cx: &mut TestAppContext) {
    let long = format!("{WORDS}\n```\n{}\n```\n", "x".repeat(super::super::AT_ONCE));
    let (document, cx) = open(&long, MarkdownMode::Visual, cx);
    document.update_in(cx, |document, window, cx| document.reveal(0, window, cx));
    super::frames(cx);
    press_cell(0, 0, cx);
    cx.simulate_input("X");
    cx.simulate_keystrokes("tab");
    cx.run_until_parked();
    assert_eq!(opened(&document, cx), "beta");
    assert!(text(&document, cx).starts_with("| alphaX | beta |"));
}
