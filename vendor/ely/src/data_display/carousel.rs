use std::{rc::Rc, time::Duration};

use gpui::{
    AnyElement, App, Div, ElementId, HoverListenerMode, InteractiveElement, IntoElement,
    MouseButton, ParentElement, RenderOnce, StatefulInteractiveElement, StyleRefinement, Styled,
    Task, Window, div, prelude::*, relative,
};
use web_time::Instant;

use crate::{
    motion,
    primitives::{FocusRing, Icon, IconName, tab_stop},
    theme::{ActiveTheme, ControlSize, IconSize, Mix},
};

/// Where the deck heads, counted past the ends so a wrap glides one step, where the glide began, and when.
struct Deck {
    at: isize,
    from: f32,
    since: Instant,
    length: Duration,
    hovered: bool,
    focused: bool,
    shown: bool,
    shown_since: Instant,
    every: Option<Duration>,
    _ticker: Option<Task<()>>,
}

impl Deck {
    fn place(&self) -> f32 {
        let share = (self.since.elapsed().as_secs_f32() / self.length.as_secs_f32()).min(1.0);
        self.from + (self.at as f32 - self.from) * motion::ease_in_out_cubic(share)
    }

    fn go(&mut self, to: isize) {
        self.from = self.place();
        self.at = to;
        self.since = Instant::now();
    }
}

/// How far slide `ix` sits from `place`, in slides, on the nearer side of a ring of `count`.
fn offset(ix: usize, place: f32, count: usize) -> f32 {
    let count = count as f32;
    let ahead = (ix as f32 - place).rem_euclid(count);
    if ahead > count / 2.0 {
        ahead - count
    } else {
        ahead
    }
}

type Step = Rc<dyn Fn(isize, &mut App)>;

/// Slides one at a time, gliding between them. Arrows show on hover, dots below say where you are, and Left and Right step when it has focus. Past the last slide comes the first. Give it a height.
#[derive(IntoElement)]
pub struct Carousel {
    id: ElementId,
    base: Div,
    slides: Vec<AnyElement>,
    autoplay: Option<Duration>,
}

impl Carousel {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            base: div(),
            slides: Vec::new(),
            autoplay: None,
        }
    }

    /// Steps on its own every `every`, holding while pointed at or focused.
    pub fn autoplay(mut self, every: Duration) -> Self {
        self.autoplay = Some(every);
        self
    }
}

impl Styled for Carousel {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl ParentElement for Carousel {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.slides.extend(elements);
    }
}

