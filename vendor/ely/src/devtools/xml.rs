use gpui::{
    App, ClipboardItem, ElementId, Entity, IntoElement, ParentElement, RenderOnce, SharedString,
    Styled, Window, div,
};

use super::datum::Unread;
use crate::{
    feedback::InlineMessage,
    lists::{Tree, TreeNode},
    primitives::Severity,
    theme::{ActiveTheme, TextSize},
};

/// A piece of an XML document: an element, or the text between tags.
#[derive(Clone, Debug, PartialEq)]
pub enum XmlNode {
    Element(XmlElement),
    Text(SharedString),
}

/// An element: its name, its attributes in order, and what it holds.
#[derive(Clone, Debug, PartialEq)]
pub struct XmlElement {
    pub name: SharedString,
    pub attributes: Vec<(SharedString, SharedString)>,
    pub children: Vec<XmlNode>,
}

struct Reader<'a> {
    text: &'a str,
    at: usize,
}

impl<'a> Reader<'a> {
    /// Fails on the line `ahead` bytes past where the reader stands.
    fn fail_at<T>(&self, ahead: usize, why: impl Into<String>) -> Result<T, Unread> {
        Err(Unread {
            line: self.text[..self.at + ahead].matches('\n').count() + 1,
            why: why.into(),
        })
    }

    fn fail<T>(&self, why: impl Into<String>) -> Result<T, Unread> {
        self.fail_at(0, why)
    }

    /// The `len` bytes where the reader stands, their entity references read.
    fn decoded(&self, len: usize) -> Result<String, Unread> {
        let raw = &self.rest()[..len];
        let mut out = String::with_capacity(len);
        let mut done = 0;
        while let Some(amp) = raw[done..].find('&').map(|at| done + at) {
            out.push_str(&raw[done..amp]);
            let Some(end) = raw[amp..].find(';').map(|at| amp + at) else {
                return self.fail_at(amp, "& starts no entity");
            };
            let name = &raw[amp + 1..end];
            if name.is_empty() || name.contains(|c: char| c.is_whitespace() || c == '&') {
                return self.fail_at(amp, "& starts no entity");
            }
            let number = match name.strip_prefix("#x") {
                Some(hex) => u32::from_str_radix(hex, 16).ok(),
                None => name.strip_prefix('#').and_then(|dec| dec.parse().ok()),
            };
            out.push(match name {
                "amp" => '&',
                "lt" => '<',
                "gt" => '>',
                "quot" => '"',
                "apos" => '\'',
                _ => match number.and_then(char::from_u32) {
                    Some(ch) => ch,
                    None => return self.fail_at(amp, format!("&{name}; is no entity")),
                },
            });
            done = end + 1;
        }
        out.push_str(&raw[done..]);
        Ok(out)
    }

