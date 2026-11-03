use std::{cell::Cell, rc::Rc};

use gpui::{
    App, ElementId, ImageSource, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    SharedString, Styled, Window, div,
};

use crate::{
    buttons::{Button, ButtonVariant},
    data_display::Avatar,
    forms::Run,
    motion::Pulse,
    overlays::Dialog,
    primitives::IconName,
    theme::AvatarSize,
};

/// A call coming in: the caller's picture ringing, their name and the kind of call, with Decline and Accept; one of the two answers each ring. Escape declines, a press on the scrim does not. Render it while the call rings.
#[derive(IntoElement)]
pub struct IncomingCallDialog {
    id: ElementId,
    caller: SharedString,
    picture: Option<ImageSource>,
    video: bool,
    on_accept: Run,
    on_decline: Run,
}

impl IncomingCallDialog {
    pub fn new(
        id: impl Into<ElementId>,
        caller: impl Into<SharedString>,
        on_accept: impl Fn(&mut Window, &mut App) + 'static,
        on_decline: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            caller: caller.into(),
            picture: None,
            video: false,
            on_accept: Rc::new(on_accept),
            on_decline: Rc::new(on_decline),
        }
    }

    pub fn picture(mut self, source: impl Into<ImageSource>) -> Self {
        self.picture = Some(source.into());
        self
    }

    /// A video call; a voice call otherwise.
    pub fn video(mut self) -> Self {
        self.video = true;
        self
    }
}

impl RenderOnce for IncomingCallDialog {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let avatar =
            Avatar::new((self.id.clone(), "avatar"), self.caller.clone()).size(AvatarSize::Xl);
        let avatar = match self.picture {
            Some(picture) => avatar.image(picture),
            None => avatar,
        };
        let (kind, icon) = if self.video {
            ("Incoming video call", IconName::Video)
        } else {
            ("Incoming voice call", IconName::Phone)
        };
        let (id, caller) = (self.id.clone(), self.caller.clone());
        let (decline, accept) = (self.on_decline, self.on_accept);
        let told = caller.clone();
        let (decline_id, accept_id) = (id.clone(), id.clone());
        // Every way out runs this close; only Accept answers first.
        let answered = Rc::new(Cell::new(false));
        let answer = answered.clone();
        Dialog::new(self.id, self.caller, move |window, cx| {
            if !answered.get() {
                log::info!("incoming call {caller}: declined");
                decline(window, cx)
            }
        })
        .detail(kind)
        .held()
        .child(
            div()
                .debug_selector(|| "incoming-call".into())
                .flex()
                .justify_center()
                .py_4()
                .child(Pulse::new((id, "ring")).rounded_full().child(avatar)),
        )
        .action(move |close| {
            Button::new((decline_id, "decline"), "Decline")
                .icon(IconName::PhoneOff)
                .variant(ButtonVariant::Danger)
                .on_click(move |_, window, cx| close(window, cx))
        })
        .action(move |close| {
            Button::new((accept_id, "accept"), "Accept")
                .icon(icon)
                .variant(ButtonVariant::Success)
                .on_click(move |_, window, cx| {
                    log::info!("incoming call {told}: accepted");
                    answer.set(true);
                    close(window, cx);
                    accept(window, cx)
                })
        })
    }
}
