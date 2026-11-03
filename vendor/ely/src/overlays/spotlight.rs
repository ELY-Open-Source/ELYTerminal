use std::{cell::RefCell, rc::Rc};

use gpui::{
    Animation, AnimationExt, AnyElement, App, Bounds, ElementId, FontWeight, InteractiveElement,
    IntoElement, ParentElement, Pixels, Point, RenderOnce, SharedString, Styled, Subscription,
    Window, anchored, canvas, div, point, prelude::*, size,
};

use crate::{
    buttons::{Button, ButtonVariant},
    forms::{Run, float_height, surface},
    motion,
    primitives::{FocusScope, give_back, raise, take_focus},
    theme::{ActiveTheme, Radius, TextSize},
};

/// One stop of a tour: the box to light, a title and a line.
#[derive(Clone, Debug)]
pub struct TourStep {
    target: Bounds<Pixels>,
    title: SharedString,
    body: SharedString,
}

impl TourStep {
    /// `target` is window coordinates, as `primitives::Measure` reports them.
    pub fn new(
        target: Bounds<Pixels>,
        title: impl Into<SharedString>,
        body: impl Into<SharedString>,
    ) -> Self {
        Self {
            target,
            title: title.into(),
            body: body.into(),
        }
    }
}

/// Escape from anywhere in the window, the lit target included, closes through the latest close.
struct Watch {
    close: Rc<RefCell<Run>>,
    _escape: Subscription,
}

/// Where the lit box glides from, and the card's measured height.
#[derive(Default)]
struct Lit {
    from: Bounds<Pixels>,
    to: Bounds<Pixels>,
    card: Pixels,
}

fn lerp(from: Bounds<Pixels>, to: Bounds<Pixels>, t: f32) -> Bounds<Pixels> {
    let mix = |a: Pixels, b: Pixels| a + (b - a) * t;
    Bounds::new(
        point(
            mix(from.origin.x, to.origin.x),
            mix(from.origin.y, to.origin.y),
        ),
        size(
            mix(from.size.width, to.size.width),
            mix(from.size.height, to.size.height),
        ),
    )
}

