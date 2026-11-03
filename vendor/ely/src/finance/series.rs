use gpui::SharedString;

use super::{
    candles::Candle,
    indicators::{Reading, bollinger, ema, kdj, macd, obv, rsi, sma, vwap},
};

/// A line drawn over the prices.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Overlay {
    /// A simple moving average of this many closes.
    Sma(usize),
    /// An exponential moving average of this many closes.
    Ema(usize),
    /// Bollinger bands over this many closes, this many deviations wide.
    Bollinger(usize, f64),
    /// The volume-weighted average price.
    Vwap,
}

/// A pane of its own under the prices.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Study {
    Macd,
    Rsi,
    Kdj,
    Obv,
}

/// What an overlay or a study draws: its name, its lines in palette colors, a band to shade, bars about zero, guide levels, and a fixed range when it has one.
#[derive(Clone, Debug, Default)]
pub(crate) struct Drawn {
    pub name: SharedString,
    pub lines: Vec<(usize, Reading)>,
    pub band: Option<(Reading, Reading)>,
    pub bars: Option<Reading>,
    pub guides: Vec<f64>,
    pub range: Option<(f64, f64)>,
}

fn closes(candles: &[Candle]) -> Vec<f64> {
    candles.iter().map(|candle| candle.close).collect()
}

/// What an overlay draws for these candles; `color` is its place in the palette.
pub(crate) fn overlay(overlay: Overlay, candles: &[Candle], color: usize) -> Drawn {
    let closes = closes(candles);
    match overlay {
        Overlay::Sma(period) => Drawn {
            name: format!("SMA {period}").into(),
            lines: vec![(color, sma(&closes, period))],
            ..Drawn::default()
        },
        Overlay::Ema(period) => Drawn {
            name: format!("EMA {period}").into(),
            lines: vec![(color, ema(&closes, period))],
            ..Drawn::default()
        },
        Overlay::Vwap => Drawn {
            name: "VWAP".into(),
            lines: vec![(color, vwap(candles))],
            ..Drawn::default()
        },
        Overlay::Bollinger(period, width) => {
            let bands = bollinger(&closes, period, width);
            let part = |pick: fn((f64, f64, f64)) -> f64| {
                bands.iter().map(|band| band.map(pick)).collect::<Reading>()
            };
            Drawn {
                name: format!("BOLL {period} {width}").into(),
                lines: vec![(color, part(|band| band.1))],
                band: Some((part(|band| band.0), part(|band| band.2))),
                ..Drawn::default()
            }
        }
    }
}

/// What a study draws for these candles, in the palette from `color` on.
pub(crate) fn study(study: Study, candles: &[Candle], color: usize) -> Drawn {
    let closes = closes(candles);
    match study {
        Study::Macd => {
            let trend = macd(&closes, (12, 26, 9));
            Drawn {
                name: "MACD 12 26 9".into(),
                lines: vec![(color, trend.line), (color + 1, trend.signal)],
                bars: Some(trend.histogram),
                guides: vec![0.0],
                ..Drawn::default()
            }
        }
        Study::Rsi => Drawn {
            name: "RSI 14".into(),
            lines: vec![(color, rsi(&closes, 14))],
            guides: vec![30.0, 70.0],
            range: Some((0.0, 100.0)),
            ..Drawn::default()
        },
        Study::Kdj => {
            let (k, d, j) = kdj(candles, 9);
            Drawn {
                name: "KDJ 9".into(),
                lines: vec![(color, k), (color + 1, d), (color + 2, j)],
                guides: vec![20.0, 80.0],
                ..Drawn::default()
            }
        }
        Study::Obv => Drawn {
            name: "OBV".into(),
            lines: vec![(color, obv(candles).into_iter().map(Some).collect())],
            ..Drawn::default()
        },
    }
}

impl Drawn {
    /// The lowest and highest it reaches over `range` of candles, bars and bands included; its fixed range when it has one.
    pub(crate) fn reach(&self, range: std::ops::Range<usize>) -> Option<(f64, f64)> {
        if self.range.is_some() {
            return self.range;
        }
        let band = self.band.iter().flat_map(|(low, high)| [low, high]);
        let readings = self
            .lines
            .iter()
            .map(|(_, reading)| reading)
            .chain(self.bars.iter())
            .chain(band);
        let values = readings.flat_map(|reading| reading[range.clone()].iter().flatten().copied());
        let guides = self.guides.iter().copied().filter(|_| self.bars.is_some());
        values.chain(guides).fold(None, |reach, value| {
            let (low, high) = reach.unwrap_or((value, value));
            Some((low.min(value), high.max(value)))
        })
    }
}

#[cfg(test)]
mod tests {
    use jiff::Timestamp;

    use super::*;

    fn rising(count: usize) -> Vec<Candle> {
        (0..count)
            .map(|ix| {
                let close = 100.0 + ix as f64;
                Candle::new(
                    Timestamp::UNIX_EPOCH,
                    (close - 0.5, close + 1.0, close - 1.0, close),
                    10.0,
                )
            })
            .collect()
    }

    #[test]
    fn overlays_name_themselves_and_reach_over_what_shows() {
        let candles = rising(30);
        let average = overlay(Overlay::Sma(5), &candles, 2);
        assert_eq!(average.name.as_ref(), "SMA 5");
        assert_eq!(average.reach(0..4), None, "nothing to average yet");
        assert_eq!(average.reach(4..6), Some((102.0, 103.0)));
        let bands = overlay(Overlay::Bollinger(20, 2.0), &candles, 0);
        let (low, high) = bands.reach(19..30).expect("bands");
        assert!(
            low < 110.0 && high > 128.0,
            "the bands spread past the average"
        );
    }

    #[test]
    fn studies_keep_their_ranges_and_guides() {
        let candles = rising(40);
        let strength = study(Study::Rsi, &candles, 0);
        assert_eq!(
            strength.reach(0..40),
            Some((0.0, 100.0)),
            "RSI always reads out of a hundred"
        );
        let trend = study(Study::Macd, &candles, 0);
        assert_eq!(trend.lines.len(), 2);
        let (low, _) = trend.reach(30..40).expect("macd");
        assert!(low <= 0.0, "the zero guide stays in view with the bars");
    }
}
