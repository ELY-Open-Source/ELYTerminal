use std::rc::Rc;

use gpui::{
    Animation, AnimationExt, AnyElement, App, ElementId, FontWeight, InteractiveElement,
    IntoElement, ParentElement, RenderOnce, StatefulInteractiveElement, Styled, Window, div,
    prelude::*, relative,
};

use crate::{
    buttons::{Button, ButtonVariant},
    forms::{Choice, Run},
    motion,
    primitives::{FocusRing, Icon, IconName, tab_stop},
    theme::{ActiveTheme, ControlSize, IconSize, TextSize},
};

type OnStep = Rc<dyn Fn(usize, &mut Window, &mut App)>;

/// Numbered steps joined by lines. Finished steps carry a check and can be revisited.
#[derive(IntoElement)]
pub struct Steps {
    id: ElementId,
    steps: Vec<Choice>,
    current: usize,
    vertical: bool,
    on_select: Option<OnStep>,
}

impl Steps {
    /// Each step's label is its title, its note a line of detail.
    pub fn new(
        id: impl Into<ElementId>,
        steps: impl IntoIterator<Item = Choice>,
        current: usize,
    ) -> Self {
        Self {
            id: id.into(),
            steps: steps.into_iter().collect(),
            current,
            vertical: false,
            on_select: None,
        }
    }

    pub fn vertical(mut self) -> Self {
        self.vertical = true;
        self
    }

    /// Runs when a finished step is clicked.
    pub fn on_select(mut self, handler: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_select = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for Steps {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id.clone();
        let turn = motion::changes((id.clone(), "current"), self.current, window, cx);
        let (current, vertical, count) = (self.current, self.vertical, self.steps.len());
        let valid = current < count;
        if !valid {
            log::error!("steps {id:?}: step {current} of {count}; none marked");
        }
        let duration = motion::duration(motion::SLOW, cx);
        let theme = cx.theme();
        let colors = &theme.colors;
        let mark = theme.control_height(ControlSize::Sm);
        let line = |ix: usize| {
            let filled = valid && ix < current;
            let fill = div().bg(colors.accent).map(|fill| {
                if vertical {
                    fill.w_full()
                } else {
                    fill.h_full()
                }
            });
            let fill = if filled && ix + 1 == current && turn > 0 {
                fill.with_animation(
                    (id.clone(), format!("fill-{turn}")),
                    Animation::new(duration).with_easing(motion::ease_out_cubic),
                    move |fill, t| {
                        if vertical {
                            fill.h(relative(t))
                        } else {
                            fill.w(relative(t))
                        }
                    },
                )
                .into_any_element()
            } else {
                let size = relative(if filled { 1.0 } else { 0.0 });
                (if vertical { fill.h(size) } else { fill.w(size) }).into_any_element()
            };
            div()
                .flex_1()
                .bg(colors.border)
                .map(|track| if vertical { track.w_px() } else { track.h_px() })
                .child(fill)
        };
        let steps = self.steps.into_iter().enumerate().map(|(ix, step)| {
            let (done, now) = (valid && ix < current, valid && ix == current);
            let circle = div()
                .flex()
                .flex_none()
                .items_center()
                .justify_center()
                .size(mark)
                .rounded_full()
                .text_size(theme.text_size(TextSize::Xs))
                .font_weight(FontWeight::SEMIBOLD)
                .map(|circle| {
                    if done {
                        circle.bg(colors.accent).child(
                            Icon::new(IconName::Check)
                                .size(IconSize::Xs)
                                .color(colors.on_accent),
                        )
                    } else if now {
                        circle
                            .border_2()
                            .border_color(colors.accent)
                            .text_color(colors.fg)
                            .child((ix + 1).to_string())
                    } else {
                        circle
                            .border_1()
                            .border_color(colors.border_strong)
                            .text_color(colors.fg_subtle)
                            .child((ix + 1).to_string())
                    }
                });
            let words = div()
                .flex()
                .flex_col()
                .gap_0p5()
                .text_size(theme.text_size(TextSize::Sm))
                .text_color(if done || now {
                    colors.fg
                } else {
                    colors.fg_muted
                })
                .when(now, |words| words.font_weight(FontWeight::MEDIUM))
                .child(step.label)
                .when_some(step.note, |words, note| {
                    words.child(
                        div()
                            .text_size(theme.text_size(TextSize::Xs))
                            .text_color(colors.fg_subtle)
                            .font_weight(FontWeight::NORMAL)
                            .child(note),
                    )
                });
            let select = self.on_select.clone().filter(|_| done);
            let head = div()
                .id((id.clone(), format!("step-{ix}")))
                .flex()
                .items_center()
                .gap_2()
                .rounded_full()
                .border_1()
                .border_color(gpui::transparent_black())
                .when_some(select, |head, select| {
                    head.cursor_pointer().tab_index(0).focus_ring(cx).on_click(
                        move |_, window, cx| {
                            log::info!("steps: back to {ix}");
                            select(ix, window, cx)
                        },
                    )
                });
            let after = (ix + 1 < count).then(|| line(ix));
            if vertical {
                div()
                    .flex()
                    .gap_3()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .items_center()
                            .gap_1()
                            .child(head.child(circle))
                            .children(after),
                    )
                    .child(div().pb_5().child(words))
                    .into_any_element()
            } else {
                div()
                    .flex()
                    .flex_1()
                    .items_center()
                    .gap_3()
                    .when(ix + 1 == count, |step| step.flex_grow_0().flex_shrink_0())
                    .child(head.child(circle).child(words))
                    .children(after)
                    .into_any_element()
            }
        });
        div()
            .debug_selector(|| "steps-root".into())
            .id(self.id)
            .flex()
            .when(vertical, |steps| steps.flex_col())
            .children(steps)
    }
}