/// Dims the window around the lit box, rings it, and floats `card` beside it. The box glides when the target moves.
fn light(
    id: &ElementId,
    target: Bounds<Pixels>,
    card: AnyElement,
    close: Run,
    keys: impl Fn(&str, &mut Window, &mut App) -> bool + 'static,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let gap = cx.theme().float_gap().to_pixels(window.rem_size());
    let target = target.dilate(gap);
    let state = window.use_keyed_state((id.clone(), "lit"), cx, |_, _| Lit {
        from: target,
        to: target,
        card: Pixels::ZERO,
    });
    if state.read(cx).to != target {
        state.update(cx, |lit, _| {
            lit.from = lit.to;
            lit.to = target;
        });
    }
    let (from, card_height) = (state.read(cx).from, state.read(cx).card);
    let turn = motion::changes((id.clone(), "target"), target, window, cx);
    let theme = cx.theme();
    let viewport = window.viewport_size();
    let colors = theme.colors.clone();
    let radius = theme.radius(Radius::Md);
    let frame = div().size_full().with_animation(
        (id.clone(), format!("glide-{turn}")),
        Animation::new(motion::duration(motion::SLOW, cx)).with_easing(motion::ease_in_out_cubic),
        move |frame, t| {
            let lit = if turn == 0 {
                target
            } else {
                lerp(from, target, t)
            };
            let dim = |left: Pixels, top: Pixels, width: Pixels, height: Pixels| {
                div()
                    .absolute()
                    .left(left)
                    .top(top)
                    .w(width.max(Pixels::ZERO))
                    .h(height.max(Pixels::ZERO))
                    .bg(colors.backdrop)
                    .occlude()
            };
            frame
                .child(dim(Pixels::ZERO, Pixels::ZERO, viewport.width, lit.top()))
                .child(dim(
                    Pixels::ZERO,
                    lit.bottom(),
                    viewport.width,
                    viewport.height - lit.bottom(),
                ))
                .child(dim(Pixels::ZERO, lit.top(), lit.left(), lit.size.height))
                .child(dim(
                    lit.right(),
                    lit.top(),
                    viewport.width - lit.right(),
                    lit.size.height,
                ))
                .child(
                    div()
                        .absolute()
                        .left(lit.left())
                        .top(lit.top())
                        .w(lit.size.width)
                        .h(lit.size.height)
                        .rounded(radius)
                        .border_2()
                        .border_color(colors.accent),
                )
        },
    );
    let measure = state.clone();
    let watch = window.use_keyed_state((id.clone(), "escape"), cx, |window, cx| {
        let latest = Rc::new(RefCell::new(close.clone()));
        let (home, current) = (window.window_handle(), latest.clone());
        let _escape = cx.intercept_keystrokes(move |event, window, cx| {
            let stroke = &event.keystroke;
            if window.window_handle() == home
                && stroke.key == "escape"
                && !stroke.modifiers.modified()
            {
                cx.stop_propagation();
                let close = current.borrow().clone();
                close(window, cx);
            }
        });
        Watch {
            close: latest,
            _escape,
        }
    });
    *watch.read(cx).close.borrow_mut() = close;
    let card = div()
        .id((id.clone(), "card"))
        .relative()
        .occlude()
        .on_key_down(move |event, window, cx| {
            if keys(event.keystroke.key.as_str(), window, cx) {
                cx.stop_propagation();
            }
        })
        .child(card)
        .child(
            canvas(
                move |bounds, window, cx| {
                    if measure.read(cx).card != bounds.size.height {
                        measure.update(cx, |lit, cx| {
                            lit.card = bounds.size.height;
                            cx.notify();
                            window.request_animation_frame();
                        });
                    }
                },
                |_, _, _, _| {},
            )
            .absolute()
            .top_0()
            .left_0()
            .size_full(),
        );
    div()
        .child(
            raise(
                (id.clone(), "raised"),
                anchored()
                    .position(Point::default())
                    .child(div().w(viewport.width).h(viewport.height).child(frame)),
            )
            .with_priority(1),
        )
        .child(float_height(
            (id.clone(), "card"),
            target,
            card_height,
            card,
            window,
            cx,
        ))
        .into_any_element()
}

/// The card's words: a title, then a line.
fn words(title: SharedString, body: SharedString, cx: &App) -> impl IntoElement {
    let theme = cx.theme();
    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(div().font_weight(FontWeight::SEMIBOLD).child(title))
        .child(
            div()
                .text_size(theme.text_size(TextSize::Sm))
                .text_color(theme.colors.fg_muted)
                .child(body),
        )
}

/// Dims the window around one box and sets a card beside it: a title, a line and one button. Escape or the button closes it.
#[derive(IntoElement)]
pub struct Spotlight {
    id: ElementId,
    step: TourStep,
    label: SharedString,
    on_close: Run,
}

impl Spotlight {
    pub fn new(
        id: impl Into<ElementId>,
        step: TourStep,
        on_close: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            step,
            label: "Got it".into(),
            on_close: Rc::new(on_close),
        }
    }

    /// The button's label, Got it unless set.
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = label.into();
        self
    }
}

impl RenderOnce for Spotlight {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let takeover = take_focus((self.id.clone(), "takeover"), window, cx);
        let focus = takeover.read(cx).focus.clone();
        let close: Run = {
            let (id, on_close) = (self.id.clone(), self.on_close);
            Rc::new(move |window, cx| {
                log::info!("spotlight {id:?}: closed");
                give_back(&takeover, window, cx);
                on_close(window, cx)
            })
        };
        let button = close.clone();
        let card = FocusScope::new(&focus).trap().child(
            surface((self.id.clone(), "surface"), cx)
                .p_4()
                .flex()
                .flex_col()
                .gap_3()
                .max_w(cx.theme().dialog_width())
                .child(words(self.step.title, self.step.body, cx))
                .child(
                    div().flex().justify_end().child(
                        Button::new((self.id.clone(), "done"), self.label)
                            .primary()
                            .on_click(move |_, window, cx| button(window, cx)),
                    ),
                ),
        );
        light(
            &self.id,
            self.step.target,
            card.into_any_element(),
            close,
            |_, _, _| false,
            window,
            cx,
        )
    }
}

