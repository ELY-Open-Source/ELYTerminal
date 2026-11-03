use std::rc::Rc;

use gpui::{
    App, Div, ElementId, IntoElement, RenderOnce, SharedString, StyleRefinement, Styled, Window,
    div, prelude::*,
};

use super::quotes::price;
use crate::{
    charts::{Geometry, Ink, Layout, Line, Linear, Pick, Plot, Scene, Tips, nice, nice_step, plot},
    theme::ActiveTheme,
    typography::format,
};

/// A leg of an options position: a call or a put at a strike, how many (negative when sold), and the premium paid or taken for each.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Leg {
    pub call: bool,
    pub strike: f64,
    pub quantity: f64,
    pub premium: f64,
}

/// What a position pays at expiry if the underlying ends at `at`.
pub(crate) fn payoff(legs: &[Leg], at: f64) -> f64 {
    legs.iter()
        .map(|leg| {
            let worth = if leg.call {
                (at - leg.strike).max(0.0)
            } else {
                (leg.strike - at).max(0.0)
            };
            leg.quantity * (worth - leg.premium)
        })
        .sum()
}

/// Where a payoff sampled at `prices` crosses zero, read between samples.
pub(crate) fn breakevens(prices: &[f64], values: &[f64]) -> Vec<f64> {
    prices
        .windows(2)
        .zip(values.windows(2))
        .filter(|(_, pair)| pair[0].signum() != pair[1].signum() && pair[0] != pair[1])
        .map(|(at, pair)| at[0] + (at[1] - at[0]) * pair[0] / (pair[0] - pair[1]))
        .collect()
}

/// What an options position pays at expiry across prices of the underlying: profit shaded above zero, loss below, the break-evens and the price marked. Hover reads the result at any price.
#[derive(IntoElement)]
pub struct PayoffDiagram {
    base: Div,
    id: ElementId,
    legs: Vec<Leg>,
    range: (f64, f64),
    spot: f64,
    red_up: bool,
}

impl PayoffDiagram {
    /// Prices from `range.0` to `range.1`, widened to round prices, and the underlying's price now.
    pub fn new(
        id: impl Into<ElementId>,
        legs: impl IntoIterator<Item = Leg>,
        range: (f64, f64),
        spot: f64,
    ) -> Self {
        assert!(
            range.0 < range.1 && range.0.is_finite() && range.1.is_finite(),
            "a range runs low to high"
        );
        Self {
            base: div(),
            id: id.into(),
            legs: legs.into_iter().collect(),
            range,
            spot,
            red_up: false,
        }
    }

    pub fn red_up(mut self) -> Self {
        self.red_up = true;
        self
    }
}

/// Samples between two round prices on the axis.
const PER_STEP: usize = 20;

impl Styled for PayoffDiagram {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for PayoffDiagram {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let ((start, end), ticks) = nice(self.range.0, self.range.1, 6);
        let step = nice_step(end - start, 6);
        let samples = ticks.len().saturating_sub(1).max(1) * PER_STEP + 1;
        let prices: Vec<f64> = (0..samples)
            .map(|ix| start + (end - start) * ix as f64 / (samples - 1) as f64)
            .collect();
        let values: Rc<Vec<f64>> =
            Rc::new(prices.iter().map(|at| payoff(&self.legs, *at)).collect());
        let labels: Vec<SharedString> = prices.iter().map(|at| price(*at, 2).into()).collect();
        let places = (-step.log10().floor()).max(0.0) as usize;
        let words: Vec<SharedString> = prices
            .iter()
            .enumerate()
            .map(|(ix, at)| {
                if ix % PER_STEP == 0 {
                    price(*at, places).into()
                } else {
                    SharedString::default()
                }
            })
            .collect();
        let (gain, loss) = if self.red_up {
            (Ink::Fall, Ink::Rise)
        } else {
            (Ink::Rise, Ink::Fall)
        };
        let drawn = values.clone();
        let layout: Layout = Rc::new(move |scene: &Scene| {
            let frame = scene.frame;
            let (low, high) = drawn.iter().fold((0.0f64, 0.0f64), |(low, high), value| {
                (low.min(*value), high.max(*value))
            });
            let ((low, high), ticks) = nice(low, high, 5);
            let ys = Linear::new((low, high), (frame.y + frame.h, frame.y));
            let x = |ix: usize| frame.x + frame.w * ix as f32 / (drawn.len() - 1) as f32;
            let zero = ys.at(0.0);
            let side = |pick: fn(f64) -> f64| {
                let mut outline: Vec<(f32, f32)> = drawn
                    .iter()
                    .enumerate()
                    .map(|(ix, value)| (x(ix), ys.at(pick(*value))))
                    .collect();
                outline.extend([(x(drawn.len() - 1), zero), (x(0), zero)]);
                outline
            };
            Geometry {
                ticks: ticks.iter().map(|tick| (ys.at(*tick), *tick)).collect(),
                labels: words
                    .iter()
                    .enumerate()
                    .map(|(ix, word)| (x(ix), word.clone()))
                    .collect(),
                lines: vec![Line {
                    ink: Ink::Strong,
                    points: drawn
                        .iter()
                        .enumerate()
                        .map(|(ix, value)| (x(ix), ys.at(*value)))
                        .collect(),
                    floor: None,
                }],
                shapes: vec![
                    (gain, side(|value| value.max(0.0))),
                    (loss, side(|value| value.min(0.0))),
                ],
                ..Geometry::default()
            }
        });
        let (named, read) = (labels.clone(), values.clone());
        let tips: Tips = Rc::new(move |ix, _| {
            let value = read[ix];
            let sign = if value > 0.0 { "+" } else { "" };
            let ink = if value >= 0.0 { gain } else { loss };
            (
                named[ix].clone(),
                vec![(
                    Some(ink),
                    "At expiry".into(),
                    format!("{sign}{}", format::currency(value, "USD")),
                )],
            )
        });
        let mut chart = Plot::new(self.id.clone(), Pick::Point, layout, tips);
        let evens = breakevens(&prices, &values);
        let nearest = |at: f64| {
            (((at - start) / (end - start)).clamp(0.0, 1.0) * (samples - 1) as f64).round() as usize
        };
        chart.notes = evens
            .iter()
            .map(|at| {
                (
                    nearest(*at),
                    SharedString::from(format!("Break-even {}", price(*at, 2))),
                )
            })
            .collect();
        if (start..=end).contains(&self.spot) {
            chart.notes.push((nearest(self.spot), "Now".into()));
        }
        chart.labels = labels;
        chart.format = Rc::new(|value| format::currency(value, "USD"));
        let mut base = div()
            .debug_selector(|| "chart-root".into())
            .h(cx.theme().chart().height);
        base.style().refine(self.base.style());
        plot(chart, base, window, cx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_spread_pays_between_its_strikes_and_breaks_even_where_it_crosses() {
        let spread = [
            Leg {
                call: true,
                strike: 100.0,
                quantity: 1.0,
                premium: 5.0,
            },
            Leg {
                call: true,
                strike: 110.0,
                quantity: -1.0,
                premium: 2.0,
            },
        ];
        assert_eq!(
            payoff(&spread, 90.0),
            -3.0,
            "both expire worthless: the net premium is lost"
        );
        assert_eq!(
            payoff(&spread, 120.0),
            7.0,
            "the most it pays: the width less the premium"
        );
        let prices: Vec<f64> = (90..=120).map(f64::from).collect();
        let values: Vec<f64> = prices.iter().map(|at| payoff(&spread, *at)).collect();
        let evens = breakevens(&prices, &values);
        assert_eq!(evens, [103.0]);
    }
}
