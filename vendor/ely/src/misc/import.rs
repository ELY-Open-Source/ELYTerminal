use std::{cell::Cell, rc::Rc};

use gpui::{
    AnyElement, App, ElementId, IntoElement, ParentElement, RenderOnce, SharedString, Styled,
    Window, div, prelude::*,
};
use smallvec::SmallVec;

use crate::{
    buttons::{Button, ButtonVariant},
    forms::{Choice, Run, Select},
    overlays::Dialog,
    tables::Table,
    theme::{ActiveTheme, TextSize},
};

type OnMap = Rc<dyn Fn(&[Option<usize>], &mut Window, &mut App)>;

/// A field the app keeps: its name, and whether an import must fill it.
#[derive(Clone, Debug, PartialEq)]
pub struct ImportField {
    name: SharedString,
    required: bool,
}

impl ImportField {
    pub fn new(name: impl Into<SharedString>) -> Self {
        Self {
            name: name.into(),
            required: false,
        }
    }

    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }
}

/// How far two names match: equal once case, spaces, dashes and underscores are set aside.
fn same(field: &str, column: &str) -> bool {
    let plain = |name: &str| {
        name.chars()
            .filter(|ch| !matches!(ch, ' ' | '-' | '_'))
            .flat_map(char::to_lowercase)
            .collect::<String>()
    };
    plain(field) == plain(column)
}

/// Each field's column to start: the first whose name matches, or none.
pub(crate) fn matched(fields: &[ImportField], columns: &[SharedString]) -> Vec<Option<usize>> {
    fields
        .iter()
        .map(|field| columns.iter().position(|column| same(&field.name, column)))
        .collect()
}

/// The mapping as the owner set it last, and the columns it was made for.
struct Mapping {
    columns: Vec<SharedString>,
    picked: Vec<Option<usize>>,
    nudged: bool,
}

/// Columns from a file mapped to the fields the app keeps: a Select of the columns for each field, matched by name to start, over the first rows as they would come in. Import asks for a column for every required field, then hands the mapping over; each way out answers once, import or cancel. Render it while open.
#[derive(IntoElement)]
pub struct ImportDialog {
    id: ElementId,
    title: SharedString,
    columns: Vec<SharedString>,
    rows: Vec<Vec<SharedString>>,
    fields: Vec<ImportField>,
    body: SmallVec<[AnyElement; 1]>,
    on_import: OnMap,
    on_cancel: Run,
}

impl ImportDialog {
    /// `on_import` gets each field's column, in field order, or none for a field left empty.
    pub fn new(
        id: impl Into<ElementId>,
        title: impl Into<SharedString>,
        columns: impl IntoIterator<Item = impl Into<SharedString>>,
        fields: impl IntoIterator<Item = ImportField>,
        on_import: impl Fn(&[Option<usize>], &mut Window, &mut App) + 'static,
        on_cancel: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            columns: columns.into_iter().map(Into::into).collect(),
            rows: Vec::new(),
            fields: fields.into_iter().collect(),
            body: SmallVec::new(),
            on_import: Rc::new(on_import),
            on_cancel: Rc::new(on_cancel),
        }
    }

    /// The first rows, by column, to show as they would come in.
    pub fn preview(mut self, rows: impl IntoIterator<Item = Vec<SharedString>>) -> Self {
        self.rows = rows.into_iter().take(PREVIEW).collect();
        self
    }
}

impl ParentElement for ImportDialog {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.body.extend(elements);
    }
}

/// How many rows the preview shows.
const PREVIEW: usize = 3;

