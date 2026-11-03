use gpui::{
    App, Div, ElementId, Entity, InteractiveElement, IntoElement, ParentElement, PathBuilder,
    Pixels, Point, Refineable, RenderOnce, StyleRefinement, Styled, Window, canvas, div, fill,
};

use super::quotes::moves;
use crate::{
    charts::{
        ChartTooltip, Linear, Rect, Tiles, anchored, at, compact, finish, measure, nice, place,
        ring, tracked,
    },
    theme::{ActiveTheme, TextSize},
    typography::{format, tabular},
};

/// Prices, each with a running total of size.
pub(crate) type Totals = Vec<(f64, f64)>;

/// A book's running totals, best price first: each bid's price with all size bid at it or higher, and each ask's with all size asked at it or lower.
pub(crate) fn depth(bids: &[(f64, f64)], asks: &[(f64, f64)]) -> (Totals, Totals) {
    let run = |mut side: Vec<(f64, f64)>, best_high: bool| {
        side.sort_by(|a, b| {
            if best_high {
                b.0.total_cmp(&a.0)
            } else {
                a.0.total_cmp(&b.0)
            }
        });
        let mut total = 0.0;
        side.into_iter()
            .map(|(price, size)| {
                total += size;
                (price, total)
            })
            .collect::<Vec<_>>()
    };
    (run(bids.to_vec(), true), run(asks.to_vec(), false))
}

/// The total on one side at a price: everything bid at it or higher, or asked at it or lower.
fn total_at(side: &[(f64, f64)], price: f64, bids: bool) -> Option<f64> {
    side.iter()
        .take_while(|(level, _)| {
            if bids {
                *level >= price
            } else {
                *level <= price
            }
        })
        .last()
        .map(|(_, total)| *total)
}

/// Orders waiting to buy and to sell as the running total at each price: bids step up to the left of the spread, asks to the right. Hover reads the total to any price.
#[derive(IntoElement)]
pub struct DepthChart {
    base: Div,
    id: ElementId,
    bids: Vec<(f64, f64)>,
    asks: Vec<(f64, f64)>,
    red_up: bool,
}

impl DepthChart {
    /// Bids and asks, each a price and the size waiting there; the best bid sits below the best ask.
    pub fn new(
        id: impl Into<ElementId>,
        bids: impl IntoIterator<Item = (f64, f64)>,
        asks: impl IntoIterator<Item = (f64, f64)>,
    ) -> Self {
        let (bids, asks): (Vec<_>, Vec<_>) =
            (bids.into_iter().collect(), asks.into_iter().collect());
        let fine = |side: &[(f64, f64)]| {
            side.iter()
                .all(|(price, size)| price.is_finite() && size.is_finite() && *size > 0.0)
        };
        assert!(
            fine(&bids) && fine(&asks) && !bids.is_empty() && !asks.is_empty(),
            "a book needs levels with positive sizes on both sides"
        );
        Self {
            base: div(),
            id: id.into(),
            bids,
            asks,
            red_up: false,
        }
    }

    /// Bids in red and asks in green, as markets in China and Japan read rises.
    pub fn red_up(mut self) -> Self {
        self.red_up = true;
        self
    }
}

