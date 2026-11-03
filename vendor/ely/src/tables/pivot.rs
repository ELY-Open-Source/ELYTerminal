use gpui::{
    App, FontWeight, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window, div,
    prelude::*,
};

use super::{Aggregate, Cell, model::combine};
use crate::{
    theme::{ActiveTheme, TextSize},
    typography::{Ellipsis, format, tabular},
};

/// A cross-tab: labels down the side and across the top, a figure where they meet, and totals.
#[derive(Debug, PartialEq)]
pub(crate) struct Pivot {
    pub rows: Vec<SharedString>,
    pub cols: Vec<SharedString>,
    pub cells: Vec<Vec<Option<f64>>>,
    pub row_totals: Vec<Option<f64>>,
    pub col_totals: Vec<Option<f64>>,
    pub grand: Option<f64>,
}

/// Records cross-tabbed by fields `down` and `across`, field `value` combined where they meet; labels keep the order they first come in.
pub(crate) fn pivot(
    records: &[Vec<Cell>],
    down: usize,
    across: usize,
    value: usize,
    how: Aggregate,
) -> Pivot {
    let mut rows: Vec<SharedString> = Vec::new();
    let mut cols: Vec<SharedString> = Vec::new();
    for record in records {
        let (row, col) = (record[down].words(), record[across].words());
        if !rows.contains(&row) {
            rows.push(row);
        }
        if !cols.contains(&col) {
            cols.push(col);
        }
    }
    let values = |keep: &dyn Fn(&Vec<Cell>) -> bool| -> Vec<f64> {
        records
            .iter()
            .filter(|record| keep(record))
            .filter_map(|record| record[value].number())
            .collect()
    };
    let cells = rows
        .iter()
        .map(|row| {
            cols.iter()
                .map(|col| {
                    combine(
                        &values(&|r| r[down].words() == *row && r[across].words() == *col),
                        how,
                    )
                })
                .collect()
        })
        .collect();
    let row_totals = rows
        .iter()
        .map(|row| combine(&values(&|r| r[down].words() == *row), how))
        .collect();
    let col_totals = cols
        .iter()
        .map(|col| combine(&values(&|r| r[across].words() == *col), how))
        .collect();
    let grand = combine(&values(&|_| true), how);
    Pivot {
        rows,
        cols,
        cells,
        row_totals,
        col_totals,
        grand,
    }
}

/// Records cross-tabbed: one field's values down the side, another's across the top, and a third combined where they meet, with totals for each row, each column, and all.
#[derive(IntoElement)]
pub struct PivotTable {
    fields: Vec<SharedString>,
    records: Vec<Vec<Cell>>,
    by: (SharedString, SharedString, SharedString),
    how: Aggregate,
    decimals: usize,
}

impl PivotTable {
    /// `fields` name each record's cells in order.
    pub fn new(
        fields: impl IntoIterator<Item = impl Into<SharedString>>,
        records: impl IntoIterator<Item = Vec<Cell>>,
    ) -> Self {
        let fields: Vec<SharedString> = fields.into_iter().map(Into::into).collect();
        let records: Vec<Vec<Cell>> = records.into_iter().collect();
        assert!(
            records.iter().all(|record| record.len() == fields.len()),
            "a record needs a cell per field"
        );
        Self {
            fields,
            records,
            by: Default::default(),
            how: Aggregate::Sum,
            decimals: 0,
        }
    }

    /// Field `down` down the side, `across` over the top, `value` combined by `how`.
    pub fn pivot(
        mut self,
        down: impl Into<SharedString>,
        across: impl Into<SharedString>,
        value: impl Into<SharedString>,
        how: Aggregate,
    ) -> Self {
        self.by = (down.into(), across.into(), value.into());
        self.how = how;
        self
    }

    pub fn decimals(mut self, decimals: usize) -> Self {
        self.decimals = decimals;
        self
    }
}

impl RenderOnce for PivotTable {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let field = |name: &SharedString| self.fields.iter().position(|known| known == name);
        let theme = cx.theme();
        let colors = &theme.colors;
        let (Some(down), Some(across), Some(value)) =
            (field(&self.by.0), field(&self.by.1), field(&self.by.2))
        else {
            let (down, across, value) = &self.by;
            log::error!(
                "pivot table: {down}, {across} or {value} is not among {:?}",
                self.fields
            );
            return div()
                .text_size(theme.text_size(TextSize::Sm))
                .text_color(colors.danger)
                .child(format!("No field to pivot by {down}, {across} and {value}"))
                .into_any_element();
        };
        let table = pivot(&self.records, down, across, value, self.how);
        let decimals = self.decimals;
        let reads = |figure: Option<f64>| {
            figure.map_or("—".to_string(), |figure| {
                format::number(figure, decimals, format::Separators::EN)
            })
        };
        let label = |text: SharedString| {
            div()
                .w(theme.label_width() * 0.8)
                .flex_none()
                .px_3()
                .child(Ellipsis::new(text))
        };
        let number = |text: String, strong: bool| {
            tabular(div())
                .flex_1()
                .min_w_0()
                .px_3()
                .flex()
                .justify_end()
                .when(strong, |cell| cell.font_weight(FontWeight::MEDIUM))
                .child(text)
        };
        let line = || {
            div()
                .flex()
                .items_center()
                .h(theme.table_row(crate::theme::Density::Standard))
                .border_b_1()
                .border_color(colors.border)
        };
        let head = line()
            .text_size(theme.text_size(TextSize::Xs))
            .font_weight(FontWeight::MEDIUM)
            .text_color(colors.fg_muted)
            .child(label(format!("{} · {}", self.by.0, self.by.1).into()))
            .children(table.cols.iter().map(|col| number(col.to_string(), false)))
            .child(number("Total".into(), false));
        let body = table.rows.iter().enumerate().map(|(ix, row)| {
            line()
                .child(label(row.clone()).text_color(colors.fg_muted))
                .children(
                    table.cells[ix]
                        .iter()
                        .map(|figure| number(reads(*figure), false)),
                )
                .child(number(reads(table.row_totals[ix]), true))
        });
        let totals = line()
            .bg(colors.sunken)
            .child(label("Total".into()).font_weight(FontWeight::MEDIUM))
            .children(
                table
                    .col_totals
                    .iter()
                    .map(|figure| number(reads(*figure), true)),
            )
            .child(number(reads(table.grand), true));
        div()
            .flex()
            .flex_col()
            .text_size(theme.text_size(TextSize::Sm))
            .child(head)
            .children(body)
            .child(totals)
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::{Aggregate, Cell, pivot};

    #[test]
    fn records_cross_tab_with_totals() {
        let records: Vec<Vec<Cell>> = [
            ("Europe", "Q1", 10.0),
            ("Asia", "Q1", 4.0),
            ("Europe", "Q2", 6.0),
            ("Europe", "Q1", 5.0),
        ]
        .into_iter()
        .map(|(region, quarter, sales)| vec![region.into(), quarter.into(), sales.into()])
        .collect();
        let table = pivot(&records, 0, 1, 2, Aggregate::Sum);
        assert_eq!(table.rows, ["Europe", "Asia"]);
        assert_eq!(table.cols, ["Q1", "Q2"]);
        assert_eq!(
            table.cells,
            [vec![Some(15.0), Some(6.0)], vec![Some(4.0), None]]
        );
        assert_eq!(table.row_totals, [Some(21.0), Some(4.0)]);
        assert_eq!(table.col_totals, [Some(19.0), Some(6.0)]);
        assert_eq!(table.grand, Some(25.0));
    }
}
