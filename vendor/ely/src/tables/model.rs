use gpui::Hsla;

use super::{Aggregate, Row};
use crate::theme::{Mix, Palette};

/// Rows holding `query` in any cell, ignoring case; every row when it is empty.
pub(crate) fn filtered(rows: &[Row], query: &str) -> Vec<usize> {
    let query = query.trim().to_lowercase();
    (0..rows.len())
        .filter(|ix| {
            query.is_empty()
                || rows[*ix]
                    .cells
                    .iter()
                    .any(|cell| cell.words().to_lowercase().contains(&query))
        })
        .collect()
}

/// Values combined as a figure; none when there are none, save a count.
pub(crate) fn combine(values: &[f64], how: Aggregate) -> Option<f64> {
    if how == Aggregate::Count {
        return Some(values.len() as f64);
    }
    if values.is_empty() {
        return None;
    }
    let sum: f64 = values.iter().sum();
    Some(match how {
        Aggregate::Sum => sum,
        Aggregate::Average => sum / values.len() as f64,
        Aggregate::Min => values.iter().copied().fold(f64::MAX, f64::min),
        Aggregate::Max => values.iter().copied().fold(f64::MIN, f64::max),
        Aggregate::Count => unreachable!("counted above"),
    })
}

/// Column `col`'s figure over `order`; none when no cell counts.
pub(crate) fn figure(rows: &[Row], order: &[usize], col: usize, how: Aggregate) -> Option<f64> {
    let values: Vec<f64> = order
        .iter()
        .filter_map(|ix| rows[*ix].cells[col].number())
        .collect();
    combine(&values, how)
}

/// The lowest and highest numbers in column `col`, across every row.
pub(crate) fn range(rows: &[Row], col: usize) -> Option<(f64, f64)> {
    rows.iter()
        .filter_map(|row| row.cells[col].number())
        .fold(None, |range, value| match range {
            None => Some((value, value)),
            Some((low, high)) => Some((low.min(value), high.max(value))),
        })
}

/// A value's tint in a range: red at the bottom, clear in the middle, green at the top.
pub(crate) fn tint(value: f64, (low, high): (f64, f64), colors: &Palette) -> Hsla {
    let share = if high > low {
        ((value - low) / (high - low)) as f32
    } else {
        0.5
    };
    if share < 0.5 {
        colors
            .danger_subtle
            .mix(&colors.danger_subtle.opacity(0.0), share * 2.0)
    } else {
        colors
            .success_subtle
            .opacity(0.0)
            .mix(&colors.success_subtle, share * 2.0 - 1.0)
    }
}

/// The part of `order` on page `page`, counted from zero, `size` rows a page.
pub(crate) fn page(order: &[usize], page: usize, size: usize) -> &[usize] {
    let start = (page * size).min(order.len());
    &order[start..(start + size).min(order.len())]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{data_display::Tone, tables::Cell};

    fn rows() -> Vec<Row> {
        [
            [
                "Grace".into(),
                42.0.into(),
                Cell::Tag("Live".into(), Tone::Success),
            ],
            [
                "alan".into(),
                7.0.into(),
                Cell::Tag("Draft".into(), Tone::Neutral),
            ],
            ["Ada".into(), 7.0.into(), Cell::Empty],
            [
                "Katherine".into(),
                105.5.into(),
                Cell::Tag("Live".into(), Tone::Success),
            ],
        ]
        .into_iter()
        .enumerate()
        .map(|(ix, cells)| Row::new(ix.to_string(), cells))
        .collect()
    }

    #[test]
    fn a_query_keeps_rows_holding_it_anywhere() {
        assert_eq!(filtered(&rows(), "live"), [0, 3]);
        assert_eq!(filtered(&rows(), "  "), [0, 1, 2, 3]);
    }

    #[test]
    fn numbers_sort_by_value_and_words_ignore_case() {
        let rows = rows();
        let sorted = |col: usize, rising: bool| {
            crate::tables::rules::sorted_by(&rows, vec![0, 1, 2, 3], &[(col, rising)])
        };
        assert_eq!(sorted(1, true), [1, 2, 0, 3], "ties keep their order");
        assert_eq!(sorted(1, false), [3, 0, 1, 2]);
        assert_eq!(sorted(0, true), [2, 1, 0, 3]);
    }

    #[test]
    fn figures_count_only_what_counts() {
        let rows = rows();
        let all = [0, 1, 2, 3];
        assert_eq!(figure(&rows, &all, 1, Aggregate::Sum), Some(161.5));
        assert_eq!(figure(&rows, &all, 1, Aggregate::Average), Some(40.375));
        assert_eq!(figure(&rows, &[1, 2], 1, Aggregate::Max), Some(7.0));
        assert_eq!(figure(&rows, &all, 0, Aggregate::Sum), None);
        assert_eq!(figure(&rows, &all, 0, Aggregate::Count), Some(0.0));
        assert_eq!(range(&rows, 1), Some((7.0, 105.5)));
    }

    #[test]
    fn a_page_is_a_slice_that_stops_at_the_end() {
        let order: Vec<usize> = (0..7).collect();
        assert_eq!(page(&order, 1, 3), [3, 4, 5]);
        assert_eq!(page(&order, 2, 3), [6]);
        assert!(page(&order, 5, 3).is_empty());
    }
}
