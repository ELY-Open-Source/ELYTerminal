use std::{
    f32::consts::{PI, TAU},
    time::Duration,
};

use gpui::{
    Animation, AnimationExt, AnyElement, App, BoxShadow, Div, ElementId, Hsla, IntoElement,
    ParentElement, Pixels, RenderOnce, StyleRefinement, Styled, Window, canvas, div, point,
    prelude::*,
};
use smallvec::SmallVec;
use web_time::Instant;

use super::{NUDGE, SLOW, duration, ease_out_cubic};
use crate::theme::ActiveTheme;

/// A caret's rhythm, on then off.
const BLINK: Duration = Duration::from_millis(1_060);
/// How long a flash takes to fade.
const FLASH: Duration = Duration::from_millis(900);
/// How long a shake lasts, and how far it swings.
const SHAKE: Duration = Duration::from_millis(420);
/// One breath of a glow, and one ring of a pulse.
const BREATH: Duration = Duration::from_millis(2_400);
const RING: Duration = Duration::from_millis(1_600);
/// How fast a marquee travels.
const PACE: f32 = 40.0;

/// How far into its effect a change is, 0 to 1, and whether it still moves; it asks for the next frame while it does.
pub(crate) fn since_change<K: Clone + PartialEq + 'static>(
    id: &ElementId,
    key: K,
    length: Duration,
    window: &mut Window,
    cx: &mut App,
) -> Option<f32> {
    let state = window.use_keyed_state((id.clone(), "since"), cx, |_, _| {
        (key.clone(), None::<Instant>)
    });
    if state.read(cx).0 != key {
        state.update(cx, |state, _| *state = (key, Some(Instant::now())));
    }
    let since = state.read(cx).1?;
    let share = since.elapsed().as_secs_f32() / length.as_secs_f32().max(f32::EPSILON);
    if share >= 1.0 {
        return None;
    }
    window.request_animation_frame();
    Some(share)
}

macro_rules! holder {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(IntoElement)]
        pub struct $name {
            id: ElementId,
            base: Div,
            body: SmallVec<[AnyElement; 2]>,
        }

        impl Styled for $name {
            fn style(&mut self) -> &mut StyleRefinement {
                self.base.style()
            }
        }

        impl ParentElement for $name {
            fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
                self.body.extend(elements);
            }
        }
    };
}

holder!(
    Blink,
    "Blinks what it holds on and off, a caret's rhythm. Under reduced motion it stays on."
);

impl Blink {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            base: div(),
            body: SmallVec::new(),
        }
    }
}

impl RenderOnce for Blink {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let held = self.base.children(self.body);
        if cx.theme().reduced_motion {
            return held.into_any_element();
        }
        held.with_animation(self.id, Animation::new(BLINK).repeat(), |held, t| {
            held.opacity(if t < 0.5 { 1.0 } else { 0.0 })
        })
        .into_any_element()
    }
}

/// Tints what it holds for a moment each time `key` changes, then lets the tint fade: a cue that a value moved.
#[derive(IntoElement)]
pub struct Flash<K: Clone + PartialEq + 'static> {
    id: ElementId,
    key: K,
    tint: Option<Hsla>,
    base: Div,
    body: SmallVec<[AnyElement; 2]>,
}

impl<K: Clone + PartialEq + 'static> Flash<K> {
    pub fn new(id: impl Into<ElementId>, key: K) -> Self {
        Self {
            id: id.into(),
            key,
            tint: None,
            base: div(),
            body: SmallVec::new(),
        }
    }

    /// The color it flashes; the selection's unless set.
    pub fn tint(mut self, tint: Hsla) -> Self {
        self.tint = Some(tint);
        self
    }
}

impl<K: Clone + PartialEq + 'static> Styled for Flash<K> {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl<K: Clone + PartialEq + 'static> ParentElement for Flash<K> {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.body.extend(elements);
    }
}

impl<K: Clone + PartialEq + 'static> RenderOnce for Flash<K> {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let share = since_change(&self.id, self.key, duration(FLASH, cx), window, cx);
        let tint = self.tint.unwrap_or(cx.theme().colors.selection);
        self.base.children(self.body).when_some(share, |held, t| {
            held.bg(tint.opacity(1.0 - ease_out_cubic(t)))
        })
    }
}

/// Shakes what it holds side to side each time `key` changes, as a field refusing an entry. Under reduced motion it holds still.
#[derive(IntoElement)]
pub struct Shake<K: Clone + PartialEq + 'static> {
    id: ElementId,
    key: K,
    base: Div,
    body: SmallVec<[AnyElement; 2]>,
}

impl<K: Clone + PartialEq + 'static> Shake<K> {
    pub fn new(id: impl Into<ElementId>, key: K) -> Self {
        Self {
            id: id.into(),
            key,
            base: div(),
            body: SmallVec::new(),
        }
    }
}

impl<K: Clone + PartialEq + 'static> Styled for Shake<K> {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl<K: Clone + PartialEq + 'static> ParentElement for Shake<K> {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.body.extend(elements);
    }
}

impl<K: Clone + PartialEq + 'static> RenderOnce for Shake<K> {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let share = since_change(&self.id, self.key, SHAKE, window, cx);
        let still = cx.theme().reduced_motion;
        let swing = NUDGE * 1.5;
        div()
            .child(self.base.children(self.body))
            .when_some(share.filter(|_| !still), move |held, t| {
                held.ml(swing * (t * PI * 6.0).sin() * (1.0 - t))
            })
    }
}

