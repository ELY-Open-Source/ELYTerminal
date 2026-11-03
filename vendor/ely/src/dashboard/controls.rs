use std::rc::Rc;

use gpui::{
    App, ElementId, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window, div,
};
use jiff::Timestamp;

use crate::{
    buttons::{Button, ButtonVariant, IconButton},
    forms::{Choice, Select},
    primitives::IconName,
    theme::{ActiveTheme, TextSize},
    typography::format,
};

/// How often a refresh may come, in seconds, with its words; none is off.
const EVERY: [(u32, &str); 7] = [
    (0, "Off"),
    (5, "5s"),
    (10, "10s"),
    (30, "30s"),
    (60, "1m"),
    (300, "5m"),
    (900, "15m"),
];

type OnInterval = Rc<dyn Fn(Option<u32>, &mut Window, &mut App)>;
type Run = Rc<dyn Fn(&mut Window, &mut App)>;

/// How often a dashboard refreshes, from off to every fifteen minutes, beside a button that refreshes it now and when it last did. The owner runs the timer.
#[derive(IntoElement)]
pub struct RefreshIntervalSelector {
    id: ElementId,
    every: Option<u32>,
    last: Timestamp,
    now: Timestamp,
    on_change: OnInterval,
    on_refresh: Run,
}

impl RefreshIntervalSelector {
    /// `every` is in seconds, one the selector offers, or none for off; `last` is when it last refreshed.
    pub fn new(
        id: impl Into<ElementId>,
        every: Option<u32>,
        last: Timestamp,
        now: Timestamp,
        on_change: impl Fn(Option<u32>, &mut Window, &mut App) + 'static,
        on_refresh: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        if let Some(every) = every {
            assert!(
                EVERY
                    .iter()
                    .any(|(seconds, _)| *seconds == every && every > 0),
                "no refresh every {every}s"
            );
        }
        Self {
            id: id.into(),
            every,
            last,
            now,
            on_change: Rc::new(on_change),
            on_refresh: Rc::new(on_refresh),
        }
    }
}

/// The seconds a choice of the selector names; its 0 is Off.
pub(crate) fn interval(value: &str) -> Option<u32> {
    let seconds: u32 = value.parse().expect("an interval the select offers");
    (seconds > 0).then_some(seconds)
}

impl RenderOnce for RefreshIntervalSelector {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let id = self.id;
        let on_change = self.on_change;
        let on_refresh = self.on_refresh;
        div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_2()
            .child(
                IconButton::new((id.clone(), "now"), IconName::RefreshCw)
                    .tooltip("Refresh now")
                    .on_click(move |_, window, cx| {
                        log::info!("refresh: now");
                        on_refresh(window, cx);
                    }),
            )
            .child(
                div().w(theme.label_width()).child(
                    Select::new(
                        (id, "every"),
                        EVERY
                            .iter()
                            .map(|(seconds, words)| Choice::new(seconds.to_string(), *words)),
                    )
                    .selected(self.every.unwrap_or(0).to_string())
                    .on_change(move |value, window, cx| {
                        let every = interval(value);
                        log::info!("refresh: every {every:?}");
                        on_change(every, window, cx);
                    }),
                ),
            )
            .child(
                div()
                    .text_size(theme.text_size(TextSize::Xs))
                    .text_color(theme.colors.fg_subtle)
                    .child(format!("Updated {}", format::relative(self.last, self.now))),
            )
    }
}

/// A window of time a dashboard shows, back from now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimeWindow {
    Minutes15,
    Hour,
    Hours6,
    Day,
    Week,
    Month,
}

impl TimeWindow {
    pub const ALL: [TimeWindow; 6] = [
        TimeWindow::Minutes15,
        TimeWindow::Hour,
        TimeWindow::Hours6,
        TimeWindow::Day,
        TimeWindow::Week,
        TimeWindow::Month,
    ];

