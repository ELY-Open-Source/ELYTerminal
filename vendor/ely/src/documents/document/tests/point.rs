//! A press lands on the pressed character (R4.5).

use std::{cell::RefCell, rc::Rc};

use gpui::{Entity, Modifiers, MouseButton, Pixels, TestAppContext, VisualTestContext, point, px};

use super::{open, type_in};
use crate::{
    documents::{DocumentEvent, DocumentTarget, MarkdownDocument, MarkdownMode, TargetBlock},
    theme::{ActiveTheme, TextSize},
};

/// Presses block `ix` on its first line's character `k`.
pub(super) fn press_on(ix: usize, k: usize, button: MouseButton, cx: &mut VisualTestContext) {
    let bounds = cx
        .debug_bounds(Box::leak(format!("markdown-block-{ix}").into_boxed_str()))
        .expect("the block is drawn");
    // The test text system sets glyphs 0.6 em apart.
    let size = cx.update(|window, cx| {
        cx.theme()
            .text_size(TextSize::Base)
            .to_pixels(window.rem_size())
    });
    let em = size * 0.6;
    let x = bounds.left() + px(13.0) + em * (k as f32 + 0.25);
    let at = point(x, bounds.top() + px(10.0));
    cx.simulate_mouse_down(at, button, Modifiers::none());
    cx.simulate_mouse_up(at, button, Modifiers::none());
    cx.run_until_parked();
}

pub(super) fn heard(
    document: &Entity<MarkdownDocument>,
    cx: &mut VisualTestContext,
) -> Rc<RefCell<Vec<DocumentTarget>>> {
    let heard = Rc::new(RefCell::new(Vec::new()));
    let kept = heard.clone();
    cx.update(|_, cx| {
        cx.subscribe(document, move |_, event: &DocumentEvent, _| {
            if let DocumentEvent::Pointed(target) = event {
                kept.borrow_mut().push(target.clone());
            }
        })
        .detach()
    });
    heard
}

fn caret(document: &Entity<MarkdownDocument>, cx: &mut VisualTestContext) -> usize {
    document.read_with(cx, |document, cx| document.editor.read(cx).primary().head)
}

