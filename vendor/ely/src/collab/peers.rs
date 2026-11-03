use std::{rc::Rc, time::Duration};

use gpui::{
    Animation, AnimationExt, AnyElement, App, ElementId, FontWeight, Hsla, InteractiveElement,
    IntoElement, MouseButton, ParentElement, RenderOnce, SharedString, StatefulInteractiveElement,
    Styled, Window, div, prelude::*, pulsating_between, transparent_black,
};
use smallvec::SmallVec;

use crate::{
    buttons::{Button, ButtonVariant},
    data_display::{Avatar, more, tucked},
    documents::source,
    forms::{OnValue, Run},
    overlays::HoverCard,
    primitives::{FocusRing, hand_back, hold_focus},
    theme::{ActiveTheme, AvatarSize, ControlSize, Radius, TextSize},
    typography::tabular,
};

/// One breath of the live dot.
const BREATH: Duration = Duration::from_millis(2000);

/// How strongly a peer's color tints the bar over a followed view.
const TINT: f32 = 0.12;

/// Someone in the document with you: a key, a name, a picture, where they are, and which of the theme's chart colors marks their cursor and selection.
#[derive(Clone, Debug, PartialEq)]
pub struct Peer {
    pub key: SharedString,
    pub name: SharedString,
    pub picture: Option<SharedString>,
    pub place: Option<SharedString>,
    pub hue: usize,
}

impl Peer {
    pub fn new(key: impl Into<SharedString>, name: impl Into<SharedString>, hue: usize) -> Self {
        Self {
            key: key.into(),
            name: name.into(),
            picture: None,
            place: None,
            hue,
        }
    }

    /// Their color, from the theme's chart colors.
    pub fn color(&self, cx: &App) -> Hsla {
        let chart = &cx.theme().colors.chart;
        chart[self.hue % chart.len()]
    }

    pub(crate) fn avatar(&self, id: impl Into<ElementId>, size: AvatarSize) -> Avatar {
        let avatar = Avatar::new(id, self.name.clone()).size(size);
        match &self.picture {
            Some(picture) => avatar.image(source(picture)),
            None => avatar,
        }
    }
}

/// Who else is here, their avatars overlapping, each ringed in their color: resting on one says where they are; a press follows them. Past `max`, the rest show as a count.
#[derive(IntoElement)]
pub struct PresenceAvatars {
    id: ElementId,
    peers: Vec<Peer>,
    max: usize,
    following: Option<SharedString>,
    on_follow: Option<OnValue>,
}

impl PresenceAvatars {
    pub fn new(id: impl Into<ElementId>, peers: impl IntoIterator<Item = Peer>) -> Self {
        Self {
            id: id.into(),
            peers: peers.into_iter().collect(),
            max: 4,
            following: None,
            on_follow: None,
        }
    }

    /// How many show before the rest become a count.
    pub fn max(mut self, max: usize) -> Self {
        assert!(max > 0, "presence shows at least one avatar");
        self.max = max;
        self
    }

    /// Whom the view follows now, by key; a press on someone asks to follow them.
    pub fn follow(
        mut self,
        following: Option<SharedString>,
        on_follow: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.following = following;
        self.on_follow = Some(Rc::new(on_follow));
        self
    }
}

impl RenderOnce for PresenceAvatars {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        if let Some(following) = &self.following
            && !self.peers.iter().any(|peer| &peer.key == following)
        {
            log::error!("presence: {following} left; following none");
        }
        let size = AvatarSize::Sm;
        let rest = self.peers.len().saturating_sub(self.max);
        let shown: Vec<AnyElement> = self
            .peers
            .iter()
            .take(self.max)
            .enumerate()
            .map(|(ix, peer)| {
                let color = peer.color(cx);
                let following = self.following.as_ref() == Some(&peer.key);
                let (key, name, place) = (peer.key.clone(), peer.name.clone(), peer.place.clone());
                let follow = self.on_follow.clone();
                let face = tucked(ix == 0, size)
                    .id((self.id.clone(), format!("peer-{key}")))
                    .rounded_full()
                    .border_1()
                    .border_color(transparent_black())
                    .tab_index(0)
                    .focus_ring(cx)
                    .cursor_pointer()
                    .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
                    .when_some(follow, |face, follow| {
                        let key = key.clone();
                        face.on_click(move |_, window, cx| {
                            log::info!("presence: follow {key}");
                            follow(&key, window, cx)
                        })
                    })
                    .child(
                        peer.avatar((self.id.clone(), format!("avatar-{key}")), size)
                            .ring_color(color),
                    );
                HoverCard::new(
                    (self.id.clone(), format!("card-{key}")),
                    face,
                    move |_, cx| {
                        let colors = &cx.theme().colors;
                        div()
                            .flex()
                            .flex_col()
                            .gap_0p5()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_1p5()
                                    .child(
                                        div()
                                            .size(cx.theme().status_dot())
                                            .rounded_full()
                                            .bg(color),
                                    )
                                    .child(
                                        div().font_weight(FontWeight::MEDIUM).child(name.clone()),
                                    ),
                            )
                            .children(
                                place
                                    .clone()
                                    .map(|place| div().text_color(colors.fg_muted).child(place)),
                            )
                            .child(div().text_color(colors.fg_subtle).child(if following {
                                "Following · press to stop"
                            } else {
                                "Press to follow"
                            }))
                    },
                )
                .into_any_element()
            })
            .collect();
        div()
            .flex()
            .items_center()
            .children(shown)
            .when(rest > 0, |row| {
                row.child(tucked(false, size).child(more(rest, size, cx)))
            })
    }
}

