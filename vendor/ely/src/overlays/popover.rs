use std::rc::Rc;

use gpui::{
    AnyElement, App, Bounds, ElementId, Entity, InteractiveElement, IntoElement, ParentElement,
    Pixels, RenderOnce, SharedString, Styled, Window, canvas, div, prelude::*,
};

use crate::{
    buttons::{Button, ButtonVariant},
    forms::{Run, float_height, surface},
    primitives::{FocusScope, IconName, Takeover, give_back, take_focus},
    theme::ControlSize,
};

type Content = Box<dyn FnOnce(Run, &mut Window, &mut App) -> AnyElement>;
type Own = Box<dyn FnOnce(Run) -> AnyElement>;

/// A popover's own button: its label and look.
struct Face {
    label: SharedString,
    icon: Option<IconName>,
    variant: ButtonVariant,
    size: ControlSize,
}

/// What opens the panel: its own button, or an element of the owner's, built with the run that opens and closes it.
enum Opener {
    Button(Face),
    Own(Own),
}

/// Whether the panel is open, where the trigger sits, and the focus to hand back.
#[derive(Default)]
struct Pop {
    open: bool,
    host: Bounds<Pixels>,
    height: Pixels,
    takeover: Option<Entity<Takeover>>,
}

fn close(state: &Entity<Pop>, window: &mut Window, cx: &mut App) {
    log::info!("popover: closed");
    let takeover = state.update(cx, |pop, cx| {
        pop.open = false;
        cx.notify();
        pop.takeover.take()
    });
    if let Some(takeover) = takeover {
        give_back(&takeover, window, cx);
    }
}

/// A button that opens a panel of any content under it, or over it when only above has room. A press outside or Escape closes it; Tab stays inside.
#[derive(IntoElement)]
pub struct Popover {
    id: ElementId,
    opener: Opener,
    content: Content,
}

impl Popover {
    /// `content` is built only while the panel is open.
    pub fn new<E: IntoElement>(
        id: impl Into<ElementId>,
        label: impl Into<SharedString>,
        content: impl FnOnce(&mut Window, &mut App) -> E + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            opener: Opener::Button(Face {
                label: label.into(),
                icon: None,
                variant: ButtonVariant::Secondary,
                size: ControlSize::default(),
            }),
            content: Box::new(move |_, window, cx| content(window, cx).into_any_element()),
        }
    }

    /// A panel that opens from an element of the owner's: `opener` builds it with the run that opens and closes the panel, and `content` gets the run that closes it and hands focus back, for a press that ends the panel's work.
    pub fn with_opener<T: IntoElement, E: IntoElement>(
        id: impl Into<ElementId>,
        opener: impl FnOnce(Run) -> T + 'static,
        content: impl FnOnce(Run, &mut Window, &mut App) -> E + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            opener: Opener::Own(Box::new(move |toggle| opener(toggle).into_any_element())),
            content: Box::new(move |close, window, cx| {
                content(close, window, cx).into_any_element()
            }),
        }
    }

    /// The popover's own button, which a popover with an opener of the owner's does not have.
    fn face(&mut self, what: &str) -> &mut Face {
        match &mut self.opener {
            Opener::Button(face) => face,
            Opener::Own(_) => panic!("popover {:?}: {what} is for its own button", self.id),
        }
    }

    pub fn icon(mut self, icon: IconName) -> Self {
        self.face("an icon").icon = Some(icon);
        self
    }

    /// `Link` sets the trigger inline in running text.
    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.face("a variant").variant = variant;
        self
    }

    pub fn size(mut self, size: ControlSize) -> Self {
        self.face("a size").size = size;
        self
    }
}

impl RenderOnce for Popover {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = window.use_keyed_state((self.id.clone(), "popover"), cx, |_, _| Pop::default());
        let open = state.read(cx).open;
        let toggle: Run = {
            let state = state.clone();
            Rc::new(move |window, cx| {
                if open {
                    close(&state, window, cx);
                } else {
                    log::info!("popover: open");
                    state.update(cx, |pop, cx| {
                        pop.open = true;
                        cx.notify();
                    });
                }
            })
        };
        let trigger = match self.opener {
            Opener::Button(face) => Button::new((self.id.clone(), "trigger"), face.label)
                .variant(face.variant)
                .size(face.size)
                .when_some(face.icon, |button, icon| button.icon(icon))
                .on_click(move |_, window, cx| toggle(window, cx))
                .into_any_element(),
            Opener::Own(build) => build(toggle),
        };
        let measure = state.clone();
        let host = div()
            .id(self.id.clone())
            .relative()
            .flex_none()
            .child(trigger)
            .child(
                canvas(
                    move |bounds, window, cx| {
                        if measure.read(cx).host != bounds {
                            measure.update(cx, |pop, cx| {
                                pop.host = bounds;
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
        if !open {
            return host;
        }
        let takeover = take_focus((self.id.clone(), "takeover"), window, cx);
        let focus = takeover.read(cx).focus.clone();
        if state.read(cx).takeover.is_none() {
            state.update(cx, |pop, _| pop.takeover = Some(takeover));
        }
        if !focus.contains_focused(window, cx) {
            log::info!("popover {:?}: focus left", self.id);
            close(&state, window, cx);
            return host;
        }
        let (escape, out, tall, done) =
            (state.clone(), state.clone(), state.clone(), state.clone());
        let (anchor, height) = (state.read(cx).host, state.read(cx).height);
        let body = (self.content)(
            Rc::new(move |window, cx| close(&done, window, cx)),
            window,
            cx,
        );
        let panel = div()
            .id((self.id.clone(), "panel"))
            .relative()
            .occlude()
            .on_key_down(move |event, window, cx| {
                if event.keystroke.key == "escape" {
                    cx.stop_propagation();
                    close(&escape, window, cx);
                }
            })
            .on_mouse_down_out(move |event, window, cx| {
                if !out.read(cx).host.contains(&event.position) {
                    close(&out, window, cx);
                }
            })
            .child(
                FocusScope::new(&focus)
                    .trap()
                    .child(surface((self.id.clone(), "surface"), cx).p_3().child(body)),
            )
            .child(
                canvas(
                    move |bounds, window, cx| {
                        if tall.read(cx).height != bounds.size.height {
                            tall.update(cx, |pop, cx| {
                                pop.height = bounds.size.height;
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
        host.child(float_height(
            self.id.clone(),
            anchor,
            height,
            panel,
            window,
            cx,
        ))
    }
}