    fn rest(&self) -> &'a str {
        &self.text[self.at..]
    }

    /// Steps past what `open` starts, through `close`.
    fn skip(&mut self, open: &str, close: &str) -> Result<(), Unread> {
        match self.rest().find(close) {
            Some(end) => {
                self.at += end + close.len();
                Ok(())
            }
            None => self.fail(format!("{open} is never closed by {close}")),
        }
    }

    fn name(&mut self) -> SharedString {
        let rest = self.rest();
        let end = rest
            .find(|c: char| c.is_whitespace() || matches!(c, '>' | '/' | '='))
            .unwrap_or(rest.len());
        self.at += end;
        rest[..end].to_string().into()
    }

    fn spaces(&mut self) {
        let rest = self.rest();
        self.at += rest.len() - rest.trim_start().len();
    }

    fn element(&mut self) -> Result<XmlElement, Unread> {
        self.at += 1;
        let name = self.name();
        if name.is_empty() {
            return self.fail("a tag has no name");
        }
        let mut attributes = Vec::new();
        loop {
            self.spaces();
            if self.rest().starts_with("/>") {
                self.at += 2;
                return Ok(XmlElement {
                    name,
                    attributes,
                    children: Vec::new(),
                });
            }
            if self.rest().starts_with('>') {
                self.at += 1;
                break;
            }
            let key = self.name();
            self.spaces();
            if key.is_empty() || !self.rest().starts_with('=') {
                return self.fail(format!("<{name}> has an attribute without a value"));
            }
            self.at += 1;
            self.spaces();
            let Some(quote) = self
                .rest()
                .chars()
                .next()
                .filter(|c| matches!(c, '"' | '\''))
            else {
                return self.fail(format!("{key} in <{name}> is not quoted"));
            };
            self.at += 1;
            let Some(end) = self.rest().find(quote) else {
                return self.fail(format!("{key} in <{name}> is never closed"));
            };
            attributes.push((key, self.decoded(end)?.into()));
            self.at += end + 1;
        }
        let children = self.content()?;
        if !self.rest().starts_with("</") {
            return self.fail(format!("<{name}> is never closed"));
        }
        self.at += 2;
        let closing = self.name();
        self.spaces();
        if closing != name || !self.rest().starts_with('>') {
            return self.fail(format!("</{closing}> closes <{name}>"));
        }
        self.at += 1;
        Ok(XmlElement {
            name,
            attributes,
            children,
        })
    }

    /// What lies between an element's tags, up to its closing tag or the end.
    fn content(&mut self) -> Result<Vec<XmlNode>, Unread> {
        let mut children = Vec::new();
        while !self.rest().is_empty() && !self.rest().starts_with("</") {
            let rest = self.rest();
            if rest.starts_with("<!--") {
                self.skip("<!--", "-->")?;
            } else if let Some(data) = rest.strip_prefix("<![CDATA[") {
                let Some(end) = data.find("]]>") else {
                    return self.fail("a CDATA section is never closed");
                };
                children.push(XmlNode::Text(data[..end].to_string().into()));
                self.at += "<![CDATA[".len() + end + 3;
            } else if rest.starts_with("<?") || rest.starts_with("<!") {
                self.skip("<?", ">")?;
            } else if rest.starts_with('<') {
                children.push(XmlNode::Element(self.element()?));
            } else {
                let end = rest.find('<').unwrap_or(rest.len());
                let words = self.decoded(end)?;
                if !words.trim().is_empty() {
                    children.push(XmlNode::Text(words.trim().to_string().into()));
                }
                self.at += end;
            }
        }
        Ok(children)
    }
}

/// An XML document read into its root element. A tag that closes the wrong element, or is never closed, fails with its line.
pub fn read_xml(text: &str) -> Result<XmlElement, Unread> {
    let mut reader = Reader { text, at: 0 };
    let mut roots = reader.content()?.into_iter().filter_map(|node| match node {
        XmlNode::Element(element) => Some(element),
        XmlNode::Text(_) => None,
    });
    if reader.at < text.len() {
        return reader.fail("a closing tag with nothing open");
    }
    match (roots.next(), roots.next()) {
        (Some(root), None) => Ok(root),
        (None, _) => Err(Unread {
            line: 1,
            why: "the document holds no element".into(),
        }),
        (Some(_), Some(second)) => Err(Unread {
            line: 1,
            why: format!("a second root, <{}>", second.name),
        }),
    }
}

/// Rows for what `element` holds, each element's path listed in `open`.
fn nodes(element: &XmlElement, path: &str, open: &mut Vec<SharedString>) -> Vec<TreeNode> {
    let mut counts: Vec<(SharedString, usize)> = Vec::new();
    let mut texts = 0;
    element
        .children
        .iter()
        .map(|child| match child {
            XmlNode::Text(text) => {
                texts += 1;
                TreeNode::new(format!("{path}/text()[{texts}]"), text.clone())
            }
            XmlNode::Element(child) => {
                let seen = match counts.iter_mut().find(|(name, _)| *name == child.name) {
                    Some((_, count)) => {
                        *count += 1;
                        *count
                    }
                    None => {
                        counts.push((child.name.clone(), 1));
                        1
                    }
                };
                let here = format!("{path}/{}[{seen}]", child.name);
                open.push(here.clone().into());
                let attributes: Vec<String> = child
                    .attributes
                    .iter()
                    .map(|(key, value)| format!("{key}=\"{value}\""))
                    .collect();
                TreeNode::new(here.clone(), format!("<{}>", child.name))
                    .note(attributes.join(" "))
                    .children(nodes(child, &here, open))
            }
        })
        .collect()
}

/// The copied path, shown under the tree.
#[derive(Default)]
struct Copied(Option<SharedString>);

