use std::{cmp::Ordering, f32::consts::TAU, time::Duration};

use gpui::{
    App, ElementId, IntoElement, ParentElement, Pixels, RenderOnce, SharedString, Styled, Task,
    Window, canvas, div, point,
};
use jiff::{Timestamp, civil::Time, tz::TimeZone};

use crate::{
    canvas::{finish, outline},
    theme::{ActiveTheme, TextSize},
    typography::{Ellipsis, format, tabular},
};

/// Where each hand points, in turns clockwise from twelve: the hour's, the minute's and the second's.
pub(crate) fn hands(time: Time) -> (f32, f32, f32) {
    let (hour, minute, second) = (
        f32::from(time.hour() % 12),
        f32::from(time.minute()),
        f32::from(time.second()),
    );
    (
        (hour + minute / 60.0 + second / 3600.0) / 12.0,
        (minute + second / 60.0) / 60.0,
        second / 60.0,
    )
}

/// How a zone's day and hour stand against home's at `now`: today, tomorrow or yesterday, and its offset from home.
pub(crate) fn against(now: Timestamp, zone: &TimeZone, home: &TimeZone) -> (&'static str, String) {
    let (there, here) = (now.to_zoned(zone.clone()), now.to_zoned(home.clone()));
    let day = match there.date().cmp(&here.date()) {
        Ordering::Greater => "Tomorrow",
        Ordering::Less => "Yesterday",
        Ordering::Equal => "Today",
    };
    let apart = there.offset().seconds() - here.offset().seconds();
    let (hours, minutes) = (apart.abs() / 3600, apart.abs() / 60 % 60);
    let sign = if apart < 0 { format::MINUS } else { '+' };
    let offset = match (hours, minutes) {
        (0, 0) => "Same time".to_string(),
        (_, 0) => format!("{sign}{hours}h"),
        _ => format!("{sign}{hours}h {minutes}m"),
    };
    (day, offset)
}

/// Wakes the view at each whole second of the wall clock while its element renders.
fn each_second(id: impl Into<ElementId>, window: &mut Window, cx: &mut App) {
    window.use_keyed_state(id, cx, |window, cx| -> Task<()> {
        cx.spawn_in(window, async move |ticker, cx| {
            loop {
                let past = Timestamp::now().subsec_nanosecond().max(0) as u64;
                cx.background_executor()
                    .timer(Duration::from_nanos(1_000_000_000 - past))
                    .await;
                let ticked = cx.update(|_, cx| ticker.update(cx, |_, cx| cx.notify()));
                if ticked.and_then(|inner| inner).is_err() {
                    return;
                }
            }
        })
    });
}

/// An analog face `side` wide: twelve hour marks, and hour, minute and second hands.
fn face(time: Time, side: Pixels, cx: &App) -> impl IntoElement + use<> {
    let colors = cx.theme().colors.clone();
    let (hour, minute, second) = hands(time);
    let marks = colors.fg_subtle;
    div()
        .flex_none()
        .size(side)
        .rounded_full()
        .bg(colors.surface)
        .border_1()
        .border_color(colors.border)
        .child(
            canvas(
                |_, _, _| {},
                move |bounds, _, window, _| {
                    let r = f32::from(bounds.size.width) / 2.0;
                    let at = |turn: f32, reach: f32| {
                        let angle = turn * TAU;
                        (r + angle.sin() * r * reach, r - angle.cos() * r * reach)
                    };
                    let width = |share: f32| Pixels::from(r * 2.0 * share);
                    for mark in 0..12 {
                        let turn = mark as f32 / 12.0;
                        let line = [at(turn, 0.78), at(turn, 0.9)];
                        finish(
                            outline(&line, bounds.origin, width(0.02), false),
                            marks,
                            window,
                        );
                    }
                    for (turn, reach, share, ink) in [
                        (hour, 0.5, 0.045, colors.fg),
                        (minute, 0.74, 0.03, colors.fg),
                        (second, 0.84, 0.012, colors.accent),
                    ] {
                        let line = [(r, r), at(turn, reach)];
                        finish(
                            outline(&line, bounds.origin, width(share), false),
                            ink,
                            window,
                        );
                    }
                    let dot = width(0.06);
                    let middle = bounds.origin
                        + point(Pixels::from(r) - dot / 2.0, Pixels::from(r) - dot / 2.0);
                    window.paint_quad(
                        gpui::fill(
                            gpui::Bounds::new(middle, gpui::size(dot, dot)),
                            colors.accent,
                        )
                        .corner_radii(dot / 2.0),
                    );
                },
            )
            .size_full(),
        )
}

/// The time in a zone, the system's unless told: an analog face over its digits, with a label under them. It ticks each second, or shows the owner's moment.
#[derive(IntoElement)]
pub struct Clock {
    id: ElementId,
    zone: Option<TimeZone>,
    now: Option<Timestamp>,
    label: Option<SharedString>,
}

impl Clock {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            zone: None,
            now: None,
            label: None,
        }
    }

    pub fn zone(mut self, zone: TimeZone) -> Self {
        self.zone = Some(zone);
        self
    }

    /// The moment it shows; the wall clock's otherwise.
    pub fn now(mut self, now: Timestamp) -> Self {
        self.now = Some(now);
        self
    }

    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }
}

