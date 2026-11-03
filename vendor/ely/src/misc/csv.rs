use std::rc::Rc;

use gpui::{
    App, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce, SharedString,
    Window, div,
};

use super::{ImportDialog, ImportField};
use crate::{
    buttons::{Button, ButtonVariant},
    feedback::InlineMessage,
    forms::{Checkbox, Run},
    overlays::Dialog,
    primitives::Severity,
};

type OnRecords = Rc<dyn Fn(&[Vec<SharedString>], &mut Window, &mut App)>;

/// Rows of fields from CSV text, as RFC 4180 has it: commas between fields, quotes around a field that holds a comma, a quote or a line break, a doubled quote for a quote inside, and CRLF or LF between rows; a byte order mark before the first field is no part of it, and a last line break adds no row. Fails on a quote that never closes, naming the line it opened on; text after a closing quote; a quote inside a field it did not open; or a row whose field count differs from the first's.
pub fn read_csv(text: &str) -> Result<Vec<Vec<String>>, String> {
    let (mut rows, mut row, mut field) = (Vec::new(), Vec::new(), String::new());
    let (mut quoted, mut closed, mut line, mut opened) = (false, false, 1, 1);
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        match (quoted, ch) {
            (true, '"') if chars.peek() == Some(&'"') => {
                chars.next();
                field.push('"');
            }
            (true, '"') => (quoted, closed) = (false, true),
            (true, ch) => {
                line += usize::from(ch == '\n');
                field.push(ch);
            }
            (false, '"') if field.is_empty() && !closed => (quoted, opened) = (true, line),
            (false, '"') => {
                return Err(format!(
                    "line {line}: a quote inside a field it did not open"
                ));
            }
            (false, ',') => {
                row.push(std::mem::take(&mut field));
                closed = false;
            }
            (false, '\r') if chars.peek() == Some(&'\n') => {}
            (false, '\n') => {
                row.push(std::mem::take(&mut field));
                rows.push(std::mem::take(&mut row));
                (closed, line) = (false, line + 1);
            }
            (false, _) if closed => return Err(format!("line {line}: text after a closing quote")),
            (false, ch) => field.push(ch),
        }
    }
    if quoted {
        return Err(format!("line {opened}: a quote never closes"));
    }
    if !field.is_empty() || !row.is_empty() || closed {
        row.push(field);
        rows.push(row);
    }
    if let Some(first) = rows.first() {
        let width = first.len();
        if let Some((ix, ragged)) = rows.iter().enumerate().find(|(_, row)| row.len() != width) {
            return Err(format!(
                "row {} has {} fields; the first has {width}",
                ix + 1,
                ragged.len()
            ));
        }
    }
    Ok(rows)
}

/// What the importer read: the text, and its rows or why it does not read as CSV.
struct Reading {
    text: SharedString,
    rows: Result<Vec<Vec<String>>, String>,
}

/// Reads the text once, a file of no rows being a fault too, and logs what does not read.
fn read(id: &ElementId, text: SharedString) -> Reading {
    let rows = read_csv(&text).and_then(|rows| match rows.is_empty() {
        true => Err("it holds no rows".to_string()),
        false => Ok(rows),
    });
    if let Err(why) = &rows {
        log::warn!("csv importer {id:?}: {why}");
    }
    Reading { text, rows }
}

/// CSV text into records: read as RFC 4180, its first row naming the columns unless the box says otherwise, then mapped to the owner's fields through an `ImportDialog`. Text that does not read as CSV, or holds no rows, says why, with only Cancel. Each way out answers once. Render it while open.
#[derive(IntoElement)]
pub struct CsvImporter {
    id: ElementId,
    title: SharedString,
    text: SharedString,
    fields: Vec<ImportField>,
    on_import: OnRecords,
    on_cancel: Run,
}

impl CsvImporter {
    /// `on_import` gets a record per row, its fields in field order, empty where a field was left empty.
    pub fn new(
        id: impl Into<ElementId>,
        title: impl Into<SharedString>,
        text: impl Into<SharedString>,
        fields: impl IntoIterator<Item = ImportField>,
        on_import: impl Fn(&[Vec<SharedString>], &mut Window, &mut App) + 'static,
        on_cancel: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            text: text.into(),
            fields: fields.into_iter().collect(),
            on_import: Rc::new(on_import),
            on_cancel: Rc::new(on_cancel),
        }
    }
}

