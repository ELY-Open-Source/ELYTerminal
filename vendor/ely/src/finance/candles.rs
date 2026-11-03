use jiff::Timestamp;

/// One period of a market: when it opened, its open, high, low and close, and how much traded.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Candle {
    pub time: Timestamp,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

impl Candle {
    pub fn new(
        time: Timestamp,
        (open, high, low, close): (f64, f64, f64, f64),
        volume: f64,
    ) -> Self {
        assert!(
            [open, high, low, close, volume]
                .iter()
                .all(|value| value.is_finite()),
            "a candle needs finite prices and volume"
        );
        assert!(
            high >= open.max(close) && low <= open.min(close),
            "a candle's high and low hold its open and close"
        );
        assert!(volume >= 0.0, "volume is zero or more");
        Self {
            time,
            open,
            high,
            low,
            close,
            volume,
        }
    }

    /// Whether it closed at or above where it opened.
    pub fn rose(&self) -> bool {
        self.close >= self.open
    }
}

/// Heikin-Ashi candles: each close the average of its four prices, each open the midpoint of the last one's body, which smooths the trend.
pub(crate) fn heikin_ashi(candles: &[Candle]) -> Vec<Candle> {
    let mut out: Vec<Candle> = Vec::with_capacity(candles.len());
    for candle in candles {
        let close = (candle.open + candle.high + candle.low + candle.close) / 4.0;
        let open = out
            .last()
            .map_or((candle.open + candle.close) / 2.0, |last| {
                (last.open + last.close) / 2.0
            });
        out.push(Candle {
            open,
            close,
            high: candle.high.max(open).max(close),
            low: candle.low.min(open).min(close),
            ..*candle
        });
    }
    out
}

/// A brick of a Renko chart: where it starts and ends, and the candle whose close laid it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Brick {
    pub from: f64,
    pub to: f64,
    pub at: usize,
}

/// Most boxes of one size a chart draws; finer ones cannot be told apart.
const MOST_BOXES: f64 = 1e5;

/// Whether boxes of `size` grid the candles: few enough by their travel, and exact in whole steps.
pub(crate) fn griddable(candles: &[Candle], size: f64) -> bool {
    let travel: f64 = candles
        .iter()
        .map(|candle| candle.high - candle.low)
        .sum::<f64>()
        + candles
            .windows(2)
            .map(|pair| (pair[1].close - pair[0].close).abs())
            .sum::<f64>();
    let reach = candles
        .iter()
        .map(|candle| candle.high.abs().max(candle.low.abs()))
        .fold(0.0, f64::max);
    travel / size <= MOST_BOXES && reach / size < 2f64.powi(53)
}

/// Renko bricks of `size`: one each time the close moves a whole brick past the last brick's far side, so turning takes two.
pub(crate) fn renko(candles: &[Candle], size: f64) -> Vec<Brick> {
    assert!(size > 0.0, "a brick needs a size");
    let Some(first) = candles.first() else {
        return Vec::new();
    };
    let mut bricks = Vec::new();
    let mut bottom = (first.close / size).floor() * size;
    let mut top = bottom;
    for (at, candle) in candles.iter().enumerate() {
        while candle.close >= top + size {
            bricks.push(Brick {
                from: top,
                to: top + size,
                at,
            });
            (bottom, top) = (top, top + size);
        }
        while candle.close <= bottom - size {
            bricks.push(Brick {
                from: bottom,
                to: bottom - size,
                at,
            });
            (top, bottom) = (bottom, bottom - size);
        }
    }
    bricks
}

/// A column of a point-and-figure chart: rising Xs or falling Os, from the box it starts in to the one it ends in.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Column {
    pub rising: bool,
    pub from: i64,
    pub to: i64,
}

/// Point-and-figure columns on boxes of `size`: a column grows while prices keep its way and turns once they come back `reversal` boxes.
pub(crate) fn point_and_figure(candles: &[Candle], size: f64, reversal: i64) -> Vec<Column> {
    assert!(
        size > 0.0 && reversal > 0,
        "boxes need a size and a reversal"
    );
    let mut columns: Vec<Column> = Vec::new();
    let boxed = |price: f64| (price / size).floor() as i64;
    for candle in candles {
        let (high, low) = (boxed(candle.high), boxed(candle.low));
        match columns.last_mut() {
            None => columns.push(Column {
                rising: true,
                from: low,
                to: high,
            }),
            Some(column) if column.rising => {
                if high > column.to {
                    column.to = high;
                } else if column.to - low >= reversal {
                    let from = column.to - 1;
                    columns.push(Column {
                        rising: false,
                        from,
                        to: low,
                    });
                }
            }
            Some(column) => {
                if low < column.to {
                    column.to = low;
                } else if high - column.to >= reversal {
                    let from = column.to + 1;
                    columns.push(Column {
                        rising: true,
                        from,
                        to: high,
                    });
                }
            }
        }
    }
    columns
}

