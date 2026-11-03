//! Presses on text the parser rewrote (R4.5).

use gpui::{MouseButton, TestAppContext, px};

use super::{
    open,
    point::{heard, opened_caret, press_at, press_lines, press_on, sizes, target},
};
use crate::documents::{MarkdownMode, TargetBlock};

#[gpui::test]
fn inline_code_kept_indent_press(cx: &mut TestAppContext) {
    let source = "a `fox\n  y` z";
    let (document, cx) = open(source, MarkdownMode::Visual, cx);
    let events = heard(&document, cx);
    press_on(0, 3, MouseButton::Right, cx);
    assert_eq!(*events.borrow(), [target(4, TargetBlock::Paragraph)]);
    press_on(0, 3, MouseButton::Left, cx);
    assert_eq!(opened_caret(&document, cx), 4);
}

#[gpui::test]
fn quoted_html_kept_mark_press(cx: &mut TestAppContext) {
    let source = "> <div>\n> > alpha beta</div>";
    let (document, cx) = open(source, MarkdownMode::Visual, cx);
    let events = heard(&document, cx);
    let bounds = cx.debug_bounds("markdown-block-0").expect("drawn");
    let (body, _) = sizes(cx);
    let x = bounds.left() + px(31.0) + body * 0.6 * 0.25;
    press_at(x, bounds.top() + px(10.0), MouseButton::Right, cx);
    assert_eq!(*events.borrow(), [target(10, TargetBlock::Quote)]);
}

#[gpui::test]
fn decoded_line_end_press(cx: &mut TestAppContext) {
    let source = "ab &#10; y";
    let (document, cx) = open(source, MarkdownMode::Visual, cx);
    let events = heard(&document, cx);
    press_on(0, 1, MouseButton::Right, cx);
    assert_eq!(*events.borrow(), [target(1, TargetBlock::Paragraph)]);
}

#[gpui::test]
fn numeric_control_press(cx: &mut TestAppContext) {
    let source = "ab &#128; y";
    let (document, cx) = open(source, MarkdownMode::Visual, cx);
    let events = heard(&document, cx);
    press_on(0, 1, MouseButton::Right, cx);
    assert_eq!(*events.borrow(), [target(1, TargetBlock::Paragraph)]);
}

#[gpui::test]
fn html_decoded_space_press(cx: &mut TestAppContext) {
    let source = "<div>ab&#32;</div>";
    let (document, cx) = open(source, MarkdownMode::Visual, cx);
    let events = heard(&document, cx);
    press_on(0, 1, MouseButton::Right, cx);
    assert_eq!(*events.borrow(), [target(6, TargetBlock::Html)]);
}

#[gpui::test]
fn inline_code_kept_indent_left_press(cx: &mut TestAppContext) {
    let source = "a `fox\n  y` z";
    let (document, cx) = open(source, MarkdownMode::Visual, cx);
    press_on(0, 3, MouseButton::Left, cx);
    assert_eq!(opened_caret(&document, cx), 4);
}

#[gpui::test]
fn autolink_literal_reference_press(cx: &mut TestAppContext) {
    let source = "<https://example.com/?a=&amp;b=2>";
    let (document, cx) = open(source, MarkdownMode::Visual, cx);
    let events = heard(&document, cx);
    press_on(0, 9, MouseButton::Right, cx);
    assert_eq!(events.borrow().len(), 1);
    assert_eq!(events.borrow()[0].at, source.find('x').unwrap());
}

#[gpui::test]
fn table_escaped_pipe_press(cx: &mut TestAppContext) {
    let source = "| `ab\\|cd` |\n| --- |\n| cell |\n";
    let (document, cx) = open(source, MarkdownMode::Visual, cx);
    let events = heard(&document, cx);
    let bounds = cx.debug_bounds("markdown-block-0").expect("drawn");
    let (body, _) = sizes(cx);
    let x = bounds.left() + px(26.0) + body * 0.6 * 1.25;
    press_at(x, bounds.top() + px(17.0), MouseButton::Right, cx);
    assert_eq!(events.borrow().len(), 1);
    assert!(events.borrow()[0].in_table);
    assert_eq!(events.borrow()[0].at, source.find('b').unwrap());
}

#[gpui::test]
fn prior_crlf_code_press(cx: &mut TestAppContext) {
    let source = "```\r\nalpha\r\nbeta\r\n```\r\n";
    let (document, cx) = open(source, MarkdownMode::Visual, cx);
    let events = heard(&document, cx);
    press_lines(2, 0, MouseButton::Right, cx);
    assert_eq!(events.borrow()[0].at, source.find("pha").unwrap());
    press_lines(2, 0, MouseButton::Left, cx);
    assert_eq!(opened_caret(&document, cx), source.find("pha").unwrap());
}

#[gpui::test]
fn prior_crlf_front_matter_press(cx: &mut TestAppContext) {
    let source = "---\r\ntitle: value\r\nauthor: person\r\n---\r\n";
    let (document, cx) = open(source, MarkdownMode::Visual, cx);
    let events = heard(&document, cx);
    press_lines(7, 0, MouseButton::Right, cx);
    assert_eq!(events.borrow()[0].at, source.find("value").unwrap());
    press_lines(7, 0, MouseButton::Left, cx);
    assert_eq!(opened_caret(&document, cx), source.find("value").unwrap());
}

#[gpui::test]
fn prior_crlf_definition_press(cx: &mut TestAppContext) {
    let source = "[ref]: https://ely.dev\r\n  \"title\"\r\n";
    let (document, cx) = open(source, MarkdownMode::Visual, cx);
    let events = heard(&document, cx);
    press_on(0, 10, MouseButton::Right, cx);
    assert_eq!(*events.borrow(), [target(10, TargetBlock::Definition)]);
}
