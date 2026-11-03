use gpui::{App, ElementId, Entity, IntoElement, ParentElement, RenderOnce, Styled, Window, div};

use super::datum::{Datum, read_json};
use crate::{
    buttons::{Button, ButtonVariant},
    editor::CodeEditor,
    feedback::InlineMessage,
    primitives::Severity,
    theme::{ActiveTheme, Radius},
};

/// How much a value holds: its keys and items, all the way down.
pub(crate) fn tally(datum: &Datum) -> (usize, usize) {
    match datum {
        Datum::Map(entries) => {
            entries
                .iter()
                .fold((entries.len(), 0), |(keys, items), (_, value)| {
                    let (more, most) = tally(value);
                    (keys + more, items + most)
                })
        }
        Datum::List(values) => values
            .iter()
            .fold((0, values.len()), |(keys, items), value| {
                let (more, most) = tally(value);
                (keys + more, items + most)
            }),
        _ => (0, 0),
    }
}

/// JSON to write in the code editor, with Format to indent it, Minify to fold it onto one line, and a line that says whether it reads: how many keys and items it holds, or the line where it breaks. Both buttons wait for JSON that reads.
#[derive(IntoElement)]
pub struct JsonEditor {
    id: ElementId,
    editor: Entity<CodeEditor>,
}

impl JsonEditor {
    /// `editor` holds the text, which the owner keeps.
    pub fn new(id: impl Into<ElementId>, editor: &Entity<CodeEditor>) -> Self {
        Self {
            id: id.into(),
            editor: editor.clone(),
        }
    }
}

/// `text` indented, or on one line.
pub(crate) fn reformat(text: &str, pretty: bool) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(text).ok()?;
    let out = if pretty {
        serde_json::to_string_pretty(&value)
    } else {
        serde_json::to_string(&value)
    };
    Some(out.expect("a value read from JSON writes back"))
}

impl RenderOnce for JsonEditor {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let text = self.editor.read(cx).text().to_string();
        let read = read_json(&text);
        let theme = cx.theme();
        let status = match &read {
            Ok(datum) => {
                let (keys, items) = tally(datum);
                InlineMessage::new(
                    Severity::Success,
                    format!("Reads as JSON · {keys} keys · {items} items"),
                )
            }
            Err(unread) => InlineMessage::new(Severity::Danger, unread.to_string()),
        };
        let button = |label: &'static str, pretty: bool| {
            let editor = self.editor.clone();
            Button::new((self.id.clone(), label), label)
                .variant(ButtonVariant::Secondary)
                .disabled(read.is_err())
                .on_click(move |_, _, cx| {
                    let text = editor.read(cx).text().to_string();
                    let out = reformat(&text, pretty)
                        .expect("Format and Minify rest until the JSON reads");
                    log::info!("json editor: {label}");
                    editor.update(cx, |editor, cx| editor.set_text(out, cx));
                })
        };
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(button("Format", true))
                    .child(button("Minify", false)),
            )
            .child(
                div()
                    .h(theme.list_max_height())
                    .rounded(theme.radius(Radius::Lg))
                    .border_1()
                    .border_color(theme.colors.border)
                    .overflow_hidden()
                    .child(self.editor),
            )
            .child(status)
    }
}

#[cfg(test)]
mod tests {
    use super::{reformat, tally};
    use crate::devtools::datum::read_json;

    #[test]
    fn a_document_counts_its_keys_and_items_and_reformats_both_ways() {
        let text = r#"{"a": [1, {"b": 2}], "c": null}"#;
        assert_eq!(tally(&read_json(text).expect("json")), (3, 2));
        assert_eq!(
            reformat(text, false).as_deref(),
            Some(r#"{"a":[1,{"b":2}],"c":null}"#)
        );
        assert_eq!(
            reformat(r#"{"a":1}"#, true).as_deref(),
            Some("{\n  \"a\": 1\n}")
        );
        assert_eq!(reformat("{", true), None);
    }
}
