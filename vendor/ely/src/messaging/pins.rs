use std::rc::Rc;

use gpui::{
    App, ElementId, FontWeight, InteractiveElement, IntoElement, MouseButton, ParentElement,
    RenderOnce, SharedString, Styled, Window, div, prelude::FluentBuilder,
};
use jiff::Timestamp;

use crate::{
    buttons::{ButtonVariant, IconButton},
    data_display::Avatar,
    forms::OnValue,
    lists::{ListItem, SelectableList},
    primitives::IconName,
    theme::{ActiveTheme, AvatarSize, ControlSize, TextSize},
    typography::format,
};

/// A pinned message: its key, who wrote it and when, and its words.
#[derive(Clone, Debug, PartialEq)]
pub struct PinnedMessage {
    pub key: SharedString,
    pub author: SharedString,
    pub at: Timestamp,
    pub text: SharedString,
}

/// Messages pinned in a chat, newest first: the words over who wrote them and when. A press, an arrow or Enter opens one where it stands; Unpin lets one go.
#[derive(IntoElement)]
pub struct PinnedMessages {
    id: ElementId,
    messages: Vec<PinnedMessage>,
    on_open: Option<OnValue>,
    on_unpin: Option<OnValue>,
}

impl PinnedMessages {
    pub fn new(
        id: impl Into<ElementId>,
        messages: impl IntoIterator<Item = PinnedMessage>,
    ) -> Self {
        let mut messages: Vec<PinnedMessage> = messages.into_iter().collect();
        for (ix, message) in messages.iter().enumerate() {
            assert!(
                !messages[..ix].iter().any(|other| other.key == message.key),
                "pin {} twice",
                message.key
            );
        }
        messages.sort_by_key(|message| std::cmp::Reverse(message.at));
        Self {
            id: id.into(),
            messages,
            on_open: None,
            on_unpin: None,
        }
    }

    /// Gets the key of the message to open.
    pub fn on_open(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_open = Some(Rc::new(handler));
        self
    }

    /// Gets the key of the message to unpin.
    pub fn on_unpin(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_unpin = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for PinnedMessages {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = &theme.colors;
        let now = Timestamp::now();
        let count = self.messages.len();
        let rows = self.messages.iter().fold(
            SelectableList::new((self.id.clone(), "pins")),
            |list, message| {
                let unpin = self.on_unpin.clone().map(|unpin| {
                    let key = message.key.clone();
                    let named = key.clone();
                    div()
                        .debug_selector(move || format!("unpin {named}"))
                        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                        .on_key_down(|event, _, cx| {
                            let key = event.keystroke.key.as_str();
                            if matches!(key, "space" | "enter")
                                && !event.keystroke.modifiers.modified()
                            {
                                cx.stop_propagation();
                            }
                        })
                        .child(
                            IconButton::new(
                                (self.id.clone(), format!("unpin-{key}")),
                                IconName::PinOff,
                            )
                            .variant(ButtonVariant::Ghost)
                            .size(ControlSize::Sm)
                            .tooltip("Unpin")
                            .on_click(move |_, window, cx| {
                                log::info!("pinned messages: unpin {key}");
                                unpin(&key, window, cx)
                            }),
                        )
                });
                let who = format!("{} · {}", message.author, format::relative(message.at, now));
                let row = ListItem::new(
                    (self.id.clone(), format!("pin-{}", message.key)),
                    message.text.clone(),
                )
                .leading(
                    Avatar::new(
                        (self.id.clone(), format!("author-{}", message.key)),
                        message.author.clone(),
                    )
                    .size(AvatarSize::Sm),
                )
                .description(who);
                let row = match unpin {
                    Some(unpin) => row.trailing(unpin),
                    None => row,
                };
                list.row(message.key.clone(), row)
            },
        );
        let open: OnValue = {
            let (id, on_open) = (self.id.clone(), self.on_open);
            Rc::new(move |key, window, cx| {
                log::info!("pinned messages {id:?}: open {key}");
                if let Some(on_open) = &on_open {
                    on_open(key, window, cx);
                }
            })
        };
        let picked = open.clone();
        let empty = (count == 0).then(|| {
            div()
                .px_3()
                .py_4()
                .text_size(theme.text_size(TextSize::Sm))
                .text_color(colors.fg_muted)
                .child("Nothing pinned yet.")
        });
        div()
            .debug_selector(|| "pinned-messages".into())
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .px_3()
                    .flex()
                    .items_baseline()
                    .gap_2()
                    .child(
                        div()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(colors.fg)
                            .child("Pinned"),
                    )
                    .child(
                        div()
                            .text_size(theme.text_size(TextSize::Sm))
                            .text_color(colors.fg_muted)
                            .child(count.to_string()),
                    ),
            )
            .children(empty)
            .when(count > 0, |pins| {
                pins.child(
                    rows.on_change(move |keys, window, cx| {
                        picked(keys.first().expect("a pick names a message"), window, cx)
                    })
                    .on_activate(move |key, window, cx| open(key, window, cx)),
                )
            })
    }
}
