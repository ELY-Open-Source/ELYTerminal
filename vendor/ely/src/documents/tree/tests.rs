use super::*;

#[test]
fn blocks_nest_as_written() {
    let nodes = parse(
        "# Lift\n\nBlend **toward** [white](https://ely.dev).\n\n- [x] clamp\n- [ ] gamma\n\n> quoted\n\n```rust\nfn main() {}\n```\n\n---\n",
    );
    assert!(matches!(&nodes[0], Node::Heading(1, text) if text.text == "Lift"));
    let Node::Paragraph(line) = &nodes[1] else {
        panic!("a paragraph, not {:?}", nodes[1])
    };
    assert_eq!(line.text, "Blend toward white.");
    assert_eq!(line.styles, [(6..12, Style::Strong), (13..18, Style::Link)]);
    assert_eq!(line.links, [(13..18, "https://ely.dev".to_string())]);
    let Node::List { start: None, items } = &nodes[2] else {
        panic!("a list, not {:?}", nodes[2])
    };
    assert_eq!(
        items.iter().map(|(task, _)| *task).collect::<Vec<_>>(),
        [Some(true), Some(false)]
    );
    assert!(matches!(&items[0].1[0], Node::Paragraph(text) if text.text == "clamp"));
    assert!(matches!(&nodes[3], Node::Quote(inner) if inner.len() == 1));
    assert_eq!(
        nodes[4],
        Node::Code {
            language: Some("rust".into()),
            text: "fn main() {}".into(),
            from: 95
        }
    );
    assert_eq!(nodes[5], Node::Rule);
}

#[test]
fn an_image_inside_an_image_is_the_outer_one() {
    // markdown-rs drops the inner image from the alt.
    assert_eq!(
        parse("![outer ![inner](inner.png)](outer.png)"),
        [Node::Image {
            alt: "outer ".into(),
            url: "outer.png".into(),
            link: None,
            at: 0
        }]
    );
}

#[test]
fn an_image_splits_its_paragraph_and_references_resolve() {
    let nodes = parse("Before ![a][pic] after [site].\n\n[pic]: a.png\n[site]: https://ely.dev\n");
    assert!(matches!(&nodes[0], Node::Paragraph(text) if text.text == "Before "));
    assert_eq!(
        nodes[1],
        Node::Image {
            alt: "a".into(),
            url: "a.png".into(),
            link: None,
            at: 7
        }
    );
    let Node::Paragraph(after) = &nodes[2] else {
        panic!("a paragraph, not {:?}", nodes[2])
    };
    assert_eq!(after.text, " after site.");
    assert_eq!(after.links, [(7..11, "https://ely.dev".to_string())]);
    assert_eq!(nodes.len(), 3, "definitions draw nothing: {nodes:?}");
}

#[test]
fn a_soft_break_draws_as_a_line_and_joins_for_output() {
    for source in [
        "one\ntwo  \nthree",
        "one\r\ntwo  \r\nthree",
        "one\rtwo  \rthree",
    ] {
        let nodes = parse(source);
        assert!(
            matches!(&nodes[0], Node::Paragraph(text) if text.text == "one\ntwo\nthree"),
            "{source:?} draws {nodes:?}"
        );
        let root = super::super::markdown::tree(source);
        let paragraph = &root.children().expect("blocks")[0];
        let joined = Reader::joined(source, &root).inline(paragraph.children().expect("inlines"));
        assert_eq!(joined.text, "one two\nthree", "{source:?} joins");
    }
}

#[test]
fn a_heading_title_reads_on_one_line() {
    let source = "one\ntwo\n===\n";
    let root = super::super::markdown::tree(source);
    assert_eq!(Reader::new(source, &root).headings[0].title, "one two");
}

#[test]
fn definitions_nested_in_quotes_and_lists_resolve() {
    for source in [
        "[site] ![pic]\n\n> [site]: https://ely.dev\n> [pic]: a.png\n",
        "[site] ![pic]\n\n- [site]: https://ely.dev\n- [pic]: a.png\n",
    ] {
        let nodes = parse(source);
        let Node::Paragraph(text) = &nodes[0] else {
            panic!("a paragraph, not {:?}", nodes[0])
        };
        assert_eq!(text.links, [(0..4, "https://ely.dev".to_string())]);
        assert_eq!(
            nodes[1],
            Node::Image {
                alt: "pic".into(),
                url: "a.png".into(),
                link: None,
                at: 7
            }
        );
    }
}

