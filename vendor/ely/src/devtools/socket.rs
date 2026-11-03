use std::rc::Rc;

use gpui::{
    AnyElement, App, ElementId, Entity, IntoElement, ParentElement, RenderOnce, SharedString,
    Styled, Window, div, prelude::*,
};
use jiff::Timestamp;

use super::json_editor::reformat;
use crate::{
    buttons::{Button, ButtonVariant},
    chat::MessageList,
    data_display::{Badge, Tone},
    forms::{Input, TextInput},
    primitives::{Icon, IconName},
    theme::{ActiveTheme, IconSize, Radius, TextSize},
    typography::{Ellipsis, format},
};

/// Which way a message went: out, in, or a note about the connection itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Sent,
    Received,
    Note,
}

/// A message on a socket: which way it went, its text, and when.
#[derive(Clone, Debug, PartialEq)]
pub struct SocketMessage {
    pub direction: Direction,
    pub text: SharedString,
    pub at: Timestamp,
}

/// Where a socket stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SocketState {
    Closed,
    Opening,
    Open,
}

type OnText = Rc<dyn Fn(SharedString, &mut Window, &mut App)>;
type Run = Rc<dyn Fn(&mut Window, &mut App)>;

/// A WebSocket's conversation: where it stands with Connect or Disconnect beside it, its messages each marked sent or received with its time, JSON indented, newest last, and a field that sends on Enter while open.
#[derive(IntoElement)]
pub struct WebSocketConsole {
    id: ElementId,
    url: SharedString,
    state: SocketState,
    messages: Vec<SocketMessage>,
    draft: Entity<TextInput>,
    now: Timestamp,
    on_send: OnText,
    on_toggle: Option<Run>,
}

impl WebSocketConsole {
    /// `draft` is the message field's text, which the owner keeps; `now` dates each message.
    pub fn new(
        id: impl Into<ElementId>,
        url: impl Into<SharedString>,
        state: SocketState,
        messages: impl IntoIterator<Item = SocketMessage>,
        draft: &Entity<TextInput>,
        now: Timestamp,
        on_send: impl Fn(SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            url: url.into(),
            state,
            messages: messages.into_iter().collect(),
            draft: draft.clone(),
            now,
            on_send: Rc::new(on_send),
            on_toggle: None,
        }
    }

    /// Asked to connect while closed, or to disconnect while open.
    pub fn on_toggle(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_toggle = Some(Rc::new(handler));
        self
    }
}

/// One message in the log: which way it went, its text, JSON indented, and when; the time drops below in a narrow box.
fn message_row(message: &SocketMessage, now: Timestamp, ix: usize, cx: &App) -> AnyElement {
    let theme = cx.theme();
    let (icon, ink) = match message.direction {
        Direction::Sent => (IconName::ArrowUpRight, theme.colors.accent),
        Direction::Received => (IconName::ArrowDownLeft, theme.colors.success),
        Direction::Note => (IconName::Info, theme.colors.fg_subtle),
    };
    let text = reformat(&message.text, true).unwrap_or_else(|| message.text.to_string());
    div()
        .flex()
        .items_start()
        .gap_2()
        .py_1p5()
        .border_b_1()
        .border_color(theme.colors.border)
        .child(
            div()
                .flex_none()
                .pt_0p5()
                .child(Icon::new(icon).size(IconSize::Sm).color(ink)),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_wrap()
                .gap_x_3()
                .gap_y_0p5()
                .child(
                    div()
                        .flex_1()
                        .min_w(theme.label_width())
                        .font_family(theme.mono_family.clone())
                        .text_size(theme.text_size(TextSize::Sm))
                        .when(message.direction == Direction::Note, |line| {
                            line.text_color(theme.colors.fg_muted)
                        })
                        .child(text),
                )
                .child(
                    div()
                        .flex_none()
                        .text_size(theme.text_size(TextSize::Xs))
                        .text_color(theme.colors.fg_subtle)
                        .child(format::relative(message.at, now)),
                ),
        )
        .w_full()
        .debug_selector(move || format!("socket-message-{ix}"))
        .into_any_element()
}

impl RenderOnce for WebSocketConsole {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let (tone, words) = match self.state {
            SocketState::Closed => (Tone::Neutral, "Closed"),
            SocketState::Opening => (Tone::Info, "Opening"),
            SocketState::Open => (Tone::Success, "Open"),
        };
        let open = self.state == SocketState::Open;
        let (messages, now) = (Rc::new(self.messages), self.now);
        let on_send = self.on_send;
        let draft = self.draft.clone();
        let ready = open && !draft.read(cx).text().trim().is_empty();
        let send = Rc::new(move |window: &mut Window, cx: &mut App| {
            let text = draft.read(cx).text().trim().to_string();
            log::info!("websocket console: send {} bytes", text.len());
            draft.update(cx, |draft, cx| draft.set_text(String::new(), cx));
            on_send(text.into(), window, cx);
        });
        let (pressed, entered, on_toggle) = (send.clone(), send, self.on_toggle);
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .min_w_0()
                            .child(Badge::new(words).tone(tone).dot())
                            .child(
                                div()
                                    .min_w_0()
                                    .text_size(theme.text_size(TextSize::Sm))
                                    .child(Ellipsis::new(self.url.clone())),
                            ),
                    )
                    .children(on_toggle.map(|on_toggle| {
                        Button::new(
                            (self.id.clone(), "toggle"),
                            if self.state == SocketState::Closed {
                                "Connect"
                            } else {
                                "Disconnect"
                            },
                        )
                        .variant(ButtonVariant::Secondary)
                        .disabled(self.state == SocketState::Opening)
                        .on_click(move |_, window, cx| {
                            log::info!("websocket console: connect or disconnect");
                            on_toggle(window, cx);
                        })
                    })),
            )
            .child(
                div()
                    .debug_selector(|| "socket-log".into())
                    .h(theme.list_max_height())
                    .px_3()
                    .rounded(theme.radius(Radius::Md))
                    .border_1()
                    .border_color(theme.colors.border)
                    .child(MessageList::new(
                        (self.id.clone(), "log"),
                        messages.len(),
                        move |ix, _, cx| message_row(&messages[ix], now, ix, cx),
                    )),
            )
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .capture_action(move |_: &crate::forms::Enter, window, cx| {
                                if ready {
                                    cx.stop_propagation();
                                    entered(window, cx);
                                }
                            })
                            .child(Input::new(&self.draft)),
                    )
                    .child(
                        Button::new((self.id, "send"), "Send")
                            .variant(ButtonVariant::Primary)
                            .disabled(!ready)
                            .on_click(move |_, window, cx| pressed(window, cx)),
                    ),
            )
    }
}
