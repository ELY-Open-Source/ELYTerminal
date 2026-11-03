use markdown::mdast::Node;

use crate::documents::markdown::{options, span, tree, walk};

/// Each formula's TeX and span, and the text left.
fn read(source: &str) -> (Vec<(String, std::ops::Range<usize>)>, String) {
    let root = tree(source);
    let mut nodes = Vec::new();
    walk(&root, &mut nodes);
    let mut maths = Vec::new();
    let mut text = String::new();
    for node in nodes {
        match node {
            Node::InlineMath(math) => maths.push((math.value.clone(), span(node))),
            Node::Text(written) => text.push_str(&written.value),
            _ => {}
        }
    }
    (maths, text)
}

fn tex(source: &str) -> Vec<String> {
    read(source).0.into_iter().map(|(tex, _)| tex).collect()
}

fn untouched(source: &str) {
    let plain = markdown::to_mdast(source, &options()).unwrap();
    assert_eq!(tree(source), plain, "{source:?}");
}

#[test]
fn a_pair_spans_its_delimiters_and_keeps_its_tex() {
    assert_eq!(
        read(r"a \(x\) b"),
        (vec![("x".into(), 2..7)], "a  b".into())
    );
    assert_eq!(tex(r"\( x \)"), [" x "]);
    assert_eq!(tex(r"\(x \)\(y\)"), ["x ", "y"]);
}

#[test]
fn adjacent_formulas_stay_two() {
    for (source, maths) in [
        (r"\(x\)\(y\)", vec!["x", "y"]),
        (r"$x$\(y\)", vec!["x", "y"]),
        (r"\(y\)$x$", vec!["y", "x"]),
        (r"\[x\]\(y\)", vec!["x", "y"]),
    ] {
        let (found, text) = read(source);
        let found: Vec<_> = found.into_iter().map(|(tex, _)| tex).collect();
        assert_eq!(found, maths, "{source:?}");
        assert_eq!(text, "", "no parting space shows: {source:?}");
    }
}

#[test]
fn dollars_stay_dollars() {
    assert_eq!(tex(r"\(\$5\)"), [r"\$5"]);
    assert_eq!(tex(r"\[a \$ b\]"), [r"a \$ b"]);
    assert_eq!(tex(r"\(a$b\)"), ["a$b"]);
    assert_eq!(
        read(r"costs $5 and \(x\)"),
        (vec![("x".into(), 13..18)], "costs $5 and ".into()),
        "a stray dollar does not pair with a formula"
    );
    assert_eq!(read(r"\$\(x\)").1, "$", "an escaped dollar before");
    assert_eq!(read(r"\(x\)\$").1, "$", "an escaped dollar after");
}

#[test]
fn addresses_keep_their_parentheses() {
    let link = r"[go](https://e.test/\(x\))";
    untouched(link);
    untouched(r"![p](pics/\(x\).png)");
    untouched("[go][a\\(x\\)]\n\n[a\\(x\\)]: /u");
    untouched(r"<https://e.test/\(x\)>");
    assert_eq!(tex(r"[\(x\)](https://e.test)"), ["x"], "link text is prose");
    untouched(r"[a\(b](c)\)");
}

#[test]
fn code_and_tags_stay_literal() {
    untouched(r"a <code>\(x\)</code> b");
    untouched(r"a <kbd>\(x\)</kbd> b");
    untouched(r"a <code>\(x\) b");
    untouched(r"a <code><code>\(x\)</code> \(y\)</code> b");
    untouched(r"a <kbd><code>\(x\)</code> \(y\)</kbd> b");
    assert_eq!(tex(r"<code>a</code> \(x\)"), ["x"], "after code, prose");
    for crossed in [
        r"a <code><kbd>x</code> \(y\) b",
        r"a <b><kbd>x</b> \(y\) b",
        r"a <code><b>x</code> \(y\)</b> b",
    ] {
        assert_eq!(
            tex(crossed),
            ["y"],
            "a crossed close ends code: {crossed:?}"
        );
    }
    untouched(r"a <b><code>x</b> \(y\) b");
    let reopened = crate::documents::tree::parse(r"a <b><code>x</b> \(y\) b");
    let crate::documents::tree::Node::Paragraph(inline) = &reopened[0] else {
        panic!("a paragraph: {reopened:?}");
    };
    let code = crate::documents::tree::Style::Code;
    assert!(
        (inline.styles.iter()).any(|(range, style)| *style == code && range.contains(&6)),
        "the reader reopens code over (y): {inline:?}"
    );
    untouched(r"`\(x\)`");
    untouched(r"\(a `b` c\)");
    assert_eq!(tex(r"<b>\(x\)</b>"), ["x"], "other tags hold prose");
}

#[test]
fn a_pair_stays_in_one_block() {
    untouched("- \\(a\n- b\\)");
    untouched("\\(a\r\rb\\)");
    untouched("\\(a\r\n\r\nb\\)");
    untouched("# \\(a\nb\\)");
    untouched("| \\(a | b\\) |\n| - | - |");
    assert_eq!(tex("> \\(a\n> b\\)"), ["a\nb"], "a quote's lines");
}