#[test]
fn an_image_inside_styles_and_links_stays_an_image() {
    let nodes = parse("*a ![x](x.png) b* [![y](y.png)](https://ely.dev)");
    let Node::Paragraph(before) = &nodes[0] else {
        panic!("a paragraph, not {:?}", nodes[0])
    };
    assert_eq!(
        (before.text.as_str(), &before.styles[..]),
        ("a ", &[(0..2, Style::Emphasis)][..])
    );
    assert_eq!(
        nodes[1],
        Node::Image {
            alt: "x".into(),
            url: "x.png".into(),
            link: None,
            at: 3
        }
    );
    let Node::Paragraph(after) = &nodes[2] else {
        panic!("a paragraph, not {:?}", nodes[2])
    };
    assert_eq!(
        (after.text.as_str(), &after.styles[..]),
        (" b ", &[(0..2, Style::Emphasis)][..])
    );
    assert_eq!(
        nodes[3],
        Node::Image {
            alt: "y".into(),
            url: "y.png".into(),
            link: Some("https://ely.dev".into()),
            at: 19
        }
    );
    assert_eq!(nodes.len(), 4, "{nodes:?}");
}

#[test]
fn a_heading_keeps_its_images_after_it() {
    let nodes = parse("# Lift ![x](x.png)");
    assert!(matches!(&nodes[0], Node::Heading(1, text) if text.text == "Lift "));
    assert!(matches!(&nodes[1], Node::Image { url, .. } if url == "x.png"));
}

#[test]
fn display_math_stands_alone() {
    assert_eq!(parse("$$c = 1$$"), [Node::Math("c = 1".into())]);
    assert_eq!(parse("$$\nc = 1\n$$"), [Node::Math("c = 1".into())]);
    let Node::Paragraph(inline) = &parse("so $c$ holds")[0] else {
        panic!("inline math stays in its paragraph")
    };
    assert_eq!(inline.styles, [(3..4, Style::Math)]);
}

#[test]
fn tables_keep_their_cells_and_alignment() {
    let nodes = parse("| Name | Lift |\n|:--|--:|\n| accent | 0.5 |\n");
    let Node::Table { aligns, head, rows } = &nodes[0] else {
        panic!("a table, not {:?}", nodes[0])
    };
    assert_eq!(aligns, &[AlignKind::Left, AlignKind::Right]);
    let text = |cell: &[Node]| match cell {
        [Node::Paragraph(line)] => line.text.clone(),
        other => panic!("one line, not {other:?}"),
    };
    assert_eq!(
        head.iter().map(|cell| text(cell)).collect::<Vec<_>>(),
        ["Name", "Lift"]
    );
    assert_eq!(text(&rows[0][1]), "0.5");
}

#[test]
fn a_picture_in_a_cell_stays_a_picture() {
    let nodes = parse("| Picture |\n| --- |\n| ![shot](assets/shot.png) |\n");
    let Node::Table { rows, .. } = &nodes[0] else {
        panic!("a table, not {:?}", nodes[0])
    };
    assert!(
        matches!(&rows[0][0][..], [Node::Image { alt, url, .. }] if alt == "shot" && url == "assets/shot.png")
    );
}

#[test]
fn footnotes_number_their_references_and_hold_their_body() {
    let nodes = parse("Lift[^a].\n\n[^a]: Toward white.\n");
    let Node::Paragraph(line) = &nodes[0] else {
        panic!("a paragraph, not {:?}", nodes[0])
    };
    assert_eq!(line.text, "Lift¹.");
    assert_eq!(line.styles, [(4..6, Style::Footnote)]);
    assert!(matches!(&nodes[1], Node::Footnote { label, body } if label == "1" && body.len() == 1));
    let alone = parse(
        "[^z]: Unread.
",
    );
    assert!(matches!(&alone[0], Node::Footnote { label, .. } if label == "z"));
}

fn line(source: &str) -> Inline {
    match parse(source).into_iter().next() {
        Some(Node::Paragraph(text)) => text,
        other => panic!("a paragraph, not {other:?}"),
    }
}

#[test]
fn inline_html_shows_as_typora_shows_it() {
    let text = line(
        "a <u>un</u> <b>bo</b> <kbd>K</kbd> <mark>hi</mark> <a href=\"https://ely.dev\">x</a><br>end <span>sp</span> <a name=\"n\">no</a>",
    );
    assert_eq!(text.text, "a un bo K hi x\nend sp no");
    assert_eq!(
        text.styles,
        [
            (2..4, Style::Underline),
            (5..7, Style::Strong),
            (8..9, Style::Code),
            (10..12, Style::Mark),
            (13..14, Style::Link)
        ]
    );
    assert_eq!(text.links, [(13..14, "https://ely.dev".to_string())]);
}

