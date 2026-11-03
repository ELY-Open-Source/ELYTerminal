/// Density over a view in square cells, the densest cell 1.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Heat {
    pub columns: usize,
    pub rows: usize,
    pub cell: f32,
    pub values: Vec<f32>,
}

impl Heat {
    pub fn at(&self, column: usize, row: usize) -> f32 {
        self.values[row * self.columns + column]
    }
}

/// Each weighted view point spreads a quartic kernel reaching `reach` pixels from it over cells `cell` pixels square across a view of `size`; the sum is scaled so its densest cell is 1. Fails on a negative or non-finite weight.
pub(crate) fn density(
    points: &[((f32, f32), f32)],
    (w, h): (f32, f32),
    cell: f32,
    reach: f32,
) -> Heat {
    assert!(
        cell > 0.0 && reach > 0.0,
        "heat cells of {cell} reaching {reach}"
    );
    let (columns, rows) = (
        (w / cell).ceil().max(0.0) as usize,
        (h / cell).ceil().max(0.0) as usize,
    );
    let mut values = vec![0.0f32; columns * rows];
    for &((x, y), weight) in points {
        assert!(
            weight.is_finite() && weight >= 0.0,
            "a heat weight of {weight}"
        );
        let span = |at: f32, count: usize| {
            let first = ((at - reach) / cell).floor().max(0.0) as usize;
            let last = (((at + reach) / cell).ceil().max(0.0) as usize).min(count);
            first..last
        };
        for row in span(y, rows) {
            for column in span(x, columns) {
                let (cx, cy) = ((column as f32 + 0.5) * cell, (row as f32 + 0.5) * cell);
                let near = ((cx - x).powi(2) + (cy - y).powi(2)) / reach.powi(2);
                if near < 1.0 {
                    values[row * columns + column] += weight * (1.0 - near).powi(2);
                }
            }
        }
    }
    let most = values.iter().copied().fold(0.0f32, f32::max);
    if most > 0.0 {
        values.iter_mut().for_each(|value| *value /= most);
    }
    Heat {
        columns,
        rows,
        cell,
        values,
    }
}

#[cfg(test)]
mod tests {
    use super::density;

    #[test]
    fn heat_peaks_at_its_point_and_ends_at_its_reach() {
        let heat = density(&[((25.0, 25.0), 2.0)], (100.0, 60.0), 10.0, 20.0);
        assert_eq!((heat.columns, heat.rows), (10, 6));
        assert_eq!(
            heat.at(2, 2),
            1.0,
            "the cell under the point is the densest"
        );
        assert_eq!(
            heat.at(3, 2),
            0.5625,
            "halfway out a quartic kernel keeps (1 - 1/4) squared"
        );
        assert_eq!(heat.at(6, 2), 0.0, "past its reach nothing");
    }

    #[test]
    fn heavier_points_run_hotter_and_heat_adds_up() {
        let heat = density(
            &[((15.0, 15.0), 1.0), ((85.0, 15.0), 3.0)],
            (100.0, 30.0),
            10.0,
            15.0,
        );
        assert!(
            heat.at(8, 1) > heat.at(1, 1) * 2.5,
            "three times the weight"
        );
        let pair = density(
            &[((45.0, 15.0), 1.0), ((55.0, 15.0), 1.0)],
            (100.0, 30.0),
            10.0,
            15.0,
        );
        assert_eq!(
            pair.at(4, 1),
            pair.at(5, 1),
            "two alike points meet in the middle"
        );
        let none = density(&[], (100.0, 30.0), 10.0, 15.0);
        assert!(
            none.values.iter().all(|value| *value == 0.0),
            "no points, no heat"
        );
    }

    #[test]
    #[should_panic(expected = "a heat weight of -1")]
    fn a_negative_weight_fails() {
        density(&[((0.0, 0.0), -1.0)], (10.0, 10.0), 5.0, 5.0);
    }
}
