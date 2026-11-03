use std::time::Duration;

use gpui::{
    App, ElementId, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Task, Window, div,
};
use jiff::{Timestamp, tz::TimeZone};

use super::{
    format::{self, DurationStyle, Separators},
    text::tabular,
};
use crate::{i18n, theme::ActiveTheme};

const TICK: Duration = Duration::from_secs(30);

fn figure(text: String) -> impl IntoElement {
    tabular(div()).child(text)
}

/// Grouped digits at fixed precision.
#[derive(IntoElement)]
pub struct NumberText {
    value: f64,
    decimals: usize,
}

impl NumberText {
    pub fn new(value: f64) -> Self {
        Self { value, decimals: 0 }
    }

    pub fn decimals(mut self, decimals: usize) -> Self {
        self.decimals = decimals;
        self
    }
}

impl RenderOnce for NumberText {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        figure(format::number(self.value, self.decimals, Separators::EN))
    }
}

/// Amount in an ISO 4217 currency.
#[derive(IntoElement)]
pub struct CurrencyText {
    amount: f64,
    code: &'static str,
}

impl CurrencyText {
    pub fn new(amount: f64, code: &'static str) -> Self {
        Self { amount, code }
    }
}

impl RenderOnce for CurrencyText {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        figure(format::currency(self.amount, self.code))
    }
}

/// Ratio as percent. `signed` shows direction in word and color.
#[derive(IntoElement)]
pub struct PercentText {
    ratio: f64,
    decimals: usize,
    signed: bool,
}

impl PercentText {
    pub fn new(ratio: f64) -> Self {
        Self {
            ratio,
            decimals: 1,
            signed: false,
        }
    }

    pub fn decimals(mut self, decimals: usize) -> Self {
        self.decimals = decimals;
        self
    }

    pub fn signed(mut self) -> Self {
        self.signed = true;
        self
    }
}

impl RenderOnce for PercentText {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let colors = &cx.theme().colors;
        let tone = match (self.signed, self.ratio) {
            (true, ratio) if ratio > 0.0 => colors.success,
            (true, ratio) if ratio < 0.0 => colors.danger,
            _ => colors.fg,
        };
        tabular(div()).text_color(tone).child(format::percent(
            self.ratio,
            self.decimals,
            self.signed,
        ))
    }
}

/// Bytes in KB, MB, GB; or KiB with `binary`.
#[derive(IntoElement)]
pub struct FileSizeText {
    bytes: u64,
    binary: bool,
}

impl FileSizeText {
    pub fn new(bytes: u64) -> Self {
        Self {
            bytes,
            binary: false,
        }
    }

    pub fn binary(mut self) -> Self {
        self.binary = true;
        self
    }
}

impl RenderOnce for FileSizeText {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        figure(format::file_size(self.bytes, self.binary))
    }
}

/// Seconds as a clock or a compact span.
#[derive(IntoElement)]
pub struct DurationText {
    seconds: u64,
    style: DurationStyle,
}

impl DurationText {
    pub fn new(seconds: u64, style: DurationStyle) -> Self {
        Self { seconds, style }
    }
}

impl RenderOnce for DurationText {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        figure(format::duration(self.seconds, self.style))
    }
}

/// Count with the right noun.
#[derive(IntoElement)]
pub struct PluralText {
    count: u64,
    one: &'static str,
    other: &'static str,
}

impl PluralText {
    pub fn new(count: u64, one: &'static str, other: &'static str) -> Self {
        Self { count, one, other }
    }
}

impl RenderOnce for PluralText {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        figure(format::plural(self.count, self.one, self.other))
    }
}

/// "3 minutes ago", kept fresh while on screen.
#[derive(IntoElement)]
pub struct RelativeTime {
    id: ElementId,
    at: Timestamp,
}

impl RelativeTime {
    pub fn new(id: impl Into<ElementId>, at: Timestamp) -> Self {
        Self { id: id.into(), at }
    }
}

/// Redraws the view each tick while `id` stays on screen.
pub(crate) fn fresh(id: impl Into<ElementId>, window: &mut Window, cx: &mut App) {
    window.use_keyed_state(id, cx, |window, cx| -> Task<()> {
        cx.spawn_in(window, async move |ticker, cx| {
            loop {
                cx.background_executor().timer(TICK).await;
                let ticked = cx.update(|_, cx| ticker.update(cx, |_, cx| cx.notify()));
                if ticked.and_then(|inner| inner).is_err() {
                    return;
                }
            }
        })
    });
}

/// `then` against `now`, in the shown locale.
pub(crate) fn relative_text(cx: &App, then: Timestamp, now: Timestamp) -> SharedString {
    let Some((count, unit, future)) = format::span(then, now) else {
        return i18n::text(cx, "time.now", &[]);
    };
    let form = if count == 1 { "one" } else { "other" };
    let n = count.to_string();
    let span = i18n::text(cx, &format!("time.{unit}.{form}"), &[("n", &n)]);
    let key = if future { "time.in" } else { "time.ago" };
    i18n::text(cx, key, &[("span", &span)])
}

impl RenderOnce for RelativeTime {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        fresh(self.id, window, cx);
        let text = relative_text(cx, self.at, Timestamp::now());
        figure(text.to_string())
    }
}

/// A moment, strftime pattern, in the system zone unless told.
#[derive(IntoElement)]
pub struct DateTimeText {
    at: Timestamp,
    pattern: &'static str,
    zone: Option<TimeZone>,
}

impl DateTimeText {
    pub fn new(at: Timestamp) -> Self {
        Self {
            at,
            pattern: "%b %-d, %Y · %H:%M",
            zone: None,
        }
    }

    pub fn pattern(mut self, pattern: &'static str) -> Self {
        self.pattern = pattern;
        self
    }

    pub fn zone(mut self, zone: TimeZone) -> Self {
        self.zone = Some(zone);
        self
    }
}

impl RenderOnce for DateTimeText {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let zone = match self.zone {
            Some(zone) => zone,
            None => format::system_zone("DateTimeText"),
        };
        let text = format::datetime(self.at, &zone, self.pattern)
            .unwrap_or_else(|error| panic!("DateTimeText pattern {:?}: {error}", self.pattern));
        figure(text.to_string())
    }
}

#[cfg(test)]
mod tests {
    use gpui::TestAppContext;
    use jiff::Timestamp;

    use super::{format, relative_text};
    use crate::i18n::I18n;

    #[gpui::test]
    fn relative_time_reads_in_the_shown_locale(cx: &mut TestAppContext) {
        let now = Timestamp::from_second(1_000_000_000).unwrap();
        let at = |s: i64| Timestamp::from_second(1_000_000_000 - s).unwrap();
        for s in [10, 60, 600, -7_200, 86_400 * 3, 86_400 * 400] {
            let english = cx.update(|cx| relative_text(cx, at(s), now));
            assert_eq!(english.as_ref(), format::relative(at(s), now));
        }
        cx.update(|cx| cx.set_global(I18n::new("zh-CN")));
        let chinese = |s: i64| cx.update(|cx| relative_text(cx, at(s), now).to_string());
        assert_eq!(chinese(10), "刚刚");
        assert_eq!(chinese(600), "10 分钟前");
        assert_eq!(chinese(-7_200), "2 小时后");
    }
}
