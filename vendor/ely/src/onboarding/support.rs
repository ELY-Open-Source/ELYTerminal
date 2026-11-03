use std::rc::Rc;

use gpui::{
    App, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce, SharedString,
    Styled, Window, div,
};

use crate::{
    buttons::{Button, ButtonVariant, CopyButton},
    forms::{Choice, FormField, Input, Run, Select, TextInput},
    primitives::{Icon, IconName},
    theme::{ActiveTheme, IconSize, TextSize},
    typography::{MiddleEllipsis, literal},
};

type OnMessage = Rc<dyn Fn(&SharedString, &str, &mut Window, &mut App)>;

/// A way to reach people: a topic and a message, and Send once both are given. It rests while the owner sends, and empties the message once sent, with focus back in it. With `address`, the address to write to instead shows under it, cut in the middle when narrow, with a Copy.
#[derive(IntoElement)]
pub struct ContactSupport {
    id: ElementId,
    topics: Vec<Choice>,
    busy: bool,
    address: Option<SharedString>,
    on_send: OnMessage,
}

impl ContactSupport {
    pub fn new(
        id: impl Into<ElementId>,
        topics: impl IntoIterator<Item = Choice>,
        on_send: impl Fn(&SharedString, &str, &mut Window, &mut App) + 'static,
    ) -> Self {
        let topics: Vec<Choice> = topics.into_iter().collect();
        assert!(!topics.is_empty(), "contact support needs a topic");
        Self {
            id: id.into(),
            topics,
            busy: false,
            address: None,
            on_send: Rc::new(on_send),
        }
    }

    /// While the owner sends: Send spins and takes no press, keeping its focus.
    pub fn busy(mut self, busy: bool) -> Self {
        self.busy = busy;
        self
    }

    /// The address to write to instead.
    pub fn address(mut self, address: impl Into<SharedString>) -> Self {
        self.address = Some(address.into());
        self
    }
}

impl RenderOnce for ContactSupport {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id;
        let on_send = self.on_send;
        let topic = window.use_keyed_state((id.clone(), "topic"), cx, |_, _| None::<SharedString>);
        let message = window.use_keyed_state((id.clone(), "message"), cx, |window, cx| {
            TextInput::new(window, cx)
                .multi_line(4, 8)
                .placeholder("What happened, and what you expected")
        });
        let chosen = topic.read(cx).clone().filter(|key| {
            let offered = self.topics.iter().any(|choice| choice.value == *key);
            if !offered {
                log::error!(
                    "contact support {id:?}: topic {key} is no longer offered; none chosen"
                );
            }
            offered
        });
        let words = message.read(cx).text().trim().to_string();
        let ready = chosen.is_some() && !words.is_empty();
        let theme = cx.theme();
        let (picked, emptied, field) = (topic, message.clone(), message.read(cx).focus().clone());
        let mut select = Select::new((id.clone(), "topic-select"), self.topics)
            .placeholder("Choose a topic")
            .on_change(move |key, _, cx| {
                let key = key.clone();
                picked.update(cx, |topic, cx| {
                    *topic = Some(key);
                    cx.notify();
                })
            });
        if let Some(key) = &chosen {
            select = select.selected(key.clone());
        }
        let send = move |window: &mut Window, cx: &mut App| {
            let key = chosen.clone().expect("Send rests until a topic is chosen");
            log::info!("contact support: send about {key}");
            on_send(&key, &words, window, cx);
            emptied.update(cx, |field, cx| field.set_text("", cx));
            window.focus(&field, cx);
        };
        div()
            .flex()
            .flex_col()
            .gap_4()
            .child(FormField::new((id.clone(), "topic-field"), "Topic").child(select))
            .child(
                FormField::new((id.clone(), "message-field"), "Message")
                    .child(Input::new(&message)),
            )
            .child(
                div().flex().child(
                    Button::new((id.clone(), "send"), "Send message")
                        .variant(ButtonVariant::Primary)
                        .loading(self.busy)
                        .disabled(!ready)
                        .on_click(move |_, window, cx| send(window, cx)),
                ),
            )
            .children(self.address.map(|address| {
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .text_size(theme.text_size(TextSize::Sm))
                    .text_color(theme.colors.fg_muted)
                    .child(div().flex_none().child("Or write to"))
                    .child(
                        literal(div())
                            .debug_selector(|| "support-address".into())
                            .flex_1()
                            .min_w_0()
                            .text_color(theme.colors.fg)
                            .child(MiddleEllipsis::new(address.clone())),
                    )
                    .child(CopyButton::new((id.clone(), "copy-address"), address))
            }))
    }
}

/// A quiet line of help beside what it explains; with `on_more`, Learn more follows it.
#[derive(IntoElement)]
pub struct InlineHelp {
    id: ElementId,
    text: SharedString,
    on_more: Option<Run>,
}

impl InlineHelp {
    pub fn new(id: impl Into<ElementId>, text: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
            on_more: None,
        }
    }

    /// Shows Learn more, which runs `handler`.
    pub fn on_more(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_more = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for InlineHelp {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let id = self.id;
        let more = self.on_more.map(|run| {
            div().flex().child(
                Button::new((id.clone(), "more"), "Learn more")
                    .variant(ButtonVariant::Link)
                    .on_click(move |_, window, cx| {
                        log::info!("inline help: learn more");
                        run(window, cx)
                    }),
            )
        });
        div()
            .flex()
            .items_start()
            .gap_2()
            .text_size(theme.text_size(TextSize::Sm))
            .text_color(theme.colors.fg_muted)
            .child(
                div().flex_none().pt_0p5().child(
                    Icon::new(IconName::Info)
                        .size(IconSize::Sm)
                        .color(theme.colors.fg_subtle),
                ),
            )
            .child(div().flex_1().min_w_0().child(self.text).children(more))
    }
}
