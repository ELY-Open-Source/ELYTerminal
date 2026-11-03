use std::rc::Rc;

use gpui::{
    App, ElementId, IntoElement, MouseButton, ParentElement, RenderOnce, SharedString, Styled,
    Window, div, prelude::*,
};
use web_time::Instant;

use crate::{
    buttons::{Button, ButtonVariant},
    data_display::{Avatar, AvatarGroup},
    feedback::Timer,
    forms::{OnFlag, Run},
    media::device_toggle,
    motion::Pulse,
    primitives::{FocusRing, Icon, IconName},
    theme::{ActiveTheme, AvatarSize, ControlSize, IconSize, Radius, TextSize},
    typography::Ellipsis,
};

fn leave(id: &ElementId, owner: &'static str, run: Run) -> Button {
    Button::new((id.clone(), "leave"), "Leave")
        .icon(IconName::PhoneOff)
        .variant(ButtonVariant::Danger)
        .size(ControlSize::Sm)
        .on_click(move |_, window, cx| {
            log::info!("{owner}: leave");
            run(window, cx)
        })
}

/// A call's controls in one pill: the microphone, the camera and a shared screen as toggles, then Leave, each only with its handler.
#[derive(IntoElement)]
pub struct CallControls {
    id: ElementId,
    mic: Option<(bool, OnFlag)>,
    camera: Option<(bool, OnFlag)>,
    share: Option<(bool, OnFlag)>,
    on_leave: Option<Run>,
}

