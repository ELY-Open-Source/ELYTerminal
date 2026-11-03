use std::rc::Rc;

use gpui::{
    App, ElementId, FontWeight, InteractiveElement, IntoElement, MouseButton, ParentElement,
    RenderOnce, SharedString, StatefulInteractiveElement, Styled, Window, div, prelude::*,
};
use jiff::{Timestamp, civil::Date, tz::TimeZone};

use crate::{
    data_display::{Badge, Tone},
    forms::flag,
    tables::{Cell, Column, DataTable, Row},
    theme::{ActiveTheme, TextSize},
    typography::{
        Ellipsis,
        format::{self, system_zone},
        tabular,
    },
};

/// An economic release: when, for which region, what, how much it moves markets from one to three, and its figure as it came, as forecast and as before.
#[derive(Clone, Debug, PartialEq)]
pub struct Release {
    pub time: Timestamp,
    pub region: SharedString,
    pub event: SharedString,
    pub weight: u8,
    pub actual: Option<f64>,
    pub forecast: Option<f64>,
    pub previous: Option<f64>,
    pub unit: SharedString,
}

/// Economic releases by day: the time, the region's flag, the event and how much it matters, then the figure as it came beside the forecast and the one before.
#[derive(IntoElement)]
pub struct EconomicCalendar {
    releases: Vec<Release>,
    zone: Option<TimeZone>,
}

impl EconomicCalendar {
    pub fn new(releases: impl IntoIterator<Item = Release>) -> Self {
        let releases: Vec<Release> = releases.into_iter().collect();
        assert!(
            releases
                .iter()
                .all(|release| (1..=3).contains(&release.weight)),
            "a release weighs one to three"
        );
        Self {
            releases,
            zone: None,
        }
    }

    pub fn zone(mut self, zone: TimeZone) -> Self {
        self.zone = Some(zone);
        self
    }
}

impl RenderOnce for EconomicCalendar {
    fn render(mut self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let zone = self
            .zone
            .clone()
            .unwrap_or_else(|| system_zone("EconomicCalendar"));
        let theme = cx.theme();
        let colors = theme.colors.clone();
        self.releases.sort_by_key(|release| release.time);
        let figure = |value: Option<f64>, unit: &SharedString| {
            value.map_or("—".to_string(), |value| {
                format!("{}{unit}", format::number(value, 1, format::Separators::EN))
            })
        };
        let mut out: Vec<gpui::AnyElement> = Vec::new();
        let mut day: Option<Date> = None;
        for release in &self.releases {
            let zoned = release.time.to_zoned(zone.clone());
            if day != Some(zoned.date()) {
                day = Some(zoned.date());
                out.push(
                    div()
                        .pt_3()
                        .pb_1()
                        .text_size(theme.text_size(TextSize::Xs))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(colors.fg_muted)
                        .child(
                            format::datetime(release.time, &zone, "%A, %B %-d")
                                .expect("a fixed pattern"),
                        )
                        .into_any_element(),
                );
            }
            let dots = (1..=3u8).map(|step| {
                div()
                    .size(theme.status_dot())
                    .rounded_full()
                    .bg(if step <= release.weight {
                        colors.fg
                    } else {
                        colors.border
                    })
            });
            let beat = release
                .actual
                .zip(release.forecast)
                .map(|(actual, forecast)| actual.total_cmp(&forecast));
            let actual_ink = match beat {
                Some(std::cmp::Ordering::Greater) => colors.success,
                Some(std::cmp::Ordering::Less) => colors.danger,
                _ => colors.fg,
            };
            let number = |words: String, ink| {
                tabular(div())
                    .w(theme.label_width() * 0.45)
                    .text_right()
                    .text_color(ink)
                    .child(words)
            };
            out.push(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .py_1p5()
                    .border_b_1()
                    .border_color(colors.border.opacity(0.5))
                    .text_size(theme.text_size(TextSize::Sm))
                    .child(
                        tabular(div())
                            .w(theme.label_width() * 0.3)
                            .text_color(colors.fg_muted)
                            .child(
                                format::datetime(release.time, &zone, "%H:%M")
                                    .expect("a fixed pattern"),
                            ),
                    )
                    .child(div().child(flag(&release.region)))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_color(colors.fg)
                            .child(Ellipsis::new(release.event.clone())),
                    )
                    .child(div().flex().gap_0p5().children(dots))
                    .child(number(figure(release.actual, &release.unit), actual_ink))
                    .child(number(
                        figure(release.forecast, &release.unit),
                        colors.fg_muted,
                    ))
                    .child(number(
                        figure(release.previous, &release.unit),
                        colors.fg_subtle,
                    ))
                    .into_any_element(),
            );
        }
        let heading = |words: &'static str| {
            div()
                .w(theme.label_width() * 0.45)
                .text_right()
                .child(words)
        };
        div()
            .flex()
            .flex_col()
            .child(
                div()
                    .flex()
                    .justify_end()
                    .gap_3()
                    .text_size(theme.text_size(TextSize::Xs))
                    .text_color(colors.fg_subtle)
                    .child(heading("Actual"))
                    .child(heading("Forecast"))
                    .child(heading("Previous")),
            )
            .children(out)
    }
}

