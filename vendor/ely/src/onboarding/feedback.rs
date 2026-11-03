use std::rc::Rc;

use gpui::{
    App, ElementId, FontWeight, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    SharedString, Styled, Window, div,
};

use crate::{
    buttons::{Button, ButtonVariant, ToggleGroup, ToggleItem},
    forms::{Input, TextInput},
    overlays::Popover,
    primitives::{Icon, IconName},
    theme::{ActiveTheme, ControlSize, IconSize, TextSize},
};

/// How it is going, from bad to great.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sentiment {
    Bad,
    Okay,
    Good,
    Great,
}

impl Sentiment {
    const ALL: [Sentiment; 4] = [Self::Bad, Self::Okay, Self::Good, Self::Great];

    pub fn key(self) -> &'static str {
        match self {
            Self::Bad => "bad",
            Self::Okay => "okay",
            Self::Good => "good",
            Self::Great => "great",
        }
    }

    fn icon(self) -> IconName {
        match self {
            Self::Bad => IconName::Frown,
            Self::Okay => IconName::Meh,
            Self::Good => IconName::Smile,
            Self::Great => IconName::Laugh,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Bad => "Not good",
            Self::Okay => "Okay",
            Self::Good => "Good",
            Self::Great => "Great",
        }
    }

    fn of(key: &str) -> Self {
        Self::ALL
            .into_iter()
            .find(|each| each.key() == key)
            .unwrap_or_else(|| panic!("no sentiment {key}"))
    }
}

type OnFeedback = Rc<dyn Fn(Sentiment, &str, &mut Window, &mut App)>;

/// What the form holds while open.
#[derive(Default)]
struct Draft {
    sentiment: Option<Sentiment>,
    sent: bool,
}

/// A Feedback button that opens a short form: how it is going, as four faces, and a note. Send waits for a face, hands both on, then says thanks. Each opening starts fresh.
#[derive(IntoElement)]
pub struct FeedbackWidget {
    id: ElementId,
    on_send: OnFeedback,
}

impl FeedbackWidget {
    pub fn new(
        id: impl Into<ElementId>,
        on_send: impl Fn(Sentiment, &str, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            on_send: Rc::new(on_send),
        }
    }
}

impl RenderOnce for FeedbackWidget {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let id = self.id;
        let on_send = self.on_send;
        Popover::new(id.clone(), "Feedback", move |window, cx| {
            form(id, on_send, window, cx)
        })
        .icon(IconName::MessageSquare)
        .variant(ButtonVariant::Ghost)
    }
}

fn form(
    id: ElementId,
    on_send: OnFeedback,
    window: &mut Window,
    cx: &mut App,
) -> impl IntoElement + use<> {
    let draft = window.use_keyed_state((id.clone(), "draft"), cx, |_, _| Draft::default());
    let note = window.use_keyed_state((id.clone(), "note"), cx, |window, cx| {
        TextInput::new(window, cx)
            .multi_line(2, 5)
            .placeholder("Tell us more (optional)")
    });
    let theme = cx.theme();
    let width = theme.tooltip_max_width();
    let (sentiment, sent) = (draft.read(cx).sentiment, draft.read(cx).sent);
    if sent {
        return div()
            .debug_selector(|| "feedback-thanks".into())
            .w(width)
            .flex()
            .items_center()
            .gap_2()
            .text_size(theme.text_size(TextSize::Sm))
            .child(
                Icon::new(IconName::CircleCheck)
                    .size(IconSize::Sm)
                    .color(theme.colors.success),
            )
            .child(div().flex_1().min_w_0().child("Thanks for telling us."));
    }
    let faces = Sentiment::ALL.into_iter().fold(
        ToggleGroup::new((id.clone(), "faces")).selected(sentiment.map(Sentiment::key)),
        |faces, each| {
            faces.item(
                ToggleItem::new(each.key())
                    .icon(each.icon())
                    .tooltip(each.name()),
            )
        },
    );
    let (pick, send, typed) = (draft.clone(), draft, note.clone());
    div()
        .w(width)
        .flex()
        .flex_col()
        .gap_3()
        .text_size(theme.text_size(TextSize::Sm))
        .child(
            div()
                .font_weight(FontWeight::MEDIUM)
                .child("How is it going?"),
        )
        .child(faces.on_change(move |keys, _, cx| {
            let chosen = keys.first().map(|key: &SharedString| Sentiment::of(key));
            pick.update(cx, |draft, cx| {
                draft.sentiment = chosen;
                cx.notify();
            })
        }))
        .child(Input::new(&note))
        .child(
            div().flex().justify_end().child(
                Button::new((id.clone(), "send"), "Send")
                    .variant(ButtonVariant::Primary)
                    .size(ControlSize::Sm)
                    .disabled(sentiment.is_none())
                    .on_click(move |_, window, cx| {
                        let chosen = sentiment.expect("Send rests until a face is picked");
                        let words = typed.read(cx).text().trim().to_string();
                        log::info!("feedback widget: sent {}", chosen.key());
                        on_send(chosen, &words, window, cx);
                        send.update(cx, |draft, cx| {
                            draft.sent = true;
                            cx.notify();
                        })
                    }),
            ),
        )
}
