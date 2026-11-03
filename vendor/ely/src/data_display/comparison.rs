use gpui::{
    App, FontWeight, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window, div,
    prelude::*,
};

use crate::{
    primitives::{Icon, IconName},
    theme::{ActiveTheme, ControlSize, IconSize, Radius, TextSize},
};

/// What a cell holds: a value, or whether the option has the feature.
enum Cell {
    Text(SharedString),
    Mark(bool),
}

/// Options side by side: a column each, a row per feature, a value or a mark in each cell. One column can stand out.
#[derive(IntoElement)]
pub struct Comparison {
    columns: Vec<SharedString>,
    featured: Option<usize>,
    rows: Vec<(SharedString, Vec<Cell>)>,
}

impl Comparison {
    pub fn new(columns: impl IntoIterator<Item = impl Into<SharedString>>) -> Self {
        let columns: Vec<SharedString> = columns.into_iter().map(Into::into).collect();
        assert!(!columns.is_empty(), "a comparison needs an option");
        Self {
            columns,
            featured: None,
            rows: Vec::new(),
        }
    }

    /// Tints column `ix`, such as the plan you suggest.
    pub fn featured(mut self, ix: usize) -> Self {
        self.featured = (ix < self.columns.len()).then_some(ix);
        if self.featured.is_none() {
            log::error!(
                "comparison: column {ix} of {}; none featured",
                self.columns.len()
            );
        }
        self
    }

    /// A feature and its value under each option.
    pub fn row(
        self,
        name: impl Into<SharedString>,
        values: impl IntoIterator<Item = impl Into<SharedString>>,
    ) -> Self {
        let cells = values
            .into_iter()
            .map(|value| Cell::Text(value.into()))
            .collect();
        self.push(name.into(), cells)
    }

    /// A feature, and whether each option has it.
    pub fn marks(
        self,
        name: impl Into<SharedString>,
        values: impl IntoIterator<Item = bool>,
    ) -> Self {
        let cells = values.into_iter().map(Cell::Mark).collect();
        self.push(name.into(), cells)
    }

    fn push(mut self, name: SharedString, cells: Vec<Cell>) -> Self {
        assert_eq!(
            cells.len(),
            self.columns.len(),
            "row {name} needs a cell per option"
        );
        self.rows.push((name, cells));
        self
    }
}

impl RenderOnce for Comparison {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = &theme.colors;
        let (featured, last) = (self.featured, self.rows.len());
        let columns = self.columns.len() as u16 + 1;
        let radius = theme.radius(Radius::Md);
        let cell = move |column: Option<usize>, row: usize| {
            let lit = column.is_some() && column == featured;
            div()
                .flex()
                .items_center()
                .min_h(theme.control_height(ControlSize::Lg))
                .px_3()
                .py_2()
                .when(row > 0, |cell| {
                    cell.border_t_1().border_color(colors.border)
                })
                .when(column.is_some(), |cell| cell.justify_center())
                .when(lit, |cell| {
                    cell.bg(colors.hover)
                        .when(row == 0, |cell| cell.rounded_t(radius))
                        .when(row == last, |cell| cell.rounded_b(radius))
                })
        };
        let head = std::iter::once(cell(None, 0).into_any_element()).chain(
            self.columns.into_iter().enumerate().map(|(ix, title)| {
                cell(Some(ix), 0)
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(colors.fg)
                    .child(title)
                    .into_any_element()
            }),
        );
        let body = self
            .rows
            .into_iter()
            .enumerate()
            .flat_map(|(ix, (name, cells))| {
                let row = ix + 1;
                std::iter::once(
                    cell(None, row)
                        .text_color(colors.fg_muted)
                        .child(name)
                        .into_any_element(),
                )
                .chain(cells.into_iter().enumerate().map(
                    move |(column, value)| {
                        let body = match value {
                            Cell::Text(text) => {
                                div().text_color(colors.fg).child(text).into_any_element()
                            }
                            Cell::Mark(true) => Icon::new(IconName::Check)
                                .size(IconSize::Sm)
                                .color(colors.fg)
                                .into_any_element(),
                            Cell::Mark(false) => Icon::new(IconName::Minus)
                                .size(IconSize::Sm)
                                .color(colors.fg_subtle)
                                .into_any_element(),
                        };
                        cell(Some(column), row).child(body).into_any_element()
                    },
                ))
            });
        div()
            .grid()
            .grid_cols(columns)
            .text_size(theme.text_size(TextSize::Sm))
            .children(head)
            .children(body)
    }
}

#[cfg(test)]
mod tests {
    use super::Comparison;

    #[test]
    fn a_featured_column_past_the_last_features_none() {
        assert_eq!(Comparison::new(["Free", "Pro"]).featured(5).featured, None);
        assert_eq!(
            Comparison::new(["Free", "Pro"]).featured(1).featured,
            Some(1)
        );
        let later = Comparison::new(["Free", "Pro"]).featured(1).featured(5);
        assert_eq!(later.featured, None, "a later bad index clears the first");
    }

    #[test]
    #[should_panic(expected = "row Storage needs a cell per option")]
    fn a_row_needs_a_cell_per_option() {
        let _ = Comparison::new(["Free", "Pro"]).row("Storage", ["5 GB"]);
    }
}