pub(super) fn opened_caret(
    document: &Entity<MarkdownDocument>,
    cx: &mut VisualTestContext,
) -> usize {
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

pub(super) fn target(at: usize, block: TargetBlock) -> DocumentTarget {
    DocumentTarget {
        at,
        block: Some(block),
        link: None,
        image: None,
        in_table: false,
        code: None,
    }
}

const TEXT: &str = "First.\n\nSay **bold** and more.\n";

#[gpui::test]
fn a_press_opens_the_block_at_the_pressed_character(cx: &mut TestAppContext) {
    let (document, cx) = open(TEXT, MarkdownMode::Visual, cx);
    press_on(1, 4, MouseButton::Left, cx);
    assert_eq!(
        opened_caret(&document, cx),
        "Say **".len(),
        "on the b of bold"
    );
}

#[gpui::test]
fn a_press_past_a_crlf_line_types_before_its_end(cx: &mut TestAppContext) {
    let source = "one\r\ntwo\r\n";
    let (document, cx) = open(source, MarkdownMode::Visual, cx);
    press_on(0, 3, MouseButton::Left, cx);
    assert_eq!(opened_caret(&document, cx), "one".len());
    cx.simulate_input("X");
    cx.simulate_keystrokes("escape");
    assert_eq!(super::text(&document, cx), "oneX\r\ntwo\r\n");
}

#[gpui::test]
fn a_press_in_code_keeping_a_crlf_lands_on_its_character(cx: &mut TestAppContext) {
    let source = "`one\r\ntwo`\n";
    let (document, cx) = open(source, MarkdownMode::Visual, cx);
    press_on(0, 2, MouseButton::Left, cx);
    assert_eq!(opened_caret(&document, cx), "`on".len());
}

#[gpui::test]
fn a_secondary_press_moves_the_caret_and_opens_nothing(cx: &mut TestAppContext) {
    let (document, cx) = open(TEXT, MarkdownMode::Visual, cx);
    let heard = heard(&document, cx);
    press_on(1, 4, MouseButton::Right, cx);
    assert!(document.read_with(cx, |document, _| document.open.is_none()));
    let at = TEXT.find("bold").unwrap();
    assert_eq!(caret(&document, cx), at);
    assert_eq!(*heard.borrow(), [target(at, TargetBlock::Paragraph)]);
}

#[gpui::test]
fn a_secondary_press_on_a_link_names_its_address(cx: &mut TestAppContext) {
    let text = "Go [there](https://ely.dev) now\n";
    let (document, cx) = open(text, MarkdownMode::Visual, cx);
    let heard = heard(&document, cx);
    press_on(0, 4, MouseButton::Right, cx);
    let expected = DocumentTarget {
        link: Some("https://ely.dev".into()),
        ..target(text.find("here").unwrap(), TargetBlock::Paragraph)
    };
    assert_eq!(*heard.borrow(), [expected]);
}

#[gpui::test]
fn a_code_press_lands_in_its_code(cx: &mut TestAppContext) {
    let text = "```rust\nlet x = 1;\n```\n";
    let (document, cx) = open(text, MarkdownMode::Visual, cx);
    let heard = heard(&document, cx);
    press_on(0, 0, MouseButton::Right, cx);
    let expected = DocumentTarget {
        code: Some("let x = 1;".into()),
        ..target("```rust\n".len(), TargetBlock::Code)
    };
    assert_eq!(*heard.borrow(), [expected]);
}

#[gpui::test]
fn a_secondary_press_on_a_picture_names_it(cx: &mut TestAppContext) {
    let text = "Lead.\n\n![pic](missing.png)\n";
    let (document, cx) = open(text, MarkdownMode::Visual, cx);
    let heard = heard(&document, cx);
    press_on(1, 1, MouseButton::Right, cx);
    let expected = DocumentTarget {
        image: Some("missing.png".into()),
        ..target(text.find("![").unwrap(), TargetBlock::Paragraph)
    };
    assert_eq!(*heard.borrow(), [expected]);
}

#[gpui::test]
fn a_secondary_press_writes_the_open_block_first(cx: &mut TestAppContext) {
    let (document, cx) = open(TEXT, MarkdownMode::Visual, cx);
    let heard = heard(&document, cx);
    type_in(&document, Some(0), Some("First, longer."), cx);
    press_on(1, 4, MouseButton::Right, cx);
    let written = "First, longer.\n\nSay **bold** and more.\n";
    assert_eq!(super::text(&document, cx), written);
    let at = written.find("bold").unwrap();
    assert_eq!(caret(&document, cx), at, "the byte moved with the write");
    assert_eq!(*heard.borrow(), [target(at, TargetBlock::Paragraph)]);
}

#[gpui::test]
fn a_secondary_press_past_the_end_goes_to_the_end(cx: &mut TestAppContext) {
    let (document, cx) = open(TEXT, MarkdownMode::Visual, cx);
    let heard = heard(&document, cx);
    let end = cx.debug_bounds("markdown-block-1").expect("drawn").bottom() + px(12.0);
    cx.simulate_mouse_down(point(px(40.0), end), MouseButton::Right, Modifiers::none());
    cx.run_until_parked();
    let expected = DocumentTarget {
        block: None,
        ..target(TEXT.len(), TargetBlock::Paragraph)
    };
    assert_eq!(*heard.borrow(), [expected]);
    assert!(document.read_with(cx, |document, _| document.open.is_none()));
}

/// Base and small text sizes, in pixels.
pub(super) fn sizes(cx: &mut VisualTestContext) -> (Pixels, Pixels) {
    cx.update(|window, cx| {
        let size = |size| cx.theme().text_size(size).to_pixels(window.rem_size());
        (size(TextSize::Base), size(TextSize::Sm))
    })
}

pub(super) fn press_at(x: Pixels, y: Pixels, button: MouseButton, cx: &mut VisualTestContext) {
    cx.simulate_mouse_down(point(x, y), button, Modifiers::none());
    cx.simulate_mouse_up(point(x, y), button, Modifiers::none());
    cx.run_until_parked();
}

/// Presses block 0's code glyph `k` on `line`.
pub(super) fn press_lines(k: usize, line: usize, button: MouseButton, cx: &mut VisualTestContext) {
    let bounds = cx.debug_bounds("markdown-block-0").expect("drawn");
    let (body, small) = sizes(cx);
    let x = bounds.left() + px(25.0) + small * 0.6 * (k as f32 + 0.25);
    let y = bounds.top() + px(17.0) + body * 1.6 * (line as f32 + 0.25);
    press_at(x, y, button, cx);
}

#[gpui::test]
fn a_secondary_press_in_a_quote_lands_past_its_mark(cx: &mut TestAppContext) {
    let source = "> alpha\n> beta";
    let (document, cx) = open(source, MarkdownMode::Visual, cx);
    let events = heard(&document, cx);
    let bounds = cx.debug_bounds("markdown-block-0").expect("drawn");
    let (body, _) = sizes(cx);
    // "beta" draws on its own line.
    let x = bounds.left() + px(31.0) + body * 0.6 * 1.25;
    press_at(
        x,
        bounds.top() + px(10.0) + body * 1.6,
        MouseButton::Right,
        cx,
    );
    let at = source.find("eta").unwrap();
    assert_eq!(*events.borrow(), [target(at, TargetBlock::Quote)]);
}

#[gpui::test]
fn a_secondary_press_in_an_alert_lands_past_its_marker(cx: &mut TestAppContext) {
    let source = "> [!NOTE]\n> alpha beta";
    let (document, cx) = open(source, MarkdownMode::Visual, cx);
    let events = heard(&document, cx);
    let bounds = cx.debug_bounds("markdown-block-0").expect("drawn");
    let (body, _) = sizes(cx);
    let x = bounds.left() + px(31.0) + body * 0.6 * 6.25;
    let y = bounds.top() + px(15.0) + body * 1.6;
    press_at(x, y, MouseButton::Right, cx);
    let at = source.find("beta").unwrap();
    assert_eq!(*events.borrow(), [target(at, TargetBlock::Alert)]);
}

#[gpui::test]
fn an_html_block_press_lands_on_its_character(cx: &mut TestAppContext) {
    let source = "<div>alpha beta</div>\n";
    let (document, cx) = open(source, MarkdownMode::Visual, cx);
    let events = heard(&document, cx);
    press_on(0, 7, MouseButton::Right, cx);
    let at = source.find("eta").unwrap();
    assert_eq!(*events.borrow(), [target(at, TargetBlock::Html)]);
    press_on(0, 7, MouseButton::Left, cx);
    assert_eq!(opened_caret(&document, cx), at);
}

#[gpui::test]
fn a_definition_press_lands_on_its_character(cx: &mut TestAppContext) {
    let source = "[ref]: https://ely.dev\n";
    let (document, cx) = open(source, MarkdownMode::Visual, cx);
    let events = heard(&document, cx);
    press_on(0, 10, MouseButton::Right, cx);
    assert_eq!(*events.borrow(), [target(10, TargetBlock::Definition)]);
}

#[gpui::test]
fn code_presses_land_line_for_line(cx: &mut TestAppContext) {
    for source in ["    alpha\n    beta\n", "```\r\nalpha\r\nbeta\r\n```\r\n"] {
        let (document, cx) = open(source, MarkdownMode::Visual, cx);
        let events = heard(&document, cx);
        press_lines(1, 1, MouseButton::Right, cx);
        let at = source.find("eta").unwrap();
        let eol = if source.contains('\r') { "\r\n" } else { "\n" };
        let expected = DocumentTarget {
            code: Some(format!("alpha{eol}beta")),
            ..target(at, TargetBlock::Code)
        };
        assert_eq!(*events.borrow(), [expected], "{source:?}");
        press_lines(1, 1, MouseButton::Left, cx);
        assert_eq!(opened_caret(&document, cx), at, "{source:?}");
    }
}

#[gpui::test]
fn a_front_matter_press_lands_past_its_fence(cx: &mut TestAppContext) {
    let source = "---\ntitle: value\n---\n";
    let (document, cx) = open(source, MarkdownMode::Visual, cx);
    let events = heard(&document, cx);
    press_lines(7, 0, MouseButton::Right, cx);
    let at = source.find("value").unwrap();
    assert_eq!(*events.borrow(), [target(at, TargetBlock::FrontMatter)]);
    press_lines(7, 0, MouseButton::Left, cx);
    assert_eq!(opened_caret(&document, cx), at);
}

#[gpui::test]
fn a_secondary_press_on_an_empty_document_opens_nothing(cx: &mut TestAppContext) {
    let (document, cx) = open("", MarkdownMode::Visual, cx);
    assert!(document.read_with(cx, |document, _| document.open.is_none()));
    let events = heard(&document, cx);
    press_at(px(25.0), px(25.0), MouseButton::Right, cx);
    assert_eq!(events.borrow().len(), 1);
    assert!(document.read_with(cx, |document, _| document.open.is_none()));
}

#[gpui::test]
fn a_write_back_that_changes_the_pressed_block_points_nowhere(cx: &mut TestAppContext) {
    let (document, cx) = open("First.\n\nalpha beta\n", MarkdownMode::Visual, cx);
    let events = heard(&document, cx);
    type_in(&document, Some(0), Some("```"), cx);
    press_on(1, 7, MouseButton::Right, cx);
    let written = "```\n\nalpha beta\n";
    assert_eq!(super::text(&document, cx), written);
    assert!(
        events.borrow().is_empty(),
        "the pressed paragraph became code"
    );
    assert_ne!(caret(&document, cx), written.find("eta").unwrap());
}

#[gpui::test]
fn crlf_lines_press_where_they_were_written(cx: &mut TestAppContext) {
    let source = "```
alpha
beta
```
";
    let (document, cx) = open(source, MarkdownMode::Visual, cx);
    let events = heard(&document, cx);
    press_lines(2, 0, MouseButton::Right, cx);
    let at = source.find("pha").unwrap();
    let expected = DocumentTarget {
        code: Some("alpha\nbeta".into()),
        ..target(at, TargetBlock::Code)
    };
    assert_eq!(*events.borrow(), [expected]);
    press_lines(2, 0, MouseButton::Left, cx);
    assert_eq!(opened_caret(&document, cx), at);
}

#[gpui::test]
fn crlf_front_matter_presses_where_written(cx: &mut TestAppContext) {
    let source = "---
title: value
author: person
---
";
    let (document, cx) = open(source, MarkdownMode::Visual, cx);
    let events = heard(&document, cx);
    press_lines(7, 0, MouseButton::Right, cx);
    let at = source.find("value").unwrap();
    assert_eq!(*events.borrow(), [target(at, TargetBlock::FrontMatter)]);
    press_lines(7, 0, MouseButton::Left, cx);
    assert_eq!(opened_caret(&document, cx), at);
}

#[gpui::test]
fn a_crlf_definition_presses_where_written(cx: &mut TestAppContext) {
    let source = "[ref]: https://ely.dev
  \"title\"
";
    let (document, cx) = open(source, MarkdownMode::Visual, cx);
    let events = heard(&document, cx);
    press_on(0, 10, MouseButton::Right, cx);
    assert_eq!(*events.borrow(), [target(10, TargetBlock::Definition)]);
}

#[gpui::test]
fn a_changed_definition_points_nowhere(cx: &mut TestAppContext) {
    let source = "[ref]: https://old.dev

Go [there][ref]
";
    let (document, cx) = open(source, MarkdownMode::Visual, cx);
    let events = heard(&document, cx);
    type_in(&document, Some(0), Some("[ref]: https://new.dev"), cx);
    press_on(1, 4, MouseButton::Right, cx);
    assert_eq!(
        super::text(&document, cx),
        source.replace("old.dev", "new.dev")
    );
    assert!(
        events.borrow().is_empty(),
        "the link changed under the press"
    );
}

#[gpui::test]
fn a_quoted_greater_than_presses_on_itself(cx: &mut TestAppContext) {
    let source = "> <div>
> >alpha beta</div>";
    let (document, cx) = open(source, MarkdownMode::Visual, cx);
    let events = heard(&document, cx);
    let bounds = cx.debug_bounds("markdown-block-0").expect("drawn");
    let (body, _) = sizes(cx);
    let x = bounds.left() + px(31.0) + body * 0.6 * 0.25;
    press_at(x, bounds.top() + px(10.0), MouseButton::Right, cx);
    let at = source.find(">alpha").unwrap();
    assert_eq!(*events.borrow(), [target(at, TargetBlock::Quote)]);
}

#[gpui::test]
fn an_escaped_backslash_presses_on_itself(cx: &mut TestAppContext) {
    let source = r"a \\ z";
    let (document, cx) = open(source, MarkdownMode::Visual, cx);
    let events = heard(&document, cx);
    press_on(0, 2, MouseButton::Right, cx);
    let at = source.find(r"\ z").unwrap();
    assert_eq!(*events.borrow(), [target(at, TargetBlock::Paragraph)]);
}

#[gpui::test]
fn an_html_space_moved_by_a_tag_presses_on_its_word(cx: &mut TestAppContext) {
    let source = "<div>alpha <b> beta</b></div>";
    let (document, cx) = open(source, MarkdownMode::Visual, cx);
    let events = heard(&document, cx);
    press_on(0, 8, MouseButton::Right, cx);
    let at = source.find("eta").unwrap();
    assert_eq!(*events.borrow(), [target(at, TargetBlock::Html)]);
}

#[gpui::test]
fn a_press_beside_the_code_still_names_it(cx: &mut TestAppContext) {
    let source = "```\nlet x;\n```\n";
    let (document, cx) = open(source, MarkdownMode::Visual, cx);
    let events = heard(&document, cx);
    let bounds = cx.debug_bounds("markdown-block-0").expect("drawn");
    press_at(
        bounds.left() + px(4.0),
        bounds.top() + px(4.0),
        MouseButton::Right,
        cx,
    );
    let code = events
        .borrow()
        .first()
        .and_then(|target| target.code.clone());
    assert_eq!(code.as_deref(), Some("let x;"));
}
