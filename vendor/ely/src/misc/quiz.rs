use std::rc::Rc;

use gpui::{
    App, ElementId, FontWeight, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    SharedString, Styled, Window, div, prelude::*,
};

use crate::{
    buttons::{Button, ButtonVariant},
    forms::{Choice, RadioGroup},
    primitives::{Icon, IconName, tab_stop},
    theme::{ActiveTheme, IconSize, TextSize},
    typography::format,
};

type OnScore = Rc<dyn Fn(usize, usize, &mut Window, &mut App)>;

/// A question, its options, and the right one.
#[derive(Clone)]
pub struct QuizQuestion {
    prompt: SharedString,
    options: Vec<SharedString>,
    right: usize,
}

impl QuizQuestion {
    pub fn new(
        prompt: impl Into<SharedString>,
        options: impl IntoIterator<Item = impl Into<SharedString>>,
        right: usize,
    ) -> Self {
        let options: Vec<SharedString> = options.into_iter().map(Into::into).collect();
        assert!(
            options.len() >= 2,
            "a quiz question needs two options or more"
        );
        assert!(
            right < options.len(),
            "right answer {right} of {} options",
            options.len()
        );
        Self {
            prompt: prompt.into(),
            options,
            right,
        }
    }
}

/// Where a quiz stands: the question, its pick, whether it was checked, the right answers so far, and whether Check came with no pick.
#[derive(Default)]
struct Sitting {
    at: usize,
    pick: Option<usize>,
    checked: bool,
    right: usize,
    done: bool,
    nudged: bool,
}

impl Sitting {
    fn pick(&mut self, option: usize) {
        self.pick = Some(option);
        self.nudged = false;
    }

    /// What the one button does next, and says.
    fn next(&self, count: usize) -> &'static str {
        match (self.done, self.checked, self.at + 1 == count) {
            (true, _, _) => "Try again",
            (false, false, _) => "Check",
            (false, true, false) => "Next",
            (false, true, true) => "See score",
        }
    }

    /// Runs the button's step for question `right` of `count`.
    fn advance(&mut self, right: usize, count: usize) {
        match (self.done, self.checked) {
            (true, _) => *self = Sitting::default(),
            (false, false) => match self.pick {
                None => self.nudged = true,
                Some(pick) => {
                    self.checked = true;
                    self.right += usize::from(pick == right);
                }
            },
            (false, true) if self.at + 1 == count => self.done = true,
            (false, true) => {
                self.at += 1;
                self.pick = None;
                self.checked = false;
            }
        }
    }
}

/// Questions one at a time: pick an option and Check shows the right one, then Next; the last gives the score, with Try again. Check, Next, See score and Try again are one button on one focus handle; Check with no pick asks for one.
#[derive(IntoElement)]
pub struct Quiz {
    id: ElementId,
    questions: Vec<QuizQuestion>,
    on_score: Option<OnScore>,
}

impl Quiz {
    pub fn new(
        id: impl Into<ElementId>,
        questions: impl IntoIterator<Item = QuizQuestion>,
    ) -> Self {
        Self {
            id: id.into(),
            questions: questions.into_iter().collect(),
            on_score: None,
        }
    }

