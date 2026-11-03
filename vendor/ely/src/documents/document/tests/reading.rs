//! Drawn text reads back to its source byte (R4.5).

use crate::documents::{
    TargetBlock,
    document::parse::parsed,
    tree::{Inline, Node, lines_at},
};

/// The source byte under drawn `shown` in block 0.
fn read_back(source: &str, shown: &str) -> Option<usize> {
    fn find<'a>(nodes: &'a [Node], shown: &str) -> Option<&'a Inline> {
        nodes.iter().find_map(|node| match node {
            Node::Paragraph(inline) | Node::Heading(_, inline) if inline.text.contains(shown) => {
                Some(inline)
            }
            Node::Quote(body) | Node::Alert { body, .. } | Node::Footnote { body, .. } => {
                find(body, shown)
            }
            _ => None,
        })
    }
    let parsed = parsed(source);
    let block = &parsed.blocks[0];
    let inline = find(block.nodes.as_ref().expect("drawn"), shown).expect("shown");
    let at = inline.text.find(shown).expect("shown");
    let written = inline.source_at(&source[block.range.start..], at)?;
    Some(block.range.start + written)
}

/// The source byte under drawn `word` of block 0's code.
fn code_back(source: &str, word: &str) -> Option<usize> {
    fn code(nodes: &[Node]) -> (&str, usize) {
        match &nodes[0] {
            Node::Code { text, from, .. } => (text, *from),
            Node::Quote(body) => code(body),
            other => panic!("not code: {other:?}"),
        }
    }
    let parsed = parsed(source);
    let block = &parsed.blocks[0];
    let (drawn, from) = code(block.nodes.as_ref().expect("drawn"));
    let at = drawn.find(word).expect("drawn");
    let written = lines_at(&source[block.range.start..], from, drawn, at)?;
    Some(block.range.start + written)
}

#[test]
fn text_reads_back_past_marks_escapes_trims_and_markers() {
    let cases = [
        ("> alpha\n> beta", "eta"),
        ("> [!NOTE]\n> alpha beta", "beta"),
        ("> [!NOTE]  \n> alpha beta", "beta"),
        ("> [!NOTE]\\\n> alpha beta", "beta"),
        ("> [!NOTE]\n> \\*beta\\*", "*beta"),
        ("> [!NOTE]&#10;alpha beta", "beta"),
        ("a \\*star\\* z", "*star"),
        ("a \\\\ z", "\\ z"),
        ("a ` xy ` z", "xy"),
        ("a `  xy  ` z", " xy "),
        ("a ` >xy ` z", ">xy"),
        ("a `\\\\xy` z", "xy"),
        ("<div>alpha beta</div>\n", "eta"),
        ("> <div>\n> alpha beta</div>\n", "beta"),
        ("> <div>\n> >alpha beta</div>", ">alpha"),
        ("> <div>\n> >alpha beta</div>", "beta"),
        ("<div>alpha <b>beta</b> gamma</div>", "gamma"),
        ("<div>alpha <b>beta</b> gamma</div>", "beta"),
        ("<div>alpha <b> beta</b></div>", "eta"),
        ("<div>one</div>\n<div>alpha beta</div>", "beta"),
    ];
    for (source, shown) in cases {
        assert_eq!(read_back(source, shown), source.find(shown), "{source:?}");
    }
}

#[test]
fn a_reference_reads_back_to_its_ampersand() {
    let source = "a &amp; b";
    assert_eq!(read_back(source, "& b"), source.find('&'));
    assert_eq!(read_back(source, "b"), source.find('b'));
}

#[test]
fn text_after_an_emoji_reads_back_and_before_it_lands_at_its_start() {
    let source = "ab :smile: y";
    assert_eq!(read_back(source, "y"), source.find('y'));
    assert_eq!(read_back(source, "b"), Some(0));
}

#[test]
fn code_reads_back_line_for_line_whatever_its_line_ends() {
    let cases = [
        "```\nalpha\nbeta\n```\n",
        "```\r\nalpha\r\nbeta\r\n```\r\n",
        "```\ralpha\rbeta\r```\r",
        "    alpha\n    beta\n",
        "> ```\n> alpha\n> beta\n> ```\n",
    ];
    for source in cases {
        for word in ["pha", "eta"] {
            assert_eq!(code_back(source, word), source.find(word), "{source:?}");
        }
    }
}

#[test]
fn code_cut_from_a_tab_reads_back_past_it() {
    for spaces in 0..=3 {
        for prefix in ["\t", " \t", "  \t", "   \t", "\t\t"] {
            let indent = " ".repeat(spaces);
            let source = format!("{indent}```\n{indent}alpha\n{prefix}beta\n{indent}```\n");
            assert_eq!(code_back(&source, "eta"), source.find("eta"), "{source:?}");
        }
    }
}

#[test]
fn a_decoded_line_end_reads_without_a_written_one() {
    let quote = parsed("> alpha&#10;beta");
    assert_eq!(quote.blocks[0].kind, TargetBlock::Quote);
    let alert = parsed("> [!NOTE]&#10;alpha beta");
    assert_eq!(alert.blocks[0].kind, TargetBlock::Alert);
}

#[test]
fn kept_leading_marks_still_map() {
    let source = "a `fox\n  y` z";
    assert_eq!(read_back(source, "ox"), source.find("ox"));
}

#[test]
fn decoded_line_end_maps_nearby_text() {
    for (source, shown) in [
        ("ab &#10; y", "b"),
        ("ab &#13; y", "b"),
        ("ab &NewLine; y", "b"),
        ("<div>ab &#10; y</div>", "b"),
        ("<div>ab &#9; y</div>", "b"),
    ] {
        assert_eq!(read_back(source, shown), source.find(shown), "{source:?}");
    }
}

#[test]
fn numeric_references_match_the_parser() {
    for source in ["ab &#128; y", "ab &#1; y", "ab &#xFDD0; y"] {
        assert_eq!(read_back(source, "b"), source.find('b'), "{source:?}");
    }
}

#[test]
fn html_trailing_encoded_space_maps_previous_word() {
    for source in [
        "<div>ab&#32;</div>",
        "<div>ab&#32;<b>word</b></div>",
        "<div>ab &#32; word</div>",
    ] {
        assert_eq!(read_back(source, "b"), source.find('b'), "{source:?}");
    }
}

#[test]
fn code_pipe_in_a_table_maps_previous_text() {
    let source = "| `ab\\|cd` |\n| --- |\n| cell |\n";
    let parsed = parsed(source);
    let nodes = parsed.blocks[0].nodes.as_ref().unwrap();
    let Node::Table { head, .. } = &nodes[0] else {
        panic!("table")
    };
    let Node::Paragraph(inline) = &head[0][0] else {
        panic!("text")
    };
    assert_eq!(inline.text, "ab|cd");
    assert_eq!(inline.source_at(source, 1), source.find('b'));
}

#[test]
fn autolink_does_not_decode_literal_reference() {
    let source = "<https://example.com/?a=&amp;b=2>";
    assert_eq!(read_back(source, "xample"), source.find("xample"));
}
