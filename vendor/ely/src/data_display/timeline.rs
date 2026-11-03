use gpui::{
    AnyElement, App, IntoElement, ParentElement, Rems, RenderOnce, Styled, Window, div, prelude::*,
};
use smallvec::SmallVec;

use super::Tone;
use crate::{
    primitives::{Icon, IconName},
    theme::{ActiveTheme, ControlSize, IconSize, TextSize},
};

/// What marks an event on the rail.
enum Marker {
    Dot,
    Icon(IconName),
    Element(AnyElement),
}

/// One event on a `Timeline`: its title, when it happened, and any detail below.
#[derive(IntoElement)]
pub struct TimelineItem {
    title: AnyElement,
    time: Option<AnyElement>,
    tone: Tone,
    marker: Marker,
    tail: bool,
    body: SmallVec<[AnyElement; 2]>,
}

impl TimelineItem {
    pub fn new(title: impl IntoElement) -> Self {
        Self {
            title: title.into_any_element(),
            time: None,
            tone: Tone::Neutral,
            marker: Marker::Dot,
            tail: false,
            body: SmallVec::new(),
        }
    }

    /// When it happened: a date, or a `RelativeTime`.
    pub fn time(mut self, time: impl IntoElement) -> Self {
        self.time = Some(time.into_any_element());
        self
    }

    /// Tints the dot or the icon's ring.
    pub fn tone(mut self, tone: impl Into<Tone>) -> Self {
        self.tone = tone.into();
        self
    }

    /// An icon in a ring in place of the dot.
    pub fn icon(mut self, icon: IconName) -> Self {
        self.marker = Marker::Icon(icon);
        self
    }

    /// An element the size of a small avatar in place of the dot.
    pub fn marker(mut self, marker: impl IntoElement) -> Self {
        self.marker = Marker::Element(marker.into_any_element());
        self
    }

    /// The rail runs on below this event, to the next.
    pub(super) fn tail(mut self, tail: bool) -> Self {
        self.tail = tail;
        self
    }
}

impl ParentElement for TimelineItem {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.body.extend(elements);
    }
}

impl RenderOnce for TimelineItem {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = &theme.colors;
        let slot = theme.control_height(ControlSize::Sm);
        let dot = theme.status_dot() * 1.34;
        let (marker, inset) = match self.marker {
            Marker::Dot => (
                div()
                    .size(dot)
                    .rounded_full()
                    .bg(self.tone.dot(colors))
                    .into_any_element(),
                (slot - dot) / 2.0,
            ),
            Marker::Icon(icon) => {
                let (fill, ink) = self.tone.colors(colors);
                let ring = div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .size(slot)
                    .rounded_full()
                    .bg(fill)
                    .child(Icon::new(icon).size(IconSize::Xs).color(ink));
                (ring.into_any_element(), Rems::default())
            }
            Marker::Element(element) => (element, Rems::default()),
        };
        let reach = dot / 2.0 - inset;
        div()
            .flex()
            .gap_3()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .flex_none()
                    .w(slot)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_center()
                            .size(slot)
                            .child(marker),
                    )
                    .when(self.tail, |rail| {
                        rail.child(
                            div()
                                .flex_1()
                                .mt(reach)
                                .mb(reach)
                                .border_l_1()
                                .border_color(colors.border),
                        )
                    }),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .when(self.tail, |content| content.pb_6())
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            .min_h(slot)
                            .text_size(theme.text_size(TextSize::Base))
                            .text_color(colors.fg)
                            .child(div().flex_1().min_w_0().child(self.title))
                            .when_some(self.time, |head, time| {
                                head.child(
                                    div()
                                        .flex_none()
                                        .text_size(theme.text_size(TextSize::Sm))
                                        .text_color(colors.fg_subtle)
                                        .child(time),
                                )
                            }),
                    )
                    .when(!self.body.is_empty(), |content| {
                        content.child(
                            div()
                                .mt_1()
                                .text_size(theme.text_size(TextSize::Sm))
                                .text_color(colors.fg_muted)
                                .children(self.body),
                        )
                    }),
            )
    }
}

/// Events down a rail in the order given, each marked by a dot, an icon or an avatar.
#[derive(IntoElement, Default)]
pub struct Timeline {
    items: Vec<TimelineItem>,
}

impl Timeline {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn item(mut self, item: TimelineItem) -> Self {
        self.items.push(item);
        self
    }
}

impl RenderOnce for Timeline {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let last = self.items.len().saturating_sub(1);
        div().flex().flex_col().children(
            self.items
                .into_iter()
                .enumerate()
                .map(|(ix, item)| item.tail(ix < last)),
        )
    }
}
