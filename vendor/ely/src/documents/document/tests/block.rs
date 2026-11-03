//! Block commands as Typora's Paragraph menu (R4.7).

use gpui::{Modifiers, TestAppContext};
use markdown::mdast::Node;

use super::{open, opened, press, text};
use crate::documents::{
    BlockCommand, EditRefusal, MarkdownMode, document::block::run, markdown::file_tree,
};

/// `marked` after `command`: `‸` is the caret, `⟨…⟩` the selection.
pub(super) fn ran(marked: &str, command: BlockCommand) -> Result<String, EditRefusal> {
    let caret = marked.find('‸');
    let text: String = marked.chars().filter(|ch| !"‸⟨⟩".contains(*ch)).collect();
    let selection = match caret {
        Some(at) => at..at,
        None => {
            let start = marked.find('⟨').expect("a selection");
            start..marked.find('⟩').expect("its end") - '⟨'.len_utf8()
        }
    };
    let (edits, selected) = run(&text, selection, command)?;
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

pub(super) fn ok(marked: &str, command: BlockCommand) -> String {
    ran(marked, command).expect("ran")
}

/// The top-level blocks `text` reads as.
pub(super) fn kinds(text: &str) -> Vec<&'static str> {
    let text: String = text.chars().filter(|ch| !"‸⟨⟩".contains(*ch)).collect();
    let root = file_tree(&text);
    let kids = root.children().map(Vec::as_slice).unwrap_or_default();
    (kids.iter())
        .map(|kid| match kid {
            Node::Paragraph(_) => "p",
            Node::Heading(_) => "h",
            Node::Blockquote(_) => "quote",
            Node::List(_) => "list",
            Node::Code(_) => "code",
            Node::Math(_) => "math",
            Node::ThematicBreak(_) => "rule",
            Node::Yaml(_) => "yaml",
            Node::Definition(_) => "definition",
            Node::FootnoteDefinition(_) => "note",
            _ => "other",
        })
        .collect()
}

#[test]
fn headings_set_as_typoras_menu() {
    use BlockCommand::*;
    assert_eq!(ok("a‸b", Heading(2)), "## a‸b");
    assert_eq!(ok("## a‸b", Heading(2)), "## a‸b");
    assert_eq!(ok("## a‸b", Heading(3)), "### a‸b");
    assert_eq!(ok("a‸", Promote), "###### a‸");
    assert_eq!(ok("# a‸", Promote), "# a‸");
    assert_eq!(ok("## a‸", Promote), "# a‸");
    assert_eq!(ok("###### a‸", Demote), "a‸");
    assert_eq!(ok("a‸", Demote), "a‸");
    assert_eq!(ok("a\n\n‸\n\nb", Heading(2)), "a\n\n## ‸\n\nb");
}

#[test]
fn a_paragraph_line_becomes_the_heading_alone() {
    let out = ok("a\nb‸\nc", BlockCommand::Heading(1));
    assert_eq!(out, "a\n# b‸\nc");
    assert_eq!(kinds(&out), ["p", "h", "p"]);
    assert_eq!(ok("> a\nb‸", BlockCommand::Heading(1)), "> a\n> # b‸");
    assert_eq!(ok("> a‸\nb", BlockCommand::Heading(1)), "> # a‸\n> b");
}

#[test]
fn setext_headings_relevel_and_go_atx_past_two() {
    use BlockCommand::*;
    assert_eq!(ok("a‸\n===", Heading(2)), "a‸\n---");
    assert_eq!(ok("a‸\n===", Heading(3)), "### a‸");
    assert_eq!(ok("a\nb‸\n---", Heading(4)), "#### a b‸");
    assert_eq!(ok("a‸\n===", Paragraph), "a‸");
}

#[test]
fn a_heading_made_text_keeps_apart_from_its_neighbors() {
    use BlockCommand::Paragraph;
    assert_eq!(ok("a\n# h‸", Paragraph), "a\n\nh‸");
    assert_eq!(ok("# h‸\nb", Paragraph), "h‸\n\nb");
    let out = ok("# h‸\n---", Paragraph);
    assert_eq!(kinds(&out), ["p", "rule"]);
    assert_eq!(ok("- a\n- # h‸", Paragraph), "- a\n- h‸");
    let out = ok("> # h‸\nb", Paragraph);
    assert_eq!(kinds(&out), ["quote", "p"]);
}