/// A company's earnings date: when it reports, before the open or after the close, and earnings per share as expected and, once reported, as they came.
#[derive(Clone, Debug, PartialEq)]
pub struct Earnings {
    pub date: Date,
    pub symbol: SharedString,
    pub name: SharedString,
    pub before_open: bool,
    pub estimate: f64,
    pub reported: Option<f64>,
}

/// How far a report beat its estimate, as a share of the estimate's size: positive when better, whatever the sign.
pub(crate) fn surprise(estimate: f64, reported: f64) -> f64 {
    (reported - estimate) / estimate.abs()
}

/// Earnings by date: each company, when in the day it reports, what is expected, and once reported, how far it beat or missed.
#[derive(IntoElement)]
pub struct EarningsCalendar {
    id: ElementId,
    earnings: Vec<Earnings>,
}

impl EarningsCalendar {
    pub fn new(id: impl Into<ElementId>, earnings: impl IntoIterator<Item = Earnings>) -> Self {
        Self {
            id: id.into(),
            earnings: earnings.into_iter().collect(),
        }
    }
}

impl RenderOnce for EarningsCalendar {
    fn render(mut self, _: &mut Window, _: &mut App) -> impl IntoElement {
        self.earnings.sort_by_key(|earnings| earnings.date);
        let columns = [
            Column::new("date", "Date"),
            Column::new("symbol", "Symbol"),
            Column::new("name", "Company"),
            Column::new("when", "When"),
            Column::new("estimate", "Expected").end().decimals(2),
            Column::new("reported", "Reported").end().decimals(2),
            Column::new("surprise", "Surprise").end(),
        ];
        let rows: Vec<Row> = self
            .earnings
            .iter()
            .map(|earnings| {
                let when = Cell::Tag(
                    if earnings.before_open {
                        "Before open"
                    } else {
                        "After close"
                    }
                    .into(),
                    Tone::Neutral,
                );
                let surprise = match earnings.reported {
                    Some(reported) if earnings.estimate != 0.0 => {
                        let share = surprise(earnings.estimate, reported);
                        Cell::Tag(
                            format::percent(share, 1, true).into(),
                            if share >= 0.0 {
                                Tone::Success
                            } else {
                                Tone::Danger
                            },
                        )
                    }
                    _ => Cell::Empty,
                };
                let reported = earnings.reported.map_or(Cell::Empty, Cell::from);
                Row::new(
                    earnings.symbol.clone(),
                    [
                        Cell::Text(earnings.date.strftime("%b %-d").to_string().into()),
                        Cell::Text(earnings.symbol.clone()),
                        Cell::Text(earnings.name.clone()),
                        when,
                        earnings.estimate.into(),
                        reported,
                        surprise,
                    ],
                )
            })
            .collect();
        DataTable::new(self.id, columns).rows(rows)
    }
}

