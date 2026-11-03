use gpui::{
    App, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce, SharedString,
    Styled, Window, div,
};

use crate::{
    data_display::Avatar,
    lists::ListItem,
    primitives::{Icon, IconName},
    theme::{ActiveTheme, AvatarSize, IconSize, TextSize},
};

/// What someone said to an invitation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Answer {
    Going,
    Maybe,
    Declined,
    Waiting,
}

impl Answer {
    fn label(self) -> &'static str {
        match self {
            Self::Going => "Going",
            Self::Maybe => "Maybe",
            Self::Declined => "Declined",
            Self::Waiting => "Waiting",
        }
    }

    fn icon(self) -> IconName {
        match self {
            Self::Going => IconName::CircleCheck,
            Self::Maybe => IconName::CircleHelp,
            Self::Declined => IconName::CircleX,
            Self::Waiting => IconName::Clock,
        }
    }
}

/// Someone asked to an event: their name, their address, their answer, and whether they asked the rest.
#[derive(Clone, Debug, PartialEq)]
pub struct Attendee {
    pub name: SharedString,
    pub email: SharedString,
    pub answer: Answer,
    pub organizer: bool,
}

impl Attendee {
    pub fn new(
        name: impl Into<SharedString>,
        email: impl Into<SharedString>,
        answer: Answer,
    ) -> Self {
        Self {
            name: name.into(),
            email: email.into(),
            answer,
            organizer: false,
        }
    }

    /// Marks them as the one who asked.
    pub fn organizer(mut self) -> Self {
        self.organizer = true;
        self
    }
}

/// "5 guests · 2 going · 1 maybe": each answer given, in order, with none left out but the zeros.
pub(crate) fn tally(attendees: &[Attendee]) -> String {
    let guests = match attendees.len() {
        1 => "1 guest".to_string(),
        count => format!("{count} guests"),
    };
    [
        Answer::Going,
        Answer::Maybe,
        Answer::Declined,
        Answer::Waiting,
    ]
    .into_iter()
    .filter_map(|answer| {
        let count = attendees
            .iter()
            .filter(|each| each.answer == answer)
            .count();
        (count > 0).then(|| format!("{count} {}", answer.label().to_lowercase()))
    })
    .fold(guests, |line, part| format!("{line} · {part}"))
}

/// Who is asked to an event: a line that counts the answers, then each person with their avatar, name, address and answer, the one who asked first.
#[derive(IntoElement)]
pub struct AttendeeList {
    id: ElementId,
    attendees: Vec<Attendee>,
}

impl AttendeeList {
    pub fn new(id: impl Into<ElementId>, attendees: impl IntoIterator<Item = Attendee>) -> Self {
        let attendees: Vec<Attendee> = attendees.into_iter().collect();
        for (ix, attendee) in attendees.iter().enumerate() {
            let twin = attendees[..ix]
                .iter()
                .any(|other| other.email == attendee.email);
            assert!(!twin, "attendee {} twice", attendee.email);
        }
        let organizers = attendees
            .iter()
            .filter(|attendee| attendee.organizer)
            .count();
        assert!(
            organizers <= 1,
            "an event has one organizer, not {organizers}"
        );
        Self {
            id: id.into(),
            attendees,
        }
    }
}

impl RenderOnce for AttendeeList {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let mut shown = self.attendees;
        shown.sort_by_key(|attendee| !attendee.organizer);
        let theme = cx.theme();
        let colors = &theme.colors;
        let rows: Vec<_> = shown
            .iter()
            .map(|attendee| {
                let tone = match attendee.answer {
                    Answer::Going => colors.success,
                    Answer::Maybe => colors.warning,
                    Answer::Declined | Answer::Waiting => colors.fg_subtle,
                };
                let about = match attendee.organizer {
                    true => format!("Organizer · {}", attendee.email),
                    false => attendee.email.to_string(),
                };
                let email = attendee.email.clone();
                let row = ListItem::new(
                    (self.id.clone(), format!("guest-{}", attendee.email)),
                    attendee.name.clone(),
                )
                .leading(
                    Avatar::new(
                        (self.id.clone(), format!("avatar-{}", attendee.email)),
                        attendee.name.clone(),
                    )
                    .size(AvatarSize::Sm),
                )
                .description(about)
                .trailing(
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .text_color(tone)
                        .child(
                            Icon::new(attendee.answer.icon())
                                .size(IconSize::Sm)
                                .color(tone),
                        )
                        .child(attendee.answer.label()),
                );
                div()
                    .debug_selector(move || format!("guest {email}"))
                    .child(row)
            })
            .collect();
        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .px_3()
                    .text_size(theme.text_size(TextSize::Sm))
                    .text_color(colors.fg_muted)
                    .child(tally(&shown)),
            )
            .children(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::{Answer, Attendee, AttendeeList, tally};

    #[test]
    fn a_tally_counts_each_answer_given() {
        let guests = [
            Attendee::new("Ana", "ana@atrium.studio", Answer::Going).organizer(),
            Attendee::new("Ben", "ben@atrium.studio", Answer::Going),
            Attendee::new("Dev", "dev@atrium.studio", Answer::Waiting),
            Attendee::new("Eli", "eli@atrium.studio", Answer::Maybe),
        ];
        assert_eq!(tally(&guests), "4 guests · 2 going · 1 maybe · 1 waiting");
        assert_eq!(tally(&guests[..1]), "1 guest · 1 going");
    }

    #[test]
    #[should_panic(expected = "attendee ana@atrium.studio twice")]
    fn an_address_is_asked_once() {
        let _ = AttendeeList::new(
            "guests",
            [
                Attendee::new("Ana", "ana@atrium.studio", Answer::Going),
                Attendee::new("Ana Lima", "ana@atrium.studio", Answer::Maybe),
            ],
        );
    }
}
