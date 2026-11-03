use super::{BlockData, BlockKind};
use crate::documents::Media;

fn block(kind: BlockKind, texts: &[&str]) -> BlockData {
    BlockData::new(kind, texts.iter().copied())
}

fn table(columns: usize, cells: &[&str]) -> BlockData {
    let kind = BlockKind::Table {
        columns,
        merged: Vec::new(),
    };
    block(kind, cells)
}

#[test]
fn every_kind_markdown_holds_comes_back_as_it_went() {
    let document = vec![
        block(BlockKind::Heading(1), &["Project *notes*"]),
        block(
            BlockKind::Paragraph,
            &["Blend **toward white**,\nthen `clamp`."],
        ),
        block(BlockKind::Bullet, &["Tokens"]),
        block(BlockKind::Todo(true), &["Light and dark"]),
        block(BlockKind::Todo(false), &["Motion"]),
        block(BlockKind::Numbered, &["First"]),
        block(BlockKind::Numbered, &["Second"]),
        block(BlockKind::Quote, &["One line\nand the next"]),
        block(BlockKind::Callout, &["Drag blocks by their handle."]),
        block(
            BlockKind::Toggle(true),
            &["Why gamma?", "It keeps **midtones**."],
        ),
        block(BlockKind::Toggle(false), &["Closed", ""]),
        block(BlockKind::Code, &["let fence = \"```\";\nlift(color)"]),
        block(BlockKind::Diagram, &["graph LR; a --> b"]),
        block(BlockKind::Math, &["c' = c + (1 - c)\\,t"]),
        block(BlockKind::Divider, &[]),
        table(2, &["Token", "Light", "a|b", "0.50"]),
        block(BlockKind::Image(Media::new("atrium.jpg")), &["An atrium"]),
        block(BlockKind::Heading(4), &["Deep"]),
    ];
    let markdown = BlockData::to_markdown(&document);
    assert_eq!(BlockData::from_markdown(&markdown), document, "{markdown}");
}

#[test]
fn lists_stay_tight_and_count_up() {
    let document = vec![
        block(BlockKind::Bullet, &["a"]),
        block(BlockKind::Todo(false), &["b"]),
        block(BlockKind::Numbered, &["c"]),
        block(BlockKind::Numbered, &["d\ne"]),
        block(BlockKind::Paragraph, &[""]),
        block(BlockKind::Callout, &["Note"]),
    ];
    assert_eq!(
        BlockData::to_markdown(&document),
        "- a\n- [ ] b\n\n1. c\n2. d\n   e\n\n> [!NOTE]\n> Note\n"
    );
}

#[test]
fn markdown_written_by_hand_reads_as_blocks() {
    let source = "#### Deep\n\n- one\n  - inner\n- two\n\n> quoted\nlazy\n\n| a | b |\n|---|---|\n| 1 |\n\nInline $x$ math\n\n<div>raw</div>\n";
    assert_eq!(
        BlockData::from_markdown(source),
        vec![
            block(BlockKind::Heading(4), &["Deep"]),
            block(BlockKind::Bullet, &["one"]),
            block(BlockKind::Bullet, &["inner"]),
            block(BlockKind::Bullet, &["two"]),
            block(BlockKind::Quote, &["quoted\nlazy"]),
            table(2, &["a", "b", "1", ""]),
            block(BlockKind::Paragraph, &["Inline $x$ math"]),
            block(BlockKind::Paragraph, &["<div>raw</div>"]),
        ]
    );
}

#[test]
fn what_markdown_cannot_hold_takes_its_nearest_form() {
    let document = vec![
        block(BlockKind::Columns(2), &["Left", "Right"]),
        block(BlockKind::Video(Media::new("clip.mp4")), &[""]),
        block(BlockKind::Synced("shared".into()), &["Kept"]),
    ];
    assert_eq!(
        BlockData::to_markdown(&document),
        "Left\n\nRight\n\n[clip.mp4](clip.mp4)\n\nKept\n"
    );
}

#[test]
fn a_caption_keeps_its_marks_and_a_toggle_its_nested_details() {
    let document = vec![
        block(
            BlockKind::Image(Media::new("a.png")),
            &["**bold** and *slant*"],
        ),
        block(
            BlockKind::Toggle(false),
            &[
                "Outer",
                "<details><summary>Inner</summary>x</details>\n\nAfter",
            ],
        ),
        block(BlockKind::Paragraph, &["Last"]),
    ];
    let markdown = BlockData::to_markdown(&document);
    assert_eq!(BlockData::from_markdown(&markdown), document, "{markdown}");
}

