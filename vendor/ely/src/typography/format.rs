use std::sync::atomic::{AtomicBool, Ordering};

use jiff::{Timestamp, fmt::strtime, tz::TimeZone};

/// Typographic minus; aligns with plus in tabular figures.
pub const MINUS: char = '\u{2212}';

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Separators {
    pub group: char,
    pub decimal: char,
}

impl Separators {
    pub const EN: Self = Self {
        group: ',',
        decimal: '.',
    };
}

/// Groups thousands. Non-finite values print as Rust prints them.
pub fn number(value: f64, decimals: usize, separators: Separators) -> String {
    if !value.is_finite() {
        return value.to_string();
    }
    let fixed = format!("{:.*}", decimals, value.abs());
    let (int, fraction) = match fixed.split_once('.') {
        Some((int, fraction)) => (int, Some(fraction)),
        None => (fixed.as_str(), None),
    };
    let mut out = String::new();
    if value < 0.0 && fixed.chars().any(|ch| ch.is_ascii_digit() && ch != '0') {
        out.push(MINUS);
    }
    out.push_str(&group_digits(int, separators.group));
    if let Some(fraction) = fraction {
        out.push(separators.decimal);
        out.push_str(fraction);
    }
    out
}

/// Inserts `group` every three digits from the right.
pub fn group_digits(digits: &str, group: char) -> String {
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (ix, digit) in digits.chars().enumerate() {
        if ix > 0 && (digits.len() - ix).is_multiple_of(3) {
            out.push(group);
        }
        out.push(digit);
    }
    out
}

/// `value` rounded to `digits` significant digits and written out in full, grouped, without trailing zeros. Non-finite values print as Rust prints them.
pub fn significant(value: f64, digits: usize, separators: Separators) -> String {
    assert!(digits > 0, "a number keeps at least one significant digit");
    if !value.is_finite() {
        return value.to_string();
    }
    let rounded = format!("{:.*e}", digits - 1, value.abs());
    let (mantissa, exponent) = rounded.split_once('e').expect("Rust writes an exponent");
    let point = exponent.parse::<i32>().expect("a whole exponent") + 1;
    let figures: String = mantissa.chars().filter(char::is_ascii_digit).collect();
    let (whole, part) = match usize::try_from(point) {
        Err(_) | Ok(0) => (
            "0".to_string(),
            "0".repeat(point.unsigned_abs() as usize) + &figures,
        ),
        Ok(point) if point >= figures.len() => (
            figures.clone() + &"0".repeat(point - figures.len()),
            String::new(),
        ),
        Ok(point) => (figures[..point].to_string(), figures[point..].to_string()),
    };
    let mut out = String::new();
    if value < 0.0 && figures.chars().any(|ch| ch != '0') {
        out.push(MINUS);
    }
    out.push_str(&group_digits(&whole, separators.group));
    let part = part.trim_end_matches('0');
    if !part.is_empty() {
        out.push(separators.decimal);
        out.push_str(part);
    }
    out
}

/// ISO 4217 code to symbol, minor units, and whether a space follows.
pub fn currency_parts(code: &str) -> (&str, usize, bool) {
    let places = iso_currency::Currency::from_code(code)
        .and_then(|currency| currency.exponent())
        .map_or(2, usize::from);
    match code {
        "USD" => ("$", places, false),
        "EUR" => ("€", places, false),
        "GBP" => ("£", places, false),
        "JPY" | "CNY" => ("¥", places, false),
        "KRW" => ("₩", places, false),
        "INR" => ("₹", places, false),
        _ => (code, places, true),
    }
}

/// Unknown codes print as a prefix.
pub fn currency(amount: f64, code: &str) -> String {
    let (symbol, decimals, spaced) = currency_parts(code);
    let body = number(amount.abs(), decimals, Separators::EN);
    let negative = amount < 0.0 && body.chars().any(|ch| ch.is_ascii_digit() && ch != '0');
    let sign = if negative {
        MINUS.to_string()
    } else {
        String::new()
    };
    let gap = if spaced { " " } else { "" };
    format!("{sign}{symbol}{gap}{body}")
}

