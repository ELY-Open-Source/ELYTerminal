use std::ops::Range;

mod tex;

use gpui::App;
use markdown::{Constructs, ParseOptions, mdast};
use unicode_segmentation::UnicodeSegmentation;

use crate::{documents::render::HEAVY, forms::Highlight, theme::ActiveTheme};

/// GitHub's flavor, and math between dollars.
pub(crate) fn options() -> ParseOptions {
    ParseOptions {
        constructs: Constructs {
            math_flow: true,
            math_text: true,
            ..Constructs::gfm()
        },
        ..ParseOptions::gfm()
    }
}

/// `text` as a syntax tree.
pub(crate) fn tree(text: &str) -> mdast::Node {
    parsed(text, &options())
}

fn parsed(text: &str, options: &ParseOptions) -> mdast::Node {
    let swapped = tex::swapped(text, options);
    let mut root =
        markdown::to_mdast(&swapped.text, options).expect("markdown without MDX always parses");
    tex::mend(&mut root, &swapped);
    root
}

/// An inline formula's TeX: inside `$` runs or `\(` pairs.
pub(crate) fn math_inside(source: &str, whole: Range<usize>) -> Range<usize> {
    match source[whole.clone()].starts_with('$') {
        true => crate::documents::tree::inside(source, whole, '$'),
        false => whole.start + 2..whole.end - 2,
    }
}

/// How a whole file parses: front matter too.
pub(crate) fn file_options() -> ParseOptions {
    let mut options = options();
    options.constructs.frontmatter = true;
    options
}

/// A whole file as a syntax tree.
pub fn file_tree(text: &str) -> mdast::Node {
    parsed(text, &file_options())
}

/// The bytes of the source a parsed node spans.
pub(crate) fn span(node: &mdast::Node) -> Range<usize> {
    let at = node.position().expect("a parsed node has its position");
    at.start.offset..at.end.offset
}

/// How a stretch of markdown prints while it is edited.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mark {
    Strong,
    Emphasis,
    Strike,
    Code,
    Link,
    Heading(u8),
    Quote,
    /// The characters that write the marks: asterisks, backticks, brackets, hashes.
    Syntax,
}

/// The run of `ch` that starts `text`, in bytes.
fn leading(text: &str, ch: char) -> usize {
    text.chars()
        .take_while(|found| *found == ch)
        .map(char::len_utf8)
        .sum()
}

/// Bytes at each end of `span` that write `mark`.
pub(crate) fn edge(span: &str, mark: Mark) -> usize {
    let first = span.chars().next().expect("a styled span has its marker");
    let run = leading(span, first).min(span.len() / 2);
    match mark {
        Mark::Emphasis => run.min(first.len_utf8()),
        _ => run,
    }
}

/// Every node under `node`, itself first, in source order.
pub(crate) fn walk<'a>(node: &'a mdast::Node, out: &mut Vec<&'a mdast::Node>) {
    out.push(node);
    for child in node.children().into_iter().flatten() {
        walk(child, out);
    }
}

/// The marks over `text`, with the syntax writing each.
pub fn marks(text: &str) -> Vec<(Range<usize>, Mark)> {
    let root = file_tree(text);
    let mut nodes = Vec::new();
    walk(&root, &mut nodes);
    let mut out = Vec::new();
    for node in nodes {
        let range = span(node);
        let written = &text[range.clone()];
        let styled = match node {
            mdast::Node::Strong(_) => Some(Mark::Strong),
            mdast::Node::Emphasis(_) => Some(Mark::Emphasis),
            mdast::Node::Delete(_) => Some(Mark::Strike),
            mdast::Node::InlineCode(_) => Some(Mark::Code),
            _ => None,
        };
        if let Some(mark) = styled {
            let edge = edge(written, mark);
            out.push((range.clone(), mark));
            out.push((range.start..range.start + edge, Mark::Syntax));
            out.push((range.end - edge..range.end, Mark::Syntax));
            continue;
        }
        match node {
            mdast::Node::Link(mdast::Link { children, .. })
            | mdast::Node::LinkReference(mdast::LinkReference { children, .. }) => {
                out.push((range.clone(), Mark::Link));
                if let (Some(first), Some(last)) = (children.first(), children.last()) {
                    let inner = span(first).start..span(last).end;
                    out.push((range.start..inner.start, Mark::Syntax));
                    out.push((inner.end..range.end, Mark::Syntax));
                }
            }
            mdast::Node::Heading(heading) => {
                let hashes = leading(written, '#');
                out.push((range.clone(), Mark::Heading(heading.depth)));
                if hashes > 0 {
                    let spaced = hashes + leading(&written[hashes..], ' ');
                    out.push((range.start..range.start + spaced, Mark::Syntax));
                }
            }
            mdast::Node::Blockquote(_) => {
                out.push((range.clone(), Mark::Quote));
                let mut at = range.start;
                for line in written.split_inclusive('\n') {
                    let marker = line.len() - line.trim_start_matches(['>', ' ']).len();
                    out.push((at..at + marker, Mark::Syntax));
                    at += line.len();
                }
            }
            _ => {}
        }
    }
    out.retain(|(range, _)| !range.is_empty());
    out
}

