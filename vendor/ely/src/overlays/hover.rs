use std::{rc::Rc, time::Duration};

use gpui::{
    AnyElement, App, Bounds, ElementId, Entity, HoverListenerMode, InteractiveElement, IntoElement,
    ParentElement, Pixels, RenderOnce, StatefulInteractiveElement, Styled, Task, Window, canvas,
    div,
};

use crate::forms::{float_height, surface};

/// How long the pointer rests on the trigger before the card opens.
const OPEN: Duration = Duration::from_millis(500);
/// How long the card waits after the pointer leaves, so it can cross the gap.
const LINGER: Duration = Duration::from_millis(200);

type Content = Rc<dyn Fn(&mut Window, &mut App) -> AnyElement>;

/// Whether the card shows, what the pointer is over, the pending change and the trigger's box.
#[derive(Default)]
struct Hover {
    shown: bool,
    on_trigger: bool,
    on_card: bool,
    host: Bounds<Pixels>,
    height: Pixels,
    _pending: Option<Task<()>>,
}

/// After `after`, shows or hides the card as the pointer then warrants.
fn settle_after(state: &Entity<Hover>, after: Duration, window: &mut Window, cx: &mut App) {
    let weak = state.downgrade();
    let task = window.spawn(cx, async move |cx| {
        cx.background_executor().timer(after).await;
        let done = cx.update(|_, cx| {
            weak.update(cx, |hover, cx| {
                let shown = hover.on_trigger || hover.on_card;
                if hover.shown != shown {
                    log::info!("hover card: {}", if shown { "shown" } else { "hidden" });
                    hover.shown = shown;
                    cx.notify();
                }
                hover._pending = None;
            })
        });
        if let Err(error) = done.and_then(|inner| inner) {
            log::error!("hover card: trigger vanished: {error:#}");
        }
    });
    state.update(cx, |hover, _| hover._pending = Some(task));
}

/// A card that opens once the pointer rests on its trigger, and stays while the pointer is on either. It never takes focus.
#[derive(IntoElement)]
pub struct HoverCard {
    id: ElementId,
    trigger: AnyElement,
    content: Content,
}

impl HoverCard {
    pub fn new<E: IntoElement>(
        id: impl Into<ElementId>,
        trigger: impl IntoElement,
        content: impl Fn(&mut Window, &mut App) -> E + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            trigger: trigger.into_any_element(),
            content: Rc::new(move |window, cx| content(window, cx).into_any_element()),
        }
    }
}

impl RenderOnce for HoverCard {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = window.use_keyed_state((self.id.clone(), "hover"), cx, |_, _| Hover::default());
        let (enter, measure) = (state.clone(), state.clone());
        let host = div()
            .id(self.id.clone())
            .relative()
            .child(self.trigger)
            .hover_listener_mode(HoverListenerMode::InputModalityIndependent)
            .on_hover(move |hovered, window, cx| {
                enter.update(cx, |hover, _| hover.on_trigger = *hovered);
                settle_after(&enter, if *hovered { OPEN } else { LINGER }, window, cx);
            })
            .child(
                canvas(
                    move |bounds, window, cx| {
                        if measure.read(cx).host != bounds {
                            measure.update(cx, |hover, cx| {
                                hover.host = bounds;
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
        if !state.read(cx).shown {
            return host;
        }
        let (over, tall) = (state.clone(), state.clone());
        let card = surface((self.id.clone(), "card"), cx)
            .relative()
            .occlude()
            .p_4()
            .hover_listener_mode(HoverListenerMode::InputModalityIndependent)
            .on_hover(move |hovered, window, cx| {
                over.update(cx, |hover, _| hover.on_card = *hovered);
                settle_after(&over, LINGER, window, cx);
            })
            .child((self.content)(window, cx))
            .child(
                canvas(
                    move |bounds, window, cx| {
                        if tall.read(cx).height != bounds.size.height {
                            tall.update(cx, |hover, cx| {
                                hover.height = bounds.size.height;
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
        let (anchor, height) = (state.read(cx).host, state.read(cx).height);
        host.child(float_height(
            self.id.clone(),
            anchor,
            height,
            card,
            window,
            cx,
        ))
    }
}
