use std::cmp::Ordering;

use gpui::SharedString;

use super::{Cell, Column, Row};

/// How a filter rule compares a cell with its value.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Test {
    #[default]
    Contains,
    Equals,
    NotEquals,
    Above,
    Below,
    Empty,
}

impl Test {
    pub const ALL: [Test; 6] = [
        Test::Contains,
        Test::Equals,
        Test::NotEquals,
        Test::Above,
        Test::Below,
        Test::Empty,
    ];

    /// Its catalog key, which also names it.
    pub fn key(self) -> &'static str {
        match self {
            Self::Contains => "builder.test.contains",
            Self::Equals => "builder.test.equals",
            Self::NotEquals => "builder.test.not_equals",
            Self::Above => "builder.test.above",
            Self::Below => "builder.test.below",
            Self::Empty => "builder.test.empty",
        }
    }

    /// Whether the test reads a value at all.
    pub fn takes_value(self) -> bool {
        self != Self::Empty
    }
}

/// One filter rule: a column by key, a test, and the value it compares with.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FilterRule {
    pub column: SharedString,
    pub test: Test,
    pub value: SharedString,
}

/// One sort key: a column by key, and which way.
#[derive(Clone, Debug, PartialEq)]
pub struct SortKey {
    pub column: SharedString,
    pub rising: bool,
}

/// Whether a cell passes a test against `value`; numbers compare as numbers when both read as one.
pub(crate) fn passes(cell: &Cell, test: Test, value: &str) -> bool {
    let words = cell.words().to_lowercase();
    let value = value.trim().to_lowercase();
    let number = value.parse::<f64>().ok();
    match (test, cell.number(), number) {
        (Test::Empty, _, _) => words.is_empty(),
        (Test::Contains, _, _) => words.contains(&value),
        (Test::Equals, Some(cell), Some(value)) => cell == value,
        (Test::Equals, _, _) => words == value,
        (Test::NotEquals, Some(cell), Some(value)) => cell != value,
        (Test::NotEquals, _, _) => words != value,
        (Test::Above, Some(cell), Some(value)) => cell > value,
        (Test::Below, Some(cell), Some(value)) => cell < value,
        (Test::Above | Test::Below, _, _) => false,
    }
}

/// `order` with the rows that pass every rule, or any when `any`. A rule with no value yet, or for a column not shown, passes.
pub(crate) fn kept(
    rows: &[Row],
    columns: &[Column],
    order: Vec<usize>,
    rules: &[FilterRule],
    any: bool,
) -> Vec<usize> {
    let tests: Vec<(usize, &FilterRule)> = rules
        .iter()
        .filter(|rule| !rule.test.takes_value() || !rule.value.trim().is_empty())
        .filter_map(|rule| {
            columns
                .iter()
                .position(|column| column.key == rule.column)
                .map(|col| (col, rule))
        })
        .collect();
    if tests.is_empty() {
        return order;
    }
    order
        .into_iter()
        .filter(|ix| {
            let pass = |(col, rule): &(usize, &FilterRule)| {
                passes(&rows[*ix].cells[*col], rule.test, &rule.value)
            };
            if any {
                tests.iter().any(pass)
            } else {
                tests.iter().all(pass)
            }
        })
        .collect()
}

/// `order` sorted by each key in turn, the first deciding most; numbers by value and before the rest, the rest by their words.
pub(crate) fn sorted_by(rows: &[Row], mut order: Vec<usize>, keys: &[(usize, bool)]) -> Vec<usize> {
    order.sort_by(|a, b| {
        keys.iter().fold(Ordering::Equal, |so_far, (col, rising)| {
            so_far.then_with(|| {
                let (a, b) = (&rows[*a].cells[*col], &rows[*b].cells[*col]);
                let by = match (a.number(), b.number()) {
                    (Some(a), Some(b)) => a.total_cmp(&b),
                    (Some(_), None) => Ordering::Less,
                    (None, Some(_)) => Ordering::Greater,
                    (None, None) => a.words().to_lowercase().cmp(&b.words().to_lowercase()),
                };
                if *rising { by } else { by.reverse() }
            })
        })
    });
    order
}

/// `order` gathered by column `col`'s words, groups in the order they first appear.
pub(crate) fn groups(rows: &[Row], order: &[usize], col: usize) -> Vec<(SharedString, Vec<usize>)> {
    let mut groups: Vec<(SharedString, Vec<usize>)> = Vec::new();
    for ix in order {
        let name = rows[*ix].cells[col].words();
        match groups.iter_mut().find(|(seen, _)| *seen == name) {
            Some((_, members)) => members.push(*ix),
            None => groups.push((name, vec![*ix])),
        }
    }
    groups
}

