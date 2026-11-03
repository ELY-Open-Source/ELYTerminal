//! Right presses that keep a selection; Delete Image (R4.10).

use gpui::{Entity, Modifiers, MouseButton, TestAppContext, VisualTestContext, point, px};

use super::{MarkdownMode, open, opened, point::heard, point::press_on, text};
use crate::documents::{DocumentTarget, EditRefusal, MarkdownDocument, TargetBlock};

/// A right press near the open block's left edge.
fn press_open(cx: &mut VisualTestContext) {
    let field = cx.debug_bounds("markdown-open").expect("a block is open");
    let at = point(field.left() + px(20.0), field.top() + px(14.0));
    cx.simulate_mouse_down(at, MouseButton::Right, Modifiers::none());
    cx.simulate_mouse_up(at, MouseButton::Right, Modifiers::none());
    cx.run_until_parked();
}

fn open_selection(
    document: &Entity<MarkdownDocument>,
    cx: &mut VisualTestContext,
) -> Option<std::ops::Range<usize>> {
    document.read_with(cx, |document, cx| {
        (document.open.as_ref()).map(|open| open.input.read(cx).selection())
    })
}

fn select_open(
    document: &Entity<MarkdownDocument>,
    range: std::ops::Range<usize>,
    cx: &mut VisualTestContext,
) {
    document.update(cx, |document, cx| {
        let input = document.open.as_ref().expect("open").input.clone();
        input.update(cx, |input, cx| input.select(range, cx));
    });
}

const LINKED: &str = "[docs](https://x.test)\n\nNext.\n";

#[gpui::test]
fn a_right_press_in_a_selection_keeps_the_block_and_the_selection(cx: &mut TestAppContext) {
    let (document, cx) = open(LINKED, MarkdownMode::Visual, cx);
    let heard = heard(&document, cx);
    press_on(0, 1, MouseButton::Left, cx);
    select_open(&document, 0..22, cx);
    press_open(cx);
    assert_eq!(
        open_selection(&document, cx),
        Some(0..22),
        "kept open and selected"
    );
    let at = heard.borrow().first().expect("heard").at;
    assert!(at < 6, "the pressed byte, not the selection's: {at}");
    let target = DocumentTarget {
        link: Some("https://x.test".into()),
        ..super::point::target(at, TargetBlock::Paragraph)
    };
    assert_eq!(*heard.borrow(), [target]);
    assert_eq!(text(&document, cx), LINKED, "nothing written");
}

#[gpui::test]
fn a_right_press_outside_the_selection_moves_the_caret(cx: &mut TestAppContext) {
    let (document, cx) = open(LINKED, MarkdownMode::Visual, cx);
    press_on(0, 1, MouseButton::Left, cx);
    select_open(&document, 22..22, cx);
    press_open(cx);
    let at = open_selection(&document, cx).expect("still open");
    assert!(
        at.is_empty() && at.start < 5,
        "the caret went to the press: {at:?}"
    );
}

#[gpui::test]
fn a_reference_in_the_open_block_reads_its_definition_outside(cx: &mut TestAppContext) {
    let source = "[docs][d]\n\n[d]: https://x.test\n";
    let (document, cx) = open(source, MarkdownMode::Visual, cx);
    let heard = heard(&document, cx);
    press_on(0, 1, MouseButton::Left, cx);
    select_open(&document, 0..9, cx);
    press_open(cx);
    let link = heard
        .borrow()
        .first()
        .and_then(|target| target.link.clone());
    assert_eq!(link.as_deref(), Some("https://x.test"));
    assert_eq!(opened(&document, cx), "[docs][d]");
}

#[gpui::test]
fn a_right_press_in_source_keeps_the_selection(cx: &mut TestAppContext) {
    let (document, cx) = open("alpha beta\n", MarkdownMode::Source, cx);
    let editor = document.read_with(cx, |document, _| document.editor().clone());
    editor.update(cx, |editor, cx| editor.select(std::iter::once(0..10), cx));
    let line = cx.debug_bounds("editor-line-0").expect("drawn");
    let at = point(line.left() + px(60.0), line.top() + px(6.0));
    cx.simulate_mouse_down(at, MouseButton::Right, Modifiers::none());
    cx.run_until_parked();
    let kept = editor.read_with(cx, |editor, _| editor.primary().range());
    assert_eq!(kept, 0..10);
    editor.update(cx, |editor, cx| editor.select(std::iter::once(10..10), cx));
    cx.simulate_mouse_down(at, MouseButton::Right, Modifiers::none());
    cx.run_until_parked();
    let moved = editor.read_with(cx, |editor, _| editor.primary().range());
    assert!(moved.is_empty() && moved.start < 10, "{moved:?}");
}

