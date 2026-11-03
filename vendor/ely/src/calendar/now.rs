use gpui::{
    App, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce, Styled, Window, div,
};
use jiff::{Timestamp, tz::TimeZone};

use crate::{
    theme::ActiveTheme,
    typography::{format, fresh},
};

/// A line across a day of a time grid at the minute now, with a dot where it starts. It moves with the clock.
#[derive(IntoElement)]
pub struct CurrentTimeIndicator {
    id: ElementId,
    zone: Option<TimeZone>,
    now: Option<Timestamp>,
}

impl CurrentTimeIndicator {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            zone: None,
            now: None,
        }
    }

    /// The zone its minute reads in; the system's otherwise.
    pub fn zone(mut self, zone: TimeZone) -> Self {
        self.zone = Some(zone);
        self
    }

    /// The moment it marks; the clock's otherwise.
    pub fn now(mut self, now: Timestamp) -> Self {
        self.now = Some(now);
        self
    }
}

impl RenderOnce for CurrentTimeIndicator {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let zone = self
            .zone
            .unwrap_or_else(|| format::system_zone("current time indicator"));
        if self.now.is_none() {
            fresh((self.id.clone(), "clock"), window, cx);
        }
        let now = self.now.unwrap_or_else(Timestamp::now).to_zoned(zone);
        let minute = f32::from(now.hour()) * 60.0 + f32::from(now.minute());
        let theme = cx.theme();
        let (hour, dot) = (theme.calendar().hour, theme.status_dot() * 1.5);
        div()
            .debug_selector(|| "now-line".into())
            .absolute()
            .top(hour * (minute / 60.0))
            .left_0()
            .right_0()
            .h_0()
            .child(
                div()
                    .absolute()
                    .left_0()
                    .right_0()
                    .border_t_2()
                    .border_color(theme.colors.accent),
            )
            .child(
                div()
                    .absolute()
                    .top(dot * -0.5)
                    .left(dot * -0.5)
                    .size(dot)
                    .rounded_full()
                    .bg(theme.colors.accent),
            )
    }
}
