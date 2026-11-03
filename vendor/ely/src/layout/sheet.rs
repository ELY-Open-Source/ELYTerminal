use std::{cell::Cell, rc::Rc};

use gpui::{
    Animation, AnimationExt, AnyElement, App, DragMoveEvent, ElementId, EmptyView, EntityId,
    FontWeight, InteractiveElement, IntoElement, MouseButton, ParentElement, Pixels, RenderOnce,
    SharedString, StatefulInteractiveElement, Styled, Window, canvas, div, prelude::*, relative,
};
use smallvec::SmallVec;

use crate::{
    buttons::IconButton,
    motion,
    primitives::{Backdrop, IconName, Place},
    theme::{ActiveTheme, Elevation, Radius, TextSize},
};

type Close = Rc<dyn Fn(&mut Window, &mut App)>;

fn title_row(title: SharedString, close: Close, cx: &App) -> impl IntoElement + use<> {
    let theme = cx.theme();
    div()
        .flex()
        .items_center()
        .justify_between()
        .gap_3()
        .child(
            div()
                .text_size(theme.text_size(TextSize::Lg))
                .font_weight(FontWeight::SEMIBOLD)
                .child(title),
        )
        .child(
            IconButton::new("close", IconName::X).on_click(move |_, window, cx| close(window, cx)),
        )
}

/// Panel that slides in from an edge over a scrim.
#[derive(IntoElement)]
pub struct Sheet {
    id: ElementId,
    side: Place,
    title: SharedString,
    on_close: Close,
    body: SmallVec<[AnyElement; 2]>,
}

impl Sheet {
    /// `side` must be an edge, not `Place::Center`.
    pub fn new(
        id: impl Into<ElementId>,
        side: Place,
        title: impl Into<SharedString>,
        on_close: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        assert!(side != Place::Center, "a sheet needs an edge");
        Self {
            id: id.into(),
            side,
            title: title.into(),
            on_close: Rc::new(on_close),
            body: SmallVec::new(),
        }
    }
}

impl ParentElement for Sheet {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.body.extend(elements);
    }
}

impl RenderOnce for Sheet {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let size = theme.sheet_size();
        let travel = size.to_pixels(window.rem_size());
        let side = self.side;
        let panel = div()
            .flex()
            .flex_col()
            .gap_5()
            .p_6()
            .bg(theme.colors.overlay)
            .shadow(theme.elevation(Elevation::Modal))
            .map(|panel| match side {
                Place::Left | Place::Right => panel.w(size).h_full(),
                Place::Top | Place::Bottom | Place::Center => panel.h(size).w_full(),
            })
            .child(title_row(self.title, self.on_close.clone(), cx))
            .children(self.body)
            .with_animation(
                "sheet-in",
                Animation::new(motion::duration(motion::BASE, cx))
                    .with_easing(motion::ease_out_cubic),
                move |panel, t| {
                    let hidden = -(travel * (1.0 - t));
                    match side {
                        Place::Left => panel.ml(hidden),
                        Place::Right => panel.mr(hidden),
                        Place::Top => panel.mt(hidden),
                        Place::Bottom | Place::Center => panel.mb(hidden),
                    }
                },
            );
        let close = self.on_close;
        Backdrop::new(self.id)
            .place(side)
            .on_dismiss(move |window, cx| close(window, cx))
            .child(panel)
    }
}

#[derive(Default)]
struct Pull {
    pulling: bool,
    offset: Pixels,
    released: Pixels,
    snaps: u64,
    height: Pixels,
}

struct DrawerPull {
    owner: EntityId,
    anchor: Rc<Cell<Option<Pixels>>>,
}

/// Share of the drawer's height a pull must pass to dismiss it.
const DISMISS: f32 = 0.3;

/// Bottom panel with a grab handle; pull it down to dismiss.
#[derive(IntoElement)]
pub struct Drawer {
    id: ElementId,
    on_close: Close,
    body: SmallVec<[AnyElement; 2]>,
}

impl Drawer {
    pub fn new(
        id: impl Into<ElementId>,
        on_close: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            on_close: Rc::new(on_close),
            body: SmallVec::new(),
        }
    }
}

impl ParentElement for Drawer {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.body.extend(elements);
    }
}