impl RenderOnce for CsvImporter {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id;
        let reading = window.use_keyed_state((id.clone(), "reading"), cx, {
            let (id, text) = (id.clone(), self.text.clone());
            move |_, _| read(&id, text)
        });
        if reading.read(cx).text != self.text {
            let fresh = read(&id, self.text.clone());
            reading.update(cx, |reading, _| *reading = fresh);
        }
        let headed = window.use_keyed_state((id.clone(), "headed"), cx, |_, _| true);
        let rows = reading.read(cx).rows.clone();
        let cancel = self.on_cancel;
        let rows = match rows {
            Ok(rows) => rows,
            Err(why) => {
                return Dialog::new(id.clone(), self.title, move |window, cx| cancel(window, cx))
                    .child(
                        div()
                            .debug_selector(|| "csv-unread".into())
                            .child(InlineMessage::new(
                                Severity::Danger,
                                format!("This file does not read as CSV: {why}."),
                            )),
                    )
                    .action(move |close| {
                        Button::new((id, "cancel"), "Cancel")
                            .variant(ButtonVariant::Ghost)
                            .on_click(move |_, window, cx| close(window, cx))
                    })
                    .into_any_element();
            }
        };
        let first_names = *headed.read(cx);
        let width = rows[0].len();
        let (columns, data): (Vec<SharedString>, Vec<Vec<SharedString>>) = {
            let shared = |row: &Vec<String>| {
                row.iter()
                    .map(|field| SharedString::from(field.clone()))
                    .collect::<Vec<_>>()
            };
            match first_names {
                true => (shared(&rows[0]), rows[1..].iter().map(shared).collect()),
                false => (
                    (1..=width).map(|n| format!("Column {n}").into()).collect(),
                    rows.iter().map(shared).collect(),
                ),
            }
        };
        let (import, turned) = (self.on_import, headed);
        let records = Rc::new(data);
        let given = records.clone();
        ImportDialog::new(
            id.clone(),
            self.title,
            columns,
            self.fields,
            move |mapping, window, cx| {
                let rows: Vec<Vec<SharedString>> = given
                    .iter()
                    .map(|row| {
                        mapping
                            .iter()
                            .map(|column| {
                                column.map(|column| row[column].clone()).unwrap_or_default()
                            })
                            .collect()
                    })
                    .collect();
                log::info!("csv importer: {} records", rows.len());
                import(&rows, window, cx)
            },
            move |window, cx| cancel(window, cx),
        )
        .preview(records.iter().cloned())
        .child(
            Checkbox::new((id, "headed"), first_names)
                .label("The first row names the columns")
                .on_change(move |on, _, cx| {
                    turned.update(cx, |headed, cx| {
                        *headed = on;
                        cx.notify();
                    })
                }),
        )
        .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::read_csv;

    #[test]
    fn csv_reads_quotes_commas_and_line_breaks() {
        let rows = read_csv("name,note\r\n\"Doe, Jane\",\"says \"\"hi\"\"\"\nAda,\"two\nlines\"\n")
            .expect("valid CSV");
        assert_eq!(
            rows,
            [
                vec!["name".to_string(), "note".to_string()],
                vec!["Doe, Jane".into(), "says \"hi\"".into()],
                vec!["Ada".into(), "two\nlines".into()],
            ]
        );
        assert_eq!(
            read_csv("a,,b").expect("empty middle"),
            [vec!["a".to_string(), String::new(), "b".into()]]
        );
        assert_eq!(read_csv("").expect("nothing"), Vec::<Vec<String>>::new());
    }

    #[test]
    fn a_byte_order_mark_is_no_part_of_the_first_field() {
        assert_eq!(
            read_csv("\u{feff}Name\nAda").expect("valid CSV"),
            [vec!["Name".to_string()], vec!["Ada".to_string()]]
        );
    }

    #[test]
    fn faults_name_their_lines() {
        assert_eq!(
            read_csv("a,b\nc,\"d\nd\ne"),
            Err("line 2: a quote never closes".into()),
            "the line the quote opened on"
        );
        assert_eq!(
            read_csv("a\nb\"c"),
            Err("line 2: a quote inside a field it did not open".into())
        );
    }

    #[test]
    fn csv_names_what_breaks_it() {
        assert_eq!(
            read_csv("a,\"open"),
            Err("line 1: a quote never closes".into())
        );
        assert_eq!(
            read_csv("\"a\"b,c"),
            Err("line 1: text after a closing quote".into())
        );
        assert_eq!(
            read_csv("ab\"c"),
            Err("line 1: a quote inside a field it did not open".into())
        );
        assert_eq!(
            read_csv("a,b\nc"),
            Err("row 2 has 1 fields; the first has 2".into())
        );
    }
}