/// Whether a document is live: a dot that breathes while others are here, and how many are.
#[derive(IntoElement)]
pub struct LiveIndicator {
    id: ElementId,
    others: usize,
}

impl LiveIndicator {
    /// `others` counts the people here besides you.
    pub fn new(id: impl Into<ElementId>, others: usize) -> Self {
        Self {
            id: id.into(),
            others,
        }
    }
}

impl RenderOnce for LiveIndicator {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let live = self.others > 0;
        let dot = div().size(theme.status_dot()).rounded_full().bg(if live {
            colors.success
        } else {
            colors.fg_subtle
        });
        let dot: AnyElement = if live && !theme.reduced_motion {
            dot.with_animation(
                (self.id.clone(), "breath"),
                Animation::new(BREATH)
                    .repeat()
                    .with_easing(pulsating_between(0.35, 1.0)),
                |dot, t| dot.opacity(t),
            )
            .into_any_element()
        } else {
            dot.into_any_element()
        };
        div()
            .id(self.id)
            .flex()
            .flex_none()
            .items_center()
            .gap_1p5()
            .px_2()
            .py_0p5()
            .rounded_full()
            .border_1()
            .border_color(colors.border)
            .text_size(theme.text_size(TextSize::Xs))
            .text_color(if live { colors.fg } else { colors.fg_muted })
            .child(dot)
            .child(tabular(div()).child(if live {
                format!("Live · {} here", self.others + 1)
            } else {
                "Only you".to_string()
            }))
    }
}

/// Following someone's view: a frame in their color around what they see, a bar that names them, and a way to stop; Escape stops too.
#[derive(IntoElement)]
pub struct FollowMode {
    id: ElementId,
    peer: Option<Peer>,
    on_stop: Run,
    body: SmallVec<[AnyElement; 2]>,
}

impl FollowMode {
    /// `peer` is whom the view follows, when anyone.
    pub fn new(
        id: impl Into<ElementId>,
        peer: Option<Peer>,
        on_stop: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            peer,
            on_stop: Rc::new(on_stop),
            body: SmallVec::new(),
        }
    }
}

impl ParentElement for FollowMode {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.body.extend(elements);
    }
}

impl RenderOnce for FollowMode {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let following = self.peer.is_some();
        let held = hold_focus((self.id.clone(), "held"), following, window, cx);
        let focus = held.read(cx).focus.clone();
        let theme = cx.theme();
        let (stop, escape) = (self.on_stop.clone(), self.on_stop);
        let released = held.clone();
        let color = self.peer.as_ref().map(|peer| peer.color(cx));
        div()
            .id(self.id.clone())
            .track_focus(&focus)
            .flex()
            .flex_col()
            .rounded(theme.radius(Radius::Lg))
            .border_2()
            .border_color(color.unwrap_or(transparent_black()))
            .on_key_down(move |event, window, cx| {
                if following && event.keystroke.key == "escape" {
                    cx.stop_propagation();
                    log::info!("follow mode: stopped");
                    hand_back(&held, window, cx);
                    escape(window, cx);
                }
            })
            .children(self.peer.zip(color).map(|(peer, color)| {
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .pl_3()
                    .pr_1()
                    .py_1()
                    .rounded_t(theme.radius(Radius::Md))
                    .bg(color.opacity(TINT))
                    .text_size(theme.text_size(TextSize::Sm))
                    .child(div().size(theme.status_dot()).rounded_full().bg(color))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .child(format!("Following {}", peer.name)),
                    )
                    .child(
                        Button::new((self.id.clone(), "stop"), "Stop following")
                            .variant(ButtonVariant::Ghost)
                            .size(ControlSize::Sm)
                            .shortcut("escape")
                            .on_click(move |_, window, cx| {
                                log::info!("follow mode: stopped");
                                hand_back(&released, window, cx);
                                stop(window, cx)
                            }),
                    )
            }))
            .children(self.body)
    }
}
