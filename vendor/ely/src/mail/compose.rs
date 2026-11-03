use std::rc::Rc;

use gpui::{
    App, Context, ElementId, Entity, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    SharedString, Styled, Window, div, prelude::*,
};
use jiff::{Timestamp, tz::TimeZone};

use super::{reader::Contact, send::ScheduleSend, signature::signature_block};
use crate::{
    buttons::{Button, ButtonVariant, IconButton},
    chat::{Attachment, AttachmentChip},
    forms::{Choice, Input, OnValue, Run, TagInput, TextInput, is_email},
    primitives::IconName,
    theme::{ActiveTheme, ControlSize, TextSize},
};

/// What a composer hands over: the addresses, the subject and the words.
#[derive(Clone, Debug, PartialEq)]
pub struct Draft {
    pub to: Vec<SharedString>,
    pub cc: Vec<SharedString>,
    pub bcc: Vec<SharedString>,
    pub subject: SharedString,
    pub body: SharedString,
}

type OnDraft = Rc<dyn Fn(&Draft, &mut Window, &mut App)>;
type OnDraftAt = Rc<dyn Fn(&Draft, Timestamp, &mut Window, &mut App)>;

/// Whether a message can go: someone to send to, and every address an address.
fn ready(to: &[SharedString], cc: &[SharedString], bcc: &[SharedString]) -> bool {
    let mut all = to.iter().chain(cc).chain(bcc).peekable();
    all.peek().is_some() && all.all(|address| is_email(address))
}

/// Which field of addresses.
#[derive(Clone, Copy)]
enum Field {
    To,
    Cc,
    Bcc,
}

/// The message being written: the owner's start it came from, the addresses, whether Cc and Bcc show, and the subject and words.
struct Composing {
    seed: (Vec<SharedString>, SharedString),
    to: Vec<SharedString>,
    cc: Vec<SharedString>,
    bcc: Vec<SharedString>,
    copies: bool,
    subject: Entity<TextInput>,
    body: Entity<TextInput>,
}

impl Composing {
    /// Starts over from the owner's addresses and subject, with no words.
    fn fill(&mut self, (to, subject): (Vec<SharedString>, SharedString), cx: &mut Context<Self>) {
        self.to = to;
        self.cc.clear();
        self.bcc.clear();
        self.copies = false;
        self.subject
            .update(cx, |input, cx| input.set_text(subject.to_string(), cx));
        self.body.update(cx, |input, cx| input.set_text("", cx));
    }

    fn draft(&self, cx: &App) -> Draft {
        Draft {
            to: self.to.clone(),
            cc: self.cc.clone(),
            bcc: self.bcc.clone(),
            subject: self.subject.read(cx).text().trim().to_string().into(),
            body: self.body.read(cx).text().to_string().into(),
        }
    }
}

/// A message to write: To, and Cc and Bcc once asked for, as fields that suggest contacts and mark what is not an address; the subject and the words; the files, each with a way off; and the signature under the words. Send goes once there is someone to send to, now or at a time; Discard lets it go. A new start from the owner begins it over.
#[derive(IntoElement)]
pub struct MailComposer {
    id: ElementId,
    contacts: Vec<Contact>,
    to: Vec<SharedString>,
    subject: SharedString,
    signature: Option<SharedString>,
    files: Vec<Attachment>,
    zone: Option<TimeZone>,
    on_send: Option<OnDraft>,
    on_schedule: Option<OnDraftAt>,
    on_discard: Option<Run>,
    on_attach: Option<Run>,
    on_remove: Option<OnValue>,
}