/// The volume traded at each of `bands` prices across the candles in view, spread over each candle's range.
pub(crate) fn profile(
    candles: &[Candle],
    (low, high): (f64, f64),
    bands: usize,
) -> Vec<(f64, f64, f64)> {
    assert!(bands > 0 && high > low, "a profile needs bands over a span");
    let step = (high - low) / bands as f64;
    let mut traded = vec![0.0; bands];
    for candle in candles {
        let span = candle.high - candle.low;
        if span <= 0.0 {
            let band = ((candle.close - low) / step)
                .floor()
                .clamp(0.0, (bands - 1) as f64);
            traded[band as usize] += candle.volume;
            continue;
        }
        for (band, volume) in traded.iter_mut().enumerate() {
            let (from, to) = (low + step * band as f64, low + step * (band + 1) as f64);
            let overlap = (to.min(candle.high) - from.max(candle.low)).max(0.0);
            *volume += candle.volume * overlap / span;
        }
    }
    traded
        .into_iter()
        .enumerate()
        .map(|(band, volume)| {
            (
                low + step * band as f64,
                low + step * (band + 1) as f64,
                volume,
            )
        })
        .collect()
}

#[cfg(test)]
mod grid {
    use jiff::Timestamp;

    use super::{Candle, griddable};

    #[test]
    fn boxes_too_fine_for_their_prices_do_not_grid() {
        let candle = Candle::new(Timestamp::UNIX_EPOCH, (10.0, 11.0, 9.0, 10.0), 1.0);
        assert!(griddable(&[candle], 1.0));
        assert!(!griddable(&[candle], 1e-18), "past exact steps");
        assert!(!griddable(&[candle], 1e-5), "past the boxes a chart draws");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candles(closes: &[f64]) -> Vec<Candle> {
        closes
            .iter()
            .enumerate()
            .map(|(ix, close)| {
                let open = if ix == 0 { *close } else { closes[ix - 1] };
                let time = Timestamp::from_second(86_400 * ix as i64).expect("a day");
                Candle::new(
                    time,
                    (open, open.max(*close) + 0.5, open.min(*close) - 0.5, *close),
                    100.0,
                )
            })
            .collect()
    }

    #[test]
    fn heikin_ashi_averages_and_carries_the_body() {
        let smooth = heikin_ashi(&candles(&[10.0, 12.0]));
        assert_eq!(smooth[0].close, (10.0 + 10.5 + 9.5 + 10.0) / 4.0);
        assert_eq!(
            smooth[1].open,
            (smooth[0].open + smooth[0].close) / 2.0,
            "each opens mid-body of the last"
        );
        assert!(smooth[1].high >= smooth[1].open.max(smooth[1].close));
    }

    #[test]
    fn renko_lays_a_brick_per_whole_move_and_two_to_turn() {
        let bricks = renko(&candles(&[10.0, 13.1, 12.0, 10.9]), 1.0);
        let steps: Vec<(f64, f64)> = bricks.iter().map(|brick| (brick.from, brick.to)).collect();
        assert_eq!(
            steps,
            [(10.0, 11.0), (11.0, 12.0), (12.0, 13.0), (12.0, 11.0)],
            "a fall of two bricks turns once"
        );
        assert_eq!(bricks[1].at, 1);
    }

    #[test]
    fn point_and_figure_grows_then_turns_on_the_reversal() {
        let columns = point_and_figure(&candles(&[10.0, 14.0, 13.0, 10.0]), 1.0, 3);
        assert_eq!(columns.len(), 2);
        assert!(columns[0].rising && !columns[1].rising);
        assert_eq!(columns[0].to, 14, "the rise reached the fourteen box");
        assert_eq!(columns[1].to, 9, "the fall reached down past ten");
    }

    #[test]
    fn a_profile_spreads_each_candle_over_its_range() {
        let candle = Candle::new(Timestamp::UNIX_EPOCH, (10.0, 12.0, 10.0, 11.0), 100.0);
        let bands = profile(&[candle], (10.0, 14.0), 4);
        let traded: Vec<f64> = bands.iter().map(|band| band.2).collect();
        assert_eq!(
            traded,
            [50.0, 50.0, 0.0, 0.0],
            "half at each price it crossed"
        );
        assert_eq!((bands[3].0, bands[3].1), (13.0, 14.0));
    }

    #[test]
    fn narrow_and_flat_candles_count_all_their_volume() {
        let narrow = Candle::new(Timestamp::UNIX_EPOCH, (10.2, 10.3, 10.1, 10.2), 100.0);
        let flat = Candle::new(Timestamp::UNIX_EPOCH, (12.5, 12.5, 12.5, 12.5), 40.0);
        let traded: Vec<f64> = profile(&[narrow, flat], (10.0, 14.0), 4)
            .iter()
            .map(|band| band.2)
            .collect();
        assert!(
            (traded[0] - 100.0).abs() < 1e-9,
            "{traded:?}: a narrow candle's whole volume"
        );
        assert_eq!(traded[2], 40.0, "a flat candle lands in its band");
        assert!((traded.iter().sum::<f64>() - 140.0).abs() < 1e-9);
    }
}