impl RenderOnce for Clock {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let zone = self.zone.unwrap_or_else(|| format::system_zone("clock"));
        if self.now.is_none() {
            each_second((self.id.clone(), "tick"), window, cx);
        }
        let now = self.now.unwrap_or_else(Timestamp::now).to_zoned(zone);
        let theme = cx.theme();
        let side = theme.misc().face.to_pixels(window.rem_size());
        div()
            .flex()
            .flex_col()
            .items_center()
            .gap_2()
            .child(face(now.time(), side, cx))
            .child(
                tabular(div())
                    .text_size(theme.text_size(TextSize::Lg))
                    .text_color(theme.colors.fg)
                    .child(now.strftime("%H:%M:%S").to_string()),
            )
            .children(self.label.map(|label| {
                div()
                    .text_size(theme.text_size(TextSize::Sm))
                    .text_color(theme.colors.fg_muted)
                    .child(label)
            }))
    }
}

/// Places and their zones, a row each: a small face, the place, how its day and hour stand against home, and its time; fails without places. Home is the system zone unless told; it ticks each second, or shows the owner's moment.
#[derive(IntoElement)]
pub struct WorldClock {
    id: ElementId,
    places: Vec<(SharedString, TimeZone)>,
    home: Option<TimeZone>,
    now: Option<Timestamp>,
}

impl WorldClock {
    pub fn new(
        id: impl Into<ElementId>,
        places: impl IntoIterator<Item = (impl Into<SharedString>, TimeZone)>,
    ) -> Self {
        let id = id.into();
        let places: Vec<_> = places
            .into_iter()
            .map(|(name, zone)| (name.into(), zone))
            .collect();
        assert!(!places.is_empty(), "world clock {id:?} has no places");
        Self {
            id,
            places,
            home: None,
            now: None,
        }
    }

    pub fn home(mut self, home: TimeZone) -> Self {
        self.home = Some(home);
        self
    }

    /// The moment it shows; the wall clock's otherwise.
    pub fn now(mut self, now: Timestamp) -> Self {
        self.now = Some(now);
        self
    }
}

impl RenderOnce for WorldClock {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let home = self
            .home
            .unwrap_or_else(|| format::system_zone("world clock"));
        if self.now.is_none() {
            each_second((self.id.clone(), "tick"), window, cx);
        }
        let now = self.now.unwrap_or_else(Timestamp::now);
        let theme = cx.theme();
        let side = theme.misc().small_face.to_pixels(window.rem_size());
        let rows = self.places.into_iter().map(|(name, zone)| {
            let (day, offset) = against(now, &zone, &home);
            let there = now.to_zoned(zone);
            div()
                .flex()
                .items_center()
                .gap_3()
                .py_2()
                .border_b_1()
                .border_color(theme.colors.border)
                .child(face(there.time(), side, cx))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .child(div().text_color(theme.colors.fg).child(Ellipsis::new(name)))
                        .child(
                            div()
                                .text_size(theme.text_size(TextSize::Xs))
                                .text_color(theme.colors.fg_muted)
                                .child(Ellipsis::new(format!("{day}, {offset}"))),
                        ),
                )
                .child(
                    tabular(div())
                        .flex_none()
                        .text_size(theme.text_size(TextSize::Lg))
                        .text_color(theme.colors.fg)
                        .child(there.strftime("%H:%M").to_string()),
                )
        });
        div().flex().flex_col().children(rows)
    }
}

#[cfg(test)]
mod tests {
    use jiff::{Timestamp, civil::time, tz::TimeZone};

    use super::{against, hands};

    #[test]
    fn hands_turn_with_the_hour_the_minute_and_the_second() {
        assert_eq!(hands(time(0, 0, 0, 0)), (0.0, 0.0, 0.0));
        assert_eq!(hands(time(15, 0, 0, 0)), (0.25, 0.0, 0.0), "three o'clock");
        let (hour, minute, second) = hands(time(9, 30, 15, 0));
        assert!(
            (hour - (9.5 + 15.0 / 3600.0) / 12.0).abs() < 1e-6,
            "the hour hand runs halfway on"
        );
        assert!((minute - 30.25 / 60.0).abs() < 1e-6);
        assert_eq!(second, 0.25);
    }

    #[test]
    #[should_panic(expected = "has no places")]
    fn a_world_clock_without_places_fails() {
        super::WorldClock::new("world", Vec::<(&str, TimeZone)>::new());
    }

    #[test]
    fn a_zone_stands_against_home_by_day_and_offset() {
        let now: Timestamp = "2026-09-28T03:00:00Z".parse().expect("a moment");
        let zone = |name: &str| TimeZone::get(name).expect("a zone in the database");
        let home = zone("America/New_York");
        assert_eq!(
            against(now, &zone("Asia/Tokyo"), &home),
            ("Tomorrow", "+13h".into())
        );
        assert_eq!(
            against(now, &zone("Asia/Kolkata"), &home),
            ("Tomorrow", "+9h 30m".into())
        );
        assert_eq!(
            against(now, &zone("America/Los_Angeles"), &home),
            ("Today", "\u{2212}3h".into())
        );
        assert_eq!(against(now, &home, &home), ("Today", "Same time".into()));
        let late: Timestamp = "2026-09-28T23:30:00Z".parse().expect("a moment");
        assert_eq!(
            against(late, &zone("Pacific/Honolulu"), &zone("Asia/Tokyo")),
            ("Yesterday", "\u{2212}19h".into())
        );
    }
}
