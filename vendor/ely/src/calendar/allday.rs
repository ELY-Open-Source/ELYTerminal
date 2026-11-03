use std::rc::Rc;

use gpui::{
    App, ElementId, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window, div,
    prelude::*, relative, rems,
};
use jiff::{civil::Date, tz::TimeZone};

use super::{
    chip::EventChip,
    event::{Event, When, unique},
    weeks::bars,
};
use crate::{
    forms::OnValue,
    theme::{ActiveTheme, TextSize},
    typography::format,
};

/// The whole days of a run of days, as bars across their columns in lanes, beside a label in the hours' column. It grows a lane at a time.
#[derive(IntoElement)]
pub struct AllDayRow {
    id: ElementId,
    days: Vec<Date>,
    events: Vec<Event>,
    zone: Option<TimeZone>,
    gutters: usize,
    on_event: Option<OnValue>,
}

impl AllDayRow {
    pub fn new(
        id: impl Into<ElementId>,
        days: impl IntoIterator<Item = Date>,
        events: impl IntoIterator<Item = Event>,
    ) -> Self {
        let id = id.into();
        let days: Vec<Date> = days.into_iter().collect();
        assert!(!days.is_empty(), "all-day row {id:?} has no days");
        Self {
            id,
            days,
            events: unique(events),
            zone: None,
            gutters: 1,
            on_event: None,
        }
    }

    /// The zone its days read in; the system's otherwise.
    pub fn zone(mut self, zone: TimeZone) -> Self {
        self.zone = Some(zone);
        self
    }

    /// Gets the key of an event pressed.
    pub fn on_event(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_event = Some(Rc::new(handler));
        self
    }

    /// The hours' columns beside it, one per zone.
    pub(crate) fn gutters(mut self, gutters: usize) -> Self {
        self.gutters = gutters;
        self
    }
}

impl RenderOnce for AllDayRow {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let zone = self
            .zone
            .unwrap_or_else(|| format::system_zone("all-day row"));
        let whole: Vec<Event> = self
            .events
            .into_iter()
            .filter(|event| matches!(event.when, When::AllDay { .. }))
            .collect();
        let count = self.days.len();
        let bars = bars(&whole, self.days[0], count, &zone);
        let lanes = bars.iter().map(|bar| bar.lane + 1).max().unwrap_or(1);
        let theme = cx.theme();
        let rem = window.rem_size();
        let sizes = theme.calendar();
        let (chip, gap, pad) = (
            sizes.chip.to_pixels(rem),
            rems(0.125).to_pixels(rem),
            rems(0.25).to_pixels(rem),
        );
        let gutters = sizes.gutter.to_pixels(rem) * self.gutters as f32;
        let pitch = chip + gap;
        let chips = bars.iter().map(|bar| {
            let event = &whole[bar.event];
            let chip = EventChip::new(
                (self.id.clone(), format!("bar-{}", event.key)),
                event.clone(),
            )
            .zone(zone.clone())
            .open(bar.before, bar.after);
            let chip = match self.on_event.clone() {
                Some(on_event) => {
                    let key = event.key.clone();
                    chip.on_click(move |window, cx| on_event(&key, window, cx))
                }
                None => chip,
            };
            let span = (bar.to - bar.from + 1) as f32;
            div()
                .absolute()
                .top(pad + pitch * bar.lane as f32)
                .left(relative(bar.from as f32 / count as f32))
                .w(relative(span / count as f32))
                .when(!bar.before, |slot| slot.pl_0p5())
                .when(!bar.after, |slot| slot.pr_0p5())
                .child(chip)
        });
        div()
            .relative()
            .h(pad * 2.0 + pitch * lanes as f32 - gap)
            .border_b_1()
            .border_color(theme.colors.border)
            .child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .w(gutters)
                    .h_full()
                    .flex()
                    .justify_end()
                    .pt_1p5()
                    .pr_2()
                    .text_size(theme.text_size(TextSize::Xs))
                    .text_color(theme.colors.fg_subtle)
                    .child("All day"),
            )
            .child(
                div()
                    .absolute()
                    .top_0()
                    .bottom_0()
                    .left(gutters)
                    .right_0()
                    .children(chips.collect::<Vec<_>>()),
            )
    }
}