/// Rows as comma-separated text, the titles first; fields with commas, quotes or breaks are quoted.
pub fn to_csv(columns: &[Column], rows: &[Row]) -> String {
    let field = |text: &str| {
        if text.contains([',', '"', '\n']) {
            format!("\"{}\"", text.replace('"', "\"\""))
        } else {
            text.to_string()
        }
    };
    let head = columns
        .iter()
        .map(|column| field(&column.title))
        .collect::<Vec<_>>()
        .join(",");
    let body = rows.iter().map(|row| {
        row.cells
            .iter()
            .map(|cell| field(&cell.words()))
            .collect::<Vec<_>>()
            .join(",")
    });
    std::iter::once(head)
        .chain(body)
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn durations_sort_by_length_not_by_their_words() {
        let rows: Vec<Row> = [("a", 600), ("b", 5_100), ("c", 90_000)]
            .into_iter()
            .map(|(key, seconds)| Row::new(key, [Cell::Duration(seconds)]))
            .collect();
        assert_eq!(sorted_by(&rows, vec![0, 1, 2], &[(0, true)]), [0, 1, 2]);
        assert_eq!(rows[1].cells[0].words(), "1h 25m");
    }

    fn table() -> (Vec<Column>, Vec<Row>) {
        let columns = vec![
            Column::new("name", "Name"),
            Column::new("region", "Region"),
            Column::new("sales", "Sales"),
        ];
        let rows = [
            ("Ada", "Europe", 30.0),
            ("Alan", "Europe", 12.0),
            ("Grace", "Americas", 30.0),
            ("Kay", "", 5.0),
        ]
        .into_iter()
        .map(|(name, region, sales)| Row::new(name, [name.into(), region.into(), sales.into()]))
        .collect();
        (columns, rows)
    }

    fn rule(column: &'static str, test: Test, value: &'static str) -> FilterRule {
        FilterRule {
            column: column.into(),
            test,
            value: value.into(),
        }
    }

    #[test]
    fn rules_keep_all_or_any_and_skip_the_unfinished() {
        let (columns, rows) = table();
        let all = || (0..4).collect::<Vec<_>>();
        assert_eq!(
            kept(
                &rows,
                &columns,
                all(),
                &[
                    rule("sales", Test::Above, "10"),
                    rule("region", Test::Equals, "europe")
                ],
                false
            ),
            [0, 1]
        );
        assert_eq!(
            kept(
                &rows,
                &columns,
                all(),
                &[
                    rule("sales", Test::Below, "10"),
                    rule("name", Test::Contains, "gra")
                ],
                true
            ),
            [2, 3]
        );
        assert_eq!(
            kept(
                &rows,
                &columns,
                all(),
                &[rule("region", Test::Empty, "")],
                false
            ),
            [3]
        );
        assert_eq!(
            kept(
                &rows,
                &columns,
                all(),
                &[rule("sales", Test::Above, " ")],
                false
            ),
            [0, 1, 2, 3],
            "no value yet"
        );
    }

    #[test]
    fn keys_sort_in_turn_and_groups_keep_first_seen_order() {
        let (_, rows) = table();
        assert_eq!(
            sorted_by(&rows, vec![0, 1, 2, 3], &[(2, false), (0, false)]),
            [2, 0, 1, 3]
        );
        let named: Vec<(String, Vec<usize>)> = groups(&rows, &[0, 1, 2, 3], 1)
            .into_iter()
            .map(|(name, rows)| (name.to_string(), rows))
            .collect();
        assert_eq!(
            named,
            [
                ("Europe".into(), vec![0, 1]),
                ("Americas".into(), vec![2]),
                ("".into(), vec![3])
            ]
        );
    }

    #[test]
    fn numbers_sort_before_words_in_a_mixed_column() {
        let cells: [Cell; 6] = [
            "11".into(),
            10.0.into(),
            "apple".into(),
            2.0.into(),
            Cell::Empty,
            "Banana".into(),
        ];
        let rows: Vec<Row> = cells
            .into_iter()
            .enumerate()
            .map(|(ix, cell)| Row::new(ix.to_string(), [cell]))
            .collect();
        let rising = sorted_by(&rows, (0..rows.len()).collect(), &[(0, true)]);
        assert_eq!(
            rising,
            [3, 1, 4, 0, 2, 5],
            "2, 10, then the words: empty, 11, apple, banana"
        );
        let many: Vec<Row> = (0..64)
            .map(|ix| {
                Row::new(
                    ix.to_string(),
                    [if ix % 3 == 0 {
                        Cell::from(format!("{ix}"))
                    } else {
                        Cell::from(ix as f64)
                    }],
                )
            })
            .collect();
        let order = sorted_by(&many, (0..64).rev().collect(), &[(0, true)]);
        let numbers = order.iter().take_while(|ix| *ix % 3 != 0).count();
        assert_eq!(numbers, 42, "every number comes before every word");
    }

    #[test]
    fn csv_quotes_what_needs_it() {
        let columns = vec![Column::new("a", "Name"), Column::new("b", "Note")];
        let rows = vec![Row::new("r", ["Ada".into(), "said \"hi\", twice".into()])];
        assert_eq!(
            to_csv(&columns, &rows),
            "Name,Note\nAda,\"said \"\"hi\"\", twice\""
        );
    }
}
