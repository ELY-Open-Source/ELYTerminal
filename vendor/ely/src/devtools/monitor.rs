use gpui::{
    App, ElementId, FontWeight, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    SharedString, Styled, Window, div,
};

use crate::{
    data_display::{Meter, Sparkline},
    theme::{ActiveTheme, Radius, TextSize},
    typography::Ellipsis,
};

/// A measure over time: its name and unit, its readings oldest first, and the most it can reach, when it has a most.
#[derive(Clone, Debug, PartialEq)]
pub struct Reading {
    pub name: SharedString,
    pub unit: SharedString,
    pub values: Vec<f32>,
    pub limit: Option<f32>,
}

impl Reading {
    pub fn new(
        name: impl Into<SharedString>,
        unit: impl Into<SharedString>,
        values: impl IntoIterator<Item = f32>,
    ) -> Self {
        let values: Vec<f32> = values.into_iter().collect();
        assert!(!values.is_empty(), "a reading has values");
        Self {
            name: name.into(),
            unit: unit.into(),
            values,
            limit: None,
        }
    }

    pub fn limit(mut self, limit: f32) -> Self {
        assert!(limit > 0.0, "a limit of {limit}");
        self.limit = Some(limit);
        self
    }
}

/// What a tile says of a reading: its latest value, and of how much when it has a limit.
pub fn reading_line(reading: &Reading) -> String {
    let latest = *reading.values.last().expect("a reading has values");
    match &reading.limit {
        Some(limit) => format!("{latest:.1} of {limit:.0} {}", reading.unit),
        None => format!("{latest:.1} {}", reading.unit),
    }
}

/// A machine's readings in tiles: each its name, its latest value, a sparkline of the readings before, and, with a limit, how full it is.
#[derive(IntoElement)]
pub struct ResourceMonitor {
    id: ElementId,
    readings: Vec<Reading>,
}

impl ResourceMonitor {
    pub fn new(id: impl Into<ElementId>, readings: impl IntoIterator<Item = Reading>) -> Self {
        Self {
            id: id.into(),
            readings: readings.into_iter().collect(),
        }
    }
}

impl RenderOnce for ResourceMonitor {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let tiles = self.readings.iter().enumerate().map(|(ix, reading)| {
            let latest = *reading.values.last().expect("a reading has values");
            let shown = reading_line(reading);
            div()
                .debug_selector(move || format!("monitor-tile-{ix}"))
                .flex_1()
                .min_w(theme.label_width())
                .flex()
                .flex_col()
                .gap_2()
                .p_4()
                .rounded(theme.radius(Radius::Lg))
                .border_1()
                .border_color(theme.colors.border)
                .child(
                    div()
                        .text_size(theme.text_size(TextSize::Sm))
                        .text_color(theme.colors.fg_muted)
                        .child(Ellipsis::new(reading.name.clone())),
                )
                .child(
                    div()
                        .flex()
                        .flex_wrap()
                        .items_end()
                        .justify_between()
                        .gap_2()
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .text_size(theme.text_size(TextSize::Lg))
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(shown),
                        )
                        .child(
                            div()
                                .flex_none()
                                .child(Sparkline::new(reading.values.clone()).area()),
                        ),
                )
                .children(reading.limit.map(|limit| {
                    Meter::new(
                        (self.id.clone(), format!("meter-{ix}")),
                        "Used",
                        (latest / limit).clamp(0.0, 1.0),
                    )
                    .thresholds(0.75, 0.9)
                }))
        });
        div().flex().flex_wrap().gap_4().children(tiles)
    }
}

#[cfg(test)]
mod tests {
    use super::{Reading, reading_line};

    #[test]
    fn a_tile_reads_its_latest_value_and_its_limit() {
        let memory = Reading::new("Memory", "GB", [3.0, 5.26]);
        assert_eq!(reading_line(&memory), "5.3 GB");
        assert_eq!(reading_line(&memory.limit(16.0)), "5.3 of 16 GB");
    }
}
