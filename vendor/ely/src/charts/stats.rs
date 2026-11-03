use super::scale::nice_step;

/// Bins of a round width near a `count`th of the data's range, each with its start, its end, and how many values fall in it; the top edge counts in the last.
pub(crate) fn bins(values: &[f64], count: usize) -> Vec<(f64, f64, usize)> {
    assert!(count > 0, "a histogram needs a bin");
    if values.is_empty() {
        return Vec::new();
    }
    let (low, high) = values
        .iter()
        .fold((f64::MAX, f64::MIN), |(low, high), value| {
            (low.min(*value), high.max(*value))
        });
    let width = nice_step(high - low, count);
    let start = (low / width).floor() * width;
    let many = (((high - start) / width).ceil() as usize).max(1);
    let mut bins: Vec<(f64, f64, usize)> = (0..many)
        .map(|ix| {
            (
                start + width * ix as f64,
                start + width * (ix + 1) as f64,
                0,
            )
        })
        .collect();
    for value in values {
        let ix = (((value - start) / width) as usize).min(many - 1);
        bins[ix].2 += 1;
    }
    bins
}

/// A spread in five numbers: the whiskers' ends within 1.5 spreads of the box, the quartiles, the median; and the values past the whiskers.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Five {
    pub low: f64,
    pub q1: f64,
    pub median: f64,
    pub q3: f64,
    pub high: f64,
    pub outliers: Vec<f64>,
}

/// The value `share` of the way through sorted `values`, between neighbors.
fn quantile(sorted: &[f64], share: f64) -> f64 {
    let at = share * (sorted.len() - 1) as f64;
    let (below, above) = (at.floor() as usize, at.ceil() as usize);
    let (low, high, part) = (sorted[below], sorted[above], at - below as f64);
    if low == high {
        return low;
    }
    let lerp = low + (high - low) * part;
    if lerp.is_finite() {
        lerp
    } else {
        low * (1.0 - part) + high * part
    }
}

pub(crate) fn five(values: &[f64]) -> Five {
    assert!(!values.is_empty(), "a box needs values");
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let (q1, median, q3) = (
        quantile(&sorted, 0.25),
        quantile(&sorted, 0.5),
        quantile(&sorted, 0.75),
    );
    let reach = (q3 - q1) * 1.5;
    let inside = |value: &&f64| **value >= q1 - reach && **value <= q3 + reach;
    let low = *sorted
        .iter()
        .find(inside)
        .expect("the quartiles lie inside");
    let high = *sorted
        .iter()
        .rev()
        .find(inside)
        .expect("the quartiles lie inside");
    let outliers = sorted
        .iter()
        .copied()
        .filter(|value| *value < low || *value > high)
        .collect();
    Five {
        low,
        q1,
        median,
        q3,
        high,
        outliers,
    }
}

/// A smooth estimate of where values gather, sampled at `count` points across a little past their range: a Gaussian kernel with Silverman's bandwidth.
pub(crate) fn density(values: &[f64], count: usize) -> Vec<(f64, f64)> {
    assert!(
        values.len() >= 2 && count >= 2,
        "a density needs two values and two samples"
    );
    let n = values.len() as f64;
    let mean = values.iter().sum::<f64>() / n;
    let spread = (values
        .iter()
        .map(|value| (value - mean).powi(2))
        .sum::<f64>()
        / (n - 1.0))
        .sqrt();
    let bandwidth = (1.06 * spread * n.powf(-0.2)).max(f64::EPSILON);
    let (low, high) = values
        .iter()
        .fold((f64::MAX, f64::MIN), |(low, high), value| {
            (low.min(*value), high.max(*value))
        });
    let (from, to) = (low - bandwidth, high + bandwidth);
    let norm = 1.0 / (n * bandwidth * (2.0 * std::f64::consts::PI).sqrt());
    (0..count)
        .map(|ix| {
            let at = from + (to - from) * ix as f64 / (count - 1) as f64;
            let sum: f64 = values
                .iter()
                .map(|value| (-0.5 * ((at - value) / bandwidth).powi(2)).exp())
                .sum();
            (at, sum * norm)
        })
        .collect()
}

/// A waterfall bar's kind: a rise, a fall, or a total that stands on zero.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Step {
    Up,
    Down,
    Total,
}

/// Each change floating from the running total, bottom and top; a total stands on zero, reaching up or down to the sum so far.
pub(crate) fn steps(changes: &[(f64, bool)]) -> Vec<(f64, f64, Step)> {
    let mut running = 0.0f64;
    changes
        .iter()
        .map(|(value, total)| {
            if *total {
                return (running.min(0.0), running.max(0.0), Step::Total);
            }
            let from = running;
            running += value;
            (
                from.min(running),
                from.max(running),
                if *value >= 0.0 { Step::Up } else { Step::Down },
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quartiles_across_all_of_f64_stay_finite() {
        let spread = five(&[-f64::MAX, f64::MAX]);
        assert!(spread.q1.is_finite() && spread.q3.is_finite());
        assert_eq!((spread.low, spread.high), (-f64::MAX, f64::MAX));
    }

    #[test]
    fn values_fall_into_equal_bins() {
        let bins = bins(&[1.0, 2.0, 2.5, 9.0, 10.0], 5);
        assert_eq!(bins.len(), 5);
        assert_eq!((bins[0].0, bins[4].1), (0.0, 10.0));
        let counts: Vec<usize> = bins.iter().map(|bin| bin.2).collect();
        assert_eq!(
            counts,
            [1, 2, 0, 0, 2],
            "the top edge counts in the last bin"
        );
    }

    #[test]
    fn five_numbers_and_outliers() {
        let spread = five(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 100.0]);
        assert_eq!((spread.q1, spread.median, spread.q3), (3.0, 5.0, 7.0));
        assert_eq!((spread.low, spread.high), (1.0, 8.0));
        assert_eq!(spread.outliers, [100.0]);
    }

    #[test]
    fn density_peaks_where_values_gather() {
        let samples = density(&[0.0, 0.1, 0.2, 5.0], 50);
        let peak = samples
            .iter()
            .max_by(|a, b| a.1.total_cmp(&b.1))
            .expect("samples");
        assert!(
            peak.0 < 1.0,
            "the peak sits near the cluster, at {}",
            peak.0
        );
    }

    #[test]
    fn a_total_below_zero_hangs_down_from_it() {
        let bars = steps(&[(-10.0, false), (0.0, true)]);
        let (bottom, top, step) = bars[1];
        assert_eq!(
            (bottom, top, step),
            (-10.0, 0.0, Step::Total),
            "the bar runs from the total up to zero"
        );
    }

    #[test]
    fn a_waterfall_floats_changes_and_stands_totals_on_zero() {
        let bars = steps(&[(100.0, false), (-30.0, false), (0.0, true), (20.0, false)]);
        assert_eq!(
            bars,
            [
                (0.0, 100.0, Step::Up),
                (70.0, 100.0, Step::Down),
                (0.0, 70.0, Step::Total),
                (70.0, 90.0, Step::Up)
            ]
        );
    }
}
