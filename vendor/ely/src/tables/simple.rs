use gpui::{
    AnyElement, App, FontWeight, IntoElement, ParentElement, RenderOnce, SharedString, Styled,
    Window, div, prelude::*,
};

use crate::{
    theme::{ActiveTheme, Density, Radius, TextSize},
    typography::{format, tabular},
};

/// A plain table: a header and rows of cells with hairlines between. To sort, page or select, use `DataTable`.
#[derive(IntoElement)]
pub struct Table {
    headers: Vec<SharedString>,
    rows: Vec<Vec<AnyElement>>,
}

impl Table {
    pub fn new(headers: impl IntoIterator<Item = impl Into<SharedString>>) -> Self {
        Self {
            headers: headers.into_iter().map(Into::into).collect(),
            rows: Vec::new(),
        }
    }

    pub fn row(mut self, cells: impl IntoIterator<Item = impl IntoElement>) -> Self {
        let cells: Vec<AnyElement> = cells
            .into_iter()
            .map(IntoElement::into_any_element)
            .collect();
        assert_eq!(
            cells.len(),
            self.headers.len(),
            "a row needs a cell per header"
        );
        self.rows.push(cells);
        self
    }
}

impl RenderOnce for Table {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = &theme.colors;
        let last = self.rows.len().saturating_sub(1);
        let cell = || div().flex_1().min_w_0().px_3().py_2();
        div()
            .flex()
            .flex_col()
            .text_size(theme.text_size(TextSize::Sm))
            .child(
                div()
                    .flex()
                    .border_b_1()
                    .border_color(colors.border)
                    .text_size(theme.text_size(TextSize::Xs))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(colors.fg_muted)
                    .children(self.headers.into_iter().map(|header| cell().child(header))),
            )
            .children(self.rows.into_iter().enumerate().map(|(ix, row)| {
                div()
                    .flex()
                    .text_color(colors.fg)
                    .when(ix < last, |row| {
                        row.border_b_1().border_color(colors.border)
                    })
                    .children(row.into_iter().map(|content| cell().child(content)))
            }))
    }
}

/// Numbers in a grid, each cell shaded by where it sits between the grid's lowest and highest, its value on top.
#[derive(IntoElement)]
pub struct HeatmapTable {
    columns: Vec<SharedString>,
    rows: Vec<(SharedString, Vec<f64>)>,
    decimals: usize,
}

impl HeatmapTable {
    pub fn new(columns: impl IntoIterator<Item = impl Into<SharedString>>) -> Self {
        Self {
            columns: columns.into_iter().map(Into::into).collect(),
            rows: Vec::new(),
            decimals: 0,
        }
    }

    pub fn row(
        mut self,
        label: impl Into<SharedString>,
        values: impl IntoIterator<Item = f64>,
    ) -> Self {
        let values: Vec<f64> = values.into_iter().collect();
        assert_eq!(
            values.len(),
            self.columns.len(),
            "a row needs a value per column"
        );
        assert!(
            values.iter().all(|value| value.is_finite()),
            "a heatmap needs finite values"
        );
        self.rows.push((label.into(), values));
        self
    }

    pub fn decimals(mut self, decimals: usize) -> Self {
        self.decimals = decimals;
        self
    }
}

impl RenderOnce for HeatmapTable {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = &theme.colors;
        let (low, high) = self
            .rows
            .iter()
            .flat_map(|(_, values)| values.iter().copied())
            .fold((f64::MAX, f64::MIN), |(low, high), value| {
                (low.min(value), high.max(value))
            });
        let share = |value: f64| {
            if high > low {
                ((value - low) / (high - low)) as f32
            } else {
                0.5
            }
        };
        let (hue, height, decimals) = (
            colors.chart[0],
            theme.table_row(Density::Standard),
            self.decimals,
        );
        let label = |text: SharedString| {
            div()
                .flex_none()
                .w(theme.label_width() * 0.6)
                .flex()
                .items_center()
                .text_size(theme.text_size(TextSize::Xs))
                .text_color(colors.fg_muted)
                .child(text)
        };
        div()
            .flex()
            .flex_col()
            .gap_0p5()
            .text_size(theme.text_size(TextSize::Sm))
            .child(
                div()
                    .flex()
                    .gap_0p5()
                    .child(label(SharedString::default()))
                    .children(self.columns.into_iter().map(|column| {
                        div()
                            .flex_1()
                            .flex()
                            .justify_center()
                            .text_size(theme.text_size(TextSize::Xs))
                            .text_color(colors.fg_muted)
                            .child(column)
                    })),
            )
            .children(self.rows.into_iter().map(|(name, values)| {
                div()
                    .flex()
                    .gap_0p5()
                    .child(label(name))
                    .children(values.into_iter().map(|value| {
                        let share = share(value);
                        tabular(div())
                            .flex_1()
                            .h(height)
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(theme.radius(Radius::Sm))
                            .bg(hue.opacity(0.08 + 0.84 * share))
                            .text_color(if share > 0.55 {
                                colors.on_media
                            } else {
                                colors.fg
                            })
                            .child(format::number(value, decimals, format::Separators::EN))
                    }))
            }))
    }
}
