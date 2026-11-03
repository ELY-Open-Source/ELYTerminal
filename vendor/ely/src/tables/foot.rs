use gpui::{App, Div, FontWeight, ParentElement, Styled, div, prelude::*, transparent_black};

use super::{Aggregate, Cell, body::Body, body::sized, model::figure};
use crate::{
    theme::{ActiveTheme, TextSize},
    typography::format,
};

impl Body {
    /// The footer's figures over the rows `order` keeps, in columns `cols`; none when no column asks for one.
    pub(super) fn figures(
        &self,
        order: &[usize],
        cols: &[usize],
        lead: bool,
        cx: &App,
    ) -> Option<Div> {
        let columns = &self.columns;
        columns
            .iter()
            .any(|column| column.aggregate.is_some())
            .then(|| {
                let theme = cx.theme();
                div()
                    .flex()
                    .items_center()
                    .h(self.height)
                    .text_size(theme.text_size(TextSize::Sm))
                    .font_weight(FontWeight::MEDIUM)
                    .when(lead, |row| {
                        row.child(div().flex_none().w(self.lead_width()))
                    })
                    .children(cols.iter().map(|col| {
                        let column = &columns[*col];
                        let shown = column.aggregate.map(|how| {
                            let shares = self
                                .rows
                                .first()
                                .is_some_and(|row| matches!(row.cells[*col], Cell::Progress(_)));
                            let value = figure(&self.rows, order, *col, how).map_or(
                                "—".to_string(),
                                |value| match how {
                                    Aggregate::Count => format!("{}", value as usize),
                                    _ if shares => format::percent(value, 0, false),
                                    _ => column.reads(value),
                                },
                            );
                            div()
                                .flex()
                                .items_baseline()
                                .gap_1()
                                .child(
                                    div()
                                        .text_size(theme.text_size(TextSize::Xs))
                                        .text_color(theme.colors.fg_subtle)
                                        .child(how.label()),
                                )
                                .child(value)
                        });
                        let cell = div()
                            .px_3()
                            .border_x_1()
                            .border_color(transparent_black())
                            .flex()
                            .items_center();
                        match self.widths[*col] {
                            Some(width) => cell.w(width).flex_none(),
                            None => sized(cell, column, self.narrowest),
                        }
                        .children(shown)
                    }))
            })
    }
}
