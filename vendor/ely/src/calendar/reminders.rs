use std::rc::Rc;

use gpui::{
    App, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce, SharedString,
    Styled, Window, div,
};

use crate::{
    buttons::{Button, ButtonVariant, IconButton},
    forms::{Choice, Select},
    primitives::IconName,
    theme::ControlSize,
};

type OnMinutes = Rc<dyn Fn(&[u32], &mut Window, &mut App)>;

/// The reminders on offer, in minutes before an event, with their words.
pub(crate) const OFFSETS: [(u32, &str); 9] = [
    (0, "At the time"),
    (5, "5 minutes before"),
    (10, "10 minutes before"),
    (15, "15 minutes before"),
    (30, "30 minutes before"),
    (60, "1 hour before"),
    (120, "2 hours before"),
    (1440, "1 day before"),
    (10080, "1 week before"),
];

/// `minutes` sorted, each asserted to be on offer and to come once.
fn checked(minutes: &[u32]) -> Vec<u32> {
    let mut sorted = minutes.to_vec();
    sorted.sort_unstable();
    for (ix, minute) in sorted.iter().enumerate() {
        let offered = OFFSETS.iter().any(|(each, _)| each == minute);
        assert!(offered, "no reminder {minute} minutes before is on offer");
        assert!(
            ix == 0 || sorted[ix - 1] != *minute,
            "reminder {minute} twice"
        );
    }
    sorted
}

/// Reminders before an event: each picked from a list, with a way off. Add brings the nearest one not yet set.
#[derive(IntoElement)]
pub struct ReminderPicker {
    id: ElementId,
    minutes: Vec<u32>,
    on_change: Option<OnMinutes>,
}

impl ReminderPicker {
    /// `minutes` before the event, each one of those on offer.
    pub fn new(id: impl Into<ElementId>, minutes: impl IntoIterator<Item = u32>) -> Self {
        let minutes: Vec<u32> = minutes.into_iter().collect();
        Self {
            id: id.into(),
            minutes: checked(&minutes),
            on_change: None,
        }
    }

    /// Gets the minutes before after each change, soonest first.
    pub fn on_change(mut self, handler: impl Fn(&[u32], &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for ReminderPicker {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let set: OnMinutes = {
            let (id, on_change) = (self.id.clone(), self.on_change);
            Rc::new(move |next, window, cx| {
                let next = checked(next);
                log::info!("reminder picker {id:?}: {next:?}");
                if let Some(on_change) = &on_change {
                    on_change(&next, window, cx);
                }
            })
        };
        let minutes = Rc::new(self.minutes);
        let rows = minutes.iter().enumerate().map(|(ix, minute)| {
            let choices = OFFSETS
                .iter()
                .filter(|(each, _)| each == minute || !minutes.contains(each))
                .map(|(each, words)| Choice::new(each.to_string(), *words));
            let (change, remove) = (set.clone(), set.clone());
            let (changing, removing) = (minutes.clone(), minutes.clone());
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    div().flex_1().min_w_0().child(
                        Select::new((self.id.clone(), format!("offset-{minute}")), choices)
                            .size(ControlSize::Sm)
                            .selected(minute.to_string())
                            .on_change(move |value: &SharedString, window, cx| {
                                let picked: u32 = value.parse().expect("an offset is a count");
                                let mut next = changing.to_vec();
                                next[ix] = picked;
                                change(&next, window, cx)
                            }),
                    ),
                )
                .child(
                    IconButton::new((self.id.clone(), format!("remove-{minute}")), IconName::X)
                        .variant(ButtonVariant::Ghost)
                        .size(ControlSize::Sm)
                        .tooltip("Remove reminder")
                        .on_click(move |_, window, cx| {
                            let next: Vec<u32> = removing
                                .iter()
                                .copied()
                                .filter(|each| each != &removing[ix])
                                .collect();
                            remove(&next, window, cx)
                        }),
                )
        });
        let spare = OFFSETS
            .iter()
            .map(|(minute, _)| *minute)
            .find(|minute| !minutes.contains(minute));
        let add = {
            let (set, minutes) = (set.clone(), minutes.clone());
            Button::new((self.id.clone(), "add"), "Add reminder")
                .variant(ButtonVariant::Ghost)
                .size(ControlSize::Sm)
                .icon(IconName::Plus)
                .disabled(spare.is_none())
                .on_click(move |_, window, cx| {
                    let mut next = minutes.to_vec();
                    next.push(spare.expect("Add waits for a reminder not yet set"));
                    set(&next, window, cx)
                })
        };
        div()
            .debug_selector(|| "reminders".into())
            .flex()
            .flex_col()
            .gap_2()
            .children(rows.collect::<Vec<_>>())
            .child(div().flex().child(add))
    }
}

#[cfg(test)]
mod tests {
    use super::checked;

    #[test]
    fn reminders_sort_soonest_first() {
        assert_eq!(checked(&[1440, 10, 0]), [0, 10, 1440]);
    }

    #[test]
    #[should_panic(expected = "no reminder 7 minutes before is on offer")]
    fn a_reminder_is_one_on_offer() {
        checked(&[7]);
    }

    #[test]
    #[should_panic(expected = "reminder 10 twice")]
    fn a_reminder_comes_once() {
        checked(&[10, 10]);
    }
}
