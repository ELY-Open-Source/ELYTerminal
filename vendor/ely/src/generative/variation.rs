use std::rc::Rc;

use gpui::{
    Animation, AnimationExt, App, ElementId, InteractiveElement, IntoElement, MouseButton,
    ParentElement, RenderOnce, SharedString, StatefulInteractiveElement, Styled, Window, div,
    transparent_black,
};

use crate::{
    buttons::{Button, ButtonVariant},
    documents::source,
    forms::{OnFlag, Pick, Run},
    motion,
    primitives::{IconName, Image, checked_ratio, framed, tab_stop},
    theme::{ActiveTheme, AvatarSize, ControlSize, Radius},
};

/// Variations of one picture: the chosen one large, every one as a thumbnail to choose, and ways on from the chosen: vary it a little or a lot, or upscale it. The large picture fades in on each choice.
#[derive(IntoElement)]
pub struct VariationPicker {
    id: ElementId,
    pictures: Vec<SharedString>,
    ratio: f32,
    chosen: Option<usize>,
    on_choose: Option<Pick>,
    on_vary: Option<OnFlag>,
    on_upscale: Option<Run>,
}

impl VariationPicker {
    /// `ratio` is the pictures' width over height; `chosen` indexes `pictures`.
    pub fn new(
        id: impl Into<ElementId>,
        pictures: impl IntoIterator<Item = impl Into<SharedString>>,
        ratio: f32,
        chosen: usize,
    ) -> Self {
        let pictures: Vec<SharedString> = pictures.into_iter().map(Into::into).collect();
        let held = chosen < pictures.len();
        if !held {
            log::error!(
                "variations: variation {chosen} of {}; none chosen",
                pictures.len()
            );
        }
        Self {
            id: id.into(),
            pictures,
            ratio: checked_ratio(ratio),
            chosen: held.then_some(chosen),
            on_choose: None,
            on_vary: None,
            on_upscale: None,
        }
    }

    pub fn on_choose(mut self, handler: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_choose = Some(Rc::new(handler));
        self
    }

    /// Gets whether to vary a lot.
    pub fn on_vary(mut self, handler: impl Fn(bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_vary = Some(Rc::new(handler));
        self
    }

    pub fn on_upscale(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_upscale = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for VariationPicker {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let turn = motion::changes((self.id.clone(), "turns"), self.chosen, window, cx);
        let thumbs: Vec<_> = (0..self.pictures.len())
            .map(|ix| {
                let focus = tab_stop(
                    (self.id.clone(), format!("focus-{ix}")).into(),
                    true,
                    window,
                    cx,
                );
                let focused = focus.is_focused(window);
                (focus, focused)
            })
            .collect();
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let (large, small) = (theme.radius(Radius::Lg), theme.radius(Radius::Md));
        let thumb = theme.avatar_size(AvatarSize::Xl);
        let length = motion::duration(motion::BASE, cx);
        let shown = self.chosen.map(|chosen| {
            let picture = Image::new(
                (self.id.clone(), format!("large-{chosen}")),
                source(&self.pictures[chosen]),
            )
            .size_full()
            .rounded(large);
            match turn {
                0 => picture.into_any_element(),
                _ => div()
                    .size_full()
                    .child(picture)
                    .with_animation(
                        (self.id.clone(), format!("fade-{turn}")),
                        Animation::new(length).with_easing(motion::ease_out_cubic),
                        |shown, t| shown.opacity(0.4 + 0.6 * t),
                    )
                    .into_any_element(),
            }
        });
        let vary = |strong: bool, label: &'static str| {
            self.on_vary
                .clone()
                .filter(|_| self.chosen.is_some())
                .map(|vary| {
                    Button::new((self.id.clone(), label), label)
                        .variant(ButtonVariant::Secondary)
                        .size(ControlSize::Sm)
                        .icon(IconName::Shuffle)
                        .on_click(move |_, window, cx| {
                            log::info!("variation picker: vary, strong {strong}");
                            vary(strong, window, cx)
                        })
                })
        };
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(framed(self.ratio, cx).rounded(large).children(shown))
            .child(div().flex().flex_wrap().gap_2().children(
                self.pictures.iter().zip(thumbs).enumerate().map(
                    |(ix, (picture, (focus, focused)))| {
                        let chosen = self.chosen == Some(ix);
                        let choose = self.on_choose.clone();
                        div()
                            .id((self.id.clone(), format!("thumb-{ix}")))
                            .track_focus(&focus)
                            .w(thumb)
                            .p_0p5()
                            .rounded(large)
                            .border_1()
                            .border_color(if focused {
                                colors.focus
                            } else if chosen {
                                colors.accent
                            } else {
                                transparent_black()
                            })
                            .cursor_pointer()
                            .on_mouse_down(MouseButton::Left, |_, window, _| {
                                window.prevent_default()
                            })
                            .on_click(move |_, window, cx| {
                                if chosen {
                                    return;
                                }
                                log::info!("variation picker: chose {ix}");
                                if let Some(choose) = &choose {
                                    choose(ix, window, cx);
                                }
                            })
                            .child(
                                framed(self.ratio, cx).rounded(small).child(
                                    Image::new(
                                        (self.id.clone(), format!("thumb-picture-{ix}")),
                                        source(picture),
                                    )
                                    .size_full()
                                    .rounded(small),
                                ),
                            )
                    },
                ),
            ))
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_2()
                    .children(vary(false, "Vary subtly"))
                    .children(vary(true, "Vary strongly"))
                    .children(
                        self.on_upscale
                            .clone()
                            .filter(|_| self.chosen.is_some())
                            .map(|upscale| {
                                Button::new((self.id.clone(), "upscale"), "Upscale")
                                    .variant(ButtonVariant::Secondary)
                                    .size(ControlSize::Sm)
                                    .icon(IconName::Maximize2)
                                    .on_click(move |_, window, cx| {
                                        log::info!("variation picker: upscale");
                                        upscale(window, cx)
                                    })
                            }),
                    ),
            )
    }
}