impl RenderOnce for Carousel {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let count = self.slides.len();
        assert!(count > 0, "carousel {:?} has no slides", self.id);
        let focus = tab_stop((self.id.clone(), "focus").into(), true, window, cx);
        let deck = window.use_keyed_state((self.id.clone(), "deck"), cx, |_, _| Deck {
            at: 0,
            from: 0.0,
            since: Instant::now(),
            length: motion::SLOW,
            hovered: false,
            focused: false,
            shown: false,
            shown_since: Instant::now(),
            every: None,
            _ticker: None,
        });
        let (length, fade) = (
            motion::duration(motion::SLOW, cx),
            motion::duration(motion::FAST, cx),
        );
        let focused = focus.contains_focused(window, cx);
        let every = self.autoplay.filter(|_| count > 1);
        let name = self.id.clone();
        deck.update(cx, |deck, cx| {
            deck.length = length;
            deck.focused = focused;
            let shown = deck.hovered || focused;
            if deck.shown != shown {
                deck.shown = shown;
                deck.shown_since = Instant::now();
            }
            if deck.every != every {
                deck.every = every;
                deck._ticker = every.map(|every| {
                    cx.spawn(async move |deck, cx| {
                        loop {
                            cx.background_executor().timer(every).await;
                            let turned = deck.update(cx, |deck, cx| {
                                if !deck.hovered && !deck.focused {
                                    deck.go(deck.at + 1);
                                    log::debug!("carousel {name:?}: autoplay to {}", deck.at);
                                    cx.notify();
                                }
                            });
                            if turned.is_err() {
                                return;
                            }
                        }
                    })
                });
            }
        });
        let (place, current, arrows, moving) = {
            let deck = deck.read(cx);
            let fading = (deck.shown_since.elapsed().as_secs_f32() / fade.as_secs_f32()).min(1.0);
            let arrows = if deck.shown { fading } else { 1.0 - fading };
            let moving = deck.since.elapsed() < length || fading < 1.0;
            (
                deck.place(),
                deck.at.rem_euclid(count as isize) as usize,
                arrows,
                moving,
            )
        };
        if moving {
            window.request_animation_frame();
        }
        let step: Step = {
            let (deck, id) = (deck.clone(), self.id.clone());
            Rc::new(move |delta, cx| {
                deck.update(cx, |deck, cx| {
                    deck.go(deck.at + delta);
                    log::info!(
                        "carousel {id:?}: slide {}",
                        deck.at.rem_euclid(count as isize)
                    );
                    cx.notify();
                })
            })
        };
        let theme = cx.theme();
        let colors = &theme.colors;
        let button = theme.control_height(ControlSize::Lg);
        let dot = theme.status_dot();
        let arrow = |name: &'static str, icon: IconName, delta: isize| {
            let step = step.clone();
            div()
                .id((self.id.clone(), name))
                .absolute()
                .top(relative(0.5))
                .mt(button * -0.5)
                .flex()
                .items_center()
                .justify_center()
                .size(button)
                .rounded_full()
                .border_1()
                .border_color(colors.border)
                .bg(colors.surface)
                .opacity(arrows)
                .when(arrows > 0.0, |arrow| {
                    arrow
                        .cursor_pointer()
                        .hover(|style| style.bg(colors.hover))
                        .on_mouse_down(MouseButton::Left, |_, window, cx| {
                            window.prevent_default();
                            cx.stop_propagation();
                        })
                        .on_click(move |_, _, cx| step(delta, cx))
                })
                .child(Icon::new(icon).size(IconSize::Sm).color(colors.fg))
        };
        let keys = step.clone();
        let viewport = div()
            .id((self.id.clone(), "viewport"))
            .relative()
            .flex_1()
            .min_h_0()
            .w_full()
            .overflow_hidden()
            .border_1()
            .border_color(gpui::transparent_black())
            .track_focus(&focus)
            .focus_ring(cx)
            .when(count > 1, |viewport| {
                viewport.on_key_down(move |event, _, cx| {
                    let delta = match event.keystroke.key.as_str() {
                        "left" => -1,
                        "right" => 1,
                        _ => return,
                    };
                    cx.stop_propagation();
                    keys(delta, cx);
                })
            })
            .children(self.slides.into_iter().enumerate().map(|(ix, slide)| {
                div()
                    .absolute()
                    .top_0()
                    .bottom_0()
                    .w_full()
                    .left(relative(offset(ix, place, count)))
                    .child(slide)
            }))
            .when(count > 1, |viewport| {
                viewport
                    .child(arrow("back", IconName::ChevronLeft, -1).left_3())
                    .child(arrow("next", IconName::ChevronRight, 1).right_3())
            });
        let dots: Vec<_> = (0..count)
            .map(|ix| {
                let near = (1.0 - offset(ix, place, count).abs()).max(0.0);
                let step = step.clone();
                div()
                    .id((self.id.clone(), format!("dot-{ix}")))
                    .py_1()
                    .cursor_pointer()
                    .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
                    .on_click(move |_, _, cx| step(ix as isize - current as isize, cx))
                    .child(
                        div()
                            .h(dot)
                            .w(dot + dot * 2.0 * near)
                            .rounded_full()
                            .bg(colors.fg_subtle.mix(&colors.fg, near)),
                    )
            })
            .collect();
        let hover = deck.clone();
        self.base
            .id(self.id)
            .flex()
            .flex_col()
            .gap_2()
            .hover_listener_mode(HoverListenerMode::InputModalityIndependent)
            .on_hover(move |hovered, _, cx| {
                hover.update(cx, |deck, cx| {
                    deck.hovered = *hovered;
                    cx.notify();
                })
            })
            .child(viewport)
            .when(count > 1, |carousel| {
                carousel.child(div().flex().justify_center().gap_1p5().children(dots))
            })
    }
}

#[cfg(test)]
mod tests {
    use super::offset;

    #[test]
    fn slides_sit_on_the_nearer_side_of_the_ring() {
        assert_eq!(offset(0, 0.0, 3), 0.0);
        assert_eq!(offset(1, 0.0, 3), 1.0);
        assert_eq!(offset(2, 0.0, 3), -1.0);
        assert_eq!(
            offset(0, 2.5, 3),
            0.5,
            "past the last, the first enters from the right"
        );
        assert_eq!(offset(2, 2.5, 3), -0.5);
        assert_eq!(offset(1, 0.5, 2), 0.5);
        assert_eq!(offset(0, 0.5, 2), -0.5);
    }
}
