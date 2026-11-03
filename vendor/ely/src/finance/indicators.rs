use super::candles::Candle;

/// A reading per candle; none until an indicator has seen enough.
pub(crate) type Reading = Vec<Option<f64>>;

/// A simple moving average of `period` values.
pub(crate) fn sma(values: &[f64], period: usize) -> Reading {
    assert!(period > 0, "an average needs a period");
    (0..values.len())
        .map(|ix| {
            (ix + 1 >= period)
                .then(|| values[ix + 1 - period..=ix].iter().sum::<f64>() / period as f64)
        })
        .collect()
}

/// An exponential moving average, seeded with the first period's simple average.
pub(crate) fn ema(values: &[f64], period: usize) -> Reading {
    assert!(period > 0, "an average needs a period");
    let weight = 2.0 / (period as f64 + 1.0);
    let mut out = vec![None; values.len()];
    if values.len() < period {
        return out;
    }
    let mut last = values[..period].iter().sum::<f64>() / period as f64;
    out[period - 1] = Some(last);
    for ix in period..values.len() {
        last = values[ix] * weight + last * (1.0 - weight);
        out[ix] = Some(last);
    }
    out
}

/// An average over only the readings there are, in place: leading gaps stay gaps.
fn ema_of(reading: &Reading, period: usize) -> Reading {
    let start = reading
        .iter()
        .position(Option::is_some)
        .unwrap_or(reading.len());
    let values: Vec<f64> = reading[start..]
        .iter()
        .map(|value| value.expect("readings run on once they start"))
        .collect();
    let mut out = vec![None; start];
    out.extend(ema(&values, period));
    out
}

/// Bollinger bands: the moving average, and `width` standard deviations below and above it.
pub(crate) fn bollinger(values: &[f64], period: usize, width: f64) -> Vec<Option<(f64, f64, f64)>> {
    sma(values, period)
        .into_iter()
        .enumerate()
        .map(|(ix, mean)| {
            let mean = mean?;
            let window = &values[ix + 1 - period..=ix];
            let spread = (window
                .iter()
                .map(|value| (value - mean).powi(2))
                .sum::<f64>()
                / period as f64)
                .sqrt();
            Some((mean - width * spread, mean, mean + width * spread))
        })
        .collect()
}

/// The volume-weighted average of each candle's typical price, from the first candle on.
pub(crate) fn vwap(candles: &[Candle]) -> Reading {
    let (mut weighted, mut volume) = (0.0, 0.0);
    candles
        .iter()
        .map(|candle| {
            weighted += (candle.high + candle.low + candle.close) / 3.0 * candle.volume;
            volume += candle.volume;
            (volume > 0.0).then(|| weighted / volume)
        })
        .collect()
}

/// MACD: a fast average less a slow one, its own average as the signal, and the gap between them.
pub(crate) struct Macd {
    pub line: Reading,
    pub signal: Reading,
    pub histogram: Reading,
}

pub(crate) fn macd(values: &[f64], (fast, slow, signal): (usize, usize, usize)) -> Macd {
    let (quick, steady) = (ema(values, fast), ema(values, slow));
    let line: Reading = quick
        .iter()
        .zip(&steady)
        .map(|(a, b)| Some((*a)? - (*b)?))
        .collect();
    let signal = ema_of(&line, signal);
    let histogram = line
        .iter()
        .zip(&signal)
        .map(|(a, b)| Some((*a)? - (*b)?))
        .collect();
    Macd {
        line,
        signal,
        histogram,
    }
}

/// The relative strength index with Wilder's smoothing: 100 when every change rose, 0 when every one fell.
pub(crate) fn rsi(values: &[f64], period: usize) -> Reading {
    assert!(period > 0, "an index needs a period");
    let mut out = vec![None; values.len()];
    if values.len() <= period {
        return out;
    }
    let change = |ix: usize| values[ix] - values[ix - 1];
    let (mut gain, mut loss) = (1..=period).fold((0.0, 0.0), |(gain, loss), ix| {
        (gain + change(ix).max(0.0), loss + (-change(ix)).max(0.0))
    });
    (gain, loss) = (gain / period as f64, loss / period as f64);
    let index = |gain: f64, loss: f64| {
        if loss == 0.0 {
            100.0
        } else {
            100.0 - 100.0 / (1.0 + gain / loss)
        }
    };
    out[period] = Some(index(gain, loss));
    for (ix, slot) in out.iter_mut().enumerate().skip(period + 1) {
        let smoothing = (period - 1) as f64;
        gain = (gain * smoothing + change(ix).max(0.0)) / period as f64;
        loss = (loss * smoothing + (-change(ix)).max(0.0)) / period as f64;
        *slot = Some(index(gain, loss));
    }
    out
}