    /// Runs with the right answers and the question count when the score shows.
    pub fn on_score(
        mut self,
        handler: impl Fn(usize, usize, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_score = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for Quiz {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id;
        let count = self.questions.len();
        assert!(count > 0, "quiz {id:?} has no questions");
        let sitting =
            window.use_keyed_state((id.clone(), "sitting"), cx, |_, _| Sitting::default());
        if sitting.read(cx).at >= count {
            log::error!(
                "quiz {id:?}: question {} of {count}; it starts over",
                sitting.read(cx).at
            );
            sitting.update(cx, |sitting, _| *sitting = Sitting::default());
        }
        let action = tab_stop((id.clone(), "action").into(), true, window, cx);
        let theme = cx.theme();
        let colors = &theme.colors;
        let (at, pick, checked, right, done, nudged) = {
            let sitting = sitting.read(cx);
            (
                sitting.at,
                sitting.pick,
                sitting.checked,
                sitting.right,
                sitting.done,
                sitting.nudged,
            )
        };
        let question = self.questions[at].clone();
        let label = sitting.read(cx).next(count);
        let body = match (done, checked) {
            (true, _) => div()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .debug_selector(|| format!("quiz-score-{right}-{count}"))
                        .text_size(theme.text_size(TextSize::Xxl))
                        .text_color(colors.fg)
                        .child(format!("{right} of {count}")),
                )
                .child(div().text_color(colors.fg_muted).child(format::percent(
                    right as f64 / count as f64,
                    0,
                    false,
                )))
                .into_any_element(),
            (false, false) => {
                let choices = question
                    .options
                    .iter()
                    .enumerate()
                    .map(|(ix, label)| Choice::new(ix.to_string(), label.clone()));
                let chose = sitting.clone();
                let group = RadioGroup::new((id.clone(), format!("options-{at}")), choices)
                    .on_change(move |value, _, cx| {
                        let ix = value.parse().expect("an option's place");
                        chose.update(cx, |sitting, cx| {
                            sitting.pick(ix);
                            cx.notify();
                        })
                    });
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(match pick {
                        Some(ix) => group.selected(ix.to_string()),
                        None => group,
                    })
                    .when(nudged, |body| {
                        body.child(
                            div()
                                .debug_selector(|| "quiz-pick-first".into())
                                .text_size(theme.text_size(TextSize::Sm))
                                .text_color(colors.danger)
                                .child("Pick an answer first."),
                        )
                    })
                    .into_any_element()
            }
            (false, true) => {
                let rows = question.options.iter().enumerate().map(|(ix, label)| {
                    let (icon, tone) = match (ix == question.right, Some(ix) == pick) {
                        (true, _) => (Some(IconName::CircleCheck), colors.success),
                        (false, true) => (Some(IconName::CircleX), colors.danger),
                        (false, false) => (None, colors.fg_muted),
                    };
                    div()
                        .debug_selector(|| format!("quiz-row-{label}-{}", ix == question.right))
                        .flex()
                        .items_center()
                        .gap_2()
                        .text_color(tone)
                        .child(
                            div()
                                .flex_none()
                                .size(theme.icon_size(IconSize::Sm))
                                .children(
                                    icon.map(|icon| Icon::new(icon).size(IconSize::Sm).color(tone)),
                                ),
                        )
                        .child(div().flex_1().min_w_0().child(label.clone()))
                });
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .children(rows)
                    .into_any_element()
            }
        };
        let on_score = self.on_score;
        let (advanced, focus) = (sitting.clone(), action.clone());
        let right_answer = question.right;
        let finishing = !done && checked && at + 1 == count;
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .text_size(theme.text_size(TextSize::Sm))
                    .text_color(colors.fg_muted)
                    .child(match done {
                        true => "Score".to_string(),
                        false => format!("Question {} of {count}", at + 1),
                    }),
            )
            .when(!done, |quiz| {
                quiz.child(
                    div().flex().child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_color(colors.fg)
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(question.prompt.clone()),
                    ),
                )
            })
            .child(body)
            .child(
                div().debug_selector(|| "quiz-action".into()).flex().child(
                    Button::new((id.clone(), "action"), label)
                        .variant(ButtonVariant::Primary)
                        .focus_handle(&action)
                        .on_click(move |_, window, cx| {
                            let score = advanced.update(cx, |sitting, cx| {
                                sitting.advance(right_answer, count);
                                cx.notify();
                                sitting.right
                            });
                            log::info!("quiz: {label}");
                            if let (true, Some(on_score)) = (finishing, &on_score) {
                                on_score(score, count, window, cx);
                            }
                            window.focus(&focus, cx);
                        }),
                ),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::Sitting;

    #[test]
    fn one_button_checks_goes_on_scores_and_starts_again() {
        let mut sitting = Sitting::default();
        assert_eq!(sitting.next(2), "Check");
        sitting.advance(1, 2);
        assert!(sitting.nudged, "Check with no pick asks for one");
        sitting.pick(1);
        assert!(!sitting.nudged, "a pick answers the ask");
        sitting.advance(1, 2);
        assert_eq!(
            (sitting.checked, sitting.right, sitting.next(2)),
            (true, 1, "Next")
        );
        sitting.advance(1, 2);
        assert_eq!(
            (sitting.at, sitting.pick, sitting.checked),
            (1, None, false)
        );
        sitting.pick(0);
        sitting.advance(2, 2);
        assert_eq!(
            (sitting.right, sitting.next(2)),
            (1, "See score"),
            "a wrong pick scores nothing"
        );
        sitting.advance(2, 2);
        assert_eq!((sitting.done, sitting.next(2)), (true, "Try again"));
        sitting.advance(2, 2);
        assert_eq!((sitting.at, sitting.right, sitting.done), (0, 0, false));
    }
}
