//! Inline formats toggle as Typora's (R4.6).

use std::{cell::RefCell, rc::Rc};

use gpui::{Entity, Modifiers, TestAppContext, VisualTestContext};
use markdown::mdast::Node;
use web_time::Instant;

use super::{open, opened, press, text};
use crate::documents::{
    DocumentEvent, EditRefusal, InlineFormat, LARGEST_READ, MarkdownDocument, MarkdownMode,
    document::{
        format::{toggle, toggle_tree},
        visual,
    },
    markdown::file_tree,
};

/// `marked` toggled: `‸` is the caret, `⟨…⟩` the selection.
fn toggled(marked: &str, format: InlineFormat) -> Result<String, EditRefusal> {
    let caret = marked.find('‸');
    let text: String = marked.chars().filter(|ch| !"‸⟨⟩".contains(*ch)).collect();
    let selection = match caret {
        Some(at) => at..at,
        None => {
            let start = marked.find('⟨').expect("a selection");
            start..marked.find('⟩').expect("its end") - '⟨'.len_utf8()
        }
    };
    let (edits, selected) = toggle(&text, selection, format)?;
    let mut out = text.clone();
    for (range, with) in edits.into_iter().rev() {
        out.replace_range(range, &with);
    }
    Ok(match selected.is_empty() {
        true => format!("{}‸{}", &out[..selected.start], &out[selected.start..]),
        false => format!(
            "{}⟨{}⟩{}",
            &out[..selected.start],
            &out[selected.clone()],
            &out[selected.end..]
        ),
    })
}

fn ok(marked: &str, format: InlineFormat) -> String {
    toggled(marked, format).expect("toggled")
}

#[test]
fn a_selection_wraps_and_a_styled_one_unwraps() {
    use InlineFormat::*;
    assert_eq!(ok("a ⟨bold⟩ z", Strong), "a **⟨bold⟩** z");
    assert_eq!(
        ok("a ⟨word ⟩z", Strong),
        "a **⟨word⟩** z",
        "end space stays out"
    );
    assert_eq!(ok("a **⟨bold⟩** z", Strong), "a ⟨bold⟩ z");
    assert_eq!(ok("⟨a⟩", Emphasis), "*⟨a⟩*");
    assert_eq!(ok("⟨a⟩", Strike), "~~⟨a⟩~~");
    assert_eq!(ok("⟨a⟩", Underline), "<u>⟨a⟩</u>");
    assert_eq!(ok("x ⟨a`b⟩ y", Code), "x ``⟨a`b⟩`` y");
    assert_eq!(ok("⟨site⟩", Link), "[site](‸)");
    assert_eq!(ok("⟨cat⟩", Image), "![cat](‸)");
}

#[test]
fn a_caret_takes_its_word_or_an_empty_pair() {
    use InlineFormat::*;
    assert_eq!(ok("a wo‸rd z", Strong), "a **⟨word⟩** z");
    assert_eq!(ok("a ‸ z", Strong), "a **‸** z");
    assert_eq!(ok("a\n\n‸", Emphasis), "a\n\n*‸*", "a blank line");
    assert_eq!(ok("a ‸ z", Link), "a [‸]() z");
    assert_eq!(
        ok("x　wo‸rd", Strong),
        "x　**⟨word⟩**",
        "a wide space stops it"
    );
}

#[test]
fn a_caret_in_a_style_takes_it_off() {
    use InlineFormat::*;
    assert_eq!(ok("a **b‸old** z", Strong), "a b‸old z");
    assert_eq!(ok("~~a‸~~", Strike), "a‸");
    assert_eq!(ok("x `co‸de` y", Code), "x co‸de y");
    assert_eq!(ok("<u>a‸b</u>", Underline), "a‸b");
    assert_eq!(ok("[si‸te](https://x)", Link), "si‸te");
    assert_eq!(ok("![c‸at](x.png)", Image), "c‸at");
}

#[test]
fn a_partly_bold_selection_takes_the_bold_in() {
    assert_eq!(ok("a **b** ⟨c⟩", InlineFormat::Strong), "a **b** **⟨c⟩**");
    assert_eq!(ok("a **⟨b** c⟩", InlineFormat::Strong), "a **⟨b c⟩**");
}