type OnStep = Rc<dyn Fn(usize, &mut Window, &mut App)>;

/// Spotlights one step after another: Back, Next and Done, the count, and Skip. Left and Right step too. The lit box glides between targets.
#[derive(IntoElement)]
pub struct Tour {
    id: ElementId,
    steps: Vec<TourStep>,
    at: usize,
    on_step: Option<OnStep>,
    on_close: Run,
}

impl Tour {
    /// Render it while open; the owner keeps `at` and moves it in `on_step`. Done and Skip both close.
    pub fn new(
        id: impl Into<ElementId>,
        steps: impl IntoIterator<Item = TourStep>,
        at: usize,
        on_close: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            steps: steps.into_iter().collect(),
            at,
            on_step: None,
            on_close: Rc::new(on_close),
        }
    }

    pub fn on_step(mut self, handler: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_step = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for Tour {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let (at, count) = (self.at, self.steps.len());
        let takeover = take_focus((self.id.clone(), "takeover"), window, cx);
        let focus = takeover.read(cx).focus.clone();
        let close: Run = {
            let (id, on_close) = (self.id.clone(), self.on_close);
            Rc::new(move |window, cx| {
                log::info!("tour {id:?}: closed at step {at}");
                give_back(&takeover, window, cx);
                on_close(window, cx)
            })
        };
        let step: OnStep = {
            let (id, on_step) = (self.id.clone(), self.on_step);
            Rc::new(move |to, window, cx| {
                log::info!("tour {id:?}: step {to}");
                if let Some(on_step) = &on_step {
                    on_step(to, window, cx);
                }
            })
        };
        if at >= count {
            log::error!("tour {:?}: step {at} of {count} is gone; closing", self.id);
            window.defer(cx, move |window, cx| close(window, cx));
            return div().into_any_element();
        }
        let (back, next) = (at.checked_sub(1), (at + 1 < count).then_some(at + 1));
        let current = self.steps[at].clone();
        let turn = motion::changes((self.id.clone(), "at"), at, window, cx);
        let theme = cx.theme();
        let (skip, done, back_step, next_step, keyed) = (
            close.clone(),
            close.clone(),
            step.clone(),
            step.clone(),
            step,
        );
        let footer = div()
            .flex()
            .items_center()
            .gap_2()
            .child(
                div()
                    .flex_1()
                    .text_size(theme.text_size(TextSize::Xs))
                    .text_color(theme.colors.fg_subtle)
                    .child(format!("{} of {count}", at + 1)),
            )
            .child(
                Button::new((self.id.clone(), "skip"), "Skip")
                    .variant(ButtonVariant::Ghost)
                    .on_click(move |_, window, cx| skip(window, cx)),
            )
            .when_some(back, |row, to| {
                row.child(
                    Button::new((self.id.clone(), "back"), "Back")
                        .on_click(move |_, window, cx| back_step(to, window, cx)),
                )
            })
            .child(
                Button::new(
                    (self.id.clone(), "primary"),
                    if next.is_some() { "Next" } else { "Done" },
                )
                .primary()
                .on_click(move |_, window, cx| match next {
                    Some(to) => next_step(to, window, cx),
                    None => done(window, cx),
                }),
            );
        let card = FocusScope::new(&focus).trap().child(
            surface((self.id.clone(), "surface"), cx)
                .p_4()
                .flex()
                .flex_col()
                .gap_4()
                .max_w(theme.dialog_width())
                .child(
                    div()
                        .child(words(current.title, current.body, cx))
                        .with_animation(
                            (self.id.clone(), format!("words-{turn}")),
                            Animation::new(motion::duration(motion::BASE, cx))
                                .with_easing(motion::ease_out_cubic),
                            move |words, t| if turn == 0 { words } else { words.opacity(t) },
                        ),
                )
                .child(footer),
        );
        let keys = move |key: &str, window: &mut Window, cx: &mut App| {
            let to = match key {
                "left" => back,
                "right" => next,
                _ => return false,
            };
            if let Some(to) = to {
                keyed(to, window, cx);
            }
            true
        };
        light(
            &self.id,
            current.target,
            card.into_any_element(),
            close,
            keys,
            window,
            cx,
        )
    }
}