    pub fn words(self) -> &'static str {
        match self {
            TimeWindow::Minutes15 => "Last 15 minutes",
            TimeWindow::Hour => "Last hour",
            TimeWindow::Hours6 => "Last 6 hours",
            TimeWindow::Day => "Last 24 hours",
            TimeWindow::Week => "Last 7 days",
            TimeWindow::Month => "Last 30 days",
        }
    }

    /// How far back it reaches, in seconds.
    pub fn seconds(self) -> i64 {
        match self {
            TimeWindow::Minutes15 => 15 * 60,
            TimeWindow::Hour => 3_600,
            TimeWindow::Hours6 => 6 * 3_600,
            TimeWindow::Day => 86_400,
            TimeWindow::Week => 7 * 86_400,
            TimeWindow::Month => 30 * 86_400,
        }
    }
}

/// A filter the bar offers: its key, its name, its choices, and the one picked, if any.
#[derive(Clone, Debug, PartialEq)]
pub struct DashboardFilter {
    pub key: SharedString,
    pub name: SharedString,
    pub choices: Vec<Choice>,
    pub picked: Option<SharedString>,
}

type OnWindow = Rc<dyn Fn(TimeWindow, &mut Window, &mut App)>;
type OnFilter = Rc<dyn Fn(&SharedString, Option<SharedString>, &mut Window, &mut App)>;

/// A dashboard's filters in a row that wraps: a window of time, a select for each filter showing its name until one is picked, and Clear while any is picked.
#[derive(IntoElement)]
pub struct DashboardFilterBar {
    id: ElementId,
    window: TimeWindow,
    filters: Vec<DashboardFilter>,
    on_window: OnWindow,
    on_filter: OnFilter,
}

impl DashboardFilterBar {
    pub fn new(
        id: impl Into<ElementId>,
        window: TimeWindow,
        filters: impl IntoIterator<Item = DashboardFilter>,
        on_window: impl Fn(TimeWindow, &mut Window, &mut App) + 'static,
        on_filter: impl Fn(&SharedString, Option<SharedString>, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            window,
            filters: filters.into_iter().collect(),
            on_window: Rc::new(on_window),
            on_filter: Rc::new(on_filter),
        }
    }
}

impl RenderOnce for DashboardFilterBar {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id;
        let theme = cx.theme();
        let on_window = self.on_window;
        let on_filter = self.on_filter;
        let picked: Vec<SharedString> = self
            .filters
            .iter()
            .filter(|filter| filter.picked.is_some())
            .map(|filter| filter.key.clone())
            .collect();
        let selects = self.filters.iter().map(|filter| {
            let (key, on_filter) = (filter.key.clone(), on_filter.clone());
            let mut select = Select::new(
                (id.clone(), format!("filter-{}", filter.key)),
                filter.choices.clone(),
            )
            .placeholder(filter.name.clone())
            .on_change(move |value, window, cx| {
                log::info!("filter bar: {key} is {value}");
                on_filter(&key, Some(value.clone()), window, cx);
            });
            if let Some(value) = &filter.picked {
                select = select.selected(value.clone());
            }
            div().w(theme.label_width()).child(select)
        });
        let clear = (!picked.is_empty()).then(|| {
            let on_filter = on_filter.clone();
            Button::new((id.clone(), "clear"), "Clear")
                .variant(ButtonVariant::Ghost)
                .on_click(move |_, window, cx| {
                    log::info!("filter bar: clear");
                    for key in &picked {
                        on_filter(key, None, window, cx);
                    }
                })
        });
        div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_2()
            .child(
                div().w(theme.label_width()).child(
                    Select::new(
                        (id.clone(), "window"),
                        TimeWindow::ALL
                            .iter()
                            .map(|window| Choice::new(window.words(), window.words())),
                    )
                    .selected(self.window.words())
                    .on_change(move |value, window, cx| {
                        let chosen = *TimeWindow::ALL
                            .iter()
                            .find(|each| each.words() == value.as_ref())
                            .expect("a window the select offers");
                        log::info!("filter bar: window {}", chosen.words());
                        on_window(chosen, window, cx);
                    }),
                ),
            )
            .children(selects)
            .children(clear)
    }
}

#[cfg(test)]
mod tests {
    use super::interval;

    #[test]
    fn off_is_no_interval() {
        assert_eq!(interval("0"), None);
        assert_eq!(interval("30"), Some(30));
    }
}
