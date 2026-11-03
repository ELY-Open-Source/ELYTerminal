use std::{cell::RefCell, rc::Rc};

use gpui::{Entity, Modifiers, MouseButton, TestAppContext, VisualTestContext, point, px};

use super::{MarkdownDocument, MarkdownMode, open};

const LINKED: &str = "[a long linked line of text](https://ely.dev)\n";

/// Links pressed so far, as the owner hears them.
fn heard(
    document: &Entity<MarkdownDocument>,
    cx: &mut VisualTestContext,
) -> Rc<RefCell<Vec<String>>> {
    let heard = Rc::new(RefCell::new(Vec::new()));
    let kept = heard.clone();
    document.update(cx, |document, _| {
        document.on_link(move |url, _, _| kept.borrow_mut().push(url.to_string()))
    });
    heard
}

/// Presses the text of block 0, past its padding.
fn press_text(modifiers: Modifiers, cx: &mut VisualTestContext) {
    let bounds = cx
        .debug_bounds("markdown-block-0")
        .expect("the block is drawn");
    let at = point(bounds.left() + px(16.0), bounds.top() + px(10.0));
    cx.simulate_mouse_down(at, MouseButton::Left, modifiers);
    cx.simulate_mouse_up(at, MouseButton::Left, modifiers);
    cx.run_until_parked();
}

fn is_open(document: &Entity<MarkdownDocument>, cx: &mut VisualTestContext) -> bool {
    document.read_with(cx, |document, _| document.open.is_some())
}

#[gpui::test]
fn a_click_in_preview_hands_the_link_over(cx: &mut TestAppContext) {
    let (document, cx) = open(LINKED, MarkdownMode::Preview, cx);
    let heard = heard(&document, cx);
    press_text(Modifiers::none(), cx);
    assert_eq!(*heard.borrow(), ["https://ely.dev"]);
}

#[gpui::test]
fn a_plain_press_in_visual_edits_the_block(cx: &mut TestAppContext) {
    let (document, cx) = open(LINKED, MarkdownMode::Visual, cx);
    let heard = heard(&document, cx);
    press_text(Modifiers::none(), cx);
    assert!(heard.borrow().is_empty() && is_open(&document, cx));
}

#[gpui::test]
fn a_secondary_press_in_visual_hands_the_link_over(cx: &mut TestAppContext) {
    let (document, cx) = open(LINKED, MarkdownMode::Visual, cx);
    let heard = heard(&document, cx);
    press_text(Modifiers::secondary_key(), cx);
    assert_eq!(*heard.borrow(), ["https://ely.dev"]);
    assert!(!is_open(&document, cx), "no block opens");
}