#[gpui::test]
fn delete_image_takes_the_picture_away_in_one_step(cx: &mut TestAppContext) {
    let source = "A ![p](p.png) B\n";
    let (document, cx) = open(source, MarkdownMode::Source, cx);
    let heard = super::format::refusals(&document, cx);
    document.update(cx, |document, cx| {
        document.delete_image(0, cx).expect("taken");
    });
    cx.run_until_parked();
    assert_eq!(*heard.borrow(), [EditRefusal::NoImage]);
    document.update(cx, |document, cx| {
        document.delete_image(4, cx).expect("taken");
    });
    cx.run_until_parked();
    assert_eq!(text(&document, cx), "A  B\n");
    let editor = document.read_with(cx, |document, _| document.editor().clone());
    editor.update(cx, |editor, cx| editor.undo(cx));
    assert_eq!(text(&document, cx), source);
}

#[gpui::test]
fn text_pastes_at_the_caret_with_no_block_open(cx: &mut TestAppContext) {
    let (document, cx) = open("First.\n", MarkdownMode::Visual, cx);
    press_on(0, 0, MouseButton::Right, cx);
    cx.write_to_clipboard(gpui::ClipboardItem::new_string("New ".into()));
    cx.simulate_keystrokes("secondary-v");
    assert_eq!(text(&document, cx), "New First.\n");
}

/// A right press on byte `at` of the open block.
fn press_open_at(document: &Entity<MarkdownDocument>, at: usize, cx: &mut VisualTestContext) {
    let spot = document.read_with(cx, |document, cx| {
        let input = document.open.as_ref().expect("open").input.read(cx);
        input.bounds_for_range(at..at + 1)[0].center()
    });
    cx.simulate_mouse_down(spot, MouseButton::Right, Modifiers::none());
    cx.simulate_mouse_up(spot, MouseButton::Right, Modifiers::none());
    cx.run_until_parked();
}

const TWO: &str = "[one](https://a.test) [two](https://b.test)\n";

#[gpui::test]
fn a_press_in_a_selection_reads_the_pressed_link(cx: &mut TestAppContext) {
    let (document, cx) = open(TWO, MarkdownMode::Visual, cx);
    let heard = heard(&document, cx);
    press_on(0, 1, MouseButton::Left, cx);
    select_open(&document, 0..43, cx);
    press_open_at(&document, TWO.find("two").expect("two") + 1, cx);
    assert_eq!(open_selection(&document, cx), Some(0..43), "kept");
    let link = heard
        .borrow()
        .first()
        .and_then(|target| target.link.clone());
    assert_eq!(link.as_deref(), Some("https://b.test"));
}

#[gpui::test]
fn only_the_latest_press_in_an_open_block_is_heard(cx: &mut TestAppContext) {
    let (document, cx) = open(TWO, MarkdownMode::Visual, cx);
    let heard = heard(&document, cx);
    press_on(0, 1, MouseButton::Left, cx);
    select_open(&document, 0..43, cx);
    let spots = document.read_with(cx, |document, cx| {
        let input = document.open.as_ref().expect("open").input.read(cx);
        ["one", "two"].map(|word| {
            let at = TWO.find(word).expect("word") + 1;
            input.bounds_for_range(at..at + 1)[0].center()
        })
    });
    document.update(cx, |document, cx| {
        document.point_open(Some(spots[0]), cx);
        document.point_open(Some(spots[1]), cx);
    });
    cx.run_until_parked();
    let links: Vec<_> = (heard.borrow().iter())
        .map(|target| target.link.clone())
        .collect();
    assert_eq!(links, [Some("https://b.test".to_string())]);
}

const PICTURES: &str = "x ![a](a.png) ![b](b.png)\n";

#[gpui::test]
fn delete_image_takes_the_pressed_picture(cx: &mut TestAppContext) {
    let (document, cx) = open(PICTURES, MarkdownMode::Visual, cx);
    let heard = heard(&document, cx);
    press_on(0, 0, MouseButton::Left, cx);
    select_open(&document, 0..25, cx);
    press_open_at(&document, PICTURES.find("b.png").expect("b"), cx);
    let target = heard.borrow().first().cloned().expect("heard");
    assert_eq!(target.image.as_deref(), Some("b.png"));
    document.update(cx, |document, cx| {
        document.delete_image(target.at, cx).expect("taken");
    });
    cx.run_until_parked();
    assert_eq!(opened(&document, cx), "x ![a](a.png) ");
}

const ANCHORED: &str = "[<a href=\"https://inner.test\">inside</a>](https://outer.test)\n";

#[gpui::test]
fn an_open_block_reads_an_anchor_as_drawn(cx: &mut TestAppContext) {
    let (document, cx) = open(ANCHORED, MarkdownMode::Visual, cx);
    let heard = heard(&document, cx);
    press_on(0, 1, MouseButton::Left, cx);
    select_open(&document, 0..0, cx);
    press_open_at(&document, ANCHORED.find("inside").expect("word") + 2, cx);
    let link = heard
        .borrow()
        .first()
        .and_then(|target| target.link.clone());
    assert_eq!(link.as_deref(), Some("https://inner.test"));
}
