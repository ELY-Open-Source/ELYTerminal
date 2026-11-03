use gpui::{Entity, Modifiers, TestAppContext, VisualTestContext};

use super::{MarkdownDocument, MarkdownMode, TEXT, open, press, text};

/// Focus on `document`'s own handle, as an owner gives it.
fn focus(document: &Entity<MarkdownDocument>, cx: &mut VisualTestContext) {
    let handle = document.read_with(cx, |document, cx| {
        gpui::Focusable::focus_handle(document, cx)
    });
    cx.update(|window, cx| window.focus(&handle, cx));
    cx.run_until_parked();
}

#[gpui::test]
fn an_empty_visual_document_takes_typing_on_focus(cx: &mut TestAppContext) {
    let (document, cx) = open("", MarkdownMode::Visual, cx);
    focus(&document, cx);
    cx.simulate_input("Hi");
    assert_eq!(text(&document, cx), "Hi", "typing lands with no press");
    cx.simulate_keystrokes("escape");
    cx.run_until_parked();
    assert_eq!(text(&document, cx), "Hi", "escape keeps it");
}

#[gpui::test]
fn focus_opens_no_block_in_a_written_document(cx: &mut TestAppContext) {
    let (document, cx) = open(TEXT, MarkdownMode::Visual, cx);
    focus(&document, cx);
    cx.simulate_input("x");
    assert_eq!(text(&document, cx), TEXT);
}

fn mode(document: &Entity<MarkdownDocument>, mode: MarkdownMode, cx: &mut VisualTestContext) {
    document.update_in(cx, |document, window, cx| {
        document.set_mode(mode, window, cx)
    });
    cx.run_until_parked();
}

#[gpui::test]
fn preview_and_back_still_take_typing_in_an_empty_document(cx: &mut TestAppContext) {
    let (document, cx) = open("", MarkdownMode::Visual, cx);
    focus(&document, cx);
    cx.simulate_keystrokes("escape");
    mode(&document, MarkdownMode::Preview, cx);
    mode(&document, MarkdownMode::Visual, cx);
    cx.simulate_input("Hi");
    assert_eq!(text(&document, cx), "Hi");
}

#[gpui::test]
fn undo_in_an_untouched_block_steps_the_document(cx: &mut TestAppContext) {
    let (document, cx) = open("Seed", MarkdownMode::Source, cx);
    document.update(cx, |document, cx| document.set_text("", cx));
    mode(&document, MarkdownMode::Visual, cx);
    cx.simulate_keystrokes("secondary-z");
    cx.run_until_parked();
    assert_eq!(text(&document, cx), "Seed", "the source edit is undone");
    cx.simulate_keystrokes("secondary-shift-z");
    cx.run_until_parked();
    assert_eq!(text(&document, cx), "", "and redone");
}

#[gpui::test]
fn undo_takes_a_typed_block_back_as_one_step(cx: &mut TestAppContext) {
    let (document, cx) = open("Seed", MarkdownMode::Source, cx);
    document.update(cx, |document, cx| document.set_text(TEXT, cx));
    mode(&document, MarkdownMode::Visual, cx);
    press(1, Modifiers::none(), cx);
    cx.simulate_input(" x");
    cx.simulate_keystrokes("secondary-z");
    cx.run_until_parked();
    assert_eq!(text(&document, cx), TEXT, "the edit before stays");
}

#[gpui::test]
fn redo_survives_a_switch_to_source(cx: &mut TestAppContext) {
    let (document, cx) = open("", MarkdownMode::Visual, cx);
    focus(&document, cx);
    cx.simulate_input("Hi");
    cx.simulate_keystrokes("secondary-z");
    cx.run_until_parked();
    assert_eq!(text(&document, cx), "");
    mode(&document, MarkdownMode::Source, cx);
    cx.simulate_keystrokes("secondary-shift-z");
    assert_eq!(text(&document, cx), "Hi");
}

#[gpui::test]
fn redo_survives_preview_and_back(cx: &mut TestAppContext) {
    let (document, cx) = open("Seed", MarkdownMode::Visual, cx);
    press(0, Modifiers::none(), cx);
    cx.simulate_input("!");
    cx.simulate_keystrokes("secondary-z");
    cx.run_until_parked();
    assert_eq!(text(&document, cx), "Seed");
    mode(&document, MarkdownMode::Preview, cx);
    mode(&document, MarkdownMode::Visual, cx);
    cx.simulate_keystrokes("secondary-shift-z");
    cx.run_until_parked();
    assert_eq!(text(&document, cx), "Seed!");
}

#[gpui::test]
fn redo_survives_a_second_undo(cx: &mut TestAppContext) {
    let (document, cx) = open("", MarkdownMode::Visual, cx);
    focus(&document, cx);
    cx.simulate_input("Hi");
    cx.simulate_keystrokes("secondary-z");
    cx.run_until_parked();
    cx.simulate_keystrokes("secondary-z");
    cx.run_until_parked();
    cx.simulate_keystrokes("secondary-shift-z");
    cx.run_until_parked();
    assert_eq!(text(&document, cx), "Hi");
}

#[gpui::test]
fn typing_lands_after_undo_empties_the_document(cx: &mut TestAppContext) {
    let (document, cx) = open("", MarkdownMode::Visual, cx);
    focus(&document, cx);
    cx.simulate_input("Hi");
    mode(&document, MarkdownMode::Source, cx);
    mode(&document, MarkdownMode::Visual, cx);
    cx.simulate_keystrokes("secondary-z");
    cx.run_until_parked();
    assert_eq!(text(&document, cx), "");
    cx.simulate_input("Next");
    assert_eq!(text(&document, cx), "Next");
}
