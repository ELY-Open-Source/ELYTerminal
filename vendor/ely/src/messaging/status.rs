use std::rc::Rc;

use gpui::{
    App, Context, ElementId, Entity, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    SharedString, Styled, Subscription, Window, div, prelude::*, rems,
};

use crate::{
    buttons::{Button, ButtonVariant},
    forms::{
        Choice, EmojiPicker, Face, Input, InputEvent, Run, Select, TextInput, dropdown,
        float_height, picker_field, surface,
    },
    lists::{ListItem, SelectableList},
    primitives::IconName,
    theme::{ActiveTheme, ControlSize, TextSize},
};

/// When a status clears itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClearAfter {
    Never,
    ThirtyMinutes,
    OneHour,
    FourHours,
    Today,
    ThisWeek,
}

impl ClearAfter {
    pub const ALL: [Self; 6] = [
        Self::Never,
        Self::ThirtyMinutes,
        Self::OneHour,
        Self::FourHours,
        Self::Today,
        Self::ThisWeek,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Never => "Don't clear",
            Self::ThirtyMinutes => "30 minutes",
            Self::OneHour => "1 hour",
            Self::FourHours => "4 hours",
            Self::Today => "Today",
            Self::ThisWeek => "This week",
        }
    }

    fn of(label: &str) -> Self {
        Self::ALL
            .into_iter()
            .find(|clear| clear.label() == label)
            .unwrap_or_else(|| panic!("no clear after {label}"))
    }
}

/// A status: an emoji when chosen, a few words, and when it clears.
#[derive(Clone, Debug, PartialEq)]
pub struct Status {
    pub emoji: Option<SharedString>,
    pub text: SharedString,
    pub clear: ClearAfter,
}

type OnStatus = Rc<dyn Fn(&Status, &mut Window, &mut App)>;

/// What the setter holds before Save: the owner's status it started from, and the edits.
struct Draft {
    seed: Option<Status>,
    emoji: Option<SharedString>,
    clear: ClearAfter,
    input: Entity<TextInput>,
    _changes: Subscription,
}

impl Draft {
    /// Starts over from `status`; a new one clears today.
    fn fill(&mut self, status: Option<&Status>, cx: &mut Context<Self>) {
        self.emoji = status.and_then(|status| status.emoji.clone());
        self.clear = status.map_or(ClearAfter::Today, |status| status.clear);
        let text = status.map_or(String::new(), |status| status.text.to_string());
        self.input.update(cx, |input, cx| input.set_text(text, cx));
        cx.notify();
    }

    fn status(&self, cx: &App) -> Status {
        Status {
            emoji: self.emoji.clone(),
            text: self.input.read(cx).text().trim().to_string().into(),
            clear: self.clear,
        }
    }
}

/// Sets your status: an emoji, a few words, and when it clears. Suggestions fill all three; Save hands the status over, and Clear status removes the one set. A new status from the owner starts the edits over.
#[derive(IntoElement)]
pub struct StatusSetter {
    id: ElementId,
    status: Option<Status>,
    suggestions: Vec<Status>,
    on_save: Option<OnStatus>,
    on_clear: Option<Run>,
}

