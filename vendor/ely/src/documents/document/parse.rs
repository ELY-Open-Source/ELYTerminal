use std::ops::Range;

use markdown::mdast;

use crate::documents::{
    markdown::{file_tree, span},
    tree::{DocumentHeading, Node, Reader},
};

use super::TargetBlock;

/// A top-level block: its source, and what it draws.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Block {
    pub range: Range<usize>,
    pub kind: TargetBlock,
    /// None shows a link definition as written.
    pub nodes: Option<Vec<Node>>,
}

/// A file read as blocks and headings.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Parsed {
    /// The text read, to compare blocks across reads.
    pub text: String,
    pub blocks: Vec<Block>,
    pub headings: Vec<DocumentHeading>,
}

/// `text` read whole; every byte is in or between blocks.
pub(crate) fn parsed(text: &str) -> Parsed {
    let root = file_tree(text);
    let reader = Reader::new(text, &root);
    let blocks = root
        .children()
        .expect("the root holds the blocks")
        .iter()
        .map(|node| {
            let range = span(node);
            let kept = text[range.clone()].trim_end_matches(['\n', '\r']).len();
            let raw = matches!(node, mdast::Node::Definition(_));
            let nodes = (!raw).then(|| {
                let mut out = Vec::new();
                reader.top(node, &mut out);
                out.iter_mut().for_each(|drawn| drawn.rebase(range.start));
                out
            });
            Block {
                range: range.start..range.start + kept,
                kind: TargetBlock::of(node, text),
                nodes,
            }
        })
        .collect();
    Parsed {
        text: text.to_string(),
        blocks,
        headings: reader.headings,
    }
}

/// Blocks alike at both ends: `(head, tail)` counts to keep.
pub(crate) fn alike(old: &Parsed, new: &Parsed) -> (usize, usize) {
    let same = |a: &Block, b: &Block| {
        a.nodes == b.nodes && old.text[a.range.clone()] == new.text[b.range.clone()]
    };
    let head = old
        .blocks
        .iter()
        .zip(&new.blocks)
        .take_while(|(a, b)| same(a, b))
        .count();
    let room = old.blocks.len().min(new.blocks.len()) - head;
    let tail = old
        .blocks
        .iter()
        .rev()
        .zip(new.blocks.iter().rev())
        .take(room)
        .take_while(|(a, b)| same(a, b))
        .count();
    (head, tail)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::documents::tree::Style;

    #[test]
    fn every_block_is_shown_definitions_as_written() {
        let text = "---\ntitle: a\n---\n\n# Lift\n\n[site]: https://ely.dev\n\n<div>x</div>\n\n> ## Quoted\n";
        let found = parsed(text);
        let shown: Vec<(&str, bool)> = found
            .blocks
            .iter()
            .map(|block| (&text[block.range.clone()], block.nodes.is_some()))
            .collect();
        assert_eq!(
            shown,
            [
                ("---\ntitle: a\n---", true),
                ("# Lift", true),
                ("[site]: https://ely.dev", false),
                ("<div>x</div>", true),
                ("> ## Quoted", true),
            ]
        );
        let drawn = |ix: usize| found.blocks[ix].nodes.clone().expect("drawn");
        let text = "title: a".into();
        assert_eq!(drawn(0), [Node::FrontMatter { text, from: 4 }]);
        assert!(matches!(&drawn(3)[..], [Node::Paragraph(line)] if line.text == "x"));
        let headings: Vec<(u8, &str, usize)> = found
            .headings
            .iter()
            .map(|heading| (heading.depth, heading.title.as_str(), heading.at))
            .collect();
        assert_eq!(headings, [(1, "Lift", 18), (2, "Quoted", 67)]);
    }

    #[test]
    fn the_extended_syntax_draws_as_content() {
        let text = "| a |\n| - |\n| 1 |\n\n- [x] done\n\n~~gone~~\n\n$$\nx^2\n$$\n\n```mermaid\ngraph TD\nA-->B\n```\n\nNote[^1]\n\n[^1]: foot\n";
        let found = parsed(text);
        let drawn: Vec<&Node> = found
            .blocks
            .iter()
            .map(|block| match &block.nodes {
                Some(nodes) => &nodes[0],
                None => panic!("{:?} shows raw", &text[block.range.clone()]),
            })
            .collect();
        assert!(matches!(drawn[0], Node::Table { .. }));
        assert!(matches!(drawn[1], Node::List { items, .. } if items[0].0 == Some(true)));
        assert!(
            matches!(drawn[2], Node::Paragraph(line) if line.styles.iter().any(|(_, style)| *style == Style::Strike))
        );
        assert!(matches!(drawn[3], Node::Math(formula) if formula == "x^2"));
        assert!(
            matches!(drawn[4], Node::Code { language: Some(language), .. } if language == "mermaid")
        );
        assert!(
            matches!(drawn[5], Node::Paragraph(line) if line.styles.iter().any(|(_, style)| *style == Style::Footnote))
        );
        assert!(matches!(drawn[6], Node::Footnote { label, .. } if label == "1"));
    }

    #[test]
    fn an_edit_keeps_the_blocks_around_it() {
        let old = parsed("# A\n\nb\n\nc\n\nd\n");
        let new = parsed("# A\n\nb and more\n\nc\n\nd\n");
        assert_eq!(alike(&old, &new), (1, 2));
        assert_eq!(alike(&old, &old), (4, 0), "all kept from the head");
        let shorter = parsed("# A\n");
        assert_eq!(alike(&old, &shorter), (1, 0));
    }

    #[test]
    fn pictures_and_code_count_from_their_block() {
        let old = parsed(
            "a

![p](p.png)

```
x
```
",
        );
        let new = parsed(
            "a and more

![p](p.png)

```
x
```
",
        );
        assert_eq!(alike(&old, &new), (0, 2));
    }

    #[test]
    fn a_reference_resolves_across_blocks() {
        let found = parsed("[site]\n\n[site]: https://ely.dev\n");
        let Some(nodes) = &found.blocks[0].nodes else {
            panic!("the paragraph draws")
        };
        let Node::Paragraph(text) = &nodes[0] else {
            panic!("a paragraph, not {nodes:?}")
        };
        assert_eq!(text.links, [(0..4, "https://ely.dev".to_string())]);
    }
}
