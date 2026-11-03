use markdown::mdast;

use super::{Node, Reader};
use crate::documents::markdown::span;

/// A heading anywhere in the file.
#[derive(Clone, Debug, PartialEq)]
pub struct DocumentHeading {
    pub depth: u8,
    pub title: String,
    /// Where its source starts, in bytes.
    pub at: usize,
}

/// A GitHub alert's kind, named by its marker.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlertKind {
    Note,
    Tip,
    Important,
    Warning,
    Caution,
}

impl AlertKind {
    /// The title GitHub shows.
    pub fn title(self) -> &'static str {
        match self {
            Self::Note => "Note",
            Self::Tip => "Tip",
            Self::Important => "Important",
            Self::Warning => "Warning",
            Self::Caution => "Caution",
        }
    }

    /// The kind `line` marks, as `[!NOTE]` in any case.
    fn marked(line: &str) -> Option<Self> {
        let name = line
            .trim_matches([' ', '\t'])
            .strip_prefix("[!")?
            .strip_suffix(']')?;
        [
            Self::Note,
            Self::Tip,
            Self::Important,
            Self::Warning,
            Self::Caution,
        ]
        .into_iter()
        .find(|kind| kind.title().eq_ignore_ascii_case(name))
    }
}

impl Reader<'_> {
    /// A root block; alerts and `[toc]` live only here.
    pub(crate) fn top(&self, node: &mdast::Node, out: &mut Vec<Node>) {
        if let Some(alert) = self.alert(node) {
            return out.push(alert);
        }
        match is_toc(node, self.source) {
            true => out.push(Node::Toc(self.headings.clone())),
            false => self.block(node, out),
        }
    }

    /// A quote whose first line marks it, with more after.
    fn alert(&self, node: &mdast::Node) -> Option<Node> {
        let mdast::Node::Blockquote(quote) = node else {
            return None;
        };
        let (kind, parts) = alert(quote)?;
        let mut body = Vec::new();
        self.blocks(&parts, &mut body);
        Some(Node::Alert { kind, body })
    }
}

/// A quote's alert kind and its blocks after the marker.
pub fn alert(quote: &mdast::Blockquote) -> Option<(AlertKind, Vec<mdast::Node>)> {
    let [mdast::Node::Paragraph(paragraph), rest @ ..] = &quote.children[..] else {
        return None;
    };
    let (kind, after) = marker(&paragraph.children)?;
    if after.is_empty() && rest.is_empty() {
        return None;
    }
    let head = (!after.is_empty()).then(|| {
        mdast::Node::Paragraph(mdast::Paragraph {
            children: after,
            position: paragraph.position.clone(),
        })
    });
    Some((kind, head.into_iter().chain(rest.iter().cloned()).collect()))
}

/// A paragraph that is `[toc]` alone, as Typora reads it.
pub fn is_toc(node: &mdast::Node, source: &str) -> bool {
    matches!(node, mdast::Node::Paragraph(_))
        && source[span(node)].trim().eq_ignore_ascii_case("[toc]")
}

