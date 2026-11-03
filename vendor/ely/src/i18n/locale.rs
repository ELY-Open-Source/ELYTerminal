use gpui::Styled;
use jiff::{Timestamp, tz::TimeZone};

use crate::typography::format::{self, Separators};

/// Which way a line of text runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Ltr,
    Rtl,
}

impl Direction {
    /// A flex row that starts at the reading side.
    pub fn row<E: Styled>(self, element: E) -> E {
        match self {
            Direction::Ltr => element.flex().flex_row(),
            Direction::Rtl => element.flex().flex_row_reverse(),
        }
    }

    /// Text set against the reading side.
    pub fn align<E: Styled>(self, element: E) -> E {
        match self {
            Direction::Ltr => element.text_left(),
            Direction::Rtl => element.text_right(),
        }
    }
}

/// A language as it writes numbers, dates and lines.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Locale {
    pub tag: &'static str,
    pub name: &'static str,
    pub separators: Separators,
    /// A numeric date, as `strftime` reads it.
    pub date: &'static str,
    pub direction: Direction,
}

const fn locale(
    tag: &'static str,
    name: &'static str,
    (group, decimal): (char, char),
    date: &'static str,
    direction: Direction,
) -> Locale {
    Locale {
        tag,
        name,
        separators: Separators { group, decimal },
        date,
        direction,
    }
}

/// The locales Ely formats for, with CLDR's short numeric dates.
pub const LOCALES: [Locale; 7] = [
    locale(
        "en-US",
        "English (US)",
        (',', '.'),
        "%-m/%-d/%y",
        Direction::Ltr,
    ),
    locale(
        "en-GB",
        "English (UK)",
        (',', '.'),
        "%d/%m/%Y",
        Direction::Ltr,
    ),
    locale("de-DE", "Deutsch", ('.', ','), "%d.%m.%y", Direction::Ltr),
    locale(
        "fr-FR",
        "Français",
        ('\u{202f}', ','),
        "%d/%m/%Y",
        Direction::Ltr,
    ),
    locale("ja-JP", "日本語", (',', '.'), "%Y/%m/%d", Direction::Ltr),
    locale("zh-CN", "中文", (',', '.'), "%Y/%-m/%-d", Direction::Ltr),
    locale("he-IL", "עברית", (',', '.'), "%-d.%-m.%Y", Direction::Rtl),
];

impl Locale {
    /// The locale with this tag; fails on one Ely does not list.
    pub fn of(tag: &str) -> Locale {
        *LOCALES
            .iter()
            .find(|locale| locale.tag == tag)
            .unwrap_or_else(|| panic!("no locale {tag:?}"))
    }

    /// `value` with this locale's separators; a right-to-left locale leads its sign with a left-to-right mark, as CLDR does.
    pub fn number(self, value: f64, decimals: usize) -> String {
        let number = format::number(value, decimals, self.separators);
        match (self.direction, number.starts_with(format::MINUS)) {
            (Direction::Rtl, true) => format!("\u{200e}{number}"),
            _ => number,
        }
    }

    /// The day `at` falls on in `zone`, written as this locale writes dates.
    pub fn date(self, at: Timestamp, zone: &TimeZone) -> String {
        format::datetime(at, zone, self.date)
            .unwrap_or_else(|error| panic!("{}'s date pattern: {error}", self.tag))
    }
}
