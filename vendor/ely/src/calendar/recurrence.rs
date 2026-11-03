use std::rc::Rc;

use gpui::{
    App, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce, SharedString,
    Styled, Window, div, prelude::*,
};
use jiff::{
    ToSpan,
    civil::{Date, Weekday},
};

use super::repeat::{Ends, Frequency, Recurrence};
use crate::{
    forms::{Choice, ChoiceChips, DatePicker, NumberInput, RadioGroup, Select},
    theme::{ActiveTheme, ControlSize, TextSize},
};

type OnRecurrence = Rc<dyn Fn(Option<Recurrence>, &mut Window, &mut App)>;

const FREQUENCIES: [(&str, &str, Option<Frequency>); 5] = [
    ("never", "Does not repeat", None),
    ("daily", "Daily", Some(Frequency::Daily)),
    ("weekly", "Weekly", Some(Frequency::Weekly)),
    ("monthly", "Monthly", Some(Frequency::Monthly)),
    ("yearly", "Yearly", Some(Frequency::Yearly)),
];

const WEEKDAYS: [(&str, &str, Weekday); 7] = [
    ("mon", "Mon", Weekday::Monday),
    ("tue", "Tue", Weekday::Tuesday),
    ("wed", "Wed", Weekday::Wednesday),
    ("thu", "Thu", Weekday::Thursday),
    ("fri", "Fri", Weekday::Friday),
    ("sat", "Sat", Weekday::Saturday),
    ("sun", "Sun", Weekday::Sunday),
];

fn frequency_key(frequency: Option<Frequency>) -> &'static str {
    FREQUENCIES
        .iter()
        .find(|(_, _, each)| *each == frequency)
        .map(|(key, _, _)| *key)
        .expect("every frequency has a key")
}

fn weekday_key(day: Weekday) -> &'static str {
    WEEKDAYS
        .iter()
        .find(|(_, _, each)| *each == day)
        .map(|(key, _, _)| *key)
        .expect("every weekday has a key")
}

/// Sets how an event repeats: not at all, or daily, weekly on its days, monthly or yearly, every so many, until a day or a count. A line under it reads the rule back with the next days it falls on.
#[derive(IntoElement)]
pub struct RecurrenceEditor {
    id: ElementId,
    first: Date,
    value: Option<Recurrence>,
    on_change: Option<OnRecurrence>,
}

impl RecurrenceEditor {
    /// For an event that starts on `first`.
    pub fn new(id: impl Into<ElementId>, first: Date, value: Option<Recurrence>) -> Self {
        Self {
            id: id.into(),
            first,
            value,
            on_change: None,
        }
    }