/// A story: where it ran, when, its headline, the symbols it touches, and which way it leans for them, if either.
#[derive(Clone, Debug, PartialEq)]
pub struct Story {
    pub source: SharedString,
    pub time: Timestamp,
    pub headline: SharedString,
    pub symbols: Vec<SharedString>,
    pub lean: Option<bool>,
}

type OnStory = Rc<dyn Fn(usize, &mut Window, &mut App)>;

/// Stories newest first: a dot for how each leans, where and when it ran, its headline, and the symbols it touches. A press opens one.
#[derive(IntoElement)]
pub struct NewsFeed {
    id: ElementId,
    stories: Vec<Story>,
    zone: Option<TimeZone>,
    on_open: Option<OnStory>,
}

impl NewsFeed {
    pub fn new(id: impl Into<ElementId>, stories: impl IntoIterator<Item = Story>) -> Self {
        Self {
            id: id.into(),
            stories: stories.into_iter().collect(),
            zone: None,
            on_open: None,
        }
    }

    pub fn zone(mut self, zone: TimeZone) -> Self {
        self.zone = Some(zone);
        self
    }

    /// Gets the place of the story pressed, as given.
    pub fn on_open(mut self, handler: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_open = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for NewsFeed {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let zone = self.zone.clone().unwrap_or_else(|| system_zone("NewsFeed"));
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let mut order: Vec<usize> = (0..self.stories.len()).collect();
        order.sort_by_key(|ix| std::cmp::Reverse(self.stories[*ix].time));
        div()
            .flex()
            .flex_col()
            .children(order.into_iter().map(|ix| {
                let story = &self.stories[ix];
                let lean = match story.lean {
                    Some(true) => colors.success,
                    Some(false) => colors.danger,
                    None => colors.border_strong,
                };
                let open = self.on_open.clone();
                let when =
                    format::datetime(story.time, &zone, "%b %-d, %H:%M").expect("a fixed pattern");
                div()
                    .id((self.id.clone(), format!("story-{ix}")))
                    .flex()
                    .gap_3()
                    .px_2()
                    .py_2p5()
                    .rounded(theme.radius(crate::theme::Radius::Md))
                    .cursor_pointer()
                    .hover(|style| style.bg(colors.hover))
                    .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
                    .when_some(open, |row, open| {
                        row.on_click(move |_, window, cx| {
                            log::info!("news feed: opened story {ix}");
                            open(ix, window, cx)
                        })
                    })
                    .child(
                        div()
                            .pt_1p5()
                            .child(div().size(theme.status_dot()).rounded_full().bg(lean)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(
                                div()
                                    .flex()
                                    .gap_2()
                                    .text_size(theme.text_size(TextSize::Xs))
                                    .child(
                                        div()
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(colors.fg_muted)
                                            .child(story.source.clone()),
                                    )
                                    .child(div().text_color(colors.fg_subtle).child(when)),
                            )
                            .child(
                                div()
                                    .text_size(theme.text_size(TextSize::Sm))
                                    .text_color(colors.fg)
                                    .child(story.headline.clone()),
                            )
                            .child(
                                div().flex().gap_1().children(
                                    story
                                        .symbols
                                        .iter()
                                        .map(|symbol| Badge::new(symbol.clone())),
                                ),
                            ),
                    )
            }))
    }
}

#[cfg(test)]
mod tests {
    use super::surprise;

    #[test]
    fn a_smaller_loss_than_expected_is_a_beat() {
        assert_eq!(surprise(-1.0, -0.5), 0.5);
        assert_eq!(surprise(2.0, 1.5), -0.25);
    }
}