impl StatusSetter {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            status: None,
            suggestions: Vec::new(),
            on_save: None,
            on_clear: None,
        }
    }

    /// The status set now.
    pub fn status(mut self, status: Status) -> Self {
        self.status = Some(status);
        self
    }

    pub fn suggestions(mut self, suggestions: impl IntoIterator<Item = Status>) -> Self {
        self.suggestions = suggestions.into_iter().collect();
        self
    }

    pub fn on_save(mut self, handler: impl Fn(&Status, &mut Window, &mut App) + 'static) -> Self {
        self.on_save = Some(Rc::new(handler));
        self
    }

    pub fn on_clear(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_clear = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for StatusSetter {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let seed = self.status.clone();
        let draft = window.use_keyed_state((self.id.clone(), "draft"), cx, |window, cx| {
            let input = cx.new(|cx| TextInput::new(window, cx).placeholder("What's your status?"));
            let changes = cx.subscribe(&input, |_, _, event: &InputEvent, cx| {
                if *event == InputEvent::Changed {
                    cx.notify();
                }
            });
            let mut draft = Draft {
                seed: seed.clone(),
                emoji: None,
                clear: ClearAfter::Today,
                input,
                _changes: changes,
            };
            draft.fill(seed.as_ref(), cx);
            draft
        });
        if draft.read(cx).seed != self.status {
            log::info!("status setter {}: started over", self.id);
            let status = self.status.clone();
            draft.update(cx, |draft, cx| {
                draft.fill(status.as_ref(), cx);
                draft.seed = status;
            });
        }
        let now = draft.read(cx).status(cx);
        let input = draft.read(cx).input.clone();
        let emoji_id: ElementId = (self.id.clone(), "emoji").into();
        let inner = dropdown(&emoji_id, window, cx).read(cx).inner.clone();
        let (picked, popup_id, shown) = (draft.clone(), emoji_id.clone(), now.emoji.clone());
        let emoji = picker_field(
            emoji_id,
            Face {
                icon: IconName::Smile,
                shown: now.emoji.clone(),
                placeholder: "".into(),
                size: ControlSize::Md,
                disabled: false,
            },
            move |close, anchor, window, cx| {
                let theme = cx.theme();
                let rem = window.rem_size();
                let height = theme.list_max_height().to_pixels(rem)
                    + theme.control_height(ControlSize::Lg).to_pixels(rem) * 2.0;
                let (escape, done) = (close.clone(), close.clone());
                let picker = EmojiPicker::new((popup_id.clone(), "picker"))
                    .when_some(shown, |picker, emoji| picker.selected(emoji))
                    .on_change(move |glyph, window, cx| {
                        let glyph = SharedString::from(glyph.to_string());
                        log::info!("status setter: emoji {glyph}");
                        picked.update(cx, |draft, cx| {
                            draft.emoji = Some(glyph);
                            cx.notify();
                        });
                        done(window, cx);
                    });
                float_height(
                    popup_id.clone(),
                    anchor,
                    height,
                    surface((popup_id, "popup"), cx)
                        .track_focus(&inner)
                        .p_2()
                        .on_mouse_down_out(move |_, window, cx| close(window, cx))
                        .on_key_down(move |event, window, cx| {
                            if event.keystroke.key == "escape" {
                                cx.stop_propagation();
                                escape(window, cx);
                            }
                        })
                        .child(picker),
                    window,
                    cx,
                )
            },
            window,
            cx,
        );
        let theme = cx.theme();
        let colors = &theme.colors;
        let label = |text: &'static str| {
            div()
                .text_size(theme.text_size(TextSize::Xs))
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(colors.fg_muted)
                .child(text)
        };
        let suggestions = (!self.suggestions.is_empty()).then(|| {
            let lit = self
                .suggestions
                .iter()
                .position(|suggestion| *suggestion == now)
                .map(|ix| format!("suggestion-{ix}"));
            let rows = self.suggestions.iter().enumerate().fold(
                SelectableList::new((self.id.clone(), "suggestions")),
                |list, (ix, suggestion)| {
                    let row = ListItem::new(
                        (self.id.clone(), format!("suggestion-{ix}")),
                        suggestion.text.clone(),
                    )
                    .trailing(suggestion.clear.label());
                    let row = match suggestion.emoji.clone() {
                        Some(emoji) => row.leading(emoji),
                        None => row,
                    };
                    list.row(format!("suggestion-{ix}"), row)
                },
            );
            let (all, filled) = (self.suggestions.clone(), draft.clone());
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(label("Suggestions"))
                .child(rows.selected(lit).on_change(move |keys, _, cx| {
                    let key = keys.first().expect("a pick names a suggestion");
                    let ix: usize = key["suggestion-".len()..]
                        .parse()
                        .expect("a suggestion's key holds its place");
                    log::info!("status setter: suggestion {ix}");
                    let suggestion = all[ix].clone();
                    filled.update(cx, |draft, cx| draft.fill(Some(&suggestion), cx));
                }))
        });
        let clearing = draft.clone();
        let clear_after = Select::new(
            (self.id.clone(), "clear"),
            ClearAfter::ALL.map(|clear| Choice::new(clear.label(), clear.label())),
        )
        .selected(now.clear.label())
        .on_change(move |value, _, cx| {
            let clear = ClearAfter::of(value);
            log::info!("status setter: clear after {}", clear.label());
            clearing.update(cx, |draft, cx| {
                draft.clear = clear;
                cx.notify();
            });
        });
        let blank = now.emoji.is_none() && now.text.trim().is_empty();
        let clear = self.on_clear.filter(|_| self.status.is_some()).map(|run| {
            Button::new((self.id.clone(), "remove"), "Clear status")
                .variant(ButtonVariant::Ghost)
                .on_click(move |_, window, cx| {
                    log::info!("status setter: clear");
                    run(window, cx)
                })
        });
        let save = self.on_save.map(|save| {
            let saving = draft.clone();
            Button::new((self.id.clone(), "save"), "Save")
                .variant(ButtonVariant::Primary)
                .disabled(blank)
                .on_click(move |_, window, cx| {
                    let status = saving.read(cx).status(cx);
                    log::info!("status setter: save {:?} {}", status.emoji, status.text);
                    save(&status, window, cx)
                })
        });
        div()
            .debug_selector(|| "status-setter".into())
            .flex()
            .flex_col()
            .gap_4()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .flex_none()
                            .w(rems(theme.control_height(ControlSize::Md).0 * 2.5))
                            .child(emoji),
                    )
                    .child(div().flex_1().min_w_0().child(Input::new(&input))),
            )
            .children(suggestions)
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .child(label("Clear after"))
                    .child(div().flex_none().w(theme.label_width()).child(clear_after)),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .justify_end()
                    .gap_2()
                    .children(clear)
                    .children(save),
            )
    }
}
