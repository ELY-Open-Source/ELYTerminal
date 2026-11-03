use std::time::Duration;

use gpui::{
    Animation, AnimationExt, AnyElement, App, ElementId, IntoElement, ParentElement, RenderOnce,
    Styled, Window, div,
};
use smallvec::SmallVec;
use web_time::Instant;

use super::{BASE, NUDGE, duration, ease_out_cubic};
use crate::theme::ActiveTheme;

/// How content comes and goes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Entrance {
    #[default]
    Fade,
    /// Fades while rising from a little below.
    Rise,
    /// Fades while sliding in from the left, and out the same way.
    Slide,
}

/// Where a transition stands: shown or not, since when, and how many times it turned.
struct Turns {
    shown: bool,
    since: Instant,
    turn: usize,
}

/// Shows or hides its content with a fade, a rise or a slide. It keeps drawing the content while it leaves, so pass the content either way.
#[derive(IntoElement)]
pub struct Transition {
    id: ElementId,
    shown: bool,
    entrance: Entrance,
    body: SmallVec<[AnyElement; 2]>,
}

impl Transition {
    pub fn new(id: impl Into<ElementId>, shown: bool) -> Self {
        Self {
            id: id.into(),
            shown,
            entrance: Entrance::default(),
            body: SmallVec::new(),
        }
    }

    pub fn entrance(mut self, entrance: Entrance) -> Self {
        self.entrance = entrance;
        self
    }
}

impl ParentElement for Transition {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.body.extend(elements);
    }
}

/// Moves `element` a share `t` of the way in: 0 is gone, 1 is in place.
pub(super) fn arrive<E: Styled>(element: E, entrance: Entrance, t: f32) -> E {
    let t = ease_out_cubic(t);
    match entrance {
        Entrance::Fade => element.opacity(t),
        // Padding: taffy's size cache drops a collapsed margin.
        Entrance::Rise => element.opacity(t).pt(NUDGE * 2.0 * (1.0 - t)),
        Entrance::Slide => element.opacity(t).ml(NUDGE * -4.0 * (1.0 - t)),
    }
}

impl RenderOnce for Transition {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let shown = self.shown;
        let state = window.use_keyed_state((self.id.clone(), "turns"), cx, |_, _| Turns {
            shown,
            since: Instant::now(),
            turn: 0,
        });
        if state.read(cx).shown != shown {
            state.update(cx, |turns, _| {
                turns.shown = shown;
                turns.since = Instant::now();
                turns.turn += 1;
            });
        }
        let (since, turn) = (state.read(cx).since, state.read(cx).turn);
        let length = duration(BASE, cx);
        let leaving = !shown && turn > 0 && since.elapsed() < length;
        if !shown && !leaving {
            return div().into_any_element();
        }
        let share =
            (since.elapsed().as_secs_f32() / length.as_secs_f32().max(f32::EPSILON)).min(1.0);
        if turn > 0 && share < 1.0 {
            window.request_animation_frame();
        }
        let content = div().children(self.body);
        match turn {
            0 => content.into_any_element(),
            _ => arrive(
                content,
                self.entrance,
                if shown { share } else { 1.0 - share },
            )
            .into_any_element(),
        }
    }
}

/// Children that enter one after another, `step` apart, each arriving once.
#[derive(IntoElement)]
pub struct Stagger {
    id: ElementId,
    step: Duration,
    entrance: Entrance,
    body: SmallVec<[AnyElement; 4]>,
}

impl Stagger {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            step: Duration::from_millis(60),
            entrance: Entrance::Rise,
            body: SmallVec::new(),
        }
    }

    /// The gap between two children's starts.
    pub fn step(mut self, step: Duration) -> Self {
        self.step = step;
        self
    }

    pub fn entrance(mut self, entrance: Entrance) -> Self {
        self.entrance = entrance;
        self
    }
}

impl ParentElement for Stagger {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.body.extend(elements);
    }
}

impl RenderOnce for Stagger {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let length = duration(BASE, cx);
        let step = if cx.theme().reduced_motion {
            Duration::ZERO
        } else {
            self.step
        };
        let (id, entrance) = (self.id, self.entrance);
        div().children(self.body.into_iter().enumerate().map(move |(ix, child)| {
            let lead = step * ix as u32;
            let total = lead + length;
            div().child(child).with_animation(
                (id.clone(), format!("child-{ix}")),
                Animation::new(total),
                move |child, t| {
                    let at = (t * total.as_secs_f32() - lead.as_secs_f32()) / length.as_secs_f32();
                    arrive(child, entrance, at.clamp(0.0, 1.0))
                },
            )
        }))
    }
}