/// Steps, the current step's content, then Back and Next. Next becomes Finish on the last step, keeping its focus; focus moves to it when Back reaches the first step or a finished step is picked.
#[derive(IntoElement)]
pub struct Wizard {
    id: ElementId,
    steps: Vec<Choice>,
    current: usize,
    content: Option<AnyElement>,
    ready: bool,
    headless: bool,
    on_step: OnStep,
    on_finish: Run,
}

impl Wizard {
    /// `on_step` runs with the step Back, Next or a finished step leads to; `on_finish` with Finish.
    pub fn new(
        id: impl Into<ElementId>,
        steps: impl IntoIterator<Item = Choice>,
        current: usize,
        on_step: impl Fn(usize, &mut Window, &mut App) + 'static,
        on_finish: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            steps: steps.into_iter().collect(),
            current,
            content: None,
            ready: true,
            headless: false,
            on_step: Rc::new(on_step),
            on_finish: Rc::new(on_finish),
        }
    }

    /// Leaves out the numbered steps; the owner shows where the flow stands.
    pub(crate) fn headless(mut self) -> Self {
        self.headless = true;
        self
    }

    /// The current step's content.
    pub fn content(mut self, content: impl IntoElement) -> Self {
        self.content = Some(content.into_any_element());
        self
    }

    /// Whether Next may run; the step's fields decide.
    pub fn ready(mut self, ready: bool) -> Self {
        self.ready = ready;
        self
    }
}

impl RenderOnce for Wizard {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let (current, last) = (self.current, self.steps.len().saturating_sub(1));
        let valid = current < self.steps.len();
        if !valid {
            log::error!(
                "wizard {:?}: step {current} of {}",
                self.id,
                self.steps.len()
            );
        }
        let turn = motion::changes((self.id.clone(), "content"), current, window, cx);
        let id = self.id.clone();
        let (on_step, on_finish) = (self.on_step, self.on_finish);
        let step: OnStep = {
            let id = id.clone();
            Rc::new(move |to, window, cx| {
                log::info!("wizard {id:?}: step {to}");
                on_step(to, window, cx);
            })
        };
        let advance = tab_stop((id.clone(), "advance").into(), true, window, cx);
        let (back, next, jump) = (step.clone(), step.clone(), step);
        let (to_advance, jumped) = (advance.clone(), advance.clone());
        let finish = {
            let id = id.clone();
            move |window: &mut Window, cx: &mut App| {
                log::info!("wizard {id:?}: finished");
                on_finish(window, cx);
            }
        };
        let content = self.content.map(|content| {
            div().child(content).with_animation(
                (id.clone(), format!("step-{turn}")),
                Animation::new(motion::duration(motion::BASE, cx))
                    .with_easing(motion::ease_out_cubic),
                move |content, t| {
                    if turn == 0 {
                        content
                    } else {
                        content.opacity(t).mt(motion::NUDGE * (1.0 - t))
                    }
                },
            )
        });
        let forward = Button::new(
            (id.clone(), "advance"),
            if current == last { "Finish" } else { "Next" },
        )
        .primary()
        .disabled(!valid || !self.ready)
        .focus_handle(&advance)
        .on_click(move |_, window, cx| match current == last {
            true => finish(window, cx),
            false => next(current + 1, window, cx),
        });
        div()
            .id(self.id.clone())
            .flex()
            .flex_col()
            .gap_6()
            .when(!self.headless, |wizard| {
                wizard.child(
                    Steps::new((id.clone(), "steps"), self.steps, current).on_select(
                        move |to, window, cx| {
                            window.focus(&jumped, cx);
                            jump(to, window, cx)
                        },
                    ),
                )
            })
            .children(content)
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        Button::new((id.clone(), "back"), "Back")
                            .variant(ButtonVariant::Ghost)
                            .disabled(!valid || current == 0)
                            .on_click(move |_, window, cx| {
                                if current == 1 {
                                    window.focus(&to_advance, cx);
                                }
                                back(current - 1, window, cx)
                            }),
                    )
                    .child(forward),
            )
    }
}