/// Markdown styled as typed; its syntax recedes.
pub fn markdown_highlights(text: &str, cx: &App) -> Vec<(Range<usize>, Highlight)> {
    let colors = &cx.theme().colors;
    let marks = marks(text);
    // A sixth-level heading reads muted, as drawn.
    let muted: Vec<Range<usize>> = marks
        .iter()
        .filter(|(_, mark)| *mark == Mark::Heading(6))
        .map(|(range, _)| range.clone())
        .collect();
    marks
        .into_iter()
        .map(|(range, mark)| {
            let within =
                |outer: &Range<usize>| outer.start <= range.start && range.end <= outer.end;
            let plain = Highlight::new(match muted.iter().any(within) {
                true => colors.fg_muted,
                false => colors.fg,
            });
            let range = match mark {
                Mark::Strike => {
                    let span = &text[range.clone()];
                    let edge = span.len() - span.trim_start_matches('~').len();
                    range.start + edge..range.end - edge
                }
                _ => range,
            };
            let highlight = match mark {
                Mark::Strong | Mark::Heading(_) => Highlight {
                    weight: Some(HEAVY),
                    ..plain
                },
                Mark::Emphasis => Highlight {
                    italic: true,
                    ..plain
                },
                Mark::Strike => Highlight {
                    strike: true,
                    ..Highlight::new(colors.fg_muted)
                },
                Mark::Code => Highlight {
                    background: Some(colors.sunken),
                    ..Highlight::new(colors.fg)
                },
                Mark::Link => Highlight::new(colors.link),
                Mark::Quote => Highlight::new(colors.fg_muted),
                Mark::Syntax => Highlight::new(colors.fg_subtle),
            };
            (range, highlight)
        })
        .collect()
}

/// The `[text](address)` link around byte `at`, if any.
pub fn link_at(text: &str, at: usize) -> Option<(Range<usize>, String, String)> {
    let root = tree(text);
    let mut nodes = Vec::new();
    walk(&root, &mut nodes);
    nodes.into_iter().find_map(|node| match node {
        mdast::Node::Link(link) => {
            let range = span(node);
            let written = &text[range.clone()];
            if !(range.start <= at && at <= range.end && written.starts_with('[')) {
                return None;
            }
            let label = &written[1..written.rfind("](")?];
            Some((range, label.to_string(), link.url.clone()))
        }
        _ => None,
    })
}

/// Top-level blocks as source ranges; definitions excluded.
pub fn block_ranges(text: &str) -> Vec<Range<usize>> {
    let root = tree(text);
    root.children()
        .expect("the root holds the blocks")
        .iter()
        .filter(|block| !matches!(block, mdast::Node::Definition(_)))
        .map(|block| {
            let range = span(block);
            let kept = text[range.clone()].trim_end_matches(['\n', '\r']).len();
            range.start..range.start + kept
        })
        .collect()
}

/// Words in `text`; each ideograph counts as one.
pub fn words(text: &str) -> usize {
    text.unicode_words().count()
}

/// Words an adult reads in a minute, on average.
const PACE: usize = 238;

/// Minutes to read `words`; at least one if any.
pub fn reading_minutes(words: usize) -> usize {
    words.div_ceil(PACE)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn found(text: &str) -> Vec<(&str, Mark)> {
        marks(text)
            .into_iter()
            .map(|(range, mark)| (&text[range], mark))
            .collect()
    }

    #[test]
    fn styles_carry_their_markers_as_syntax() {
        use Mark::*;
        assert_eq!(
            found("a **bold** and *it* ~~no~~"),
            [
                ("**bold**", Strong),
                ("**", Syntax),
                ("**", Syntax),
                ("*it*", Emphasis),
                ("*", Syntax),
                ("*", Syntax),
                ("~~no~~", Strike),
                ("~~", Syntax),
                ("~~", Syntax),
            ]
        );
    }

    #[test]
    fn code_links_and_headings_mark_their_syntax() {
        use Mark::*;
        assert_eq!(found("`x`"), [("`x`", Code), ("`", Syntax), ("`", Syntax)]);
        assert_eq!(
            found("see [the docs](https://ely.dev)"),
            [
                ("[the docs](https://ely.dev)", Link),
                ("[", Syntax),
                ("](https://ely.dev)", Syntax)
            ]
        );
        assert_eq!(found("## Lift"), [("## Lift", Heading(2)), ("## ", Syntax)]);
    }

    #[test]
    fn reference_links_mark_their_syntax() {
        use Mark::*;
        let defined = "\n\n[site]: https://ely.dev\n";
        for (link, open, close) in [
            ("[the site][site]", "[", "][site]"),
            ("[site][]", "[", "][]"),
            ("[site]", "[", "]"),
        ] {
            let text = format!("{link}{defined}");
            assert_eq!(
                found(&text),
                [(link, Link), (open, Syntax), (close, Syntax)],
                "{link}"
            );
        }
    }

    #[test]
    fn a_link_gives_its_range_text_and_address() {
        let text = "see [the docs](https://ely.dev) or <https://x.dev>";
        assert_eq!(
            link_at(text, 6),
            Some((4..31, "the docs".to_string(), "https://ely.dev".to_string()))
        );
        assert_eq!(link_at(text, 1), None);
        assert_eq!(link_at(text, 40), None, "an autolink has no text to edit");
    }

    #[test]
    fn blocks_are_the_top_level_stretches() {
        let text = "# Lift\n\nBlend **it**.\n\n- a\n- b\n\n---\n";
        let blocks: Vec<&str> = block_ranges(text)
            .into_iter()
            .map(|range| &text[range])
            .collect();
        assert_eq!(blocks, ["# Lift", "Blend **it**.", "- a\n- b", "---"]);
    }

    #[test]
    fn reading_takes_a_minute_per_pace() {
        assert_eq!(words("Blend toward white, by lift."), 5);
        assert_eq!(words("你好世界"), 4);
        assert_eq!(
            (reading_minutes(0), reading_minutes(1), reading_minutes(239)),
            (0, 1, 2)
        );
    }
}
