use gpui::{Context, IntoElement, ParentElement, Render, Styled, TestAppContext, Window, div, px};

use crate::{
    tables::{Aggregate, Column, DataTable, PivotTable, Row, Spreadsheet},
    theme::Theme,
};

/// A pivot, a grouping and a merge that name what their data no longer holds.
struct Stale;

impl Render for Stale {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let pivot = PivotTable::new(["region", "sales"], [vec!["Europe".into(), 4.0.into()]])
            .pivot("region", "quarter", "sales", Aggregate::Sum);
        let table = DataTable::new("table", [Column::new("name", "Name")])
            .rows(vec![Row::new("a", ["Ann".into()])])
            .group_by("region");
        let sheet = Spreadsheet::new("sheet", 2, 2)
            .merge((0, 0), (5, 1))
            .w(px(300.0))
            .h(px(120.0));
        div().w(px(400.0)).child(pivot).child(table).child(sheet)
    }
}

#[gpui::test]
fn a_pivot_grouping_or_merge_past_its_data_draws(cx: &mut TestAppContext) {
    cx.update(Theme::init);
    let (_, cx) = cx.add_window_view(|_, _| Stale);
    cx.run_until_parked();
}