#[test]
fn heading_commands_refuse_or_add_a_paragraph_as_typora() {
    use BlockCommand::*;
    assert_eq!(ran("```\na‸\n```", Heading(1)), Err(EditRefusal::Code));
    assert_eq!(ran("$$\na‸\n$$", Promote), Err(EditRefusal::Math));
    assert_eq!(ok("$$\na‸\n$$", Heading(1)), "$$\na\n$$\n\n‸");
    assert_eq!(
        ran("---\nt: a‸\n---\n", Heading(1)),
        Err(EditRefusal::FrontMatter)
    );
    assert_eq!(ran("- [ ] a‸", Heading(1)), Err(EditRefusal::Outside));
}

#[test]
fn quotes_wrap_and_unwrap() {
    use BlockCommand::Quote;
    assert_eq!(ok("a‸", Quote), "> a‸");
    assert_eq!(ok("> a‸", Quote), "a‸");
    assert_eq!(ok("> a\n> b‸", Quote), "a\nb‸");
    assert_eq!(ok("> a\nb‸", Quote), "a\nb‸");
    assert_eq!(ok("⟨a\n\nb⟩", Quote), "> ⟨a\n>\n> b⟩");
    assert_eq!(ok("x\n> a‸", Quote), "x\n\na‸");
    assert_eq!(ok("> [!NOTE]\n> a‸", Quote), "a‸");
    assert_eq!(ok("- a‸", Quote), "- > a‸");
    assert_eq!(ok("> a\n>\n> ⟨b⟩", Quote), "> a\n>\n> > ⟨b⟩");
}

#[test]
fn lists_wrap_each_line() {
    use BlockCommand::*;
    assert_eq!(ok("a‸", BulletList), "- a‸");
    assert_eq!(ok("a‸", OrderedList), "1. a‸");
    assert_eq!(ok("a‸", TaskList), "- [ ] a‸");
    assert_eq!(ok("⟨a\nb⟩", OrderedList), "1. ⟨a\n2. b⟩");
    assert_eq!(ok("> a‸", BulletList), "> - a‸");
    assert_eq!(ok("- a\n\n  b‸", BulletList), "- a\n\n  - b‸");
}

#[test]
fn a_list_of_the_kind_unwraps_and_another_retypes() {
    use BlockCommand::*;
    let out = ok("- a\n- b‸", BulletList);
    assert_eq!(out, "a\n\nb‸");
    assert_eq!(kinds(&out), ["p", "p"]);
    assert_eq!(ok("- a\n- b‸", OrderedList), "1. a\n2. b‸");
    assert_eq!(ok("- a\n  b‸", OrderedList), "1. a\n   b‸");
    assert_eq!(ok("- [ ] a‸", BulletList), "- a‸");
    assert_eq!(ok("* a‸", TaskList), "* [ ] a‸");
    assert_eq!(ok("3. a\n4. b‸", BulletList), "- a\n- b‸");
}

#[test]
fn inserts_split_the_paragraph_at_the_caret() {
    use BlockCommand::*;
    assert_eq!(ok("ab‸cd", Rule), "ab\n\n---\n\n‸cd");
    assert_eq!(ok("ab‸", CodeBlock), "ab\n\n```\n‸\n```");
    assert_eq!(ok("a ⟨b⟩", CodeBlock), "a\n\n```\n‸b\n```");
    assert_eq!(ok("‸ab", Toc), "[toc]‸\n\nab");
    assert_eq!(ok("# ab‸cd", Rule), "# ab\n\n---\n\n# ‸cd");
    assert_eq!(ok("- ab‸", Rule), "- ab\n\n  ---\n\n  ‸");
    assert_eq!(ok("```\na‸\n```", Rule), "```\na\n```\n\n---\n\n‸");
    assert_eq!(ok("a\n\n‸\n\nb", Rule), "a\n\n---\n\n‸\n\nb");
    assert_eq!(ran("⟨a\n\nb⟩", Rule), Err(EditRefusal::Across));
}

