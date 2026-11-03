use std::rc::Rc;

use gpui::{
    App, ElementId, FontWeight, IntoElement, ParentElement, RenderOnce, SharedString, Styled,
    Window, div,
};
use jiff::Timestamp;

use crate::{
    buttons::{Button, ButtonVariant},
    data_display::{Badge, Timeline, TimelineItem, Tone},
    lists::{ListItem, SelectableList, row_action},
    primitives::{Icon, Severity},
    theme::{ActiveTheme, IconSize, Radius, TextSize},
    typography::{Ellipsis, format},
};

/// Where an alert stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlertState {
    Firing,
    Acknowledged,
    Resolved,
}

/// An alert from a monitor: its key and title, what raised it, how grave it is, where it stands, and since when.
#[derive(Clone, Debug, PartialEq)]
pub struct MonitorAlert {
    pub key: SharedString,
    pub title: SharedString,
    pub source: SharedString,
    pub severity: Severity,
    pub state: AlertState,
    pub since: Timestamp,
}

type OnKey = Rc<dyn Fn(&SharedString, &mut Window, &mut App)>;

/// Alerts, each with its severity's icon, what raised it and since when, and where it stands; a firing one can be acknowledged from its row. Enter or a double press opens one.
#[derive(IntoElement)]
pub struct AlertList {
    id: ElementId,
    alerts: Vec<MonitorAlert>,
    now: Timestamp,
    on_open: Option<OnKey>,
    on_acknowledge: Option<OnKey>,
}

impl AlertList {
    /// `now` dates each alert.
    pub fn new(
        id: impl Into<ElementId>,
        alerts: impl IntoIterator<Item = MonitorAlert>,
        now: Timestamp,
    ) -> Self {
        Self {
            id: id.into(),
            alerts: alerts.into_iter().collect(),
            now,
            on_open: None,
            on_acknowledge: None,
        }
    }

    pub fn on_open(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_open = Some(Rc::new(handler));
        self
    }

    pub fn on_acknowledge(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_acknowledge = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for AlertList {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id;
        let theme = cx.theme();
        let list = self.alerts.iter().fold(
            SelectableList::new((id.clone(), "alerts")),
            |list, alert| {
                let (tone, words) = match alert.state {
                    AlertState::Firing => (Tone::from(alert.severity), "Firing"),
                    AlertState::Acknowledged => (Tone::Neutral, "Acknowledged"),
                    AlertState::Resolved => (Tone::Success, "Resolved"),
                };
                let key = alert.key.clone();
                let firing = alert.state == AlertState::Firing;
                let button = self
                    .on_acknowledge
                    .clone()
                    .filter(|_| firing)
                    .map(|acknowledge| {
                        row_action(
                            Button::new((id.clone(), format!("ack-{}", alert.key)), "Acknowledge")
                                .variant(ButtonVariant::Ghost)
                                .on_click(move |_, window, cx| {
                                    log::info!("alert list: acknowledge {key}");
                                    acknowledge(&key, window, cx);
                                }),
                        )
                    });
                list.row(
                    alert.key.clone(),
                    ListItem::new(
                        (id.clone(), format!("row-{}", alert.key)),
                        alert.title.clone(),
                    )
                    .description(format!(
                        "{} · {}",
                        alert.source,
                        format::relative(alert.since, self.now)
                    ))
                    .leading(
                        Icon::new(alert.severity.icon())
                            .size(IconSize::Sm)
                            .color(alert.severity.color(&theme.colors)),
                    )
                    .trailing(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(Badge::new(words).tone(tone))
                            .children(button),
                    ),
                )
            },
        );
        match self.on_open {
            Some(on_open) => list.on_activate(move |key, window, cx| {
                log::info!("alert list: open {key}");
                on_open(key, window, cx)
            }),
            None => list,
        }
    }
}

/// How far an incident has come.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IncidentPhase {
    Investigating,
    Identified,
    Monitoring,
    Resolved,
}

impl IncidentPhase {
    fn words(self) -> &'static str {
        match self {
            IncidentPhase::Investigating => "Investigating",
            IncidentPhase::Identified => "Identified",
            IncidentPhase::Monitoring => "Monitoring",
            IncidentPhase::Resolved => "Resolved",
        }
    }

    fn tone(self) -> Tone {
        match self {
            IncidentPhase::Investigating => Tone::Danger,
            IncidentPhase::Identified => Tone::Warning,
            IncidentPhase::Monitoring => Tone::Info,
            IncidentPhase::Resolved => Tone::Success,
        }
    }
}