    /// Gets the rule after each change, or none when it stops repeating.
    pub fn on_change(
        mut self,
        handler: impl Fn(Option<Recurrence>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for RecurrenceEditor {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let set: OnRecurrence = {
            let (id, on_change) = (self.id.clone(), self.on_change);
            Rc::new(move |next, window, cx| {
                log::info!("recurrence editor {id:?}: {next:?}");
                if let Some(on_change) = &on_change {
                    on_change(next, window, cx);
                }
            })
        };
        let rule = self.value.clone();
        let choices = FREQUENCIES.map(|(key, label, _)| Choice::new(key, label));
        let pick = {
            let (set, rule) = (set.clone(), rule.clone());
            move |key: &SharedString, window: &mut Window, cx: &mut App| {
                let frequency = FREQUENCIES
                    .iter()
                    .find(|(each, _, _)| *each == key.as_ref())
                    .and_then(|(_, _, frequency)| *frequency);
                let next = frequency.map(|frequency| match &rule {
                    Some(rule) => Recurrence {
                        frequency,
                        ..rule.clone()
                    },
                    None => Recurrence::new(frequency),
                });
                set(next, window, cx)
            }
        };
        let theme = cx.theme();
        let frequency = Select::new((self.id.clone(), "frequency"), choices)
            .selected(frequency_key(rule.as_ref().map(|rule| rule.frequency)))
            .on_change(pick);
        let Some(rule) = rule else {
            return div().child(frequency);
        };
        let unit = match (rule.frequency, rule.interval) {
            (Frequency::Daily, 1) => "day",
            (Frequency::Daily, _) => "days",
            (Frequency::Weekly, 1) => "week",
            (Frequency::Weekly, _) => "weeks",
            (Frequency::Monthly, 1) => "month",
            (Frequency::Monthly, _) => "months",
            (Frequency::Yearly, 1) => "year",
            (Frequency::Yearly, _) => "years",
        };
        let every = {
            let (set, rule) = (set.clone(), rule.clone());
            NumberInput::new((self.id.clone(), "interval"), f64::from(rule.interval))
                .range(1.0, 99.0)
                .size(ControlSize::Sm)
                .on_change(move |value, window, cx| {
                    let interval = value as u16;
                    set(
                        Some(Recurrence {
                            interval,
                            ..rule.clone()
                        }),
                        window,
                        cx,
                    )
                })
        };
        let days = (rule.frequency == Frequency::Weekly).then(|| {
            let (set, rule, first) = (set.clone(), rule.clone(), self.first);
            let on: Vec<&str> = match rule.weekdays.is_empty() {
                true => vec![weekday_key(first.weekday())],
                false => rule.weekdays.iter().map(|day| weekday_key(*day)).collect(),
            };
            ChoiceChips::new(
                (self.id.clone(), "weekdays"),
                WEEKDAYS.map(|(key, label, _)| Choice::new(key, label)),
            )
            .multiple()
            .selected(on)
            .on_change(move |keys, window, cx| {
                let weekdays = WEEKDAYS
                    .iter()
                    .filter(|(key, _, _)| keys.iter().any(|each| each.as_ref() == *key))
                    .map(|(_, _, day)| *day)
                    .collect();
                set(
                    Some(Recurrence {
                        weekdays,
                        ..rule.clone()
                    }),
                    window,
                    cx,
                )
            })
        });
        let ends = {
            let (set, rule, first) = (set.clone(), rule.clone(), self.first);
            let key = match rule.ends {
                Ends::Never => "never",
                Ends::On(_) => "on",
                Ends::After(_) => "after",
            };
            RadioGroup::new(
                (self.id.clone(), "ends"),
                [
                    Choice::new("never", "Never"),
                    Choice::new("on", "On a day"),
                    Choice::new("after", "After a number of times"),
                ],
            )
            .selected(key)
            .on_change(move |key, window, cx| {
                let ends = match key.as_ref() {
                    "on" => match first.checked_add(1.month()) {
                        Ok(last) => Ends::On(last),
                        Err(error) => {
                            log::error!("recurrence: no day a month after {first}: {error}");
                            return;
                        }
                    },
                    "after" => Ends::After(10),
                    _ => Ends::Never,
                };
                set(
                    Some(Recurrence {
                        ends,
                        ..rule.clone()
                    }),
                    window,
                    cx,
                )
            })
        };
        let until = match rule.ends {
            Ends::On(last) => {
                let (set, rule) = (set.clone(), rule.clone());
                Some(
                    DatePicker::new((self.id.clone(), "until"), Some(last))
                        .on_change(move |last, window, cx| {
                            let ends = Ends::On(last);
                            set(
                                Some(Recurrence {
                                    ends,
                                    ..rule.clone()
                                }),
                                window,
                                cx,
                            )
                        })
                        .into_any_element(),
                )
            }
            Ends::After(count) => {
                let (set, rule) = (set.clone(), rule.clone());
                Some(
                    NumberInput::new((self.id.clone(), "count"), f64::from(count))
                        .range(1.0, 999.0)
                        .size(ControlSize::Sm)
                        .on_change(move |value, window, cx| {
                            let ends = Ends::After(value as u16);
                            set(
                                Some(Recurrence {
                                    ends,
                                    ..rule.clone()
                                }),
                                window,
                                cx,
                            )
                        })
                        .into_any_element(),
                )
            }
            Ends::Never => None,
        };
        let next: Vec<String> = rule
            .days(self.first, 4)
            .iter()
            .map(|day| day.strftime("%a %b %-d").to_string())
            .collect();
        let quiet = |text: String| {
            div()
                .text_size(theme.text_size(TextSize::Sm))
                .text_color(theme.colors.fg_muted)
                .child(text)
        };
        div()
            .debug_selector(|| "recurrence".into())
            .flex()
            .flex_col()
            .gap_3()
            .child(frequency)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(quiet("Every".into()))
                    .child(div().w_24().child(every))
                    .child(quiet(unit.into())),
            )
            .children(days)
            .child(ends)
            .children(until)
            .child(quiet(rule.describe(self.first)))
            .when(!next.is_empty(), |editor| {
                editor.child(quiet(format!("Next: {}", next.join(", "))))
            })
    }
}