#[test]
fn clear_takes_every_mark_off_the_touched_blocks() {
    let cleared = ok(
        "**a** ‸`b` [c](u) <u>d</u> ![e](f.png)",
        InlineFormat::Clear,
    );
    assert_eq!(cleared, "a ‸b c d e");
    assert_eq!(ok("**a‸**\n\n*b*", InlineFormat::Clear), "a‸\n\n*b*");
    assert_eq!(ok("⟨**a**\n\n*b*⟩", InlineFormat::Clear), "⟨a\n\nb⟩");
}

#[test]
fn code_math_html_and_front_matter_refuse() {
    use EditRefusal as Why;
    use InlineFormat::*;
    assert_eq!(toggled("```\nco‸de\n```", Strong), Err(Why::Code));
    assert_eq!(toggled("x `co‸de` y", Strong), Err(Why::Code));
    assert_eq!(toggled("$$\nx‸\n$$", Strong), Err(Why::Math));
    assert_eq!(toggled("a $x‸$ b", Emphasis), Err(Why::Math));
    assert_eq!(toggled("<div>\nx‸\n</div>", Strong), Err(Why::Html));
    assert_eq!(
        toggled("---\nti‸tle: a\n---\n\nbody", Strong),
        Err(Why::FrontMatter)
    );
    assert_eq!(toggled("```\nco‸de\n```", Clear), Err(Why::Code));
}

#[test]
fn a_selection_across_a_link_or_into_its_address_refuses() {
    use InlineFormat::*;
    assert_eq!(toggled("[a ⟨b](u) c⟩", Strong), Err(EditRefusal::Across));
    assert_eq!(toggled("[a](ht‸tp://x)", Strong), Err(EditRefusal::Across));
    assert_eq!(toggled("a\n\n> ‸", Strong), Err(EditRefusal::Outside));
}

#[gpui::test]
fn an_open_block_stays_open_and_undoes_as_one_step(cx: &mut TestAppContext) {
    let (document, cx) = open("alpha beta\n", MarkdownMode::Visual, cx);
    press(0, Modifiers::none(), cx);
    document.update(cx, |document, cx| {
        let open = document.open.as_ref().expect("opened");
        open.input.update(cx, |input, cx| input.select(6..10, cx));
        document
            .format(InlineFormat::Strong, cx)
            .expect("formatted");
    });
    cx.run_until_parked();
    assert_eq!(
        opened(&document, cx),
        "alpha **beta**",
        "still open to type in"
    );
    document.update(cx, |document, cx| {
        let open = document.open.as_ref().expect("opened");
        assert_eq!(
            open.input.read(cx).selection(),
            8..12,
            "the word stays picked"
        );
    });
    cx.dispatch_action(visual::Undo);
    cx.run_until_parked();
    assert_eq!(text(&document, cx), "alpha beta\n");
}

#[gpui::test]
fn the_source_takes_a_format_as_one_step(cx: &mut TestAppContext) {
    let (document, cx) = open("alpha beta\n", MarkdownMode::Source, cx);
    document.update(cx, |document, cx| {
        document
            .editor
            .update(cx, |editor, cx| editor.select(std::iter::once(2..2), cx));
        document.format(InlineFormat::Code, cx).expect("formatted");
    });
    cx.run_until_parked();
    assert_eq!(text(&document, cx), "`alpha` beta\n");
    document.update(cx, |document, cx| {
        document.editor.update(cx, |editor, cx| editor.undo(cx));
    });
    assert_eq!(text(&document, cx), "alpha beta\n");
}

#[gpui::test]
fn preview_refuses_a_format(cx: &mut TestAppContext) {
    let (document, cx) = open("alpha\n", MarkdownMode::Preview, cx);
    let refused = document.update(cx, |document, cx| document.format(InlineFormat::Strong, cx));
    assert_eq!(refused, Err(EditRefusal::Preview));
    assert_eq!(text(&document, cx), "alpha\n");
}

#[test]
fn an_end_in_marks_or_an_address_refuses() {
    use InlineFormat::*;
    let across = Err(EditRefusal::Across);
    assert_eq!(toggled("a *‸*bold** z", Strong), across);
    assert_eq!(toggled("[a](ht‸tps://example.com)", Link), across);
    assert_eq!(toggled("a *⟨*b** c⟩", Strong), across);
    assert_eq!(toggled("*a⟨b*⟩", Strong), across);
    assert_eq!(toggled("⟨alpha\n\nbeta⟩", Strong), across);
}