/// XML as a tree of elements, each with its attributes, and the text between tags; comments are dropped. Enter or a double press on a text copies its path. XML that does not read shows the line where it breaks.
#[derive(IntoElement)]
pub struct XmlViewer {
    id: ElementId,
    text: SharedString,
}

impl XmlViewer {
    pub fn new(id: impl Into<ElementId>, text: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
        }
    }
}

impl RenderOnce for XmlViewer {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let copied: Entity<Copied> =
            window.use_keyed_state((self.id.clone(), "copied"), cx, |_, _| Copied::default());
        let theme = cx.theme();
        let body = match read_xml(&self.text) {
            Err(unread) => {
                InlineMessage::new(Severity::Danger, unread.to_string()).into_any_element()
            }
            Ok(root) => {
                let path = format!("/{}", root.name);
                let mut open = vec![SharedString::from(path.clone())];
                let rows = nodes(&root, &path, &mut open);
                let copy = copied.clone();
                div()
                    .h(theme.list_max_height())
                    .child(
                        Tree::new(
                            (self.id, "tree"),
                            [TreeNode::new(path.clone(), format!("<{}>", root.name))
                                .children(rows)],
                        )
                        .open(open)
                        .on_activate(move |path, _, cx| {
                            log::info!("xml viewer: copied {path}");
                            cx.write_to_clipboard(ClipboardItem::new_string(path.to_string()));
                            copy.update(cx, |copied, cx| {
                                copied.0 = Some(path.clone());
                                cx.notify();
                            });
                        })
                        .size_full(),
                    )
                    .into_any_element()
            }
        };
        let note = copied.read(cx).0.clone().map(|path| {
            div()
                .text_size(theme.text_size(TextSize::Xs))
                .text_color(theme.colors.fg_subtle)
                .child(format!("Copied {path}"))
        });
        div().flex().flex_col().gap_2().child(body).children(note)
    }
}

#[cfg(test)]
mod tests {
    use super::{XmlNode, nodes, read_xml};

    #[test]
    fn entities_read_in_text_and_attributes() {
        let root =
            read_xml("<title a=\"Tom &amp; Jerry\">Tom &amp; Jerry &lt;3 &#65;&#x42;</title>")
                .expect("xml");
        assert_eq!(root.attributes, [("a".into(), "Tom & Jerry".into())]);
        assert_eq!(root.children, [XmlNode::Text("Tom & Jerry <3 AB".into())]);
        let unread = read_xml("<a>\n  &nope;</a>").expect_err("an unknown entity");
        assert_eq!(
            (unread.line, unread.why.as_str()),
            (2, "&nope; is no entity")
        );
        assert_eq!(
            read_xml("<a>Q&A</a>").expect_err("a bare ampersand").why,
            "& starts no entity"
        );
    }

    #[test]
    fn a_text_path_counts_text_nodes_from_one() {
        let root = read_xml("<a>x<b/>y</a>").expect("xml");
        let keys: Vec<_> = nodes(&root, "/a", &mut Vec::new())
            .into_iter()
            .map(|node| node.key)
            .collect();
        assert_eq!(keys, ["/a/text()[1]", "/a/b[1]", "/a/text()[2]"]);
    }

    #[test]
    fn elements_attributes_and_text_read_in_order() {
        let root = read_xml(
            "<?xml version=\"1.0\"?>\n<!-- feed -->\n<feed lang=\"en\">\n  <entry id='1'>First</entry>\n  <entry id=\"2\"/>\n</feed>",
        )
        .expect("xml");
        assert_eq!(root.name, "feed");
        assert_eq!(root.attributes, [("lang".into(), "en".into())]);
        let XmlNode::Element(first) = &root.children[0] else {
            panic!("an element")
        };
        assert_eq!(first.children, [XmlNode::Text("First".into())]);
        assert_eq!(root.children.len(), 2);
    }

    #[test]
    fn a_tag_that_closes_the_wrong_element_says_where() {
        let unread = read_xml("<a>\n  <b>\n  </a>\n</b>").expect_err("crossed tags");
        assert_eq!((unread.line, unread.why.as_str()), (3, "</a> closes <b>"));
        assert_eq!(
            read_xml("<a><b></b>").expect_err("unclosed").why,
            "<a> is never closed"
        );
    }
}
