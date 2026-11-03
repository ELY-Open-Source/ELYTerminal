use std::rc::Rc;

use gpui::{
    App, ElementId, FontWeight, IntoElement, ParentElement, RenderOnce, Styled, Window, div,
    relative,
};

use super::quotes::price;
use crate::{
    charts::nice_step,
    data_display::{Gauge, Meter},
    forms::Slider,
    theme::{ActiveTheme, TextSize},
    typography::tabular,
};

/// Where leverage up to `max` is marked: one times, round steps, and the most, none crowding it.
fn marks(max: f64) -> Vec<f64> {
    let step = nice_step(max, 5).max(1.0);
    let between = (1..)
        .map(|ix| step * f64::from(ix))
        .take_while(|mark| *mark <= max - step / 2.0)
        .filter(|mark| *mark > 1.0);
    std::iter::once(1.0)
        .chain(between)
        .chain(std::iter::once(max))
        .collect()
}

type OnValue = Rc<dyn Fn(f64, &mut Window, &mut App)>;

/// Leverage from one times up to `max`, marked at the common steps, its number turning amber, then red, as the risk climbs.
#[derive(IntoElement)]
pub struct LeverageSlider {
    id: ElementId,
    value: f64,
    max: f64,
    on_change: Option<OnValue>,
}

impl LeverageSlider {
    pub fn new(id: impl Into<ElementId>, value: f64, max: f64) -> Self {
        assert!(
            max > 1.0 && max.fract() == 0.0 && value.is_finite(),
            "leverage runs in whole steps from one up to its most"
        );
        Self {
            id: id.into(),
            value,
            max,
            on_change: None,
        }
    }

    pub fn on_change(mut self, handler: impl Fn(f64, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for LeverageSlider {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        if !(1.0..=self.max).contains(&self.value) {
            log::error!(
                "leverage {:?}: {} is outside 1..={}",
                self.id,
                self.value,
                self.max
            );
        }
        let share = ((self.value - 1.0) / (self.max - 1.0)).clamp(0.0, 1.0);
        let ink = if share >= 0.5 {
            colors.danger
        } else if share >= 0.2 {
            colors.warning
        } else {
            colors.fg
        };
        let slider = Slider::new(self.id.clone(), self.value)
            .range(1.0, self.max)
            .step(1.0);
        let slider = match self.on_change {
            Some(on_change) => {
                slider.on_change(move |value, window, cx| on_change(value, window, cx))
            }
            None => slider,
        };
        let marks = marks(self.max).into_iter().map(|step| {
            let at = ((step - 1.0) / (self.max - 1.0)) as f32;
            div()
                .absolute()
                .top_0()
                .left(relative(at))
                .w_0()
                .flex()
                .justify_center()
                .child(tabular(div()).child(format!("{step:.0}×")))
        });
        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .flex()
                    .justify_between()
                    .text_size(theme.text_size(TextSize::Sm))
                    .child(div().text_color(colors.fg_muted).child("Leverage"))
                    .child(
                        tabular(div())
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(ink)
                            .child(format!("{:.0}×", self.value)),
                    ),
            )
            .child(slider)
            .child(
                div()
                    .relative()
                    .h(theme.text_size(TextSize::Xs) * 1.5)
                    .mx(theme.slider_thumb() * 0.5)
                    .text_size(theme.text_size(TextSize::Xs))
                    .text_color(colors.fg_subtle)
                    .children(marks),
            )
    }
}

/// Margin in use against what the account holds, as a bar that turns amber, then red, and what is left.
#[derive(IntoElement)]
pub struct MarginIndicator {
    id: ElementId,
    used: f64,
    equity: f64,
}

impl MarginIndicator {
    pub fn new(id: impl Into<ElementId>, used: f64, equity: f64) -> Self {
        assert!(
            equity.is_finite() && used.is_finite() && used >= 0.0,
            "margin needs finite equity and a use of zero or more"
        );
        Self {
            id: id.into(),
            used,
            equity,
        }
    }
}

impl RenderOnce for MarginIndicator {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let share = match self.equity > 0.0 {
            true => (self.used / self.equity) as f32,
            false => 1.0,
        };
        let detail = format!(
            "${} of ${} · ${} free",
            price(self.used, 0),
            price(self.equity, 0),
            price((self.equity - self.used).max(0.0), 0)
        );
        Meter::new(self.id, "Margin used", share.min(1.0))
            .detail(detail)
            .thresholds(0.6, 0.85)
    }
}

/// How much risk a position carries, from none to all, on a dial that turns amber, then red.
#[derive(IntoElement)]
pub struct RiskMeter {
    id: ElementId,
    risk: f64,
}

impl RiskMeter {
    /// `risk` from 0 to 100.
    pub fn new(id: impl Into<ElementId>, risk: f64) -> Self {
        assert!((0.0..=100.0).contains(&risk), "risk reads from 0 to 100");
        Self {
            id: id.into(),
            risk,
        }
    }
}

impl RenderOnce for RiskMeter {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        Gauge::new(self.id, "Risk", self.risk, 0.0, 100.0)
            .decimals(0)
            .thresholds(0.5, 0.8)
    }
}

#[cfg(test)]
mod tests {
    use super::marks;

    #[test]
    fn leverage_marks_step_evenly_to_the_most() {
        assert_eq!(marks(100.0), [1.0, 20.0, 40.0, 60.0, 80.0, 100.0]);
        assert_eq!(marks(125.0), [1.0, 50.0, 100.0, 125.0]);
        assert_eq!(marks(20.0), [1.0, 5.0, 10.0, 15.0, 20.0]);
        assert_eq!(marks(3.0), [1.0, 2.0, 3.0]);
        assert_eq!(marks(2.0), [1.0, 2.0]);
    }
}
