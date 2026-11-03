use gpui::{
    App, ElementId, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window, div,
    prelude::*,
};

use crate::{
    data_display::{Meter, Sparkline},
    theme::ActiveTheme,
};

/// One part of the machine: its name, how full it is from 0 to 1, that in units, and its recent shares, oldest first.
#[derive(Clone, Debug, PartialEq)]
pub struct Reading {
    pub name: SharedString,
    pub share: f32,
    pub detail: SharedString,
    pub history: Vec<f32>,
}

/// The machine at work: a meter for each part, as GPU, its memory and the CPU, that turns amber then red as it fills, and beside it a line of its recent history.
#[derive(IntoElement)]
pub struct HardwareMonitor {
    id: ElementId,
    readings: Vec<Reading>,
}

impl HardwareMonitor {
    pub fn new(id: impl Into<ElementId>, readings: impl IntoIterator<Item = Reading>) -> Self {
        let readings: Vec<Reading> = readings.into_iter().collect();
        for reading in &readings {
            let within = |share: &f32| (0.0..=1.0).contains(share);
            assert!(
                within(&reading.share) && reading.history.iter().all(within),
                "{} reads outside 0 to 1",
                reading.name
            );
        }
        Self {
            id: id.into(),
            readings,
        }
    }
}

impl RenderOnce for HardwareMonitor {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let spark = cx.theme().spark_size();
        div()
            .flex()
            .flex_col()
            .gap_3()
            .children(self.readings.into_iter().enumerate().map(|(ix, reading)| {
                div()
                    .flex()
                    .items_end()
                    .gap_3()
                    .child(
                        div().flex_1().min_w_0().child(
                            Meter::new(
                                (self.id.clone(), format!("meter-{ix}")),
                                reading.name,
                                reading.share,
                            )
                            .detail(reading.detail)
                            .thresholds(0.75, 0.9),
                        ),
                    )
                    .when(reading.history.len() > 1, |row| {
                        row.child(
                            div()
                                .flex_none()
                                .w(spark.width)
                                .h(spark.height)
                                .child(Sparkline::new(reading.history).area()),
                        )
                    })
            }))
    }
}
