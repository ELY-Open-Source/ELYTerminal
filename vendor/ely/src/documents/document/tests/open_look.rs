//! An open block keeps its place and size (R7.5).

use gpui::{
    Bounds, Fill, Modifiers, Pixels, StyleRefinement, Styled, TestAppContext, VisualTestContext,
    rems,
};

use super::{open, press};
use crate::documents::{TargetBlock, document::visual::open_text};
use crate::{
    documents::{MarkdownMode, markdown_highlights, render::HEAVY},
    forms::Highlight,
    theme::ActiveTheme,
};

/// Block 0's bounds closed, then open.
fn closed_then_open<'a>(
    source: &str,
    cx: &'a mut TestAppContext,
) -> (Bounds<Pixels>, Bounds<Pixels>, &'a mut VisualTestContext) {
    let (_, cx) = open(source, MarkdownMode::Visual, cx);
    let closed = cx.debug_bounds("markdown-block-0").expect("drawn");
    press(0, Modifiers::none(), cx);
    let open = cx.debug_bounds("markdown-open").expect("the block opens");
    (closed, open, cx)
}

/// Within a pixel: a field rounds its line height.
fn near(a: Pixels, b: Pixels) -> bool {
    (a - b).abs() < gpui::px(1.0)
}

#[gpui::test]
fn an_open_paragraph_keeps_its_place_and_height(cx: &mut TestAppContext) {
    let (closed, open, _) = closed_then_open("Plain words here.\n", cx);
    assert!(
        near(closed.top(), open.top()) && near(closed.left(), open.left()),
        "{closed:?} {open:?}"
    );
    assert!(
        near(closed.size.height, open.size.height),
        "{closed:?} {open:?}"
    );
}

#[gpui::test]
fn an_open_heading_keeps_its_size(cx: &mut TestAppContext) {
    let (closed, open, _) = closed_then_open("## Plain heading\n", cx);
    assert!(
        near(closed.top(), open.top()) && near(closed.left(), open.left()),
        "{closed:?} {open:?}"
    );
    assert!(
        near(closed.size.height, open.size.height),
        "{closed:?} {open:?}"
    );
}

#[gpui::test]
fn an_open_code_block_keeps_its_panel(cx: &mut TestAppContext) {
    keeps_its_panel("```\nlet x = 1;\n```\n", cx);
}

#[gpui::test]
fn open_front_matter_keeps_its_panel(cx: &mut TestAppContext) {
    keeps_its_panel("---\nkey: value\n---\n\nText.\n", cx);
}

/// Block 0 opens with its panel at the closed inset.
fn keeps_its_panel(source: &str, cx: &mut TestAppContext) {
    let (closed, open, cx) = closed_then_open(source, cx);
    assert!(
        near(closed.top(), open.top()) && near(closed.left(), open.left()),
        "{closed:?} {open:?}"
    );
    // The panel sits inside the frame's border and padding.
    let (x, y) = cx.update(|window, _| {
        let rem = window.rem_size();
        (rems(0.75).to_pixels(rem), rems(0.25).to_pixels(rem))
    });
    let panel = cx.debug_bounds("markdown-open-text").expect("drawn");
    let border = gpui::px(1.0);
    assert!(
        near(panel.left(), closed.left() + border + x),
        "{panel:?} {closed:?}"
    );
    assert!(
        near(panel.top(), closed.top() + border + y),
        "{panel:?} {closed:?}"
    );
}

/// The highlight on the run that holds `word`.
fn run(source: &str, word: &str, cx: &mut TestAppContext) -> Highlight {
    cx.update(crate::init_for_tests);
    let at = source.find(word).expect("the word");
    cx.update(|cx| {
        markdown_highlights(source, cx)
            .into_iter()
            .rfind(|(range, _)| range.start <= at && at < range.end)
            .expect("a run")
            .1
    })
}

#[gpui::test]
fn an_open_heading_keeps_its_weight(cx: &mut TestAppContext) {
    for source in [
        "# Heavy words
",
        "a **Heavy** b
",
    ] {
        assert_eq!(run(source, "Heavy", cx).weight, Some(HEAVY), "{source}");
    }
}

#[gpui::test]
fn an_open_sixth_heading_stays_muted(cx: &mut TestAppContext) {
    let muted = cx.update(|cx| {
        crate::init_for_tests(cx);
        cx.theme().colors.fg_muted
    });
    for word in ["Quiet", "Loud"] {
        let source = "###### Quiet **Loud**
";
        assert_eq!(run(source, word, cx).color, muted, "{word}");
    }
    let fg = cx.update(|cx| cx.theme().colors.fg);
    assert_eq!(
        run(
            "##### Plain
",
            "Plain",
            cx
        )
        .color,
        fg
    );
    let code = run("###### Quiet `Code`\n", "Code", cx);
    assert_eq!(code.color, fg, "code keeps its own color");
}

/// The open text box's style for `kind`.
fn styled(kind: TargetBlock, cx: &mut TestAppContext) -> StyleRefinement {
    cx.update(crate::init_for_tests);
    cx.update(|cx| {
        open_text(Some((kind, true)), 1.0, cx.theme())
            .style()
            .clone()
    })
}

#[gpui::test]
fn open_prose_has_no_panel(cx: &mut TestAppContext) {
    for kind in [
        TargetBlock::Paragraph,
        TargetBlock::Heading(2),
        TargetBlock::List,
        TargetBlock::Quote,
    ] {
        assert_eq!(styled(kind, cx).background, None, "{kind:?}");
    }
}

#[gpui::test]
fn open_code_math_and_front_matter_keep_their_panel(cx: &mut TestAppContext) {
    let (sunken, muted, mono) = cx.update(|cx| {
        crate::init_for_tests(cx);
        let theme = cx.theme();
        (
            theme.colors.sunken,
            theme.colors.fg_muted,
            theme.mono_family.clone(),
        )
    });
    for kind in [
        TargetBlock::Code,
        TargetBlock::Math,
        TargetBlock::FrontMatter,
    ] {
        assert_eq!(
            styled(kind, cx).background,
            Some(Fill::from(sunken)),
            "{kind:?}"
        );
    }
    let front = styled(TargetBlock::FrontMatter, cx).text;
    assert_eq!((front.color, front.font_family), (Some(muted), Some(mono)));
}

#[gpui::test]
fn open_front_matter_has_no_runs(cx: &mut TestAppContext) {
    cx.update(crate::init_for_tests);
    let runs = cx.update(|cx| markdown_highlights("---\ntitle: R75\n---", cx));
    assert!(runs.is_empty(), "{runs:?}");
}