/// The KDJ stochastic: where each close sits in the last `period` candles' range as K, K smoothed as D, and J pulling ahead of both.
pub(crate) fn kdj(candles: &[Candle], period: usize) -> (Reading, Reading, Reading) {
    assert!(period > 0, "a stochastic needs a period");
    let (mut k, mut d) = (50.0, 50.0);
    let (mut ks, mut ds, mut js) = (
        vec![None; candles.len()],
        vec![None; candles.len()],
        vec![None; candles.len()],
    );
    for ix in period.saturating_sub(1)..candles.len() {
        let window = &candles[ix + 1 - period..=ix];
        let (low, high) = window
            .iter()
            .fold((f64::MAX, f64::MIN), |(low, high), candle| {
                (low.min(candle.low), high.max(candle.high))
            });
        let raw = if high > low {
            (candles[ix].close - low) / (high - low) * 100.0
        } else {
            50.0
        };
        k = k * 2.0 / 3.0 + raw / 3.0;
        d = d * 2.0 / 3.0 + k / 3.0;
        (ks[ix], ds[ix], js[ix]) = (Some(k), Some(d), Some(3.0 * k - 2.0 * d));
    }
    (ks, ds, js)
}

/// On-balance volume: the running total of volume, added on a rising close and taken on a falling one.
pub(crate) fn obv(candles: &[Candle]) -> Vec<f64> {
    let mut total = 0.0;
    candles
        .iter()
        .enumerate()
        .map(|(ix, candle)| {
            if let Some(last) = ix.checked_sub(1).map(|before| candles[before].close) {
                total += if candle.close > last {
                    candle.volume
                } else if candle.close < last {
                    -candle.volume
                } else {
                    0.0
                };
            }
            total
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use jiff::Timestamp;

    use super::*;

    fn candle(close: f64, volume: f64) -> Candle {
        Candle::new(
            Timestamp::UNIX_EPOCH,
            (close, close + 1.0, close - 1.0, close),
            volume,
        )
    }

    #[test]
    fn averages_start_once_they_have_a_period() {
        assert_eq!(
            sma(&[1.0, 2.0, 3.0, 4.0], 2),
            [None, Some(1.5), Some(2.5), Some(3.5)]
        );
        assert_eq!(
            ema(&[1.0, 2.0, 3.0, 4.0, 5.0], 3),
            [None, None, Some(2.0), Some(3.0), Some(4.0)]
        );
    }

    #[test]
    fn bands_sit_two_deviations_either_side() {
        let bands = bollinger(&[2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0], 8, 2.0);
        assert_eq!(bands[7], Some((1.0, 5.0, 9.0)));
        assert_eq!(bands[6], None);
    }

    #[test]
    fn momentum_reads_rises_and_falls() {
        let rising: Vec<f64> = (0..20).map(f64::from).collect();
        assert_eq!(rsi(&rising, 14)[19], Some(100.0), "nothing fell");
        let falling: Vec<f64> = rising.iter().rev().copied().collect();
        assert_eq!(rsi(&falling, 14)[19], Some(0.0), "nothing rose");
        let trend = macd(&rising, (3, 6, 2));
        assert_eq!(trend.line[4], None);
        assert!(trend.line[5].is_some() && trend.signal[5].is_none() && trend.signal[6].is_some());
        assert!(
            trend.line[10].expect("a line") > 0.0,
            "a rise holds the fast average above the slow"
        );
    }

    #[test]
    fn volume_follows_the_close() {
        let candles = [
            candle(10.0, 100.0),
            candle(11.0, 200.0),
            candle(10.5, 50.0),
            candle(10.5, 70.0),
        ];
        assert_eq!(obv(&candles), [0.0, 200.0, 150.0, 150.0]);
        let mean = vwap(&candles);
        assert_eq!(mean[0], Some(10.0));
        assert!((mean[1].expect("a price") - (10.0 * 100.0 + 11.0 * 200.0) / 300.0).abs() < 1e-9);
        let (k, d, j) = kdj(&candles, 2);
        assert_eq!(k[0], None);
        assert!(k[1].is_some() && d[1].is_some());
        assert!(
            (j[1].expect("j") - (3.0 * k[1].expect("k") - 2.0 * d[1].expect("d"))).abs() < 1e-9
        );
    }
}