/// Where `style` holds, touching stretches joined.
fn marked(inline: &Inline, style: Style) -> Vec<(usize, usize)> {
    let mut ranges: Vec<Range<usize>> = inline
        .styles
        .iter()
        .filter(|(_, held)| *held == style)
        .map(|(range, _)| range.clone())
        .collect();
    ranges.sort_by_key(|range| range.start);
    let mut out: Vec<Range<usize>> = Vec::new();
    for range in ranges {
        match out.last_mut() {
            Some(last) if last.end >= range.start => last.end = last.end.max(range.end),
            _ => out.push(range),
        }
    }
    out.into_iter()
        .map(|range| (range.start, range.end))
        .collect()
}

#[test]
fn tags_and_emphasis_cross_as_a_browser_repairs_them() {
    let opened_inside = line("**a <u>b** c</u> d");
    assert_eq!(opened_inside.text, "a b c d");
    assert_eq!(marked(&opened_inside, Style::Strong), [(0, 3)]);
    assert_eq!(marked(&opened_inside, Style::Underline), [(2, 5)]);
    let closed_inside = line("a <u>*text</u> outside* end");
    assert_eq!(closed_inside.text, "a text outside end");
    assert_eq!(marked(&closed_inside, Style::Underline), [(2, 6)]);
    assert_eq!(marked(&closed_inside, Style::Emphasis), [(2, 14)]);
    assert!(line("a </u> b").styles.is_empty(), "a stray close");
}

#[test]
fn a_raw_close_ends_the_markdown_element() {
    let em = line("a *inside</em> outside* end");
    assert_eq!(em.text, "a inside outside end");
    assert_eq!(marked(&em, Style::Emphasis), [(2, 8)]);
    let strong = line("a **inside</strong> outside** end");
    assert_eq!(marked(&strong, Style::Strong), [(2, 8)]);
}

#[test]
fn only_formatting_elements_reopen_past_a_close() {
    for (tag, style) in [
        ("mark", Style::Mark),
        ("kbd", Style::Code),
        ("del", Style::Strike),
    ] {
        let text = line(&format!("a **bold <{tag}>x** outside</{tag}> end"));
        assert_eq!(text.text, "a bold x outside end", "{tag}");
        assert_eq!(marked(&text, style), [(7, 8)], "{tag}");
    }
    let reopened = line("<b>1<i>2</b>3</i>4");
    assert_eq!(marked(&reopened, Style::Strong), [(0, 2)]);
    assert_eq!(marked(&reopened, Style::Emphasis), [(1, 3)]);
}

#[test]
fn a_new_anchor_ends_the_open_one() {
    let text = line("[one <a href='/inner'>two</a> three](/outer)");
    assert_eq!(text.text, "one two three");
    assert_eq!(
        text.links,
        [(0..4, "/outer".to_string()), (4..7, "/inner".to_string())]
    );
    assert_eq!(marked(&text, Style::Link), [(0, 7)]);
    let named = line("[x <a name=n>y</a> z](/u)");
    assert_eq!(named.links, [(0..2, "/u".to_string())]);
}

#[test]
fn a_closing_br_breaks_as_html_reads_it() {
    assert_eq!(line("before<br></br>after").text, "before\n\nafter");
}

#[test]
fn shortcodes_mean_what_github_means() {
    assert_eq!(line(":cricket: :cricket_game:").text, "🦗 🏏");
}

#[test]
fn shortcodes_draw_as_emoji_outside_code() {
    let text = line("go :rocket: `:rocket:`");
    assert_eq!(text.text, "go 🚀 :rocket:");
}

#[test]
fn footnotes_number_by_their_first_reference() {
    let nodes = parse("x[^b] y[^a] z[^b]\n\n[^a]: A\n\n[^b]: B\n");
    let Node::Paragraph(text) = &nodes[0] else {
        panic!("a paragraph, not {:?}", nodes[0])
    };
    assert_eq!(text.text, "x¹ y² z¹");
    let labels: Vec<&str> = nodes[1..]
        .iter()
        .map(|node| match node {
            Node::Footnote { label, .. } => label.as_str(),
            other => panic!("a footnote, not {other:?}"),
        })
        .collect();
    assert_eq!(labels, ["2", "1"]);
}

#[test]
fn one_tilde_still_strikes() {
    assert_eq!(line("H~2~O").styles, [(1..2, Style::Strike)]);
}

#[test]
fn shortcodes_read_as_github_reads_them() {
    let decoded = line(r"Show \:smile: and &#58;smile: and :sm&#105;le:.");
    assert_eq!(decoded.text, "Show 😄 and 😄 and 😄.");
    let tags = line("a <code>:smile:</code> <kbd>:smile:</kbd> <tt>:smile:</tt> :smile:");
    assert_eq!(tags.text, "a :smile: 😄 😄 😄");
    assert_eq!(
        line("<https://x.test/:smile:>").text,
        "https://x.test/:smile:"
    );
    assert_eq!(line("[go :smile:](https://x.test)").text, "go 😄");
}