impl RenderOnce for Drawer {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = window.use_keyed_state(self.id.clone(), cx, |_, _| Pull::default());
        let owner = state.entity_id();
        let (offset, released, snaps, threshold) = {
            let pull = state.read(cx);
            (
                pull.offset,
                pull.released,
                pull.snaps,
                pull.height * DISMISS,
            )
        };
        let theme = cx.theme();
        let grip = theme.grip();
        let travel = theme.sheet_size().to_pixels(window.rem_size());
        let quick = motion::duration(motion::BASE, cx);
        let (move_state, release, out, measure) =
            (state.clone(), state.clone(), state.clone(), state);
        let close = self.on_close.clone();
        let finish = move |pull: &mut Pull| -> bool {
            let dismiss = pull.pulling && pull.offset > threshold;
            log::info!("drawer: pulled {:?} of {threshold:?}", pull.offset);
            pull.pulling = false;
            pull.released = pull.offset;
            pull.offset = Pixels::ZERO;
            pull.snaps += 1;
            dismiss
        };
        let finish_out = finish;
        let (close_up, close_out) = (close.clone(), close);
        let panel = div()
            .id("drawer-panel")
            .debug_selector(|| "drawer-panel".into())
            .relative()
            .w_full()
            .max_h(relative(0.85))
            .flex()
            .flex_col()
            .gap_4()
            .px_6()
            .pb_8()
            .rounded_t(theme.radius(Radius::Xl))
            .bg(theme.colors.overlay)
            .shadow(theme.elevation(Elevation::Modal))
            .top(offset)
            .on_drag_move(move |event: &DragMoveEvent<DrawerPull>, _, cx| {
                let pull = event.drag(cx);
                if pull.owner != owner {
                    return;
                }
                let y = event.event.position.y;
                let anchor = pull.anchor.get().unwrap_or_else(|| {
                    pull.anchor.set(Some(y));
                    y
                });
                move_state.update(cx, |pull, cx| {
                    pull.pulling = true;
                    pull.offset = (y - anchor).max(Pixels::ZERO);
                    cx.notify();
                })
            })
            .on_mouse_up(MouseButton::Left, move |_, window, cx| {
                let pulled = release.read(cx).pulling;
                if pulled
                    && release.update(cx, |pull, cx| {
                        cx.notify();
                        finish(pull)
                    })
                {
                    close_up(window, cx);
                }
            })
            .on_mouse_up_out(MouseButton::Left, move |_, window, cx| {
                let pulled = out.read(cx).pulling;
                if pulled
                    && out.update(cx, |pull, cx| {
                        cx.notify();
                        finish_out(pull)
                    })
                {
                    close_out(window, cx);
                }
            })
            .child(
                canvas(
                    move |bounds, _, cx| {
                        if measure.read(cx).height != bounds.size.height {
                            measure.update(cx, |pull, _| pull.height = bounds.size.height);
                        }
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            )
            .child(
                div()
                    .id("drawer-grab")
                    .flex()
                    .justify_center()
                    .pt_3()
                    .pb_2()
                    .cursor_grab()
                    .on_drag(
                        DrawerPull {
                            owner,
                            anchor: Rc::new(Cell::new(None)),
                        },
                        |_, _, _, cx| cx.new(|_| EmptyView),
                    )
                    .child(
                        div()
                            .w(grip.width)
                            .h(grip.height)
                            .rounded_full()
                            .bg(theme.colors.border_strong),
                    ),
            )
            .children(self.body);
        let snapping = snaps > 0 && released > Pixels::ZERO;
        let settled = div().w_full().child(panel);
        let settled = if snapping {
            settled
                .with_animation(
                    ("drawer-snap", snaps),
                    Animation::new(quick).with_easing(motion::ease_out_cubic),
                    move |wrap, t| wrap.top(released * (1.0 - t)),
                )
                .into_any_element()
        } else {
            settled.into_any_element()
        };
        let entering = div().w_full().child(settled).with_animation(
            "drawer-in",
            Animation::new(quick).with_easing(motion::ease_out_cubic),
            move |wrap, t| wrap.mb(-(travel * (1.0 - t))),
        );
        let close = self.on_close;
        Backdrop::new(self.id)
            .place(Place::Bottom)
            .on_dismiss(move |window, cx| close(window, cx))
            .child(entering)
    }
}