impl CallControls {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            mic: None,
            camera: None,
            share: None,
            on_leave: None,
        }
    }

    /// Whether the microphone is on, and what gets the change.
    pub fn mic(
        mut self,
        on: bool,
        handler: impl Fn(bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.mic = Some((on, Rc::new(handler)));
        self
    }

    /// Whether the camera is on, and what gets the change.
    pub fn camera(
        mut self,
        on: bool,
        handler: impl Fn(bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.camera = Some((on, Rc::new(handler)));
        self
    }

    /// Whether the screen is shared, and what gets the change.
    pub fn share(
        mut self,
        on: bool,
        handler: impl Fn(bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.share = Some((on, Rc::new(handler)));
        self
    }

    pub fn on_leave(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_leave = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for CallControls {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = &theme.colors;
        let id = self.id.clone();
        let toggle = |key, icons, tip, set: Option<(bool, OnFlag)>| {
            set.map(|set| device_toggle("call controls", &id, key, icons, tip, set))
        };
        let toggles = [
            toggle(
                "mic",
                (IconName::Mic, IconName::MicOff),
                "Microphone",
                self.mic,
            ),
            toggle(
                "camera",
                (IconName::Video, IconName::VideoOff),
                "Camera",
                self.camera,
            ),
            toggle(
                "share",
                (IconName::ScreenShare, IconName::ScreenShareOff),
                "Share screen",
                self.share,
            ),
        ];
        let some = toggles.iter().any(Option::is_some);
        let leave = self.on_leave.map(|run| leave(&id, "call controls", run));
        div()
            .flex_none()
            .flex()
            .items_center()
            .gap_1()
            .p_1()
            .rounded_full()
            .border_1()
            .border_color(colors.border)
            .bg(colors.surface)
            .children(toggles.into_iter().flatten())
            .when(some && leave.is_some(), |pill| {
                pill.child(
                    div()
                        .w_px()
                        .h(theme.icon_size(IconSize::Md))
                        .bg(colors.border),
                )
            })
            .children(leave)
    }
}

/// A call under way in one bar: a live dot, where it is, how long it has run, who is in it, the microphone and Leave. The bar wraps by the header rule.
#[derive(IntoElement)]
pub struct VoiceCallBar {
    id: ElementId,
    place: SharedString,
    since: Instant,
    people: Vec<Avatar>,
    mic: Option<(bool, OnFlag)>,
    on_leave: Option<Run>,
}

impl VoiceCallBar {
    pub fn new(id: impl Into<ElementId>, place: impl Into<SharedString>, since: Instant) -> Self {
        Self {
            id: id.into(),
            place: place.into(),
            since,
            people: Vec::new(),
            mic: None,
            on_leave: None,
        }
    }

    /// Who is in the call; the first three show.
    pub fn people(mut self, people: impl IntoIterator<Item = Avatar>) -> Self {
        self.people = people.into_iter().collect();
        self
    }

    /// Whether the microphone is on, and what gets the change.
    pub fn mic(
        mut self,
        on: bool,
        handler: impl Fn(bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.mic = Some((on, Rc::new(handler)));
        self
    }

    pub fn on_leave(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_leave = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for VoiceCallBar {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = &theme.colors;
        let id = self.id.clone();
        let dot = div()
            .size(theme.status_dot())
            .rounded_full()
            .bg(colors.success);
        let mic = self.mic.map(|set| {
            device_toggle(
                "voice call",
                &id,
                "mic",
                (IconName::Mic, IconName::MicOff),
                "Microphone",
                set,
            )
        });
        let people = (!self.people.is_empty())
            .then(|| AvatarGroup::new(self.people).max(3).size(AvatarSize::Xs));
        div()
            .debug_selector(|| "voice-call-bar".into())
            .flex()
            .flex_wrap()
            .items_center()
            .justify_between()
            .gap_2()
            .px_3()
            .py_2()
            .rounded(theme.radius(Radius::Lg))
            .border_1()
            .border_color(colors.border)
            .bg(colors.surface)
            .text_size(theme.text_size(TextSize::Sm))
            .child(
                div()
                    .flex_1()
                    .min_w(theme.label_width())
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .flex_none()
                            .child(Pulse::new((id.clone(), "live")).child(dot)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_color(colors.fg)
                            .child(Ellipsis::new(self.place)),
                    )
                    .child(
                        div()
                            .flex_none()
                            .text_color(colors.fg_muted)
                            .child(Timer::new((id.clone(), "time"), self.since).size(TextSize::Sm)),
                    ),
            )
            .child(
                div()
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap_2()
                    .children(people)
                    .children(mic)
                    .children(self.on_leave.map(|run| leave(&id, "voice call", run))),
            )
    }
}

/// A huddle going on in a chat: headphones, how many are in, and their faces. With a handler a press joins it; its mark rings while live.
#[derive(IntoElement)]
pub struct HuddleIndicator {
    id: ElementId,
    people: Vec<Avatar>,
    on_join: Option<Run>,
}

impl HuddleIndicator {
    pub fn new(id: impl Into<ElementId>, people: impl IntoIterator<Item = Avatar>) -> Self {
        let people: Vec<Avatar> = people.into_iter().collect();
        assert!(!people.is_empty(), "a huddle holds someone");
        Self {
            id: id.into(),
            people,
            on_join: None,
        }
    }

    pub fn on_join(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_join = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for HuddleIndicator {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = &theme.colors;
        let count = self.people.len();
        let (id, ring) = (self.id.clone(), (self.id.clone(), "ring"));
        div()
            .id(self.id)
            .debug_selector(|| "huddle".into())
            .flex_none()
            .flex()
            .items_center()
            .gap_1p5()
            .pl_2()
            .pr_1()
            .py_0p5()
            .rounded_full()
            .border_1()
            .border_color(colors.success.alpha(0.4))
            .bg(colors.success_subtle)
            .text_size(theme.text_size(TextSize::Sm))
            .text_color(colors.success)
            .when_some(self.on_join, |chip, join| {
                chip.cursor_pointer()
                    .tab_index(0)
                    .focus_ring(cx)
                    .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
                    .on_click(move |_, window, cx| {
                        log::info!("huddle {id}: join");
                        join(window, cx)
                    })
            })
            .child(
                Pulse::new(ring).child(
                    Icon::new(IconName::Headphones)
                        .size(IconSize::Sm)
                        .color(colors.success),
                ),
            )
            .child(count.to_string())
            .child(AvatarGroup::new(self.people).max(3).size(AvatarSize::Xs))
    }
}