/// `ratio` 0.1234 reads "12.34%". `signed` adds a plus to gains.
pub fn percent(ratio: f64, decimals: usize, signed: bool) -> String {
    let body = number(ratio * 100.0, decimals, Separators::EN);
    let gain = signed && ratio > 0.0 && body.chars().any(|ch| ch.is_ascii_digit() && ch != '0');
    format!("{}{body}%", if gain { "+" } else { "" })
}

/// Decimal units (KB = 1000) or binary (KiB = 1024).
pub fn file_size(bytes: u64, binary: bool) -> String {
    let (base, units) = if binary {
        (1024.0, ["B", "KiB", "MiB", "GiB", "TiB", "PiB"])
    } else {
        (1000.0, ["B", "KB", "MB", "GB", "TB", "PB"])
    };
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= base && unit < units.len() - 1 {
        value /= base;
        unit += 1;
    }
    if unit == 0 {
        return format!("{bytes} B");
    }
    let decimals = if value < 10.0 { 1 } else { 0 };
    let shown = format!("{value:.decimals$}");
    if shown.parse::<f64>().is_ok_and(|rounded| rounded >= base) && unit < units.len() - 1 {
        return format!("1.0 {}", units[unit + 1]);
    }
    format!("{shown} {}", units[unit])
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DurationStyle {
    /// `01:23:45`, or `23:45` under an hour.
    Clock,
    /// `1h 23m`, `4m 5s`.
    Compact,
}

pub fn duration(seconds: u64, style: DurationStyle) -> String {
    let (days, hours, minutes, secs) = (
        seconds / 86_400,
        seconds / 3_600,
        seconds / 60 % 60,
        seconds % 60,
    );
    match style {
        DurationStyle::Clock if hours > 0 => format!("{hours:02}:{minutes:02}:{secs:02}"),
        DurationStyle::Clock => format!("{minutes:02}:{secs:02}"),
        DurationStyle::Compact if days > 0 => format!("{days}d {}h", hours % 24),
        DurationStyle::Compact if hours > 0 => format!("{hours}h {minutes}m"),
        DurationStyle::Compact if minutes > 0 => format!("{minutes}m {secs}s"),
        DurationStyle::Compact => format!("{secs}s"),
    }
}

/// How long a step took: "340 ms", "1.2 s", "4m 5s".
pub fn took(spent: std::time::Duration) -> String {
    let ms = spent.as_millis() as u64;
    let tenths = (ms + 50) / 100;
    match ms {
        0..1_000 => format!("{ms} ms"),
        _ if tenths < 600 => format!("{}.{} s", tenths / 10, tenths % 10),
        _ => duration((ms + 500) / 1_000, DurationStyle::Compact),
    }
}

/// Digits kept past a step's first; float noise sits near the sixteenth.
const DIGITS: usize = 6;

/// Places after the point that show every multiple of `step` exactly: none for whole steps, four for 0.0025.
pub(crate) fn decimals(step: f64) -> usize {
    assert!(
        step.is_finite() && step > 0.0,
        "a step is positive, not {step}"
    );
    let leading = (-step.log10().floor()).max(0.0) as usize;
    let text = format!("{step:.*}", leading + DIGITS);
    let fraction = text.split_once('.').map_or("", |(_, fraction)| fraction);
    fraction.trim_end_matches('0').len()
}

/// "1 file", "12,345 files".
pub fn plural(count: u64, one: &str, other: &str) -> String {
    let word = if count == 1 { one } else { other };
    format!(
        "{} {word}",
        group_digits(&count.to_string(), Separators::EN.group)
    )
}

/// "just now", "3 minutes ago", "in 2 days".
pub fn relative(then: Timestamp, now: Timestamp) -> String {
    let Some((count, unit, future)) = span(then, now) else {
        return "just now".to_string();
    };
    let label = plural(count, unit, &format!("{unit}s"));
    if future {
        format!("in {label}")
    } else {
        format!("{label} ago")
    }
}

/// The rounded distance: count, unit, and whether ahead; none under 45 s.
pub(crate) fn span(then: Timestamp, now: Timestamp) -> Option<(u64, &'static str, bool)> {
    let delta = now.as_second() - then.as_second();
    let seconds = delta.unsigned_abs();
    if seconds < 45 {
        return None;
    }
    let round = |unit: u64| (seconds + unit / 2) / unit;
    let (count, unit) = match seconds {
        s if s < 90 => (1, "minute"),
        s if s < 45 * 60 => (round(60), "minute"),
        s if s < 90 * 60 => (1, "hour"),
        s if s < 22 * 3_600 => (round(3_600), "hour"),
        s if s < 36 * 3_600 => (1, "day"),
        s if s < 26 * 86_400 => (round(86_400), "day"),
        s if s < 45 * 86_400 => (1, "month"),
        s if s < 320 * 86_400 => (round(30 * 86_400), "month"),
        s if s < 548 * 86_400 => (1, "year"),
        _ => (round(365 * 86_400), "year"),
    };
    Some((count, unit, delta < 0))
}

/// The system's zone; an unknown one logs once, naming `user`, and reads as jiff's unknown zone.
pub(crate) fn system_zone(user: &str) -> TimeZone {
    TimeZone::try_system().unwrap_or_else(|error| {
        static TOLD: AtomicBool = AtomicBool::new(false);
        if !TOLD.swap(true, Ordering::Relaxed) {
            log::error!("{user}: the system time zone is unknown ({error}); times show in UTC");
        }
        TimeZone::unknown()
    })
}

/// strftime in `zone`; a bad pattern is an error. The unknown zone says UTC.
pub fn datetime(at: Timestamp, zone: &TimeZone, pattern: &str) -> Result<String, jiff::Error> {
    let text = strtime::format(pattern, &at.to_zoned(zone.clone()))?;
    Ok(match zone.is_unknown() && !text.contains("UTC") {
        true => format!("{text} UTC"),
        false => text,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn took_reads_in_the_unit_that_fits() {
        let took = |ms| took(std::time::Duration::from_millis(ms));
        assert_eq!(took(340), "340 ms");
        assert_eq!(took(999), "999 ms");
        assert_eq!(took(1_000), "1.0 s");
        assert_eq!(took(12_449), "12.4 s");
        assert_eq!(took(59_960), "1m 0s", "rounding never shows 60.0 s");
        assert_eq!(took(65_000), "1m 5s");
    }

    #[test]
    fn steps_read_to_the_places_they_need() {
        assert_eq!(
            (
                decimals(10.0),
                decimals(0.5),
                decimals(0.05),
                decimals(0.001)
            ),
            (0, 1, 2, 3)
        );
        assert_eq!(decimals(0.0025), 4, "a quarter of a cent needs four places");
        assert_eq!(
            decimals(110.2 - 110.0),
            1,
            "float noise from a difference drops away"
        );
        assert_eq!(decimals(1e-11), 11, "a tiny step keeps its places");
    }

    #[test]
    fn significant_digits_round_at_every_size() {
        let en = Separators::EN;
        assert_eq!(
            significant(0.1 + 0.2, 12, en),
            "0.3",
            "binary noise rounds away"
        );
        assert_eq!(significant(2.0 / 3.0, 12, en), "0.666666666667");
        assert_eq!(significant(1234.5678, 6, en), "1,234.57");
        assert_eq!(
            significant(9.999_999_6, 6, en),
            "10",
            "a carry adds a digit"
        );
        assert_eq!(
            significant(12_345_678_901_234_567_890.0, 12, en),
            "12,345,678,901,200,000,000"
        );
        assert_eq!(significant(-1.0e-15, 12, en), "−0.000000000000001");
        assert_eq!(significant(0.0, 12, en), "0");
        assert_eq!(significant(-0.0, 12, en), "0");
        assert_eq!(significant(f64::INFINITY, 12, en), "inf");
    }

    #[test]
    fn number_groups_and_rounds() {
        assert_eq!(number(1234567.891, 2, Separators::EN), "1,234,567.89");
        assert_eq!(number(999.5, 0, Separators::EN), "1,000");
        assert_eq!(number(12.0, 0, Separators::EN), "12");
        assert_eq!(number(-1234.5, 1, Separators::EN), "−1,234.5");
        assert_eq!(number(-0.001, 2, Separators::EN), "0.00");
        let de = Separators {
            group: '.',
            decimal: ',',
        };
        assert_eq!(number(1234.5, 2, de), "1.234,50");
        assert_eq!(number(f64::NAN, 2, Separators::EN), "NaN");
    }

    #[test]
    fn currency_uses_symbols_and_minor_units() {
        assert_eq!(currency(1234.5, "USD"), "$1,234.50");
        assert_eq!(currency(-42.0, "EUR"), "−€42.00");
        assert_eq!(currency(1500.4, "JPY"), "¥1,500");
        assert_eq!(currency(9.99, "CHF"), "CHF 9.99");
        assert_eq!(currency(1.5, "BHD"), "BHD 1.500");
        assert_eq!(currency(25000.0, "VND"), "VND 25,000");
    }

    #[test]
    fn percent_signs_gains_only_when_asked() {
        assert_eq!(percent(0.1234, 2, false), "12.34%");
        assert_eq!(percent(0.05, 1, true), "+5.0%");
        assert_eq!(percent(-0.031, 1, true), "−3.1%");
        assert_eq!(percent(0.0, 1, true), "0.0%");
    }

    #[test]
    fn file_size_picks_units_and_carries() {
        assert_eq!(file_size(512, false), "512 B");
        assert_eq!(file_size(1536, false), "1.5 KB");
        assert_eq!(file_size(15_360, false), "15 KB");
        assert_eq!(file_size(999_999, false), "1.0 MB");
        assert_eq!(file_size(1536, true), "1.5 KiB");
        assert_eq!(file_size(3 * 1024 * 1024 * 1024, true), "3.0 GiB");
    }

    #[test]
    fn duration_in_both_styles() {
        assert_eq!(duration(5_025, DurationStyle::Clock), "01:23:45");
        assert_eq!(duration(1_425, DurationStyle::Clock), "23:45");
        assert_eq!(duration(5_025, DurationStyle::Compact), "1h 23m");
        assert_eq!(duration(245, DurationStyle::Compact), "4m 5s");
        assert_eq!(duration(183_600, DurationStyle::Compact), "2d 3h");
        assert_eq!(duration(12, DurationStyle::Compact), "12s");
    }

    #[test]
    fn plural_counts() {
        assert_eq!(plural(1, "file", "files"), "1 file");
        assert_eq!(plural(0, "file", "files"), "0 files");
        assert_eq!(plural(12_345, "file", "files"), "12,345 files");
        let huge = plural(9_007_199_254_740_993, "file", "files");
        assert_eq!(huge, "9,007,199,254,740,993 files");
    }

    #[test]
    fn relative_reads_both_directions() {
        let now = Timestamp::from_second(1_000_000_000).unwrap();
        let ago = |s: i64| relative(Timestamp::from_second(1_000_000_000 - s).unwrap(), now);
        assert_eq!(ago(10), "just now");
        assert_eq!(ago(60), "1 minute ago");
        assert_eq!(ago(180), "3 minutes ago");
        assert_eq!(ago(7_200), "2 hours ago");
        assert_eq!(ago(3 * 86_400), "3 days ago");
        assert_eq!(ago(3 * 365 * 86_400), "3 years ago");
        assert_eq!(ago(-120), "in 2 minutes");
    }

    #[test]
    fn datetime_formats_in_zone() {
        let at = Timestamp::from_second(0).unwrap();
        let formatted = datetime(at, &TimeZone::UTC, "%Y-%m-%d %H:%M").unwrap();
        assert_eq!(formatted, "1970-01-01 00:00");
        assert!(datetime(at, &TimeZone::UTC, "%Y %").is_err());
        let unknown = TimeZone::unknown();
        assert_eq!(
            datetime(at, &unknown, "%H:%M").expect("a pattern"),
            "00:00 UTC"
        );
        let named = datetime(at, &unknown, "%H:%M %Z %%Z").expect("a pattern");
        assert_eq!(named, "00:00 UTC %Z");
        let escaped = datetime(at, &unknown, "%H:%M %%Z").expect("a pattern");
        assert_eq!(escaped, "00:00 %Z UTC");
        assert_eq!(
            datetime(at, &TimeZone::UTC, "%H:%M").expect("a pattern"),
            "00:00"
        );
    }
}