impl Styled for DepthChart {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for DepthChart {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let tiles: Entity<Tiles> =
            window.use_keyed_state((self.id.clone(), "depth"), cx, |_, _| Tiles::default());
        let (bounds, hover) = (tiles.read(cx).bounds, tiles.read(cx).hover);
        let (bids, asks) = depth(&self.bids, &self.asks);
        let (best_bid, best_ask) = (bids[0].0, asks[0].0);
        let mid = (best_bid + best_ask) / 2.0;
        let reach = bids
            .iter()
            .chain(&asks)
            .map(|(price, _)| (price - mid).abs())
            .fold(0.0, f64::max);
        let most = bids
            .last()
            .expect("bids")
            .1
            .max(asks.last().expect("asks").1);
        let ((_, top), ticks) = nice(0.0, most, 4);
        let theme = cx.theme();
        let (colors, sizes, rem) = (theme.colors.clone(), theme.chart(), window.rem_size());
        let pixels = |length: gpui::Rems| f32::from(length.to_pixels(rem));
        let (inset, gutter, foot) = (
            pixels(sizes.inset),
            pixels(sizes.gutter),
            pixels(sizes.foot),
        );
        let frame = Rect {
            x: inset,
            y: inset * 3.0,
            w: (f32::from(bounds.size.width) - inset - gutter).max(0.0),
            h: (f32::from(bounds.size.height) - inset * 3.0 - foot).max(0.0),
        };
        let xs = Linear::new((mid - reach, mid + reach), (frame.x, frame.x + frame.w));
        let ys = Linear::new((0.0, top), (frame.y + frame.h, frame.y));
        let (rise, fall) = moves(self.red_up, cx);
        let text = |words: String, color| {
            div()
                .flex_none()
                .whitespace_nowrap()
                .text_size(theme.text_size(TextSize::Xs))
                .text_color(color)
                .child(words)
        };
        let edge = frame.x + frame.w;
        let mut words: Vec<Div> = ticks
            .iter()
            .map(|tick| {
                div()
                    .absolute()
                    .left(Pixels::from(edge))
                    .top(Pixels::from(ys.at(*tick)))
                    .h_0()
                    .flex()
                    .items_center()
                    .pl_2()
                    .child(tabular(text(compact(*tick), colors.fg_subtle)))
            })
            .collect();
        let (_, price_ticks) = nice(mid - reach, mid + reach, 5);
        let digits = crate::typography::format::decimals(
            price_ticks
                .get(1)
                .zip(price_ticks.first())
                .map_or(1.0, |(b, a)| b - a),
        )
        .max(2);
        words.extend(
            price_ticks
                .iter()
                .filter(|tick| (mid - reach..=mid + reach).contains(*tick))
                .map(|tick| {
                    let label = tabular(text(
                        format::number(*tick, digits, format::Separators::EN),
                        colors.fg_subtle,
                    ));
                    div()
                        .absolute()
                        .left(Pixels::from(xs.at(*tick)))
                        .top(Pixels::from(frame.y + frame.h))
                        .w_0()
                        .flex()
                        .justify_center()
                        .pt_1p5()
                        .child(label)
                }),
        );
        let spread = best_ask - best_bid;
        let heading = format!(
            "Spread {} · {}",
            format::number(spread, digits, format::Separators::EN),
            format::percent(spread / mid, 3, false)
        );
        words.push(
            div()
                .absolute()
                .left(Pixels::from(xs.at(mid)))
                .top_0()
                .w_0()
                .flex()
                .justify_center()
                .child(text(heading, colors.fg_muted)),
        );
        let steps = |side: &[(f64, f64)]| -> Vec<(f32, f32)> {
            let mut points = vec![(xs.at(side[0].0), ys.at(0.0))];
            let mut last = 0.0;
            for (price, total) in side {
                points.push((xs.at(*price), ys.at(last)));
                points.push((xs.at(*price), ys.at(*total)));
                last = *total;
            }
            points
        };
        let (bid_steps, ask_steps) = (steps(&bids), steps(&asks));
        let reading = hover.map(|column| {
            let price = xs.value(frame.x + column as f32);
            let is_bid = price < mid;
            let total = if is_bid {
                total_at(&bids, price, true)
            } else {
                total_at(&asks, price, false)
            };
            (price, is_bid, total.unwrap_or(0.0))
        });
        if let Some((price, is_bid, total)) = reading {
            let card = ChartTooltip::new(format::number(price, digits, format::Separators::EN))
                .row(
                    Some(if is_bid { rise } else { fall }),
                    if is_bid {
                        "Bids at or above"
                    } else {
                        "Asks at or below"
                    },
                    compact(total),
                );
            let x = xs.at(price);
            words.push(anchored(
                (x, ys.at(total)),
                x > f32::from(bounds.size.width) * 0.6,
                card,
            ));
        }
        let (stroke, bottom) = (sizes.stroke.to_pixels(rem), frame.y + frame.h);
        let mark = reading.map(|(price, _, total)| (xs.at(price), ys.at(total)));
        let palette = colors.clone();
        let mut root = div()
            .debug_selector(|| "chart-root".into())
            .relative()
            .h(sizes.height);
        root.style().refine(self.base.style());
        tracked(
            root,
            (self.id.clone(), "book").into(),
            &tiles,
            move |(x, y)| {
                frame
                    .contains((x, y))
                    .then(|| (x - frame.x).round() as usize)
            },
        )
        .child(
            canvas(
                |_, _, _| {},
                move |bounds, _, window, _| {
                    let origin: Point<Pixels> = bounds.origin;
                    for tick in &ticks {
                        let line = Rect {
                            x: frame.x,
                            y: ys.at(*tick),
                            w: frame.w,
                            h: f32::from(sizes.hairline),
                        };
                        window.paint_quad(fill(place(origin, line), palette.border.opacity(0.5)));
                    }
                    for (points, ink) in [(&bid_steps, rise), (&ask_steps, fall)] {
                        let (Some(first), Some(last)) = (points.first(), points.last()) else {
                            continue;
                        };
                        let mut area = PathBuilder::fill();
                        let mut edge = PathBuilder::stroke(stroke);
                        area.move_to(at(origin, *first));
                        edge.move_to(at(origin, *first));
                        points[1..].iter().for_each(|point| {
                            area.line_to(at(origin, *point));
                            edge.line_to(at(origin, *point));
                        });
                        area.line_to(at(origin, (last.0, bottom)));
                        area.close();
                        finish(area, ink.opacity(0.14), window);
                        finish(edge, ink, window);
                    }
                    let middle = Rect {
                        x: xs.at(mid),
                        y: frame.y,
                        w: f32::from(sizes.hairline),
                        h: frame.h,
                    };
                    window.paint_quad(fill(place(origin, middle), palette.border_strong));
                    if let Some(point) = mark {
                        let upright = Rect {
                            x: point.0,
                            y: frame.y,
                            w: f32::from(sizes.hairline),
                            h: frame.h,
                        };
                        window.paint_quad(fill(place(origin, upright), palette.fg_subtle));
                        ring(
                            at(origin, point),
                            stroke * 2.0,
                            (palette.bg, palette.fg),
                            stroke,
                            window,
                        );
                    }
                },
            )
            .absolute()
            .inset_0(),
        )
        .children(words)
        .child(measure(tiles, |tiles| &mut tiles.bounds))
    }
}

#[cfg(test)]
mod tests {
    use super::{depth, total_at};

    #[test]
    fn a_book_runs_totals_outward_from_the_spread() {
        let (bids, asks) = depth(
            &[(99.0, 2.0), (100.0, 1.0), (98.0, 3.0)],
            &[(102.0, 4.0), (101.0, 1.0)],
        );
        assert_eq!(bids, [(100.0, 1.0), (99.0, 3.0), (98.0, 6.0)]);
        assert_eq!(asks, [(101.0, 1.0), (102.0, 5.0)]);
        assert_eq!(
            total_at(&bids, 98.5, true),
            Some(3.0),
            "bids at 99 and above"
        );
        assert_eq!(total_at(&asks, 101.5, false), Some(1.0));
        assert_eq!(total_at(&bids, 100.5, true), None, "nothing bid that high");
    }
}