#[test]
fn notes_and_definitions_take_free_names() {
    use BlockCommand::*;
    let out = ok("x[^1]\n\n[^1]: y\n\nab‸", Footnote);
    assert_eq!(out, "x[^1]\n\n[^1]: y\n\nab\n\n[^2]: ‸");
    assert_eq!(ok("‸", LinkDefinition), "[‸]: ");
    assert_eq!(ok("⟨see⟩", LinkDefinition), "[see]: ‸");
    assert_eq!(ok("⟨a/b.md⟩", LinkDefinition), "[‸]: a/b.md");
    let out = ok("ab‸", MathBlock);
    assert_eq!(kinds(&out), ["p", "math"]);
}

#[test]
fn empty_paragraphs_come_before_and_after() {
    use BlockCommand::*;
    assert_eq!(ok("a‸", ParagraphAfter), "a\n\n‸");
    assert_eq!(ok("# a‸\nb", ParagraphAfter), "# a\n\n‸\n\nb");
    assert_eq!(ok("a\n\nb‸", ParagraphBefore), "a\n\n‸\n\nb");
    assert_eq!(ok("a\n# h‸", ParagraphBefore), "a\n\n‸\n\n# h");
    let out = ok("- a‸", ParagraphBefore);
    assert_eq!(out, "- ‸\n  a");
    assert_eq!(kinds(&out), ["list"]);
    let out = ok("x[^1]\n\n[^1]: a‸", ParagraphBefore);
    assert_eq!(out, "x[^1]\n\n[^1]: ‸\n    a");
    assert_eq!(kinds(&out), ["p", "note"]);
}

#[test]
fn front_matter_goes_on_top_once() {
    let out = ok("a‸", BlockCommand::FrontMatter);
    assert_eq!(out, "---\n‸\n---\n\na");
    assert_eq!(kinds(&out), ["yaml", "p"]);
    assert_eq!(
        ran("---\nt: 1\n---\n\na‸", BlockCommand::FrontMatter),
        Err(EditRefusal::Present)
    );
}

#[test]
fn delete_takes_the_block_or_its_lone_holder() {
    use BlockCommand::Delete;
    assert_eq!(ok("a\n\nb‸\n\nc", Delete), "a\n\n‸c");
    assert_eq!(ok("x\n\n> b‸\n\nc", Delete), "x\n\n‸c");
    assert_eq!(ok("- a\n- b‸\n- c", Delete), "- a\n‸- c");
    assert_eq!(ok("- a‸\n\n  b", Delete), "- ‸\n\n  b");
}

#[test]
fn line_endings_follow_the_text() {
    assert_eq!(
        ok("ab‸\r\n", BlockCommand::ParagraphAfter),
        "ab\r\n\r\n‸\r\n"
    );
}

#[gpui::test]
fn a_source_command_is_one_undo_step(cx: &mut TestAppContext) {
    let (document, cx) = open("alpha\n", MarkdownMode::Source, cx);
    document.update(cx, |document, cx| {
        document
            .editor
            .update(cx, |editor, cx| editor.select(std::iter::once(2..2), cx));
        document.block(BlockCommand::Heading(2), cx).expect("taken");
    });
    cx.run_until_parked();
    assert_eq!(text(&document, cx), "## alpha\n");
    document.update(cx, |document, cx| {
        document.editor.update(cx, |editor, cx| editor.undo(cx));
    });
    assert_eq!(text(&document, cx), "alpha\n");
}

#[gpui::test]
fn an_open_block_takes_a_command_and_stays_open(cx: &mut TestAppContext) {
    let (document, cx) = open("alpha\n", MarkdownMode::Visual, cx);
    press(0, Modifiers::none(), cx);
    document.update(cx, |document, cx| {
        document.block(BlockCommand::Quote, cx).expect("taken");
    });
    cx.run_until_parked();
    assert_eq!(opened(&document, cx), "> alpha", "still open to type in");
}

#[gpui::test]
fn a_command_past_the_open_block_writes_it_back_first(cx: &mut TestAppContext) {
    let (document, cx) = open("first\n\nalpha\n", MarkdownMode::Visual, cx);
    press(1, Modifiers::none(), cx);
    document.update(cx, |document, cx| {
        let open = document.open.as_ref().expect("opened");
        open.input.update(cx, |input, cx| input.insert("x", cx));
        document
            .block(BlockCommand::FrontMatter, cx)
            .expect("taken");
    });
    cx.run_until_parked();
    let written = text(&document, cx);
    assert!(written.starts_with("---\n\n---\n"), "{written:?}");
    assert!(
        written.contains("alpha") && written.contains('x'),
        "{written:?}"
    );
    document.read_with(cx, |document, _| assert!(document.open.is_none()));
}

