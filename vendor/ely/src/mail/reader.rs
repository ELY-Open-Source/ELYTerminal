use std::rc::Rc;

use gpui::{
    AnyElement, App, Div, ElementId, FontWeight, InteractiveElement, IntoElement, ParentElement,
    RenderOnce, SharedString, Styled, Window, div, prelude::*, relative,
};
use jiff::{Timestamp, tz::TimeZone};
use smallvec::SmallVec;

use crate::{
    buttons::{ButtonVariant, IconButton},
    chat::FileMessage,
    data_display::Avatar,
    forms::OnValue,
    primitives::IconName,
    theme::{ActiveTheme, AvatarSize, ControlSize, TextSize},
    typography::{Ellipsis, format},
};

pub(super) type OnFile = Rc<dyn Fn(&SharedString, &SharedString, &mut Window, &mut App)>;

/// Someone a message is from or to: a name and an address.
#[derive(Clone, Debug, PartialEq)]
pub struct Contact {
    pub name: SharedString,
    pub address: SharedString,
}

impl Contact {
    pub fn new(name: impl Into<SharedString>, address: impl Into<SharedString>) -> Self {
        Self {
            name: name.into(),
            address: address.into(),
        }
    }
}

/// A message as a reader shows it: who sent it and to whom, when, its first words for when it folds, and its files by name and size.
#[derive(Clone, Debug, PartialEq)]
pub struct Message {
    pub key: SharedString,
    pub from: Contact,
    pub to: Vec<Contact>,
    pub cc: Vec<Contact>,
    pub at: Timestamp,
    pub snippet: SharedString,
    pub files: Vec<(SharedString, u64)>,
}

impl Message {
    pub fn new(key: impl Into<SharedString>, from: Contact, at: Timestamp) -> Self {
        Self {
            key: key.into(),
            from,
            to: Vec::new(),
            cc: Vec::new(),
            at,
            snippet: SharedString::default(),
            files: Vec::new(),
        }
    }

    pub fn to(mut self, contacts: impl IntoIterator<Item = Contact>) -> Self {
        self.to = contacts.into_iter().collect();
        self
    }

    pub fn cc(mut self, contacts: impl IntoIterator<Item = Contact>) -> Self {
        self.cc = contacts.into_iter().collect();
        self
    }

    /// Its first words, for when it folds.
    pub fn snippet(mut self, text: impl Into<SharedString>) -> Self {
        self.snippet = text.into();
        self
    }

    /// A file it carries, by name and size in bytes.
    pub fn file(mut self, name: impl Into<SharedString>, bytes: u64) -> Self {
        self.files.push((name.into(), bytes));
        self
    }
}

/// Who a message went to, as a line: "to Ana Lima, Ben Ito · cc Dev Rao"; none when it names nobody.
pub(super) fn recipients(to: &[Contact], cc: &[Contact]) -> Option<String> {
    let names = |contacts: &[Contact]| {
        contacts
            .iter()
            .map(|contact| contact.name.as_ref())
            .collect::<Vec<_>>()
            .join(", ")
    };
    match (to.is_empty(), cc.is_empty()) {
        (true, true) => None,
        (false, true) => Some(format!("to {}", names(to))),
        (true, false) => Some(format!("cc {}", names(cc))),
        (false, false) => Some(format!("to {} · cc {}", names(to), names(cc))),
    }
}

/// What an open message can ask: a reply, a reply to all, a forward, and a file.
#[derive(Clone, Default)]
pub(super) struct Asks {
    pub reply: Option<OnValue>,
    pub reply_all: Option<OnValue>,
    pub forward: Option<OnValue>,
    pub download: Option<OnFile>,
}

