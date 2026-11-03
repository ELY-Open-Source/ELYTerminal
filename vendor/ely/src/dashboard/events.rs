use gpui::{
    App, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce, SharedString,
    Styled, Window, div,
};
use jiff::Timestamp;

use crate::{
    buttons::{Button, ButtonVariant},
    data_display::{Badge, Tone},
    forms::{Choice, ChoiceChips},
    theme::{ActiveTheme, TextSize},
    typography::{Ellipsis, format},
};

/// Something that happened: its key, its kind, what it says, its tone, and when.
#[derive(Clone, Debug, PartialEq)]
pub struct StreamEvent {
    pub key: SharedString,
    pub kind: SharedString,
    pub text: SharedString,
    pub tone: Tone,
    pub at: Timestamp,
}

/// The stream's own state: the kinds shown, and while paused how many events there were.
#[derive(Default)]
struct Hold {
    kinds: Vec<SharedString>,
    held: Option<usize>,
}

/// Events as they come, newest first, each with its kind, its words and its time. Chips keep the kinds shown; Pause holds the list still while new events count up on a button that shows them.
#[derive(IntoElement)]
pub struct EventStream {
    id: ElementId,
    events: Vec<StreamEvent>,
    now: Timestamp,
}

impl EventStream {
    /// `events` oldest first, dated against `now`.
    pub fn new(
        id: impl Into<ElementId>,
        events: impl IntoIterator<Item = StreamEvent>,
        now: Timestamp,
    ) -> Self {
        Self {
            id: id.into(),
            events: events.into_iter().collect(),
            now,
        }
    }
}

/// The events shown: those up to the hold, of the kinds kept, newest first.
pub(crate) fn shown<'a>(
    events: &'a [StreamEvent],
    kinds: &[SharedString],
    held: Option<usize>,
) -> Vec<&'a StreamEvent> {
    let upto = held.unwrap_or(events.len()).min(events.len());
    events[..upto]
        .iter()
        .rev()
        .filter(|event| kinds.is_empty() || kinds.contains(&event.kind))
        .collect()
}

impl RenderOnce for EventStream {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id;
        let hold = window.use_keyed_state((id.clone(), "hold"), cx, |_, _| Hold::default());
        let (kinds, held) = {
            let hold = hold.read(cx);
            (hold.kinds.clone(), hold.held)
        };
        let mut all: Vec<SharedString> = Vec::new();
        for event in &self.events {
            if !all.contains(&event.kind) {
                all.push(event.kind.clone());
            }
        }
        let fresh = held.map(|held| self.events.len().saturating_sub(held));
        let theme = cx.theme();
        let rows = shown(&self.events, &kinds, held).into_iter().map(|event| {
            div()
                .flex()
                .items_center()
                .gap_2()
                .py_1p5()
                .border_b_1()
                .border_color(theme.colors.border)
                .text_size(theme.text_size(TextSize::Sm))
                .child(
                    div()
                        .flex_none()
                        .child(Badge::new(event.kind.clone()).tone(event.tone)),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .child(Ellipsis::new(event.text.clone())),
                )
                .child(
                    div()
                        .flex_none()
                        .text_size(theme.text_size(TextSize::Xs))
                        .text_color(theme.colors.fg_subtle)
                        .child(format::relative(event.at, self.now)),
                )
        });
        let count = self.events.len();
        let (picked, paused) = (hold.clone(), hold);
        let toggle = move |_: &gpui::ClickEvent, _: &mut Window, cx: &mut App| {
            paused.update(cx, |hold, cx| {
                hold.held = match hold.held {
                    Some(_) => None,
                    None => Some(count),
                };
                log::info!(
                    "event stream: {}",
                    if hold.held.is_some() {
                        "paused"
                    } else {
                        "running"
                    }
                );
                cx.notify();
            })
        };
        let label = match fresh {
            Some(0) => "Resume".to_string(),
            Some(new) => format!("{new} new · Resume"),
            None => "Pause".to_string(),
        };
        let state = match fresh {
            Some(new) => format!("event-stream-held-{new}"),
            None => "event-stream-live".to_string(),
        };
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .child(
                        ChoiceChips::new(
                            (id.clone(), "kinds"),
                            all.iter()
                                .map(|kind| Choice::new(kind.clone(), kind.clone())),
                        )
                        .multiple()
                        .selected(kinds)
                        .on_change(move |chosen, _, cx| {
                            picked.update(cx, |hold, cx| {
                                hold.kinds = chosen.to_vec();
                                cx.notify();
                            })
                        }),
                    )
                    .child(
                        div().debug_selector(move || state).child(
                            Button::new((id, "pause"), label)
                                .variant(ButtonVariant::Secondary)
                                .on_click(toggle),
                        ),
                    ),
            )
            .child(div().flex().flex_col().children(rows))
    }
}

#[cfg(test)]
mod tests {
    use super::{StreamEvent, shown};
    use crate::data_display::Tone;

    #[test]
    fn a_hold_and_the_kinds_keep_what_shows_newest_first() {
        let at: jiff::Timestamp = "2026-09-27T12:00:00Z".parse().expect("a time");
        let event = |key: &str, kind: &str| StreamEvent {
            key: key.to_string().into(),
            kind: kind.to_string().into(),
            text: "x".into(),
            tone: Tone::Neutral,
            at,
        };
        let events = [
            event("1", "deploy"),
            event("2", "alert"),
            event("3", "deploy"),
            event("4", "deploy"),
        ];
        let keys = |shown: Vec<&StreamEvent>| {
            shown
                .iter()
                .map(|event| event.key.to_string())
                .collect::<Vec<_>>()
        };
        assert_eq!(keys(shown(&events, &[], None)), ["4", "3", "2", "1"]);
        assert_eq!(
            keys(shown(&events, &["deploy".into()], Some(3))),
            ["3", "1"]
        );
    }
}
