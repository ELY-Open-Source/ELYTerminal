use gpui::SharedString;

/// A named run of values, one for each of the chart's categories.
#[derive(Clone, Debug, PartialEq)]
pub struct Series {
    pub(crate) name: SharedString,
    pub(crate) values: Vec<f64>,
}

/// The largest value a chart draws: sums and pixels of such values stay within `f32`.
pub const LIMIT: f64 = 1e30;

/// A value a chart can draw: finite and within ±`LIMIT`.
pub(crate) fn drawable(value: f64) -> bool {
    value.is_finite() && value.abs() <= LIMIT
}

impl Series {
    pub fn new(name: impl Into<SharedString>, values: impl IntoIterator<Item = f64>) -> Self {
        let values: Vec<f64> = values.into_iter().collect();
        assert!(
            values.iter().all(|value| drawable(*value)),
            "a series needs values, within charts::LIMIT"
        );
        Self {
            name: name.into(),
            values,
        }
    }
}

/// A named cloud of points, each an x and a y, sized for bubbles when `sizes` are given.
#[derive(Clone, Debug, PartialEq)]
pub struct Points {
    pub(crate) name: SharedString,
    pub(crate) points: Vec<(f64, f64)>,
    pub(crate) sizes: Option<Vec<f64>>,
}

impl Points {
    pub fn new(
        name: impl Into<SharedString>,
        points: impl IntoIterator<Item = (f64, f64)>,
    ) -> Self {
        let points: Vec<(f64, f64)> = points.into_iter().collect();
        assert!(
            points.iter().all(|(x, y)| drawable(*x) && drawable(*y)),
            "points need values, within charts::LIMIT"
        );
        Self {
            name: name.into(),
            points,
            sizes: None,
        }
    }

    /// A size for each point, drawn as its area; makes a bubble chart.
    pub fn sizes(mut self, sizes: impl IntoIterator<Item = f64>) -> Self {
        let sizes: Vec<f64> = sizes.into_iter().collect();
        assert_eq!(
            sizes.len(),
            self.points.len(),
            "a bubble needs a size for each point"
        );
        assert!(
            sizes.iter().all(|size| drawable(*size) && *size >= 0.0),
            "bubble sizes are not negative, within charts::LIMIT"
        );
        self.sizes = Some(sizes);
        self
    }
}

/// Each series' bottom and top at each category when stacked on the ones before it; negatives stack down from zero.
pub(crate) fn stacked(series: &[&[f64]]) -> Vec<Vec<(f64, f64)>> {
    let count = series.first().map_or(0, |values| values.len());
    let mut up = vec![0.0; count];
    let mut down = vec![0.0; count];
    series
        .iter()
        .map(|values| {
            values
                .iter()
                .enumerate()
                .map(|(ix, value)| {
                    let base = if *value >= 0.0 {
                        &mut up[ix]
                    } else {
                        &mut down[ix]
                    };
                    let bottom = *base;
                    *base += value;
                    (bottom, *base)
                })
                .collect()
        })
        .collect()
}

/// The lowest and highest a chart must show: zero, and every value or stacked total.
pub(crate) fn extent(values: impl Iterator<Item = f64>) -> (f64, f64) {
    values.fold((0.0, 0.0), |(low, high), value| {
        (low.min(value), high.max(value))
    })
}

/// Tangents for a smooth line through `points` that never overshoots between them (Fritsch–Carlson); points at one x rise straight.
pub(crate) fn tangents(points: &[(f32, f32)]) -> Vec<f32> {
    let count = points.len();
    if count < 2 {
        return vec![0.0; count];
    }
    let slopes: Vec<f32> = points
        .windows(2)
        .map(|pair| {
            let slope = (pair[1].1 - pair[0].1) / (pair[1].0 - pair[0].0);
            if slope.is_finite() { slope } else { 0.0 }
        })
        .collect();
    let mut tangents: Vec<f32> = (0..count)
        .map(|ix| match ix {
            0 => slopes[0],
            last if last == count - 1 => slopes[count - 2],
            ix if slopes[ix - 1] * slopes[ix] <= 0.0 => 0.0,
            ix => (slopes[ix - 1] + slopes[ix]) / 2.0,
        })
        .collect();
    for (ix, slope) in slopes.iter().enumerate() {
        if *slope == 0.0 {
            tangents[ix] = 0.0;
            tangents[ix + 1] = 0.0;
            continue;
        }
        let (a, b) = (tangents[ix] / slope, tangents[ix + 1] / slope);
        let length = a.hypot(b);
        if length > 3.0 {
            tangents[ix] = 3.0 * a / length * slope;
            tangents[ix + 1] = 3.0 * b / length * slope;
        }
    }
    tangents
}