#[test]
fn end_spaces_cells_and_cr_lines_take_a_pair() {
    use InlineFormat::*;
    assert_eq!(ok("alpha ‸", Strong), "alpha **‸**");
    assert_eq!(ok("| h |\n| - |\n| ‸ |", Strong), "| h |\n| - |\n| **‸** |");
    assert_eq!(ok("a\r‸\rb", Strong), "a\r**‸**\rb");
}

#[test]
fn clear_takes_math_footnote_and_escape_marks() {
    use InlineFormat::Clear;
    assert_eq!(ok("**a** $x‸+y$", Clear), "a x‸+y");
    assert_eq!(ok("a‸ [^n]\n\n[^n]: note", Clear), "a‸ n\n\n[^n]: note");
    assert_eq!(ok(r"a‸ \*b\*", Clear), "a‸ *b*");
}

#[test]
fn underline_comes_off_past_other_tags() {
    use InlineFormat::Underline;
    assert_eq!(ok("<u>a <br>b‸c</u>", Underline), "a <br>b‸c");
    assert_eq!(ok("<u>a <em>b‸c</em> d</u>", Underline), "a <em>b‸c</em> d");
}

/// A source document of one word, caret at 1.
fn word(cx: &mut TestAppContext, len: usize) -> (Entity<MarkdownDocument>, &mut VisualTestContext) {
    let (document, cx) = open(&"x".repeat(len), MarkdownMode::Source, cx);
    document.update(cx, |document, cx| {
        document
            .editor
            .update(cx, |editor, cx| editor.select(std::iter::once(1..1), cx));
    });
    (document, cx)
}

#[gpui::test]
fn a_format_reads_apart(cx: &mut TestAppContext) {
    let (document, cx) = word(cx, 64);
    document.update(cx, |document, cx| {
        document.format(InlineFormat::Strong, cx).expect("taken")
    });
    assert!(
        !text(&document, cx).starts_with("**"),
        "not on the UI thread"
    );
    cx.run_until_parked();
    assert!(text(&document, cx).starts_with("**x"));
}

#[gpui::test]
fn many_short_paragraphs_format_apart(cx: &mut TestAppContext) {
    let source = "a\n\n".repeat(100);
    let (document, cx) = open(&source, MarkdownMode::Source, cx);
    document.update(cx, |document, cx| {
        document
            .editor
            .update(cx, |editor, cx| editor.select(std::iter::once(0..1), cx));
        document.format(InlineFormat::Strong, cx).expect("taken");
    });
    assert_eq!(text(&document, cx), source, "not on the UI thread");
    cx.run_until_parked();
    assert!(text(&document, cx).starts_with("**a**\n\na"));
}

#[gpui::test]
fn a_format_lapses_when_the_text_moves(cx: &mut TestAppContext) {
    let (document, cx) = word(cx, 64);
    document.update(cx, |document, cx| {
        document.format(InlineFormat::Strong, cx).expect("taken");
        document
            .editor
            .update(cx, |editor, cx| editor.edit([(0..0, "y".to_string())], cx));
    });
    cx.run_until_parked();
    assert!(
        text(&document, cx).starts_with("yx"),
        "the typing stays, unformatted"
    );
}

/// The refusals a document sends by event.
pub(super) fn refusals(
    document: &Entity<MarkdownDocument>,
    cx: &mut VisualTestContext,
) -> Rc<RefCell<Vec<EditRefusal>>> {
    let heard = Rc::new(RefCell::new(Vec::new()));
    let kept = heard.clone();
    cx.update(|_, cx| {
        cx.subscribe(document, move |_, event: &DocumentEvent, _| {
            if let DocumentEvent::EditRefused(why) = event {
                kept.borrow_mut().push(*why);
            }
        })
        .detach()
    });
    heard
}

#[gpui::test]
fn a_lapse_comes_by_event(cx: &mut TestAppContext) {
    let (document, cx) = word(cx, 64);
    let heard = refusals(&document, cx);
    document.update(cx, |document, cx| {
        document.format(InlineFormat::Strong, cx).expect("taken");
        document
            .editor
            .update(cx, |editor, cx| editor.edit([(0..0, "y".to_string())], cx));
    });
    cx.run_until_parked();
    assert_eq!(*heard.borrow(), [EditRefusal::Changed]);
}

