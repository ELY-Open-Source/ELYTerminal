use std::rc::Rc;

use gpui::{
    App, ElementId, Entity, InteractiveElement, ParentElement, SharedString, Styled, Window, div,
};
use jiff::{
    Timestamp, ToSpan,
    civil::{Date, DateTime},
    tz::TimeZone,
};

use crate::{
    buttons::{Button, ButtonVariant},
    forms::{DateTimePicker, FormError},
    menus::MenuItem,
    overlays::Dialog,
    primitives::IconName,
    typography::format,
};

pub(super) type OnTime = Rc<dyn Fn(Timestamp, &mut Window, &mut App)>;

/// `hour`:00 on `day` in `zone`.
pub(super) fn at_hour(day: Date, hour: i8, zone: &TimeZone) -> Timestamp {
    day.at(hour, 0, 0, 0)
        .to_zoned(zone.clone())
        .expect("the zone holds the hour")
        .timestamp()
}

/// The first Monday after `day`: a week on when `day` is a Monday.
pub(super) fn next_monday(day: Date) -> Date {
    let ahead = 7 - i64::from(day.weekday().to_monday_zero_offset());
    day.checked_add(ahead.days()).expect("a Monday lies ahead")
}

/// `name` beside the day and hour of `at` in `zone`, as a menu row shows a time.
pub(super) fn named(name: &str, at: Timestamp, zone: &TimeZone) -> (String, Timestamp) {
    let hour = format::datetime(at, zone, "%a %H:%M").expect("a fixed pattern formats");
    (format!("{name} · {hour}"), at)
}

/// The moment `chosen` names in `zone`, once it lies after `now`; none past the times jiff holds.
pub(super) fn ahead(chosen: DateTime, zone: &TimeZone, now: Timestamp) -> Option<Timestamp> {
    let at = match chosen.to_zoned(zone.clone()) {
        Ok(at) => at.timestamp(),
        Err(error) => {
            log::error!("time dialog: {chosen} has no moment in its zone: {error}");
            return None;
        }
    };
    (at > now).then_some(at)
}

/// `times` less each moment an earlier one names, as tomorrow names next Monday on a Sunday.
pub(super) fn distinct(
    times: impl IntoIterator<Item = (String, Timestamp)>,
) -> Vec<(String, Timestamp)> {
    let mut kept: Vec<(String, Timestamp)> = Vec::new();
    for (name, at) in times {
        if kept.iter().all(|(_, seen)| *seen != at) {
            kept.push((name, at));
        }
    }
    kept
}

/// Whether the dialog for a time of one's own shows, and the time picked in it.
#[derive(Default)]
pub(super) struct Picking {
    open: bool,
    chosen: Option<DateTime>,
}

pub(super) fn picking(id: &ElementId, window: &mut Window, cx: &mut App) -> Entity<Picking> {
    window.use_keyed_state((id.clone(), "picking"), cx, |_, _| Picking::default())
}

/// The menu row that opens the dialog.
pub(super) fn pick_item(owner: &'static str, picking: &Entity<Picking>) -> MenuItem {
    let picking = picking.clone();
    MenuItem::new("Pick a time…")
        .icon(IconName::Calendar)
        .on_click(move |_, cx| {
            log::info!("{owner}: pick a time");
            picking.update(cx, |picking, cx| {
                picking.open = true;
                cx.notify();
            })
        })
}

/// What the dialog asks, and the action that takes its time.
pub(super) struct Ask {
    pub title: &'static str,
    pub detail: &'static str,
    pub action: &'static str,
}

/// While open, a dialog to pick a day and an hour in `zone`. Its action waits for a time ahead of now, then closes and hands the time on.
pub(super) fn time_dialog(
    id: &ElementId,
    picking: &Entity<Picking>,
    ask: Ask,
    zone: &TimeZone,
    on_time: Option<OnTime>,
    cx: &App,
) -> Option<Dialog> {
    let now = picking.read(cx);
    if !now.open {
        return None;
    }
    let chosen = now.chosen;
    let ahead = chosen.and_then(|chosen| ahead(chosen, zone, Timestamp::now()));
    let (closing, choosing, id) = (picking.clone(), picking.clone(), id.clone());
    let late = (chosen.is_some() && ahead.is_none()).then(|| {
        div()
            .debug_selector(|| "time-late".into())
            .child(FormError::new(
                (id.clone(), "late"),
                "That time has passed.",
            ))
    });
    let dialog = Dialog::new((id.clone(), "dialog"), ask.title, move |_, cx| {
        closing.update(cx, |picking, cx| {
            *picking = Picking::default();
            cx.notify();
        })
    })
    .detail(ask.detail)
    .child(
        div()
            .debug_selector(|| "time-dialog".into())
            .flex()
            .flex_col()
            .gap_2()
            .child(
                DateTimePicker::new((id.clone(), "when"), chosen).on_change(move |at, _, cx| {
                    choosing.update(cx, |picking, cx| {
                        picking.chosen = Some(at);
                        cx.notify();
                    })
                }),
            )
            .children(late),
    )
    .action({
        let id = id.clone();
        move |close| {
            Button::new((id, "cancel"), "Cancel")
                .variant(ButtonVariant::Ghost)
                .on_click(move |_, window, cx| close(window, cx))
        }
    })
    .action(move |close| {
        Button::new((id.clone(), "take"), SharedString::from(ask.action))
            .variant(ButtonVariant::Primary)
            .disabled(ahead.is_none())
            .on_click(move |_, window, cx| {
                let at = ahead.expect("the action waits for a time ahead");
                log::info!("time dialog {id:?}: {at}");
                close(window, cx);
                if let Some(on_time) = &on_time {
                    on_time(at, window, cx);
                }
            })
    });
    Some(dialog)
}

#[cfg(test)]
mod tests {
    use jiff::{
        SignedDuration,
        civil::date,
        tz::{TimeZone, offset},
    };

    use super::{ahead, at_hour};

    #[test]
    fn a_picked_time_counts_once_it_lies_ahead_of_now() {
        let day = date(2026, 9, 27);
        let now = at_hour(day, 9, &TimeZone::UTC);
        let utc = &TimeZone::UTC;
        assert_eq!(
            ahead(day.at(8, 59, 0, 0), utc, now),
            None,
            "a minute behind"
        );
        assert_eq!(ahead(day.at(9, 0, 0, 0), utc, now), None, "now itself");
        assert_eq!(
            ahead(day.at(9, 1, 0, 0), utc, now),
            Some(now + SignedDuration::from_mins(1)),
            "a minute ahead"
        );
        let tokyo = TimeZone::fixed(offset(9));
        assert_eq!(
            ahead(day.at(10, 0, 0, 0), &tokyo, now),
            None,
            "10:00 in Tokyo is 01:00 here"
        );
        let west = TimeZone::fixed(offset(-9));
        assert_eq!(
            ahead(jiff::civil::DateTime::MAX, &west, now),
            None,
            "past what jiff holds"
        );
    }
}