/// A plain first line naming a kind, and what follows.
fn marker(children: &[mdast::Node]) -> Option<(AlertKind, Vec<mdast::Node>)> {
    let mut line = String::new();
    for (at, child) in children.iter().enumerate() {
        let head = match child {
            mdast::Node::Text(text) => match text.value.find(['\n', '\r']) {
                None => {
                    line.push_str(&text.value);
                    continue;
                }
                Some(end) => {
                    line.push_str(&text.value[..end]);
                    let after = &text.value[end..];
                    let after = after.strip_prefix("\r\n").unwrap_or(&after[1..]);
                    let mut kept = text.clone();
                    kept.value = after.to_string();
                    (!after.is_empty()).then_some(mdast::Node::Text(kept))
                }
            },
            mdast::Node::Break(_) => None,
            _ => return None,
        };
        let after = head.into_iter().chain(children[at + 1..].iter().cloned());
        return Some((AlertKind::marked(&line)?, after.collect()));
    }
    Some((AlertKind::marked(&line)?, Vec::new()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::documents::tree::{Style, parse};

    fn alert(source: &str) -> Option<(AlertKind, Vec<Node>)> {
        match parse(source).into_iter().next() {
            Some(Node::Alert { kind, body }) => Some((kind, body)),
            _ => None,
        }
    }

    fn text(nodes: &[Node]) -> Vec<String> {
        nodes
            .iter()
            .map(|node| match node {
                Node::Paragraph(line) => line.text.clone(),
                Node::Heading(_, line) => format!("# {}", line.text),
                other => format!("{other:?}"),
            })
            .collect()
    }

    #[test]
    fn alerts_read_as_github_reads_them() {
        let note = alert("> [!NOTE]\n> one").expect("an alert");
        assert_eq!(
            (note.0, text(&note.1)),
            (AlertKind::Note, vec!["one".into()])
        );
        let cases = [
            ("> [!tip]\n> two", Some(AlertKind::Tip)),
            ("> [!NOTE]   \n> trailing", Some(AlertKind::Note)),
            (">   [!TIP]\n> leading", Some(AlertKind::Tip)),
            ("> [!Warning]\n>\n> after a blank", Some(AlertKind::Warning)),
            ("> [!WARNING] same line", None),
            ("> [!IMPORTANT]", None),
            ("> [!FOO]\n> five", None),
            ("> text\n> [!NOTE]", None),
            ("> \\[!NOTE]\n> escaped", Some(AlertKind::Note)),
            ("> [!NOT&#69;]\n> referenced", Some(AlertKind::Note)),
            ("> [!NOTE]  \n> hard line", Some(AlertKind::Note)),
            ("> [!NOTE]\rcarriage", Some(AlertKind::Note)),
            ("> [!NOTE]\nlazy", Some(AlertKind::Note)),
            ("> [!NOTE]\u{a0}\n> no-break space", None),
            ("> **[!TIP]**\n> strong", None),
            ("> <span>[!WARNING]</span>\n> tag", None),
            ("[!NOTE]: https://example.com\n\n> [!NOTE]\n> a link", None),
        ];
        for (source, kind) in cases {
            assert_eq!(alert(source).map(|found| found.0), kind, "{source:?}");
        }
        let nested = parse("- > [!CAUTION]\n  > in a list");
        assert!(matches!(&nested[0], Node::List { .. }), "{nested:?}");
        let blocks = alert("> [!NOTE]\n> # head\n> - item").expect("an alert");
        assert!(matches!(
            &blocks.1[..],
            [Node::Heading(1, _), Node::List { .. }]
        ));
        let styled = alert("> [!CAUTION]\n> a **b**\n> c").expect("an alert");
        let Node::Paragraph(line) = &styled.1[0] else {
            panic!("a paragraph, not {:?}", styled.1)
        };
        let bold = 2..3;
        assert_eq!(
            (line.text.as_str(), &line.styles[..]),
            ("a b\nc", &[(bold, Style::Strong)][..])
        );
    }

    #[test]
    fn a_toc_paragraph_lists_every_heading() {
        let nodes = parse("[toc]\n\n# A\n\n> ## B :smile:\n");
        let Node::Toc(headings) = &nodes[0] else {
            panic!("a toc, not {nodes:?}")
        };
        let listed: Vec<(u8, &str, usize)> = headings
            .iter()
            .map(|heading| (heading.depth, heading.title.as_str(), heading.at))
            .collect();
        assert_eq!(listed, [(1, "A", 7), (2, "B 😄", 14)]);
        assert!(matches!(&parse("  [TOC]  ")[0], Node::Toc(_)));
        assert!(matches!(&parse("[toc] now")[0], Node::Paragraph(_)));
        assert!(matches!(&parse("> [toc]")[0], Node::Quote(_)));
    }
}