#[test]
fn reference_definitions_stay_with_the_document() {
    let source = "See [the site][id].\n\n[id]: https://ely.dev \"Ely\"\n";
    let blocks = BlockData::from_markdown(source);
    assert_eq!(
        blocks,
        vec![
            block(BlockKind::Paragraph, &["See [the site][id]."]),
            block(BlockKind::Paragraph, &["[id]: https://ely.dev \"Ely\""]),
        ]
    );
    assert_eq!(
        BlockData::from_markdown(&BlockData::to_markdown(&blocks)),
        blocks
    );
}

#[test]
fn a_toggle_left_open_keeps_the_rest_as_its_body() {
    assert_eq!(
        BlockData::from_markdown("<details>\n<summary>Open</summary>\n\nBody\n\nMore\n"),
        vec![block(BlockKind::Toggle(false), &["Open", "Body\n\nMore"])]
    );
}

#[test]
fn brackets_and_comments_keep_their_blocks() {
    let document = vec![
        block(
            BlockKind::Image(Media::new("a)b c<d>.png")),
            &[r"`[x` \[y\] [z]"],
        ),
        block(
            BlockKind::Toggle(true),
            &["Closes", "<!-- </details> -->\n\nkept"],
        ),
        block(
            BlockKind::Toggle(true),
            &["Opens", "<!-- <details> -->\n\nkept"],
        ),
        block(
            BlockKind::Toggle(false),
            &["Quoted", r#"<div title="</details>">kept</div>"#],
        ),
        block(BlockKind::Paragraph, &["After"]),
    ];
    let markdown = BlockData::to_markdown(&document);
    assert_eq!(BlockData::from_markdown(&markdown), document, "{markdown}");
}

#[test]
fn a_bracket_left_alone_comes_back_escaped_in_its_image() {
    let image = || BlockKind::Image(Media::new("a.png"));
    let document = vec![block(image(), &[r"a ] b [c\"])];
    assert_eq!(
        BlockData::from_markdown(&BlockData::to_markdown(&document)),
        vec![block(image(), &[r"a \] b \[c\\"])]
    );
}

#[test]
fn math_with_a_blank_line_and_a_piped_cell_come_back() {
    let document = vec![
        block(BlockKind::Math, &["a = b\n\nc = d $$"]),
        table(2, &["Code", "Note", "`a|b`", "x | y"]),
        block(BlockKind::Paragraph, &["$$\nx\n$$"]),
    ];
    let markdown = BlockData::to_markdown(&document);
    let mut expected = document.clone();
    expected[2] = block(BlockKind::Math, &["x"]);
    assert_eq!(BlockData::from_markdown(&markdown), expected, "{markdown}");
}

#[test]
fn one_line_fields_keep_one_line() {
    let document = vec![
        block(BlockKind::Heading(2), &["Two\nlines"]),
        block(BlockKind::Toggle(true), &["Title\nrest", "Body"]),
        block(BlockKind::Image(Media::new("a.png")), &["Cap\ntion"]),
    ];
    assert_eq!(
        BlockData::from_markdown(&BlockData::to_markdown(&document)),
        vec![
            block(BlockKind::Heading(2), &["Two lines"]),
            block(BlockKind::Toggle(true), &["Title rest", "Body"]),
            block(BlockKind::Image(Media::new("a.png")), &["Cap tion"]),
        ]
    );
}

#[test]
fn toggle_tags_read_as_html_reads_them() {
    let document = vec![
        block(
            BlockKind::Toggle(true),
            &[
                r#"a <span title="</summary>">b</span>"#,
                "<details-extra>\n\nkept",
            ],
        ),
        block(BlockKind::Paragraph, &["After"]),
    ];
    let markdown = BlockData::to_markdown(&document);
    assert_eq!(BlockData::from_markdown(&markdown), document, "{markdown}");
    let source = "<DETAILS class=\"x\" OPEN>\n<summary>T</summary>\n\nB\n\n</details>\n";
    assert_eq!(
        BlockData::from_markdown(source),
        vec![block(BlockKind::Toggle(true), &["T", "B"])]
    );
    let bare = "<details title=\"open\">\n\nBody\n\n</details>\n";
    assert_eq!(
        BlockData::from_markdown(bare),
        vec![block(BlockKind::Toggle(false), &["", "Body"])]
    );
}

#[test]
fn an_unquoted_value_runs_to_a_space() {
    let source = "<details data-url=https://ely.dev/open>\n\nBody\n\n</details>\n";
    assert_eq!(
        BlockData::from_markdown(source),
        vec![block(BlockKind::Toggle(false), &["", "Body"])]
    );
}
