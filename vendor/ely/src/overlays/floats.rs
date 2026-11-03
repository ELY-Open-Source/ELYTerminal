use std::rc::Rc;

use gpui::{
    Anchor, AnimationExt, AnyElement, App, ElementId, Entity, FontWeight, InteractiveElement,
    IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window, anchored, canvas, div,
    point, prelude::*,
};
use smallvec::SmallVec;

use crate::{
    buttons::{ButtonVariant, IconButton},
    forms::{Run, TextInput, surface},
    layout::Collapsible,
    motion,
    primitives::{IconName, raise},
    theme::{ActiveTheme, ControlSize, TextSize},
};

/// Tools that float over a text field's selection while it has one and holds focus. Its buttons leave focus in the field.
#[derive(IntoElement)]
pub struct FloatingToolbar {
    id: ElementId,
    field: Entity<TextInput>,
    tools: SmallVec<[AnyElement; 4]>,
}

impl FloatingToolbar {
    pub fn new(id: impl Into<ElementId>, field: &Entity<TextInput>) -> Self {
        Self {
            id: id.into(),
            field: field.clone(),
            tools: SmallVec::new(),
        }
    }
}

impl ParentElement for FloatingToolbar {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.tools.extend(elements);
    }
}

impl RenderOnce for FloatingToolbar {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let seen = window.use_keyed_state((self.id.clone(), "seen"), cx, |_, _| None);
        let input = self.field.read(cx);
        let selection = input.selection();
        let shown = !selection.is_empty() && input.focus().is_focused(window);
        let start = input.bounds_for(selection.start);
        let anchor = start.filter(|_| shown);
        let (field, from) = (self.field.clone(), selection.start);
        div()
            .id(self.id.clone())
            .child(canvas(
                |_, _, _| {},
                move |_, _, window, cx| {
                    let now = field.read(cx).bounds_for(from);
                    if now != start {
                        log::debug!("floating toolbar: the selection moved to {now:?}");
                        seen.update(cx, |seen, cx| {
                            *seen = now;
                            cx.notify();
                            window.request_animation_frame();
                        });
                    }
                },
            ))
            .when_some(anchor, |host, anchor| {
                let lift = cx.theme().float_gap().to_pixels(window.rem_size());
                host.child(
                    raise(
                        (self.id.clone(), "raised"),
                        anchored()
                            .position(anchor.origin - point(gpui::Pixels::ZERO, lift))
                            .anchor(Anchor::BottomLeft)
                            .snap_to_window()
                            .child(
                                surface((self.id.clone(), "bar"), cx)
                                    .occlude()
                                    .flex()
                                    .items_center()
                                    .gap_0p5()
                                    .p_1()
                                    .children(self.tools)
                                    .with_animation(
                                        (self.id.clone(), "in"),
                                        gpui::Animation::new(motion::duration(motion::FAST, cx))
                                            .with_easing(motion::ease_out_cubic),
                                        |bar, t| bar.opacity(t).mb(motion::NUDGE * (1.0 - t)),
                                    ),
                            ),
                    )
                    .with_priority(1),
                )
            })
    }
}

/// A panel that opens in the flow under a line, the way an editor peeks at a definition: a title bar with a close button over the owner's content.
#[derive(IntoElement)]
pub struct Peek {
    id: ElementId,
    title: SharedString,
    open: bool,
    body: SmallVec<[AnyElement; 2]>,
    on_close: Run,
}

impl Peek {
    pub fn new(
        id: impl Into<ElementId>,
        title: impl Into<SharedString>,
        open: bool,
        on_close: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            open,
            body: SmallVec::new(),
            on_close: Rc::new(on_close),
        }
    }
}

impl ParentElement for Peek {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.body.extend(elements);
    }
}

impl RenderOnce for Peek {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = &theme.colors;
        let (id, close) = (self.id.clone(), self.on_close);
        Collapsible::new(self.id.clone(), self.open).child(
            div()
                .my_2()
                .border_l_2()
                .border_color(colors.accent)
                .bg(colors.surface)
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap_2()
                        .h(theme.control_height(ControlSize::Lg))
                        .pl_3()
                        .pr_1()
                        .border_b_1()
                        .border_color(colors.border)
                        .text_size(theme.text_size(TextSize::Sm))
                        .font_weight(FontWeight::MEDIUM)
                        .child(self.title)
                        .child(
                            IconButton::new((self.id.clone(), "close"), IconName::X)
                                .variant(ButtonVariant::Ghost)
                                .size(ControlSize::Sm)
                                .tooltip("Close")
                                .on_click(move |_, window, cx| {
                                    log::info!("peek {id:?}: closed");
                                    close(window, cx)
                                }),
                        ),
                )
                .child(div().p_3().children(self.body)),
        )
    }
}
