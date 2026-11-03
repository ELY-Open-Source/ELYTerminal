use std::rc::Rc;

use gpui::{
    App, ElementId, FontWeight, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    SharedString, Styled, Window, div, prelude::*,
};

use crate::{
    buttons::{Button, ButtonVariant},
    forms::{Choice, RadioGroup},
    motion::ProgressBar,
    primitives::{Icon, IconName, tab_stop},
    theme::{ActiveTheme, IconSize, TextSize},
    typography::{Ellipsis, format, tabular},
};

type OnVote = Rc<dyn Fn(Option<usize>, &mut Window, &mut App)>;

/// A question with options to vote on, one each. Once the owner holds the vote, the options turn to results: each one's share of the votes as a bar, the vote marked, and the count. Vote and Change vote share one focus handle.
#[derive(IntoElement)]
pub struct Poll {
    id: ElementId,
    question: SharedString,
    options: Vec<SharedString>,
    votes: Vec<u64>,
    voted: Option<usize>,
    on_vote: OnVote,
}

impl Poll {
    pub fn new(
        id: impl Into<ElementId>,
        question: impl Into<SharedString>,
        options: impl IntoIterator<Item = impl Into<SharedString>>,
        on_vote: impl Fn(Option<usize>, &mut Window, &mut App) + 'static,
    ) -> Self {
        let options: Vec<SharedString> = options.into_iter().map(Into::into).collect();
        Self {
            id: id.into(),
            question: question.into(),
            votes: vec![0; options.len()],
            options,
            voted: None,
            on_vote: Rc::new(on_vote),
        }
    }

    /// Each option's count, in order.
    pub fn votes(mut self, votes: impl IntoIterator<Item = u64>) -> Self {
        self.votes = votes.into_iter().collect();
        self
    }

    /// The option this viewer voted for, as the owner holds it.
    pub fn voted(mut self, option: usize) -> Self {
        self.voted = Some(option);
        self
    }
}

impl RenderOnce for Poll {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id;
        let count = self.options.len();
        assert!(count > 0, "poll {id:?} has no options");
        assert_eq!(
            self.votes.len(),
            count,
            "poll {id:?} has {} counts for {count} options",
            self.votes.len()
        );
        let voted = self.voted.filter(|voted| {
            let held = *voted < count;
            if !held {
                log::error!("poll {id:?}: vote {voted} of {count} options; none counted");
            }
            held
        });
        let on_vote = self.on_vote;
        let pick = window.use_keyed_state((id.clone(), "pick"), cx, |_, _| None::<usize>);
        if voted.is_some() && *pick.read(cx) != voted {
            pick.update(cx, |pick, _| *pick = voted);
        }
        let action = tab_stop((id.clone(), "action").into(), true, window, cx);
        let theme = cx.theme();
        let colors = &theme.colors;
        let picked = pick.read(cx).filter(|picked| *picked < count);
        let body =
            match voted {
                None => {
                    let choices = self
                        .options
                        .iter()
                        .enumerate()
                        .map(|(ix, label)| Choice::new(ix.to_string(), label.clone()));
                    let chose = pick.clone();
                    let group = RadioGroup::new((id.clone(), "options"), choices).on_change(
                        move |value, _, cx| {
                            let ix = value.parse().expect("an option's place");
                            chose.update(cx, |pick, cx| {
                                *pick = Some(ix);
                                cx.notify();
                            })
                        },
                    );
                    match picked {
                        Some(ix) => group.selected(ix.to_string()),
                        None => group,
                    }
                    .into_any_element()
                }
                Some(voted) => {
                    let total: u64 = self.votes.iter().sum();
                    let rows = self.options.iter().zip(&self.votes).enumerate().map(
                        |(ix, (label, votes))| {
                            let share = match total {
                                0 => 0.0,
                                _ => *votes as f32 / total as f32,
                            };
                            let percent = format::percent(share as f64, 0, false);
                            let marked = ix == voted;
                            div()
                                .debug_selector(|| match marked {
                                    true => format!("poll-{label}-{percent}-voted"),
                                    false => format!("poll-{label}-{percent}"),
                                })
                                .flex()
                                .flex_col()
                                .gap_1()
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap_2()
                                        .child(
                                            div()
                                                .flex_1()
                                                .min_w_0()
                                                .text_color(colors.fg)
                                                .child(Ellipsis::new(label.clone())),
                                        )
                                        .when(marked, |row| {
                                            row.child(
                                                Icon::new(IconName::CircleCheck)
                                                    .size(IconSize::Sm)
                                                    .color(colors.accent),
                                            )
                                        })
                                        .child(
                                            tabular(div())
                                                .flex_none()
                                                .text_color(colors.fg_muted)
                                                .child(percent.clone()),
                                        ),
                                )
                                .child(ProgressBar::new((id.clone(), format!("bar-{ix}")), share))
                        },
                    );
                    div()
                        .flex()
                        .flex_col()
                        .gap_3()
                        .children(rows)
                        .child(
                            div()
                                .text_size(theme.text_size(TextSize::Sm))
                                .text_color(colors.fg_muted)
                                .child(format::plural(total, "vote", "votes")),
                        )
                        .into_any_element()
                }
            };
        let button = match voted {
            None => {
                let (focus, on_vote) = (action.clone(), on_vote.clone());
                Button::new((id.clone(), "vote"), "Vote")
                    .variant(ButtonVariant::Primary)
                    .focus_handle(&action)
                    .disabled(picked.is_none())
                    .on_click(move |_, window, cx| {
                        let ix = picked.expect("Vote waits for a pick");
                        log::info!("poll: voted for option {ix}");
                        on_vote(Some(ix), window, cx);
                        window.focus(&focus, cx);
                    })
            }
            Some(_) => Button::new((id.clone(), "change"), "Change vote")
                .focus_handle(&action)
                .on_click(move |_, window, cx| {
                    log::info!("poll: vote taken back");
                    on_vote(None, window, cx);
                }),
        };
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div().flex().child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .text_color(colors.fg)
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(self.question),
                ),
            )
            .child(body)
            .child(
                div()
                    .debug_selector(|| "poll-action".into())
                    .flex()
                    .child(button),
            )
    }
}
