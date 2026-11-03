use std::rc::Rc;

use gpui::{
    App, AppContext as _, ElementId, Entity, InteractiveElement, IntoElement, ParentElement,
    RenderOnce, SharedString, Styled, Window, div, prelude::*,
};

use crate::{
    buttons::{Button, ButtonVariant},
    forms::{CheckboxGroup, Choice, Input, RadioGroup, TextInput},
    primitives::{FocusRing, Icon, IconName, tab_stop},
    theme::{ActiveTheme, IconSize, Radius, TextSize},
};

type OnSubmit = Rc<dyn Fn(&[Answer], &mut Window, &mut App)>;

/// What a survey asks, and how it is answered.
#[derive(Clone)]
pub enum Question {
    /// One of the options.
    Choice(SharedString, Vec<SharedString>),
    /// Any of the options, one at least.
    Several(SharedString, Vec<SharedString>),
    /// Words.
    Text(SharedString),
}

impl Question {
    fn title(&self) -> &SharedString {
        match self {
            Question::Choice(title, _) | Question::Several(title, _) | Question::Text(title) => {
                title
            }
        }
    }
}

/// An answer, by the options' places or in words.
#[derive(Clone, Debug, PartialEq)]
pub enum Answer {
    Choice(usize),
    Several(Vec<usize>),
    Text(SharedString),
}

/// What the survey holds: the answers picked, a field for each question in words, whether it was sent, and whether Submit came with an answer missing.
struct Sheet {
    picked: Vec<Option<Answer>>,
    fields: Vec<Option<Entity<TextInput>>>,
    sent: bool,
    nudged: bool,
}

/// Questions on one page, each answered by one option, several, or words. Submit hands the answers to the owner once every one is answered, and asks for the rest before; then the survey thanks the viewer in Submit's place and focus.
#[derive(IntoElement)]
pub struct Survey {
    id: ElementId,
    questions: Vec<Question>,
    on_submit: OnSubmit,
}

impl Survey {
    pub fn new(
        id: impl Into<ElementId>,
        questions: impl IntoIterator<Item = Question>,
        on_submit: impl Fn(&[Answer], &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            questions: questions.into_iter().collect(),
            on_submit: Rc::new(on_submit),
        }
    }
}

/// Each question's answer as it stands, none where it waits for one.
fn answers(questions: &[Question], sheet: &Sheet, cx: &App) -> Vec<Option<Answer>> {
    questions
        .iter()
        .enumerate()
        .map(|(ix, question)| match question {
            Question::Text(_) => {
                let field = sheet.fields[ix].as_ref().expect("a field for words");
                let words = field.read(cx).text().trim().to_string();
                (!words.is_empty()).then(|| Answer::Text(words.into()))
            }
            _ => sheet.picked[ix].clone(),
        })
        .collect()
}

/// Options as choices keyed by their places.
fn choices(options: &[SharedString]) -> impl Iterator<Item = Choice> + '_ {
    options
        .iter()
        .enumerate()
        .map(|(ix, label)| Choice::new(ix.to_string(), label.clone()))
}

/// A sheet with no answers, a field for each question answered in words.
fn blank(questions: &[Question], window: &mut Window, cx: &mut App) -> Sheet {
    Sheet {
        picked: vec![None; questions.len()],
        fields: questions
            .iter()
            .map(|question| {
                matches!(question, Question::Text(_))
                    .then(|| cx.new(|cx| TextInput::new(window, cx).multi_line(2, 5)))
            })
            .collect(),
        sent: false,
        nudged: false,
    }
}

fn place(value: &SharedString) -> usize {
    value.parse().expect("an option's place")
}