/// One message open: the sender's picture, name and address, who it went to, when, and replies, each only with its handler; then its words and its files.
pub(super) fn open_letter(
    id: &ElementId,
    message: &Message,
    body: SmallVec<[AnyElement; 2]>,
    zone: &TimeZone,
    asks: &Asks,
    cx: &App,
) -> Div {
    let theme = cx.theme();
    let colors = &theme.colors;
    let key = message.key.clone();
    let act = |name: &'static str, icon, tip: &'static str, ask: Option<OnValue>| {
        ask.map(|ask| {
            let key = key.clone();
            IconButton::new((id.clone(), format!("{name}-{key}")), icon)
                .variant(ButtonVariant::Ghost)
                .size(ControlSize::Sm)
                .tooltip(tip)
                .on_click(move |_, window, cx| {
                    log::info!("mail {key}: {name}");
                    ask(&key, window, cx)
                })
        })
    };
    let many = message.to.len() + message.cc.len() > 1;
    let date =
        format::datetime(message.at, zone, "%b %-d, %Y · %H:%M").expect("a fixed pattern formats");
    let files = message.files.iter().enumerate().map(|(ix, (name, bytes))| {
        let file = FileMessage::new(
            (id.clone(), format!("file-{key}-{ix}")),
            name.clone(),
            *bytes,
        );
        match asks.download.clone() {
            Some(download) => {
                let (key, name) = (key.clone(), name.clone());
                file.on_download(move |window, cx| {
                    log::info!("mail {key}: download {name}");
                    download(&key, &name, window, cx)
                })
                .into_any_element()
            }
            None => file.into_any_element(),
        }
    });
    let avatar = Avatar::new(
        (id.clone(), format!("from-{key}")),
        message.from.name.clone(),
    )
    .size(AvatarSize::Md);
    let named = key.clone();
    div()
        .debug_selector(move || format!("letter {named}"))
        .flex()
        .flex_col()
        .gap_3()
        .child(
            div()
                .flex()
                .flex_wrap()
                .items_start()
                .justify_between()
                .gap_2()
                .child(
                    div()
                        .flex_1()
                        .min_w(theme.label_width())
                        .flex()
                        .gap_3()
                        .child(div().flex_none().child(avatar))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .flex()
                                .flex_col()
                                .text_size(theme.text_size(TextSize::Sm))
                                .child(
                                    div()
                                        .flex()
                                        .items_baseline()
                                        .gap_2()
                                        .child(
                                            div()
                                                .flex_none()
                                                .max_w(theme.label_width())
                                                .text_size(theme.text_size(TextSize::Base))
                                                .font_weight(FontWeight::SEMIBOLD)
                                                .text_color(colors.fg)
                                                .child(Ellipsis::new(message.from.name.clone())),
                                        )
                                        .child(
                                            div()
                                                .flex_1()
                                                .min_w_0()
                                                .text_color(colors.fg_muted)
                                                .child(Ellipsis::new(message.from.address.clone())),
                                        ),
                                )
                                .children(recipients(&message.to, &message.cc).map(|line| {
                                    div()
                                        .min_w_0()
                                        .text_color(colors.fg_muted)
                                        .child(Ellipsis::new(line))
                                })),
                        ),
                )
                .child(
                    div()
                        .flex_none()
                        .flex()
                        .items_center()
                        .gap_1()
                        .child(
                            div()
                                .pr_1()
                                .text_size(theme.text_size(TextSize::Xs))
                                .text_color(colors.fg_subtle)
                                .child(date),
                        )
                        .children(act("reply", IconName::Reply, "Reply", asks.reply.clone()))
                        .children(act(
                            "reply-all",
                            IconName::ReplyAll,
                            "Reply all",
                            asks.reply_all.clone().filter(|_| many),
                        ))
                        .children(act(
                            "forward",
                            IconName::Forward,
                            "Forward",
                            asks.forward.clone(),
                        )),
                ),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap_3()
                .text_size(theme.text_size(TextSize::Base))
                .line_height(relative(1.6))
                .text_color(colors.fg)
                .children(body),
        )
        .when(!message.files.is_empty(), |letter| {
            letter.child(div().flex().flex_wrap().gap_2().children(files))
        })
}

/// A message open for reading: its subject over the sender's picture, name and address, who it went to, when it came, and replies, each only with its handler; then its words and its files.
#[derive(IntoElement)]
pub struct MailReader {
    id: ElementId,
    subject: SharedString,
    message: Message,
    zone: Option<TimeZone>,
    body: SmallVec<[AnyElement; 2]>,
    asks: Asks,
}

impl MailReader {
    pub fn new(
        id: impl Into<ElementId>,
        subject: impl Into<SharedString>,
        message: Message,
    ) -> Self {
        Self {
            id: id.into(),
            subject: subject.into(),
            message,
            zone: None,
            body: SmallVec::new(),
            asks: Asks::default(),
        }
    }

    /// The zone its date reads in; the system's otherwise.
    pub fn zone(mut self, zone: TimeZone) -> Self {
        self.zone = Some(zone);
        self
    }

    /// Gets the message's key.
    pub fn on_reply(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.asks.reply = Some(Rc::new(handler));
        self
    }

    /// Offered when the message went to more than one; gets its key.
    pub fn on_reply_all(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.asks.reply_all = Some(Rc::new(handler));
        self
    }

    /// Gets the message's key.
    pub fn on_forward(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.asks.forward = Some(Rc::new(handler));
        self
    }

    /// Gets the message's key and the file's name.
    pub fn on_download(
        mut self,
        handler: impl Fn(&SharedString, &SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.asks.download = Some(Rc::new(handler));
        self
    }
}

impl ParentElement for MailReader {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.body.extend(elements);
    }
}

impl RenderOnce for MailReader {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let zone = self
            .zone
            .unwrap_or_else(|| format::system_zone("mail reader"));
        let theme = cx.theme();
        div()
            .debug_selector(|| "mail-reader".into())
            .flex()
            .flex_col()
            .gap_4()
            .child(
                div()
                    .text_size(theme.text_size(TextSize::Xl))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(theme.colors.fg)
                    .child(self.subject),
            )
            .child(open_letter(
                &self.id,
                &self.message,
                self.body,
                &zone,
                &self.asks,
                cx,
            ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recipients_read_as_one_line() {
        let ana = Contact::new("Ana Lima", "ana@atrium.studio");
        let ben = Contact::new("Ben Ito", "ben@atrium.studio");
        let dev = Contact::new("Dev Rao", "dev@atrium.studio");
        assert_eq!(
            recipients(&[ana.clone(), ben], std::slice::from_ref(&dev)).as_deref(),
            Some("to Ana Lima, Ben Ito · cc Dev Rao")
        );
        assert_eq!(recipients(&[ana], &[]).as_deref(), Some("to Ana Lima"));
        assert_eq!(recipients(&[], &[dev]).as_deref(), Some("cc Dev Rao"));
        assert_eq!(recipients(&[], &[]), None);
    }
}