#[test]
fn an_unwrap_refuses_an_end_in_nested_marks() {
    use InlineFormat::*;
    let across = Err(EditRefusal::Across);
    assert_eq!(toggled("**[a](ht‸tps://x)**", Strong), across);
    assert_eq!(toggled("**~~a~‸~**", Strong), across);
    assert_eq!(toggled("<u>[a](ht‸tps://x)</u>", Underline), across);
    assert_eq!(toggled("**a *b⟨* c⟩**", Strong), across);
}

#[test]
fn clear_keeps_an_autolink_backslash() {
    assert_eq!(
        ok(r"a‸ <https://x/\*>", InlineFormat::Clear),
        r"a‸ https://x/\*"
    );
}

/// The walk past the parse, on `n` then `4n` repeats.
fn linear(unit: &str, run: impl Fn(&str, &Node)) {
    let time = |n: usize| {
        let text = unit.repeat(n);
        let root = file_tree(&text);
        let start = Instant::now();
        run(&text, &root);
        start.elapsed()
    };
    let (small, large) = (time(4000), time(16000));
    assert!(large < small * 8, "{small:?} then {large:?}");
}

#[test]
fn many_paragraphs_format_in_linear_time() {
    linear("a\n\n", |text, root| {
        toggle_tree(text, root, 0..1, InlineFormat::Strong).expect("toggled");
    });
    linear("**a**\n\n", |text, root| {
        toggle_tree(text, root, 0..text.len(), InlineFormat::Clear).expect("cleared");
    });
}

#[gpui::test]
fn a_second_format_refuses_while_one_runs(cx: &mut TestAppContext) {
    let (document, cx) = word(cx, 64);
    let second = document.update(cx, |document, cx| {
        document.format(InlineFormat::Strong, cx).expect("taken");
        document.format(InlineFormat::Emphasis, cx)
    });
    assert_eq!(second, Err(EditRefusal::Busy));
    cx.run_until_parked();
    document.update(cx, |document, cx| {
        document.format(InlineFormat::Emphasis, cx).expect("taken")
    });
    cx.run_until_parked();
    assert!(text(&document, cx).starts_with("***x"));
    document.update(cx, |document, cx| {
        document.editor.update(cx, |editor, cx| editor.undo(cx));
    });
    assert!(text(&document, cx).starts_with("**x"));
    assert!(!text(&document, cx).starts_with("***"));
}

#[gpui::test]
fn a_refusal_comes_by_event(cx: &mut TestAppContext) {
    let source = format!("`code` {}", "x".repeat(64));
    let (document, cx) = open(&source, MarkdownMode::Source, cx);
    let heard = refusals(&document, cx);
    let taken = document.update(cx, |document, cx| {
        document
            .editor
            .update(cx, |editor, cx| editor.select(std::iter::once(2..2), cx));
        document.format(InlineFormat::Strong, cx)
    });
    assert_eq!(taken, Ok(()));
    cx.run_until_parked();
    assert_eq!(*heard.borrow(), [EditRefusal::Code]);
    assert_eq!(text(&document, cx), source);
}

#[gpui::test]
fn a_format_drops_once_preview_shows(cx: &mut TestAppContext) {
    let (document, cx) = word(cx, 64);
    let heard = refusals(&document, cx);
    document.update_in(cx, |document, window, cx| {
        document.format(InlineFormat::Strong, cx).expect("taken");
        document.set_mode(MarkdownMode::Preview, window, cx);
    });
    cx.run_until_parked();
    assert!(text(&document, cx).starts_with("xx"));
    assert_eq!(*heard.borrow(), [EditRefusal::Preview]);
}

#[gpui::test]
fn a_text_past_the_largest_read_refuses(cx: &mut TestAppContext) {
    let (document, cx) = word(cx, LARGEST_READ + 1);
    let refused = document.update(cx, |document, cx| document.format(InlineFormat::Strong, cx));
    assert_eq!(refused, Err(EditRefusal::TooLong));
}

#[gpui::test]
fn a_visual_format_reads_apart(cx: &mut TestAppContext) {
    let (document, cx) = open("word\n\nmore\n", MarkdownMode::Visual, cx);
    document.update(cx, |document, cx| {
        document
            .editor
            .update(cx, |editor, cx| editor.select(std::iter::once(1..1), cx));
        document.format(InlineFormat::Strong, cx).expect("taken");
    });
    assert_eq!(
        text(&document, cx),
        "word\n\nmore\n",
        "not on the UI thread"
    );
    cx.run_until_parked();
    assert_eq!(text(&document, cx), "**word**\n\nmore\n");
}
