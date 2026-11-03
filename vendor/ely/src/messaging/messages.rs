use gpui::{
    AnyElement, App, ElementId, FontWeight, ImageSource, IntoElement, ParentElement, RenderOnce,
    SharedString, Styled, Window, div, prelude::*, relative, rems,
};
use jiff::{SignedDuration, Timestamp, tz::TimeZone};
use smallvec::SmallVec;

use crate::{
    data_display::Avatar,
    theme::{ActiveTheme, AvatarSize, Radius, TextSize},
    typography::{Ellipsis, LEADING, format},
};

/// How long after one message the next by the same author joins its run.
const RUN: SignedDuration = SignedDuration::from_mins(5);

/// Whether `next` joins the run of `before`: the same author, the same day in `zone`, within five minutes.
fn continues(before: (&str, Timestamp), next: (&str, Timestamp), zone: &TimeZone) -> bool {
    let gap = next.1.duration_since(before.1);
    let day = |at: Timestamp| at.to_zoned(zone.clone()).date();
    before.0 == next.0 && gap >= SignedDuration::ZERO && gap <= RUN && day(before.1) == day(next.1)
}

/// A message in a chat: the author's picture, name and time over the words. After a message by the same author within five minutes, only the words, with the time in the gutter on hover. Reactions and a thread go in the footer.
#[derive(IntoElement)]
pub struct ChatMessage {
    id: ElementId,
    author: SharedString,
    at: Timestamp,
    picture: Option<ImageSource>,
    after: Option<(SharedString, Timestamp)>,
    zone: Option<TimeZone>,
    body: SmallVec<[AnyElement; 2]>,
    footer: Option<AnyElement>,
}

impl ChatMessage {
    pub fn new(id: impl Into<ElementId>, author: impl Into<SharedString>, at: Timestamp) -> Self {
        Self {
            id: id.into(),
            author: author.into(),
            at,
            picture: None,
            after: None,
            zone: None,
            body: SmallVec::new(),
            footer: None,
        }
    }

    pub fn picture(mut self, source: impl Into<ImageSource>) -> Self {
        self.picture = Some(source.into());
        self
    }

    /// The message before this one, by its author and time.
    pub fn after(mut self, author: impl Into<SharedString>, at: Timestamp) -> Self {
        self.after = Some((author.into(), at));
        self
    }

    /// The zone its time and day read in; the system's otherwise.
    pub fn zone(mut self, zone: TimeZone) -> Self {
        self.zone = Some(zone);
        self
    }

    pub fn footer(mut self, footer: impl IntoElement) -> Self {
        self.footer = Some(footer.into_any_element());
        self
    }
}

impl ParentElement for ChatMessage {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.body.extend(elements);
    }
}

impl RenderOnce for ChatMessage {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = &theme.colors;
        let zone = self
            .zone
            .unwrap_or_else(|| format::system_zone("chat message"));
        let joined = self.after.as_ref().is_some_and(|(author, then)| {
            continues((author, *then), (&self.author, self.at), &zone)
        });
        let time = format::datetime(self.at, &zone, "%H:%M").expect("a fixed pattern formats");
        let group = SharedString::from(format!("chat-message-{}", self.id));
        let line = rems(theme.text_size(TextSize::Base).0 * LEADING);
        let gutter = div().flex_none().w(theme.avatar_size(AvatarSize::Md));
        let gutter = if joined {
            let named = self.id.clone();
            gutter
                .h(line)
                .flex()
                .items_center()
                .justify_center()
                .text_size(theme.text_size(TextSize::Xs))
                .text_color(colors.fg_subtle)
                .invisible()
                .group_hover(group.clone(), |style| style.visible())
                .child(
                    div()
                        .debug_selector(move || format!("run-time {named}"))
                        .child(time.clone()),
                )
        } else {
            let avatar =
                Avatar::new((self.id.clone(), "avatar"), self.author.clone()).size(AvatarSize::Md);
            gutter.child(match self.picture {
                Some(picture) => avatar.image(picture),
                None => avatar,
            })
        };
        let named = self.id.clone();
        let head = (!joined).then(|| {
            div()
                .debug_selector(move || format!("message-head {named}"))
                .flex()
                .items_baseline()
                .gap_2()
                .child(
                    div()
                        .min_w_0()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(colors.fg)
                        .child(Ellipsis::new(self.author.clone())),
                )
                .child(
                    div()
                        .flex_none()
                        .text_size(theme.text_size(TextSize::Xs))
                        .text_color(colors.fg_subtle)
                        .child(time),
                )
        });
        div()
            .id(self.id.clone())
            .debug_selector(move || format!("message {}", self.id))
            .group(group)
            .flex()
            .gap_3()
            .px_3()
            .map(|row| {
                if joined {
                    row.py_0p5()
                } else {
                    row.pt_2().pb_0p5()
                }
            })
            .rounded(theme.radius(Radius::Md))
            .hover(|style| style.bg(colors.hover))
            .child(gutter)
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .text_size(theme.text_size(TextSize::Base))
                    .line_height(relative(LEADING))
                    .children(head)
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .text_color(colors.fg)
                            .children(self.body),
                    )
                    .children(self.footer.map(|footer| div().pt_1().child(footer))),
            )
    }
}

/// Where unread messages start: a rule and "New" in the danger tone.
#[derive(IntoElement, Default)]
pub struct UnreadDivider;

impl UnreadDivider {
    pub fn new() -> Self {
        Self
    }
}

impl RenderOnce for UnreadDivider {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let danger = theme.colors.danger;
        div()
            .flex()
            .items_center()
            .gap_2()
            .py_1()
            .child(div().flex_1().border_t_1().border_color(danger))
            .child(
                div()
                    .flex_none()
                    .text_size(theme.text_size(TextSize::Xs))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(danger)
                    .child("New"),
            )
    }
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;

    use super::*;

    fn at(hour: i8, minute: i8, second: i8) -> Timestamp {
        date(2026, 9, 26)
            .at(hour, minute, second, 0)
            .to_zoned(TimeZone::UTC)
            .expect("a UTC time")
            .timestamp()
    }

    #[test]
    fn a_run_holds_one_author_for_five_minutes_of_a_day() {
        let joins = |before, next| continues(before, next, &TimeZone::UTC);
        assert!(joins(("Ana", at(9, 0, 0)), ("Ana", at(9, 4, 59))));
        assert!(joins(("Ana", at(9, 0, 0)), ("Ana", at(9, 5, 0))));
        assert!(!joins(("Ana", at(9, 0, 0)), ("Ana", at(9, 5, 1))));
        assert!(!joins(("Ana", at(9, 0, 0)), ("Ben", at(9, 1, 0))));
        assert!(
            !joins(("Ana", at(9, 1, 0)), ("Ana", at(9, 0, 0))),
            "an earlier message starts its own run"
        );
        let midnight = at(0, 1, 0) + SignedDuration::from_hours(24);
        assert!(
            !joins(("Ana", at(23, 58, 0)), ("Ana", midnight)),
            "a new day starts a new run"
        );
    }
}
