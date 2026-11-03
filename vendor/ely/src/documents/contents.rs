use gpui::SharedString;

use super::tree::{Node, parse};

/// The anchor of the document's `ordinal`th heading.
pub(crate) fn anchor(ordinal: usize) -> SharedString {
    format!("heading-{ordinal}").into()
}

/// Headings in order for a contents list: anchor, text, depth.
pub fn outline(markdown: &str) -> Vec<(SharedString, SharedString, usize)> {
    fn walk(nodes: &[Node], out: &mut Vec<(SharedString, SharedString, usize)>) {
        for node in nodes {
            match node {
                Node::Heading(level, inline) => {
                    let ordinal = out.len();
                    out.push((
                        anchor(ordinal),
                        inline.text.clone().into(),
                        *level as usize - 1,
                    ));
                }
                Node::Quote(inner) | Node::Footnote { body: inner, .. } => walk(inner, out),
                Node::List { items, .. } => {
                    for (_, body) in items {
                        walk(body, out);
                    }
                }
                _ => {}
            }
        }
    }
    let mut out = Vec::new();
    walk(&parse(markdown), &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::outline;

    #[test]
    fn the_outline_numbers_headings_in_order_with_their_depth() {
        let found =
            outline("# Lift\n\ntext\n\n## In light\n\n> ### Quoted\n\n- #### Listed\n\n## In dark");
        let shape: Vec<(&str, &str, usize)> = found
            .iter()
            .map(|(anchor, text, depth)| (anchor.as_ref(), text.as_ref(), *depth))
            .collect();
        assert_eq!(
            shape,
            [
                ("heading-0", "Lift", 0),
                ("heading-1", "In light", 1),
                ("heading-2", "Quoted", 2),
                ("heading-3", "Listed", 3),
                ("heading-4", "In dark", 1),
            ]
        );
    }
}
