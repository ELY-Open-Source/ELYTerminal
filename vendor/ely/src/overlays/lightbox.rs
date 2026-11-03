use std::rc::Rc;

use gpui::{
    Animation, AnimationExt, App, Div, ElementId, ImageSource, InteractiveElement, IntoElement,
    MouseButton, ObjectFit, ParentElement, Pixels, Point, RenderOnce, SharedString, Size, Stateful,
    StatefulInteractiveElement, Styled, Window, anchored, canvas, div, prelude::*,
};

use crate::{
    buttons::label_size,
    forms::Run,
    motion,
    primitives::{FocusRing, FocusScope, Icon, IconName, Image, give_back, raise, take_focus},
    theme::{ActiveTheme, ControlSize, TextSize},
};

/// One picture in a lightbox, and the line under it.
#[derive(Clone)]
pub struct Slide {
    pub(crate) source: ImageSource,
    pub(crate) caption: SharedString,
}

impl Slide {
    pub fn new(source: impl Into<ImageSource>, caption: impl Into<SharedString>) -> Self {
        Self {
            source: source.into(),
            caption: caption.into(),
        }
    }
}

type OnStep = Rc<dyn Fn(usize, &mut Window, &mut App)>;

/// A round button drawn for a dark layer over pictures: light icon, a faint fill on hover, dim when it has nowhere to go.
pub(crate) fn media_button(
    id: (ElementId, &'static str),
    icon: IconName,
    size: ControlSize,
    enabled: bool,
    run: impl Fn(&mut Window, &mut App) + 'static,
    cx: &App,
) -> Stateful<Div> {
    let theme = cx.theme();
    let light = theme.colors.on_media;
    div()
        .debug_selector(|| format!("media-button {}", id.1))
        .id(id)
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .size(theme.control_height(size))
        .rounded_full()
        .border_1()
        .border_color(gpui::transparent_black())
        .child(Icon::new(icon).size(label_size(size).1).color(if enabled {
            light
        } else {
            light.opacity(0.3)
        }))
        .on_mouse_down(MouseButton::Left, |_, window, cx| {
            window.prevent_default();
            cx.stop_propagation();
        })
        .when(enabled, |button| {
            button
                .cursor_pointer()
                .tab_index(0)
                .focus_ring(cx)
                .hover(|style| style.bg(light.opacity(0.12)))
                .on_key_down(|event, _, cx| {
                    if event.keystroke.key == "space" && !event.keystroke.modifiers.modified() {
                        cx.stop_propagation();
                    }
                })
                .on_click(move |_, window, cx| run(window, cx))
        })
}

/// Pictures over a dark layer, one at a time. Left and Right or the side buttons step through them; Escape, the close button or a press on the dark closes it.
#[derive(IntoElement)]
pub struct Lightbox {
    id: ElementId,
    slides: Vec<Slide>,
    at: usize,
    on_step: Option<OnStep>,
    on_close: Run,
}

impl Lightbox {
    /// Render it while open; the owner keeps `at` and moves it in `on_step`. With no pictures left it closes, handing focus back.
    pub fn new(
        id: impl Into<ElementId>,
        slides: impl IntoIterator<Item = Slide>,
        at: usize,
        on_close: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            slides: slides.into_iter().collect(),
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

impl RenderOnce for Lightbox {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let (at, count) = (self.at, self.slides.len());
        let turn = motion::changes((self.id.clone(), "at"), at, window, cx);
        let takeover = take_focus((self.id.clone(), "takeover"), window, cx);
        let focus = takeover.read(cx).focus.clone();
        let area = window.use_keyed_state((self.id.clone(), "area"), cx, |_, _| {
            Size::<Pixels>::default()
        });
        let room = *area.read(cx);
        let close: Run = {
            let (id, on_close) = (self.id.clone(), self.on_close);
            Rc::new(move |window, cx| {
                log::info!("lightbox {id:?}: closed");
                give_back(&takeover, window, cx);
                on_close(window, cx)
            })
        };
        if count == 0 {
            log::info!("lightbox {:?}: no pictures left", self.id);
            window.defer(cx, move |window, cx| close(window, cx));
            return div().into_any_element();
        }
        if at >= count {
            log::error!(
                "lightbox {:?}: slide {at} of {count} is gone; closing",
                self.id
            );
            window.defer(cx, move |window, cx| close(window, cx));
            return div().into_any_element();
        }
        let step: OnStep = {
            let (id, on_step) = (self.id.clone(), self.on_step);
            Rc::new(move |to, window, cx| {
                log::info!("lightbox {id:?}: slide {to}");
                if let Some(on_step) = &on_step {
                    on_step(to, window, cx);
                }
            })
        };
        let (back, next) = (at.checked_sub(1), (at + 1 < count).then_some(at + 1));
        let side = |name: &'static str, icon: IconName, to: Option<usize>, cx: &App| {
            let step = step.clone();
            media_button(
                (self.id.clone(), name),
                icon,
                ControlSize::Lg,
                to.is_some(),
                move |window, cx| {
                    if let Some(to) = to {
                        step(to, window, cx);
                    }
                },
                cx,
            )
        };
        let theme = cx.theme();
        let colors = &theme.colors;
        let slide = self.slides[at].clone();
        let picture = div()
            .size_full()
            .child(
                Image::new((self.id.clone(), format!("slide-{at}")), slide.source)
                    .fit(ObjectFit::Contain)
                    .w(room.width)
                    .h(room.height),
            )
            .with_animation(
                (self.id.clone(), format!("turn-{turn}")),
                Animation::new(motion::duration(motion::BASE, cx))
                    .with_easing(motion::ease_out_cubic),
                move |picture, t| {
                    if turn == 0 {
                        picture
                    } else {
                        picture.opacity(t)
                    }
                },
            );
        let (keys, escape, press, button) = (step.clone(), close.clone(), close.clone(), close);
        let viewport = window.viewport_size();
        let stage = div()
            .id((self.id.clone(), "stage"))
            .w(viewport.width)
            .h(viewport.height)
            .bg(colors.media_backdrop)
            .text_color(colors.on_media)
            .occlude()
            .on_mouse_down(MouseButton::Left, move |_, window, cx| press(window, cx))
            .on_key_down(move |event, window, cx| {
                match (event.keystroke.key.as_str(), back, next) {
                    ("left", Some(to), _) | ("right", _, Some(to)) => keys(to, window, cx),
                    ("escape", _, _) => escape(window, cx),
                    _ => return,
                }
                cx.stop_propagation();
            })
            .child(
                FocusScope::new(&focus)
                    .trap()
                    .size_full()
                    .flex()
                    .flex_col()
                    .child(div().flex().justify_end().p_3().child(media_button(
                        (self.id.clone(), "close"),
                        IconName::X,
                        ControlSize::Lg,
                        true,
                        move |window, cx| button(window, cx),
                        cx,
                    )))
                    .child(
                        div()
                            .relative()
                            .flex_1()
                            .min_h_0()
                            .child(
                                div()
                                    .absolute()
                                    .top_0()
                                    .bottom_0()
                                    .left_16()
                                    .right_16()
                                    .on_mouse_down(MouseButton::Left, |_, _, cx| {
                                        cx.stop_propagation()
                                    })
                                    .child(picture)
                                    .child(
                                        canvas(
                                            move |bounds, window, cx| {
                                                if area.read(cx) != &bounds.size {
                                                    area.update(cx, |area, cx| {
                                                        *area = bounds.size;
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
                                    ),
                            )
                            .child(
                                div()
                                    .absolute()
                                    .top_0()
                                    .bottom_0()
                                    .left_3()
                                    .flex()
                                    .items_center()
                                    .child(side("back", IconName::ChevronLeft, back, cx)),
                            )
                            .child(
                                div()
                                    .absolute()
                                    .top_0()
                                    .bottom_0()
                                    .right_3()
                                    .flex()
                                    .items_center()
                                    .child(side("next", IconName::ChevronRight, next, cx)),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .justify_center()
                            .gap_3()
                            .p_5()
                            .text_size(theme.text_size(TextSize::Sm))
                            .child(slide.caption)
                            .child(
                                div()
                                    .text_color(colors.on_media.opacity(0.6))
                                    .child(format!("{} / {count}", at + 1)),
                            ),
                    ),
            );
        raise(
            (self.id.clone(), "raised"),
            anchored().position(Point::default()).child(stage),
        )
        .with_priority(1)
        .into_any_element()
    }
}
