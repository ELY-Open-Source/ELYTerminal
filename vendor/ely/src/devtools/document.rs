use gpui::{
    App, ClipboardItem, ElementId, Entity, IntoElement, ParentElement, RenderOnce, SharedString,
    Styled, Window, div,
};

use super::datum::{Datum, Unread, read_json, read_yaml};
use crate::{
    feedback::InlineMessage,
    forms::{Input, TextInput},
    lists::{Tree, TreeNode},
    primitives::{Icon, IconName, Severity},
    theme::{ActiveTheme, IconSize, TextSize},
};

/// A scalar as a row shows it.
fn preview(datum: &Datum) -> SharedString {
    match datum {
        Datum::Null => "null".into(),
        Datum::Bool(on) => on.to_string().into(),
        Datum::Number(number) => number.clone(),
        Datum::Text(text) => format!("\"{text}\"").into(),
        Datum::List(items) => format!("[{}]", items.len()).into(),
        Datum::Map(entries) => format!("{{{}}}", entries.len()).into(),
    }
}

/// The path to a key under `path`, in JSONPath.
fn keyed(path: &str, key: &str) -> String {
    let plain = !key.is_empty() && key.chars().all(|c| c.is_alphanumeric() || c == '_');
    if plain {
        format!("{path}.{key}")
    } else {
        format!("{path}[\"{key}\"]")
    }
}

fn children<'a>(datum: &'a Datum, path: &str) -> Vec<(SharedString, String, &'a Datum)> {
    match datum {
        Datum::Map(entries) => entries
            .iter()
            .map(|(key, value)| (key.clone(), keyed(path, key), value))
            .collect(),
        Datum::List(items) => items
            .iter()
            .enumerate()
            .map(|(ix, item)| (format!("[{ix}]").into(), format!("{path}[{ix}]"), item))
            .collect(),
        _ => Vec::new(),
    }
}

/// Whether a key or a value at or under `datum` holds `query`.
fn holds(label: &str, datum: &Datum, query: &str) -> bool {
    label.to_lowercase().contains(query)
        || match datum {
            Datum::Map(_) | Datum::List(_) => children(datum, "")
                .iter()
                .any(|(label, _, child)| holds(label, child, query)),
            scalar => preview(scalar).to_lowercase().contains(query),
        }
}

/// Rows for what lies under `datum`, each keyed by its path. With a query, only rows that hold it, or lead to one that does, stay; each one kept is listed in `open`.
pub(crate) fn nodes(
    datum: &Datum,
    path: &str,
    query: &str,
    open: &mut Vec<SharedString>,
) -> Vec<TreeNode> {
    children(datum, path)
        .into_iter()
        .filter(|(label, _, child)| query.is_empty() || holds(label, child, query))
        .map(|(label, path, child)| {
            let node = TreeNode::new(path.clone(), label.clone()).note(preview(child));
            match child {
                Datum::Map(_) | Datum::List(_) => {
                    let whole = label.to_lowercase().contains(query);
                    if !query.is_empty() {
                        open.push(path.clone().into());
                    }
                    node.children(nodes(child, &path, if whole { "" } else { query }, open))
                }
                _ => node,
            }
        })
        .collect()
}

/// The copied path, shown under the tree.
#[derive(Default)]
struct Copied(Option<SharedString>);

/// A document read into a tree, or where it failed to read.
fn document(
    id: ElementId,
    read: Result<Datum, Unread>,
    search: &Entity<TextInput>,
    window: &mut Window,
    cx: &mut App,
) -> gpui::AnyElement {
    let copied = window.use_keyed_state((id.clone(), "copied"), cx, |_, _| Copied::default());
    let query = search.read(cx).text().trim().to_lowercase();
    let theme = cx.theme();
    let body = match read {
        Err(unread) => InlineMessage::new(Severity::Danger, unread.to_string()).into_any_element(),
        Ok(datum) => {
            let mut open = vec![SharedString::from("$")];
            let rows = nodes(&datum, "$", &query, &mut open);
            let root = TreeNode::new("$", "$").note(preview(&datum)).children(rows);
            let copy = copied.clone();
            div()
                .h(theme.list_max_height())
                .child(
                    Tree::new((id, "tree"), [root])
                        .open(open)
                        .on_activate(move |path, _, cx| {
                            log::info!("document: copied {path}");
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
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(Input::new(search).prefix(Icon::new(IconName::Search).size(IconSize::Sm)))
        .child(body)
        .children(note)
        .into_any_element()
}

/// JSON as a tree: each key or index with its value, maps and lists with their counts. A search keeps the rows that hold its words and opens the way to them; Enter or a double press on a value copies its path. JSON that does not read shows the line where it breaks.
#[derive(IntoElement)]
pub struct JsonViewer {
    id: ElementId,
    text: SharedString,
    search: Entity<TextInput>,
}

impl JsonViewer {
    /// `search` is the find field's text, which the owner keeps.
    pub fn new(
        id: impl Into<ElementId>,
        text: impl Into<SharedString>,
        search: &Entity<TextInput>,
    ) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
            search: search.clone(),
        }
    }
}

impl RenderOnce for JsonViewer {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        document(self.id, read_json(&self.text), &self.search, window, cx)
    }
}

/// YAML as a tree, as `JsonViewer` shows JSON: block maps and lists, with comments dropped.
#[derive(IntoElement)]
pub struct YamlViewer {
    id: ElementId,
    text: SharedString,
    search: Entity<TextInput>,
}

impl YamlViewer {
    /// `search` is the find field's text, which the owner keeps.
    pub fn new(
        id: impl Into<ElementId>,
        text: impl Into<SharedString>,
        search: &Entity<TextInput>,
    ) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
            search: search.clone(),
        }
    }
}

impl RenderOnce for YamlViewer {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        document(self.id, read_yaml(&self.text), &self.search, window, cx)
    }
}

#[cfg(test)]
mod tests {
    use super::nodes;
    use crate::devtools::datum::read_json;

    #[test]
    fn a_search_keeps_what_holds_it_and_opens_the_way() {
        let datum = read_json(
            r#"{"spec": {"replicas": 3, "image": "web:1.2"}, "kind": "Deployment", "odd key": 1}"#,
        )
        .expect("json");
        let mut open = Vec::new();
        let rows = nodes(&datum, "$", "web", &mut open);
        assert_eq!(rows.len(), 1, "only spec holds web");
        assert_eq!(open, ["$.spec"]);
        let mut all = Vec::new();
        let every = nodes(&datum, "$", "", &mut all);
        assert_eq!(every.len(), 3);
        assert_eq!(
            every[2].key, "$[\"odd key\"]",
            "a key beyond letters takes brackets"
        );
        assert!(all.is_empty(), "without a query nothing is opened");
    }
}
