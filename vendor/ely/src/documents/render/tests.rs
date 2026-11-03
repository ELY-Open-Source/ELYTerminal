use gpui::{Context, IntoElement, Render, TestAppContext, Window, div, prelude::*, px, rems};

use super::{HEAVY, MarkdownRenderer, Style, highlight};
use crate::theme::{ActiveTheme, TextSize, Theme};

/// `source` drawn in a 400-point column.
struct Page(&'static str);

impl Render for Page {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .w(px(400.0))
            .child(MarkdownRenderer::new("page", self.0))
    }
}

#[gpui::test]
fn a_rule_draws_a_line_two_pixels_high(cx: &mut TestAppContext) {
    cx.update(Theme::init);
    let (_, cx) = cx.add_window_view(|_, _| Page("above\n\n---\n\nbelow\n"));
    cx.run_until_parked();
    let rule = cx.debug_bounds("markdown-rule").expect("the rule is drawn");
    assert_eq!(rule.size.height, px(2.0), "{rule:?}");
    assert!(rule.size.width > px(300.0), "{rule:?}");
}

#[gpui::test]
fn bullets_turn_with_depth(cx: &mut TestAppContext) {
    cx.update(Theme::init);
    let (_, cx) = cx.add_window_view(|_, _| Page("- a\n  - b\n    1. c\n       - d\n"));
    cx.run_until_parked();
    for depth in [0, 1, 3] {
        let selector = Box::leak(format!("markdown-bullet-{depth}").into_boxed_str());
        assert!(cx.debug_bounds(selector).is_some(), "depth {depth}");
    }
    assert!(
        cx.debug_bounds("markdown-bullet-2").is_none(),
        "an ordered list"
    );
}

#[gpui::test]
fn alerts_lists_of_headings_and_front_matter_draw(cx: &mut TestAppContext) {
    cx.update(Theme::init);
    let source = "> [!IMPORTANT]\n> read\n\n[toc]\n\n# One\n\n## Two\n";
    let (_, cx) = cx.add_window_view(|_, _| Page(source));
    cx.run_until_parked();
    let (one, two) = (
        cx.debug_bounds("markdown-toc-0").expect("drawn"),
        cx.debug_bounds("markdown-toc-1").expect("drawn"),
    );
    assert!(two.top() > one.top(), "{one:?} {two:?}");
}

/// One heading of every level.
const HEADINGS: &str = "# a

## a

### a

#### a

##### a

###### a
";

/// `source` in a 400-point column, body at `size`.
struct Sized(&'static str, TextSize);

impl Render for Sized {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .w(px(400.0))
            .child(MarkdownRenderer::new("page", self.0).text_size(self.1))
    }
}

#[gpui::test]
fn headings_step_up_from_the_body_by_typoras_ratios(cx: &mut TestAppContext) {
    cx.update(Theme::init);
    for body in [TextSize::Base, TextSize::Lg] {
        let (_, cx) = cx.add_window_view(move |_, _| Sized(HEADINGS, body));
        cx.run_until_parked();
        let (base, pad) = cx.update(|window, cx| {
            let rem = window.rem_size();
            (
                cx.theme().text_size(body).to_pixels(rem),
                rems(0.5).to_pixels(rem),
            )
        });
        for (ordinal, step) in [2.25, 1.75, 1.5, 1.25, 1.0, 1.0].into_iter().enumerate() {
            let selector = Box::leak(format!("markdown-heading-{ordinal}").into_boxed_str());
            let high = cx.debug_bounds(selector).expect("drawn").size.height;
            let off = high - pad - base * step * 1.3;
            assert!(
                off.abs() < px(0.5),
                "{body:?} level {}: {high:?}",
                ordinal + 1
            );
        }
    }
}

#[gpui::test]
fn strong_text_in_a_heading_keeps_its_weight(cx: &mut TestAppContext) {
    cx.update(Theme::init);
    let (_, cx) = cx.add_window_view(|_, _| {
        Page(
            "### Plain **Strong** plain
",
        )
    });
    cx.run_until_parked();
    assert!(cx.debug_bounds("markdown-heading-0").is_some(), "drawn");
    let strong = cx.update(|_, cx| {
        let theme = cx.theme();
        highlight(&Style::Strong, &theme.colors, theme.underline_thickness()).font_weight
    });
    assert_eq!(strong, Some(HEAVY), "strong never lightens a heading");
}

#[gpui::test]
fn a_zoomed_document_scales_its_headings(cx: &mut TestAppContext) {
    use crate::documents::{DocumentLook, MarkdownDocument, MarkdownMode};
    cx.update(crate::init_for_tests);
    let (document, cx) = cx.add_window_view(|window, cx| {
        MarkdownDocument::new("### a\n", MarkdownMode::Preview, window, cx)
    });
    document.update(cx, |document, cx| {
        let look = DocumentLook {
            scale: 1.5,
            ..DocumentLook::default()
        };
        document.set_look(look, cx);
    });
    cx.run_until_parked();
    let (base, pad) = cx.update(|window, cx| {
        let rem = window.rem_size();
        (
            cx.theme().text_size(TextSize::Base).to_pixels(rem),
            rems(0.5).to_pixels(rem),
        )
    });
    let high = cx
        .debug_bounds("markdown-heading-0")
        .expect("drawn")
        .size
        .height;
    let off = high - pad - base * 1.5 * 1.5 * 1.3;
    assert!(off.abs() < px(0.5), "{high:?}");
}