/// How grave an incident is, in words.
fn grave(severity: Severity) -> &'static str {
    match severity {
        Severity::Info | Severity::Success => "Minor",
        Severity::Warning => "Major",
        Severity::Danger => "Critical",
    }
}

/// A word on an incident: the phase it marks, what it says, and when.
#[derive(Clone, Debug, PartialEq)]
pub struct IncidentUpdate {
    pub phase: IncidentPhase,
    pub text: SharedString,
    pub at: Timestamp,
}

/// An incident: its title, how grave it is, the services it touches, when it began, and its updates oldest first.
#[derive(Clone, Debug, PartialEq)]
pub struct Incident {
    pub title: SharedString,
    pub severity: Severity,
    pub services: Vec<SharedString>,
    pub started: Timestamp,
    pub updates: Vec<IncidentUpdate>,
}

impl Incident {
    /// Its phase, the latest update's.
    pub fn phase(&self) -> IncidentPhase {
        self.updates
            .last()
            .map_or(IncidentPhase::Investigating, |update| update.phase)
    }
}

/// An incident: its title under its severity and phase, when it began and what it touches, then its updates newest first.
#[derive(IntoElement)]
pub struct IncidentCard {
    incident: Incident,
    now: Timestamp,
}

impl IncidentCard {
    /// `now` dates the incident and its updates.
    pub fn new(incident: Incident, now: Timestamp) -> Self {
        Self { incident, now }
    }
}

impl RenderOnce for IncidentCard {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let (incident, now) = (self.incident, self.now);
        let phase = incident.phase();
        let touched = if incident.services.is_empty() {
            String::new()
        } else {
            format!(" · {}", incident.services.join(", "))
        };
        let timeline = incident
            .updates
            .iter()
            .rev()
            .fold(Timeline::new(), |timeline, update| {
                let when = format!(
                    "{} · {}",
                    update.phase.words(),
                    format::relative(update.at, now)
                );
                timeline.item(
                    TimelineItem::new(
                        div().child(update.text.clone()).child(
                            div()
                                .text_size(theme.text_size(TextSize::Xs))
                                .text_color(theme.colors.fg_muted)
                                .child(when),
                        ),
                    )
                    .tone(update.phase.tone()),
                )
            });
        div()
            .flex()
            .flex_col()
            .gap_3()
            .p_4()
            .rounded(theme.radius(Radius::Lg))
            .border_1()
            .border_color(theme.colors.border)
            .bg(theme.colors.surface)
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap_2()
                    .child(Badge::new(grave(incident.severity)).tone(Tone::from(incident.severity)))
                    .child(Badge::new(phase.words()).tone(phase.tone()).dot()),
            )
            .child(
                div()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(Ellipsis::new(incident.title)),
            )
            .child(
                div()
                    .text_size(theme.text_size(TextSize::Sm))
                    .text_color(theme.colors.fg_muted)
                    .child(format!(
                        "Began {}{touched}",
                        format::relative(incident.started, now)
                    )),
            )
            .child(timeline)
    }
}

#[cfg(test)]
mod tests {
    use super::{Incident, IncidentPhase, IncidentUpdate, grave};
    use crate::primitives::Severity;

    #[test]
    fn an_incident_reads_its_latest_phase_and_its_gravity() {
        let at: jiff::Timestamp = "2026-09-27T12:00:00Z".parse().expect("a time");
        let mut incident = Incident {
            title: "Slow checkout".into(),
            severity: Severity::Warning,
            services: vec!["api".into()],
            started: at,
            updates: Vec::new(),
        };
        assert_eq!(
            incident.phase(),
            IncidentPhase::Investigating,
            "nothing said yet"
        );
        incident.updates = vec![
            IncidentUpdate {
                phase: IncidentPhase::Identified,
                text: "A slow query".into(),
                at,
            },
            IncidentUpdate {
                phase: IncidentPhase::Monitoring,
                text: "A fix is out".into(),
                at,
            },
        ];
        assert_eq!(incident.phase(), IncidentPhase::Monitoring);
        assert_eq!(
            [Severity::Info, Severity::Warning, Severity::Danger].map(grave),
            ["Minor", "Major", "Critical"]
        );
    }
}
