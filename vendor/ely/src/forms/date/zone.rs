use std::{rc::Rc, sync::LazyLock};

use gpui::{App, ElementId, Entity, IntoElement, RenderOnce, SharedString, Window};
use jiff::{
    Timestamp,
    tz::{self, Offset, TimeZone},
};

use super::super::{Choice, Combobox, TextInput, options::OnValue};
use crate::typography::format::MINUS;

/// An offset as "UTC+09:00", with a true minus sign west of Greenwich.
pub(crate) fn show_offset(offset: Offset) -> String {
    let seconds = offset.seconds();
    let sign = if seconds < 0 { MINUS } else { '+' };
    let minutes = seconds.unsigned_abs() / 60;
    format!("UTC{sign}{:02}:{:02}", minutes / 60, minutes % 60)
}

/// Every zone the system knows, by name.
static ZONES: LazyLock<Vec<(SharedString, TimeZone)>> = LazyLock::new(|| {
    let mut zones: Vec<(SharedString, TimeZone)> = tz::db()
        .available()
        .filter_map(|name| match TimeZone::get(name.as_str()) {
            Ok(zone) => Some((SharedString::from(name.as_str().to_string()), zone)),
            Err(error) => {
                log::error!("timezone select: skipped {}: {error}", name.as_str());
                None
            }
        })
        .collect();
    zones.sort_by(|a, b| a.0.cmp(&b.0));
    log::info!("timezone select: {} zones", zones.len());
    zones
});

/// A searchable list of the system's time zones, each with its offset now.
#[derive(IntoElement)]
pub struct TimezoneSelect {
    id: ElementId,
    state: Entity<TextInput>,
    selected: Option<SharedString>,
    on_change: Option<OnValue>,
}

impl TimezoneSelect {
    pub fn new(id: impl Into<ElementId>, state: &Entity<TextInput>) -> Self {
        Self {
            id: id.into(),
            state: state.clone(),
            selected: None,
            on_change: None,
        }
    }

    /// A zone by its IANA name, such as `Asia/Tokyo`.
    pub fn selected(mut self, zone: impl Into<SharedString>) -> Self {
        self.selected = Some(zone.into());
        self
    }

    pub fn on_change(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for TimezoneSelect {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let now = Timestamp::now();
        let zones = ZONES.iter().map(|(name, zone)| {
            Choice::new(name.clone(), name.clone()).note(show_offset(zone.to_offset(now)))
        });
        let mut combo = Combobox::new(self.id, &self.state, zones);
        if let Some(zone) = self.selected {
            combo = combo.selected(zone);
        }
        if let Some(on_change) = self.on_change {
            combo = combo.on_change(move |zone, window, cx| on_change(zone, window, cx));
        }
        combo
    }
}

#[cfg(test)]
mod tests {
    use jiff::tz::Offset;

    use super::show_offset;

    #[test]
    fn offsets_read_as_utc_plus_or_minus() {
        assert_eq!(
            show_offset(Offset::from_seconds(9 * 3600).unwrap()),
            "UTC+09:00"
        );
        assert_eq!(
            show_offset(Offset::from_seconds(-(5 * 3600 + 1800)).unwrap()),
            "UTC\u{2212}05:30"
        );
        assert_eq!(show_offset(Offset::UTC), "UTC+00:00");
    }
}