#[test]
fn unknown_tags_show_text_and_only_anchors_link() {
    let custom = line("a <u-box>word</u-box> <br-box>in</br-box> end");
    assert_eq!(
        (custom.text.as_str(), custom.styles.len()),
        ("a word in end", 0)
    );
    let bold = line("See <b href=\"https://x.test/\">bold</b>.");
    assert_eq!(bold.styles, [(4..8, Style::Strong)]);
    assert!(bold.links.is_empty(), "{bold:?}");
    let query = line("<a href=\"https://x.test/?q=1&amp;p=2\">r</a> <a href=/path/>s</a>");
    let urls: Vec<&str> = query.links.iter().map(|(_, url)| url.as_str()).collect();
    assert_eq!(urls, ["https://x.test/?q=1&p=2", "/path/"]);
}

#[test]
fn the_first_shortcode_decides_its_text_node() {
    // GitHub's answers, `.work/R4.3-fix3/github-*.log`.
    for (source, shown) in [
        ("a>:smile: :smile:", "a>:smile: :smile:"),
        ("a> :smile: :smile:", "a> 😄 😄"),
        ("x:smile: :smile:", "x:smile: :smile:"),
        ("a;:smile: :smile:", "a;:smile: :smile:"),
        ("a :smile: b>:smile:", "a 😄 b>😄"),
        ("a :zzzq: x:smile: :smile:", "a :zzzq: x😄 😄"),
        ("ax:zzzq: :smile:", "ax:zzzq: :smile:"),
        ("a :x-y: z:smile:", "a :x-y: z😄"),
        ("a :x.y: z:smile:", "a :x.y: z:smile:"),
        ("a.:x.y: :smile:", "a.:x.y: 😄"),
        ("x\t:smile: y:smile:", "x\t:smile: y:smile:"),
        ("x\u{a0}:smile: y:smile:", "x\u{a0}:smile: y:smile:"),
        ("x\n:smile: y:smile:", "x\n:smile: y:smile:"),
        ("*x*:smile: :smile:", "x😄 😄"),
        ("*x*\n:smile: y:smile:", "x\n😄 y😄"),
        ("x:smile: `c` :smile:", "x:smile: c 😄"),
        ("x:smile:<!-- c --> :smile:", "x:smile: 😄"),
        ("x:smile:<span>s</span> :smile:", "x:smile:s 😄"),
        ("x&amp;:smile: :smile:", "x&:smile: :smile:"),
        ("x:smile:  \n:smile:", "x:smile:\n😄"),
        (
            "a <script>x</style>:smile:</script> :smile:",
            "a <script>x</style>:smile:</script> :smile:",
        ),
        (
            "a <script><b>:smile:</b></script> :smile:",
            "a <script>😄</script> 😄",
        ),
    ] {
        assert_eq!(line(source).text, shown, "{source:?}");
    }
}

#[test]
fn latex_delimiters_read_as_math() {
    assert_eq!(parse(r"\[c = 1\]"), [Node::Math("c = 1".into())]);
    assert_eq!(parse("\\[\nc = 1\n\\]"), [Node::Math("c = 1".into())]);
    for indent in [" ", "  ", "   "] {
        let source = format!(r"{indent}\[c = 1\]");
        assert_eq!(parse(&source), [Node::Math("c = 1".into())], "{source:?}");
        let source = format!("{indent}$$c = 1$$");
        assert_eq!(parse(&source), [Node::Math("c = 1".into())], "{source:?}");
    }
    let Node::Paragraph(inline) = &parse(r"so \(c\) holds")[0] else {
        panic!("inline math stays in its paragraph")
    };
    assert_eq!(inline.text, "so c holds");
    assert_eq!(inline.styles, [(3..4, Style::Math)]);
    let Node::Paragraph(alone) = &parse(r"\(c\)")[0] else {
        panic!("a lone formula stays inline")
    };
    assert_eq!(alone.styles, [(0..1, Style::Math)]);
}

#[test]
fn latex_delimiters_leave_code_escapes_and_open_pairs() {
    let text = |source: &str| match &parse(source)[0] {
        Node::Paragraph(inline) => (inline.text.clone(), inline.styles.clone()),
        other => panic!("a paragraph, not {other:?}"),
    };
    assert_eq!(text(r"`\(c\)`").1, [(0..5, Style::Code)], "code stays code");
    assert_eq!(
        text(r"\\(c\\)"),
        (r"\(c\)".into(), Vec::new()),
        "escaped backslashes"
    );
    assert_eq!(text(r"a \(c").0, "a (c", "no closing, no formula");
    let split = parse("a \\(c\n\nd\\) e");
    assert!(
        split.iter().all(|node| match node {
            Node::Paragraph(inline) => inline.styles.is_empty(),
            _ => false,
        }),
        "a blank line ends the formula: {split:?}"
    );
}