/// The one list's items, each as its blocks' kinds.
fn items(text: &str) -> Vec<Vec<&'static str>> {
    let text: String = text.chars().filter(|ch| !"‸⟨⟩".contains(*ch)).collect();
    let root = file_tree(&text);
    let Some(Node::List(list)) = root.children().and_then(|kids| kids.first()) else {
        panic!("a list first");
    };
    (list.children.iter())
        .map(|item| {
            (item
                .children()
                .map(Vec::as_slice)
                .unwrap_or_default()
                .iter())
            .map(|kid| match kid {
                Node::Paragraph(_) => "p",
                Node::Code(_) => "code",
                _ => "other",
            })
            .collect()
        })
        .collect()
}

#[test]
fn a_caret_in_heading_marks_splits_at_the_text() {
    use BlockCommand::Rule;
    assert_eq!(ok("‸# alpha", Rule), "---\n\n# ‸alpha");
    let out = ok("# alpha #‸##", Rule);
    assert_eq!(kinds(&out), ["h", "rule"]);
}

#[test]
fn a_task_list_keeps_checked_boxes() {
    assert_eq!(
        ok("- a\n- [x] b‸", BlockCommand::TaskList),
        "- [ ] a\n- [x] b‸"
    );
}

#[test]
fn an_alert_in_an_item_keeps_the_item() {
    let out = ok("- > [!NOTE]\n  > a‸\n- b", BlockCommand::Quote);
    assert_eq!(out, "- a‸\n- b");
    assert_eq!(items(&out), [vec!["p"], vec!["p"]]);
}

#[test]
fn a_split_closes_and_reopens_inline_marks() {
    use BlockCommand::Rule;
    let out = ok("**ab‸cd**", Rule);
    assert_eq!(out, "**ab**\n\n---\n\n**‸cd**");
    assert_eq!(kinds(&out), ["p", "rule", "p"]);
    assert_eq!(ran("[a‸b](u)", Rule), Err(EditRefusal::Across));
    assert_eq!(ran("`a‸b`", Rule), Err(EditRefusal::Across));
}

#[test]
fn a_retyped_item_keeps_its_code_indent() {
    let out = ok("-     code‸", BlockCommand::OrderedList);
    assert_eq!(out, "1.     code‸");
    assert_eq!(items(&out), [vec!["code"]]);
}

#[test]
fn deleting_keeps_neighbors_apart() {
    let out = ok("p\n# h‸\nq", BlockCommand::Delete);
    assert_eq!(out, "p\n‸\nq");
    assert_eq!(kinds(&out), ["p", "p"]);
}

#[test]
fn heading_marks_keep_their_text() {
    use BlockCommand::*;
    let out = ok("alpha #‸", Heading(1));
    assert_eq!(out, "# alpha \\#‸");
    assert_eq!(kinds(&out), ["h"]);
    let out = ok("# - alpha‸", Paragraph);
    assert_eq!(out, "\\- alpha‸");
    assert_eq!(kinds(&out), ["p"]);
    assert_eq!(kinds(&ok("# 1. a‸", Paragraph)), ["p"]);
}

#[test]
fn a_tasks_first_text_takes_no_block() {
    use BlockCommand::*;
    assert_eq!(ran("- [ ] a‸", Quote), Err(EditRefusal::Outside));
    assert_eq!(ran("- [ ] ‸ab", Rule), Err(EditRefusal::Outside));
    assert_eq!(ran("- [ ] ‸ab", CodeBlock), Err(EditRefusal::Outside));
    assert_eq!(ran("- [ ] a‸", ParagraphBefore), Err(EditRefusal::Outside));
}

#[test]
fn a_selection_past_a_list_takes_its_siblings() {
    let out = ok("- ⟨a\n\nb⟩", BlockCommand::OrderedList);
    assert_eq!(out, "1. ⟨a\n\n2. b⟩");
    assert_eq!(kinds(&out), ["list"]);
}