impl RenderOnce for ImportDialog {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id;
        assert!(
            !self.fields.is_empty(),
            "import dialog {id:?} has no fields"
        );
        assert!(
            !self.columns.is_empty(),
            "import dialog {id:?} has no columns"
        );
        let (fields, columns) = (Rc::new(self.fields), self.columns);
        let mapping = window.use_keyed_state((id.clone(), "mapping"), cx, {
            let (fields, columns) = (fields.clone(), columns.clone());
            move |_, _| Mapping {
                picked: matched(&fields, &columns),
                columns,
                nudged: false,
            }
        });
        let now = mapping.read(cx);
        if now.columns != columns || now.picked.len() != fields.len() {
            let picked = matched(&fields, &columns);
            mapping.update(cx, |mapping, _| {
                *mapping = Mapping {
                    columns: columns.clone(),
                    picked,
                    nudged: false,
                }
            });
        }
        let (picked, nudged) = (mapping.read(cx).picked.clone(), mapping.read(cx).nudged);
        let missing: Vec<SharedString> = fields
            .iter()
            .zip(&picked)
            .filter(|(field, column)| field.required && column.is_none())
            .map(|(field, _)| field.name.clone())
            .collect();
        let theme = cx.theme();
        let colors = &theme.colors;
        let rows = fields.iter().enumerate().map(|(ix, field)| {
            let skip = (!field.required).then(|| Choice::new("none", "Leave empty"));
            let choices = skip.into_iter().chain(
                columns
                    .iter()
                    .enumerate()
                    .map(|(column, name)| Choice::new(column.to_string(), name.clone())),
            );
            let set = mapping.clone();
            let select = Select::new((id.clone(), format!("field-{ix}")), choices)
                .placeholder("Choose a column")
                .on_change(move |value, _, cx| {
                    let column = (value.as_ref() != "none")
                        .then(|| value.parse().expect("a column's place"));
                    set.update(cx, |mapping, cx| {
                        mapping.picked[ix] = column;
                        cx.notify();
                    })
                });
            let select = match (picked[ix], field.required) {
                (Some(column), _) => select.selected(column.to_string()),
                (None, false) => select.selected("none"),
                (None, true) => select,
            };
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .flex_1()
                        .min_w(theme.label_width())
                        .text_color(colors.fg)
                        .child(match field.required {
                            true => format!("{} (required)", field.name),
                            false => field.name.to_string(),
                        }),
                )
                .child(div().flex_1().min_w(theme.label_width()).child(select))
        });
        let preview = self.rows.iter().fold(
            Table::new(fields.iter().map(|field| field.name.clone())),
            |table, row| {
                table.row(picked.iter().map(|column| {
                    column
                        .map(|column| row[column].clone())
                        .unwrap_or_else(|| "—".into())
                }))
            },
        );
        let (cancel, import) = (self.on_cancel, self.on_import);
        let (cancelled, imported, asked) = (id.clone(), id.clone(), mapping.clone());
        let answered = Rc::new(Cell::new(false));
        let answer = answered.clone();
        Dialog::new(id.clone(), self.title, move |window, cx| {
            if !answered.get() {
                log::info!("import dialog {cancelled:?}: cancelled");
                cancel(window, cx)
            }
        })
        .children(self.body)
        .child(
            div()
                .flex()
                .flex_col()
                .gap_3()
                .text_size(theme.text_size(TextSize::Sm))
                .children(rows),
        )
        .when(!self.rows.is_empty(), |dialog| {
            dialog.child(
                div()
                    .debug_selector(|| "import-preview".into())
                    .child(preview),
            )
        })
        .when(nudged && !missing.is_empty(), |dialog| {
            dialog.child(
                div()
                    .debug_selector(|| format!("import-missing-{}", missing.join(",")))
                    .text_size(theme.text_size(TextSize::Sm))
                    .text_color(colors.danger)
                    .child(format!("Choose a column for {}.", missing.join(", "))),
            )
        })
        .action({
            let id = id.clone();
            move |close| {
                Button::new((id, "cancel"), "Cancel")
                    .variant(ButtonVariant::Ghost)
                    .on_click(move |_, window, cx| close(window, cx))
            }
        })
        .action(move |close| {
            Button::new((id, "import"), "Import")
                .primary()
                .on_click(move |_, window, cx| {
                    if !missing.is_empty() {
                        asked.update(cx, |mapping, cx| {
                            mapping.nudged = true;
                            cx.notify();
                        });
                        return;
                    }
                    log::info!("import dialog {imported:?}: imported with {picked:?}");
                    answer.set(true);
                    close(window, cx);
                    import(&picked, window, cx);
                })
        })
    }
}

#[cfg(test)]
mod tests {
    use gpui::SharedString;

    use super::{ImportField, matched};

    #[test]
    fn fields_find_their_columns_by_name() {
        let columns: Vec<SharedString> = ["E-mail", "Full name", "Age"].map(Into::into).into();
        let fields = [
            ImportField::new("full_name").required(),
            ImportField::new("email"),
            ImportField::new("phone"),
        ];
        assert_eq!(matched(&fields, &columns), [Some(1), Some(0), None]);
    }
}