impl RenderOnce for Survey {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id;
        assert!(!self.questions.is_empty(), "survey {id:?} has no questions");
        let on_submit = self.on_submit;
        let questions = Rc::new(self.questions);
        let sheet = window.use_keyed_state((id.clone(), "sheet"), cx, {
            let questions = questions.clone();
            move |window, cx| blank(&questions, window, cx)
        });
        let fits = {
            let now = sheet.read(cx);
            now.picked.len() == questions.len()
                && questions.iter().zip(&now.fields).all(|(question, field)| {
                    matches!(question, Question::Text(_)) == field.is_some()
                })
        };
        if !fits {
            log::error!("survey {id:?}: its questions changed; the answers start over");
            let fresh = blank(&questions, window, cx);
            sheet.update(cx, |sheet, _| *sheet = fresh);
        }
        let sent = sheet.read(cx).sent;
        let action = tab_stop((id.clone(), "submit").into(), !sent, window, cx);
        let theme = cx.theme();
        let colors = &theme.colors;
        if sent {
            return div()
                .id((id, "sent"))
                .debug_selector(|| "survey-sent".into())
                .track_focus(&action)
                .flex()
                .items_center()
                .gap_2()
                .p_4()
                .rounded(theme.radius(Radius::Md))
                .border_1()
                .border_color(colors.border)
                .bg(colors.surface)
                .focus_ring(cx)
                .child(
                    Icon::new(IconName::CircleCheck)
                        .size(IconSize::Sm)
                        .color(colors.success),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .text_color(colors.fg)
                        .child("Thanks. Your answers were sent."),
                )
                .into_any_element();
        }
        let now = answers(&questions, sheet.read(cx), cx);
        let complete = now.iter().all(Option::is_some);
        let items = questions.iter().enumerate().map(|(ix, question)| {
            let key = (id.clone(), format!("question-{ix}"));
            let set = sheet.clone();
            let input = match question {
                Question::Choice(_, options) => {
                    let group =
                        RadioGroup::new(key, choices(options)).on_change(move |value, _, cx| {
                            set.update(cx, |sheet, cx| {
                                sheet.picked[ix] = Some(Answer::Choice(place(value)));
                                cx.notify();
                            })
                        });
                    match &now[ix] {
                        Some(Answer::Choice(option)) => group.selected(option.to_string()),
                        _ => group,
                    }
                    .into_any_element()
                }
                Question::Several(_, options) => {
                    let ticked = match &now[ix] {
                        Some(Answer::Several(ticked)) => {
                            ticked.iter().map(usize::to_string).collect()
                        }
                        _ => Vec::new(),
                    };
                    CheckboxGroup::new(key, choices(options))
                        .selected(ticked)
                        .on_change(move |values, _, cx| {
                            let ticked: Vec<usize> = values.iter().map(place).collect();
                            set.update(cx, |sheet, cx| {
                                sheet.picked[ix] =
                                    (!ticked.is_empty()).then_some(Answer::Several(ticked));
                                cx.notify();
                            })
                        })
                        .into_any_element()
                }
                Question::Text(_) => {
                    let field = sheet.read(cx).fields[ix]
                        .clone()
                        .expect("a field for words");
                    Input::new(&field).into_any_element()
                }
            };
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(
                    div().flex().child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_color(colors.fg)
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .child(question.title().clone()),
                    ),
                )
                .child(input)
        });
        let nudged = sheet.read(cx).nudged && !complete;
        let (submitted, focus, asked) = (sheet.clone(), action.clone(), questions.clone());
        div()
            .flex()
            .flex_col()
            .gap_5()
            .text_size(theme.text_size(TextSize::Sm))
            .children(items)
            .when(nudged, |survey| {
                survey.child(
                    div()
                        .debug_selector(|| "survey-answer-first".into())
                        .text_color(colors.danger)
                        .child("Answer each question first."),
                )
            })
            .child(
                div()
                    .debug_selector(|| "survey-action".into())
                    .flex()
                    .child(
                        Button::new((id, "submit-button"), "Submit")
                            .variant(ButtonVariant::Primary)
                            .focus_handle(&action)
                            .on_click(move |_, window, cx| {
                                let given: Option<Vec<Answer>> =
                                    answers(&asked, submitted.read(cx), cx)
                                        .into_iter()
                                        .collect();
                                let Some(given) = given else {
                                    log::info!("survey: an answer is missing");
                                    submitted.update(cx, |sheet, cx| {
                                        sheet.nudged = true;
                                        cx.notify();
                                    });
                                    return;
                                };
                                log::info!("survey: sent {} answers", given.len());
                                on_submit(&given, window, cx);
                                submitted.update(cx, |sheet, cx| {
                                    sheet.sent = true;
                                    cx.notify();
                                });
                                window.focus(&focus, cx);
                            }),
                    ),
            )
            .into_any_element()
    }
}