impl MailComposer {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            contacts: Vec::new(),
            to: Vec::new(),
            subject: SharedString::default(),
            signature: None,
            files: Vec::new(),
            zone: None,
            on_send: None,
            on_schedule: None,
            on_discard: None,
            on_attach: None,
            on_remove: None,
        }
    }

    /// People to suggest as addresses are typed.
    pub fn contacts(mut self, contacts: impl IntoIterator<Item = Contact>) -> Self {
        self.contacts = contacts.into_iter().collect();
        self
    }

    /// The addresses it starts with, as a reply has them.
    pub fn to(mut self, addresses: impl IntoIterator<Item = impl Into<SharedString>>) -> Self {
        self.to = addresses.into_iter().map(Into::into).collect();
        self
    }

    /// The subject it starts with.
    pub fn subject(mut self, subject: impl Into<SharedString>) -> Self {
        self.subject = subject.into();
        self
    }

    pub fn signature(mut self, text: impl Into<SharedString>) -> Self {
        self.signature = Some(text.into());
        self
    }

    /// Files the owner attached.
    pub fn files(mut self, files: impl IntoIterator<Item = Attachment>) -> Self {
        self.files = files.into_iter().collect();
        self
    }

    /// The zone its times to send read in; the system's otherwise.
    pub fn zone(mut self, zone: TimeZone) -> Self {
        self.zone = Some(zone);
        self
    }

    pub fn on_send(mut self, handler: impl Fn(&Draft, &mut Window, &mut App) + 'static) -> Self {
        self.on_send = Some(Rc::new(handler));
        self
    }

    /// Gets the draft and the moment to send it.
    pub fn on_schedule(
        mut self,
        handler: impl Fn(&Draft, Timestamp, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_schedule = Some(Rc::new(handler));
        self
    }

    pub fn on_discard(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_discard = Some(Rc::new(handler));
        self
    }

    /// Asks the owner for files to attach.
    pub fn on_attach(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_attach = Some(Rc::new(handler));
        self
    }

    /// Gets the key of the file to take off.
    pub fn on_remove(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_remove = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for MailComposer {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let seed = (self.to.clone(), self.subject.clone());
        let state = window.use_keyed_state(
            (self.id.clone(), "draft"),
            cx,
            |window, cx: &mut Context<Composing>| {
                let subject = cx.new(|cx| TextInput::new(window, cx).placeholder("Subject"));
                let body = cx.new(|cx| {
                    TextInput::new(window, cx)
                        .multi_line(6, 16)
                        .placeholder("Write your message")
                });
                let mut composing = Composing {
                    seed: seed.clone(),
                    to: Vec::new(),
                    cc: Vec::new(),
                    bcc: Vec::new(),
                    copies: false,
                    subject,
                    body,
                };
                composing.fill(seed.clone(), cx);
                composing
            },
        );
        if state.read(cx).seed != seed {
            log::info!("mail composer {}: started over", self.id);
            state.update(cx, |composing, cx| {
                composing.seed = seed.clone();
                composing.fill(seed, cx);
            });
        }
        let now = state.read(cx);
        let (to, cc, bcc, copies) = (now.to.clone(), now.cc.clone(), now.bcc.clone(), now.copies);
        let (subject, body) = (now.subject.clone(), now.body.clone());
        let can_send = ready(&to, &cc, &bcc);
        let choices: Vec<Choice> = self
            .contacts
            .iter()
            .map(|contact| Choice::new(contact.address.clone(), contact.name.clone()))
            .collect();
        let theme = cx.theme();
        let colors = &theme.colors;
        let row = |label: &'static str, field: Field, tags: Vec<SharedString>| {
            let owner = state.clone();
            div()
                .debug_selector(move || format!("composer-{label}"))
                .flex()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .flex_none()
                        .w_10()
                        .text_size(theme.text_size(TextSize::Sm))
                        .text_color(colors.fg_muted)
                        .child(label),
                )
                .child(
                    div().flex_1().min_w_0().child(
                        TagInput::new((self.id.clone(), label), tags)
                            .placeholder("Add people")
                            .check(is_email)
                            .suggestions(choices.clone())
                            .on_change(move |next, _, cx| {
                                log::info!("mail composer: {label} holds {}", next.len());
                                owner.update(cx, |composing, cx| {
                                    match field {
                                        Field::To => composing.to = next,
                                        Field::Cc => composing.cc = next,
                                        Field::Bcc => composing.bcc = next,
                                    }
                                    cx.notify();
                                })
                            }),
                    ),
                )
        };
        let showing = state.clone();
        let copies_button = (!copies).then(|| {
            Button::new((self.id.clone(), "copies"), "Cc Bcc")
                .variant(ButtonVariant::Ghost)
                .size(ControlSize::Sm)
                .on_click(move |_, _, cx| {
                    log::info!("mail composer: shows Cc and Bcc");
                    showing.update(cx, |composing, cx| {
                        composing.copies = true;
                        cx.notify();
                    })
                })
        });
        let files = self.files.iter().map(|file| {
            let chip = AttachmentChip::new(
                (self.id.clone(), format!("file-{}", file.key)),
                file.clone(),
            );
            match self.on_remove.clone() {
                Some(remove) => {
                    let key = file.key.clone();
                    chip.on_remove(move |window, cx| {
                        log::info!("mail composer: takes off {key}");
                        remove(&key, window, cx)
                    })
                    .into_any_element()
                }
                None => chip.into_any_element(),
            }
        });
        let sending = state.clone();
        let send = ScheduleSend::new((self.id.clone(), "send"))
            .disabled(!can_send)
            .when_some(self.zone.clone(), |send, zone| send.zone(zone))
            .when_some(self.on_send.clone(), |send, on_send| {
                send.on_send(move |window, cx| {
                    let draft = sending.read(cx).draft(cx);
                    log::info!("mail composer: sends to {}", draft.to.len());
                    on_send(&draft, window, cx)
                })
            });
        let scheduling = state.clone();
        let send = match self.on_schedule.clone() {
            Some(on_schedule) => send.on_schedule(move |at, window, cx| {
                let draft = scheduling.read(cx).draft(cx);
                log::info!("mail composer: sends at {at}");
                on_schedule(&draft, at, window, cx)
            }),
            None => send,
        };
        let attach = self.on_attach.map(|attach| {
            IconButton::new((self.id.clone(), "attach"), IconName::Paperclip)
                .variant(ButtonVariant::Ghost)
                .tooltip("Attach files")
                .on_click(move |_, window, cx| {
                    log::info!("mail composer: attach");
                    attach(window, cx)
                })
        });
        let discard = self.on_discard.map(|discard| {
            IconButton::new((self.id.clone(), "discard"), IconName::Trash2)
                .variant(ButtonVariant::Ghost)
                .tooltip("Discard")
                .on_click(move |_, window, cx| {
                    log::info!("mail composer: discard");
                    discard(window, cx)
                })
        });
        div()
            .debug_selector(|| "mail-composer".into())
            .flex()
            .flex_col()
            .gap_3()
            .child(
                row("To", Field::To, to)
                    .children(copies_button.map(|button| div().flex_none().child(button))),
            )
            .when(copies, |form| {
                form.child(row("Cc", Field::Cc, cc))
                    .child(row("Bcc", Field::Bcc, bcc))
            })
            .child(Input::new(&subject))
            .child(Input::new(&body))
            .when(!self.files.is_empty(), |form| {
                form.child(div().flex().flex_wrap().gap_2().children(files))
            })
            .children(self.signature.map(|text| signature_block(&text, cx)))
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
                            .gap_1()
                            .child(send)
                            .children(attach),
                    )
                    .children(discard),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_message_goes_once_someone_holds_an_address() {
        let ben: SharedString = "ben@atrium.studio".into();
        let bad: SharedString = "ben@".into();
        assert!(!ready(&[], &[], &[]), "nobody to send to");
        assert!(ready(std::slice::from_ref(&ben), &[], &[]));
        assert!(
            ready(&[], &[], std::slice::from_ref(&ben)),
            "a blind copy alone"
        );
        assert!(!ready(&[ben], &[bad], &[]), "an address that is not one");
    }
}
