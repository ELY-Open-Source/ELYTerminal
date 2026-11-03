//! Block commands whose results reread as written.

use markdown::mdast::Node;

use super::block::{kinds, ok, ran};
use crate::documents::{BlockCommand, EditRefusal, markdown::file_tree};

fn plain(marked: &str) -> String {
    marked.chars().filter(|ch| !"‸⟨⟩".contains(*ch)).collect()
}

/// Every node of `text`, in document order.
fn nodes(text: &str, keep: &mut dyn FnMut(&Node)) {
    fn walk(node: &Node, keep: &mut dyn FnMut(&Node)) {
        keep(node);
        for kid in node.children().map(Vec::as_slice).unwrap_or_default() {
            walk(kid, keep);
        }
    }
    walk(&file_tree(text), keep);
}

fn checks(text: &str) -> Vec<Option<bool>> {
    let mut out = Vec::new();
    nodes(text, &mut |node| {
        if let Node::ListItem(item) = node {
            out.push(item.checked);
        }
    });
    out
}

fn codes(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    nodes(text, &mut |node| {
        if let Node::Code(code) = node {
            out.push(code.value.clone());
        }
    });
    out
}

fn texts(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    nodes(text, &mut |node| {
        if let Node::Text(text) = node {
            out.push(text.value.clone());
        }
    });
    out
}

#[test]
fn a_split_that_would_reread_otherwise_refuses() {
    assert_eq!(
        ran("~~ab‸~cd~~", BlockCommand::Rule),
        Err(EditRefusal::Misread)
    );
    assert_eq!(
        ran("__ab‸_cd__", BlockCommand::Rule),
        Err(EditRefusal::Misread)
    );
}

#[test]
fn a_split_rest_that_would_open_a_block_is_escaped() {
    let out = plain(&ok("a ‸- b", BlockCommand::Rule));
    assert_eq!(out, "a\n\n---\n\n\\- b");
    assert_eq!(kinds(&out), ["p", "rule", "p"]);
}

#[test]
fn closing_hashes_stay_text_wherever_a_heading_line_ends() {
    let out = plain(&ok("alpha #‸\n===", BlockCommand::Heading(3)));
    assert_eq!(out, "### alpha \\#");
    assert_eq!(texts(&out), ["alpha #"]);
    let out = plain(&ok("# a #‸ b", BlockCommand::Rule));
    assert_eq!(kinds(&out), ["h", "rule", "h"]);
    assert_eq!(texts(&out), ["a #", "b"]);
}

#[test]
fn a_paragraph_reads_the_heading_body_without_closing_hashes() {
    let out = plain(&ok("# ---‸ ###", BlockCommand::Paragraph));
    assert_eq!(out, "\\---");
    assert_eq!(kinds(&out), ["p"]);
    let out = plain(&ok("# [x]: /u‸ ###", BlockCommand::Paragraph));
    assert_eq!(kinds(&out), ["p"]);
    assert_eq!(texts(&out).concat(), "[x]: /u");
}

#[test]
fn a_task_box_below_its_marker_stays_checked() {
    let out = plain(&ok("1.‸\n   [x] a", BlockCommand::TaskList));
    assert_eq!(checks(&out), [Some(true)]);
    assert_eq!(texts(&out), ["a"]);
}

#[test]
fn retyping_keeps_code_columns_past_tabs() {
    let source = "-\t\t‸code";
    let out = plain(&ok(source, BlockCommand::OrderedList));
    assert_eq!(codes(&out), codes(&plain(source)));
    assert_eq!(codes(&out).len(), 1);
}

#[test]
fn a_task_cannot_box_code() {
    assert_eq!(
        ran("-     ‸a", BlockCommand::TaskList),
        Err(EditRefusal::Code)
    );
}

#[test]
fn an_alert_unwraps_into_its_holder() {
    assert_eq!(
        plain(&ok("- > [!NOTE]‸\n    a", BlockCommand::Quote)),
        "- a"
    );
    assert_eq!(
        plain(&ok("> > [!NOTE]‸\n>   a", BlockCommand::Quote)),
        "> a"
    );
    let out = plain(&ok("- > [!NOTE]‸\n  >\n  >\n  > a", BlockCommand::Quote));
    assert_eq!(out, "- a");
    assert_eq!(kinds(&out), ["list"]);
}

#[test]
fn an_alert_marker_sharing_its_line_refuses() {
    assert_eq!(
        ran("> [!NOTE]&#10;a‸", BlockCommand::Quote),
        Err(EditRefusal::Misread)
    );
}

#[test]
fn delete_keeps_neighbors_apart_past_gone_openers() {
    let out = plain(&ok("p\n> # ‸h\nq", BlockCommand::Delete));
    assert_eq!(out, "p\n\nq");
    assert_eq!(kinds(&out), ["p", "p"]);
}

fn inline_codes(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    nodes(text, &mut |node| {
        if let Node::InlineCode(code) = node {
            out.push(code.value.clone());
        }
    });
    out
}

#[test]
fn a_heading_join_keeps_white_space_in_code() {
    let source = "a `x\n  y`‸\n===";
    match ran(source, BlockCommand::Heading(3)) {
        Err(EditRefusal::Misread) => {}
        Ok(out) => assert_eq!(inline_codes(&plain(&out)), inline_codes(&plain(source))),
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_split_that_draws_otherwise_refuses() {
    for (source, command) in [
        ("<u>ab‸cd</u>", BlockCommand::Rule),
        ("<u>a‸\nb</u>", BlockCommand::Heading(1)),
        ("x:smile: ‸:smile:", BlockCommand::Rule),
    ] {
        assert_eq!(ran(source, command), Err(EditRefusal::Misread), "{source}");
    }
}

#[test]
fn an_alert_body_block_keeps_its_indent() {
    let out = plain(&ok("> [!NOTE]\n>\n>     co‸de", BlockCommand::Quote));
    assert_eq!(
        (kinds(&out), codes(&out)),
        (vec!["code"], vec!["code".to_string()])
    );
    let out = plain(&ok(
        "> [!NOTE]\n>\n>   ```\n>   co‸de\n>   ```",
        BlockCommand::Quote,
    ));
    assert_eq!(
        (kinds(&out), codes(&out)),
        (vec!["code"], vec!["code".to_string()])
    );
}

#[test]
fn retyping_reads_a_fence_from_its_source() {
    let out = plain(&ok("-     ```lang‸", BlockCommand::OrderedList));
    assert_eq!(codes(&out), vec!["```lang".to_string()]);
    let out = plain(&ok("-     \u{a0}code‸", BlockCommand::OrderedList));
    assert_eq!(codes(&out), vec!["\u{a0}code".to_string()]);
}

#[test]
fn a_removed_box_leaves_text() {
    assert_eq!(
        plain(&ok("- [x] ---‸", BlockCommand::BulletList)),
        "- \\---"
    );
    assert_eq!(
        plain(&ok("- [x] # h‸", BlockCommand::BulletList)),
        "- \\# h"
    );
    assert_eq!(plain(&ok("-\n  [x] a‸", BlockCommand::TaskList)), "a");
    assert_eq!(plain(&ok("- [x] # h‸", BlockCommand::TaskList)), "\\# h");
}

#[test]
fn an_empty_heading_becomes_an_empty_paragraph() {
    assert!(ran("# ‸", BlockCommand::Paragraph).is_ok());
}

#[test]
fn an_escaped_alert_marker_on_its_own_line_unwraps() {
    assert_eq!(plain(&ok("> \\[!NOTE]\n> a‸", BlockCommand::Quote)), "a");
}