holder!(
    Glow,
    "A soft light that breathes around what it holds. Under reduced motion it holds its middle glow."
);

impl Glow {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            base: div(),
            body: SmallVec::new(),
        }
    }
}

fn glow(color: Hsla, reach: Pixels, strength: f32) -> Vec<BoxShadow> {
    vec![BoxShadow {
        color: color.opacity(0.25 * strength),
        offset: point(Pixels::ZERO, Pixels::ZERO),
        blur_radius: reach * (0.4 + 0.6 * strength),
        spread_radius: Pixels::ZERO,
        inset: false,
    }]
}

impl RenderOnce for Glow {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let (color, reach) = (
            theme.colors.accent,
            theme.glow_reach().to_pixels(window.rem_size()),
        );
        let held = self.base.children(self.body);
        if theme.reduced_motion {
            return held.shadow(glow(color, reach, 0.5)).into_any_element();
        }
        held.with_animation(self.id, Animation::new(BREATH).repeat(), move |held, t| {
            held.shadow(glow(color, reach, 0.5 - 0.5 * (TAU * t).cos()))
        })
        .into_any_element()
    }
}

holder!(
    Pulse,
    "Rings that widen and fade from what it holds, as a live dot calls out. Under reduced motion no ring shows."
);

impl Pulse {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            base: div(),
            body: SmallVec::new(),
        }
    }
}

impl RenderOnce for Pulse {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let color = theme.colors.accent;
        let held = self.base.relative().children(self.body);
        if theme.reduced_motion {
            return held.into_any_element();
        }
        held.child(div().absolute().inset_0().with_animation(
            self.id,
            Animation::new(RING).repeat(),
            move |ring, t| {
                ring.child(
                    canvas(
                        |_, _, _| {},
                        move |bounds, _, window, _| {
                            let side = bounds.size.width.max(bounds.size.height);
                            let radius = side / 2.0 * (1.0 + t);
                            let reach = bounds.center();
                            let corner = point(reach.x - radius, reach.y - radius);
                            window.paint_quad(
                                gpui::outline(
                                    gpui::Bounds::new(
                                        corner,
                                        gpui::size(radius * 2.0, radius * 2.0),
                                    ),
                                    color.opacity(0.5 * (1.0 - t)),
                                    gpui::BorderStyle::Solid,
                                )
                                .corner_radii(radius),
                            );
                        },
                    )
                    .size_full(),
                )
            },
        ))
        .into_any_element()
    }
}

/// Copies that keep the box covered while the row slides one copy's width: enough to span the box, and one more.
fn copies(span: Pixels, room: Pixels) -> usize {
    if span <= Pixels::ZERO {
        return 2;
    }
    ((f32::from(room) / f32::from(span)).ceil() as usize + 1).max(2)
}

/// Runs its content past in a loop, right to left, at an even pace. Under reduced motion it stands still.
#[derive(IntoElement)]
pub struct Marquee {
    id: ElementId,
    base: Div,
    content: Box<dyn Fn() -> AnyElement>,
}

impl Marquee {
    /// `content` is drawn as many times as it takes to cover the box, and once more, so the loop has no seam.
    pub fn new<E: IntoElement>(
        id: impl Into<ElementId>,
        content: impl Fn() -> E + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            base: div(),
            content: Box::new(move || content().into_any_element()),
        }
    }
}

impl Styled for Marquee {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for Marquee {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let widths = window.use_keyed_state((self.id.clone(), "widths"), cx, |_, _| {
            (Pixels::ZERO, Pixels::ZERO)
        });
        let (span, room) = *widths.read(cx);
        let measure = |pick: fn(&mut (Pixels, Pixels)) -> &mut Pixels| {
            let widths = widths.clone();
            canvas(
                move |bounds, window, cx| {
                    let mut now = *widths.read(cx);
                    if *pick(&mut now) != bounds.size.width {
                        *pick(&mut now) = bounds.size.width;
                        widths.update(cx, |widths, cx| {
                            *widths = now;
                            cx.notify();
                        });
                        window.request_animation_frame();
                    }
                },
                |_, _, _, _| {},
            )
            .absolute()
            .top_0()
            .left_0()
            .size_full()
        };
        let copies = copies(span, room);
        let first = div()
            .flex_none()
            .relative()
            .child((self.content)())
            .child(measure(|widths| &mut widths.0));
        let row = div()
            .flex()
            .flex_none()
            .child(first)
            .children((1..copies).map(|_| div().flex_none().child((self.content)())));
        let still = cx.theme().reduced_motion || span <= Pixels::ZERO;
        let lap = Duration::from_secs_f32(f32::from(span) / PACE).max(SLOW);
        let row = if still {
            row.into_any_element()
        } else {
            row.with_animation(
                (self.id.clone(), "lap"),
                Animation::new(lap).repeat(),
                move |row, t| row.ml(-span * t),
            )
            .into_any_element()
        };
        self.base
            .relative()
            .overflow_hidden()
            .child(measure(|widths| &mut widths.1))
            .child(row)
    }
}

#[cfg(test)]
mod tests {
    use gpui::px;

    use super::copies;

    #[test]
    fn a_marquee_draws_enough_copies_to_stay_covered() {
        assert_eq!(copies(px(80.0), px(450.0)), 7);
        assert!(copies(px(80.0), px(450.0)) as f32 * 80.0 >= 450.0 + 80.0);
        assert_eq!(copies(px(600.0), px(450.0)), 2);
        assert_eq!(copies(px(0.0), px(450.0)), 2);
    }
}