/// A field as CSV reads it: quoted when it holds a comma, a quote or a line break.
fn field(text: &str) -> String {
    if text.contains([',', '"', '\n']) {
        format!("\"{}\"", text.replace('"', "\"\""))
    } else {
        text.to_string()
    }
}

/// Rows of comma-separated text: a header of `labels`, then each series' name and values.
pub(crate) fn csv(labels: &[SharedString], series: &[Series]) -> String {
    let head = std::iter::once(String::new())
        .chain(labels.iter().map(|label| field(label)))
        .collect::<Vec<_>>()
        .join(",");
    let rows = series.iter().map(|series| {
        std::iter::once(field(&series.name))
            .chain(series.values.iter().map(|value| value.to_string()))
            .collect::<Vec<_>>()
            .join(",")
    });
    std::iter::once(head)
        .chain(rows)
        .collect::<Vec<_>>()
        .join("\n")
}

/// Rows of comma-separated text: a header, then each point's series, x, y and size when it has one.
pub(crate) fn points_csv(clouds: &[Points]) -> String {
    let rows = clouds.iter().flat_map(|cloud| {
        cloud.points.iter().enumerate().map(move |(ix, (x, y))| {
            let size = cloud
                .sizes
                .as_ref()
                .map_or(String::new(), |sizes| sizes[ix].to_string());
            format!("{},{x},{y},{size}", field(&cloud.name))
        })
    });
    std::iter::once("series,x,y,size".to_string())
        .chain(rows)
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn points_at_one_x_rise_with_flat_tangents() {
        let rising = tangents(&[(10.0, 0.0), (10.0, 5.0), (10.0, 9.0), (11.0, 9.5)]);
        assert!(
            rising.iter().all(|tangent| tangent.is_finite()),
            "{rising:?}"
        );
        assert_eq!(&rising[..2], &[0.0, 0.0]);
    }

    #[test]
    fn stacks_rise_from_zero_and_negatives_fall() {
        let stacks = stacked(&[&[2.0, -1.0], &[3.0, -2.0]]);
        assert_eq!(
            stacks,
            [
                vec![(0.0, 2.0), (0.0, -1.0)],
                vec![(2.0, 5.0), (-1.0, -3.0)]
            ]
        );
        assert_eq!(extent([3.0, -2.0, 7.0].into_iter()), (-2.0, 7.0));
        assert_eq!(
            extent([3.0, 4.0].into_iter()),
            (0.0, 4.0),
            "zero stays in view"
        );
    }

    #[test]
    fn a_smooth_line_flattens_at_peaks_and_never_overshoots() {
        let points = [(0.0, 0.0), (1.0, 2.0), (2.0, 1.0), (3.0, 1.0)];
        let tangents = tangents(&points);
        assert_eq!(tangents[1], 0.0, "a peak is flat");
        assert_eq!(
            (tangents[2], tangents[3]),
            (0.0, 0.0),
            "a level run stays level"
        );
    }

    #[test]
    fn csv_has_a_header_and_a_row_per_series() {
        let labels = ["Jan", "Feb"].map(SharedString::from);
        let text = csv(&labels, &[Series::new("Sales, net", [1.0, 2.5])]);
        assert_eq!(text, ",Jan,Feb\n\"Sales, net\",1,2.5");
        let points = points_csv(&[
            Points::new("Paid", [(1.0, 2.0)]),
            Points::new("Cities", [(3.0, 4.0)]).sizes([9.0]),
        ]);
        assert_eq!(points, "series,x,y,size\nPaid,1,2,\nCities,3,4,9");
    }
}
