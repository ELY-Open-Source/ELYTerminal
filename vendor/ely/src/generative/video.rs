use std::{rc::Rc, time::Duration};

use gpui::{
    App, Axis, ElementId, InteractiveElement, IntoElement, MouseButton, ParentElement, RenderOnce,
    ScrollHandle, SharedString, StatefulInteractiveElement, Styled, Window, canvas, div,
    prelude::*, relative, transparent_black,
};

use super::Outcome;
use crate::{
    buttons::{Button, ButtonVariant},
    debug::ruler,
    documents::source,
    forms::{OnValue, Run},
    layout::{bring_into_view, on_axis},
    motion::Skeleton,
    primitives::{Icon, IconName, Image, tab_stop},
    theme::{ActiveTheme, ControlSize, IconSize, TextSize},
    typography::{
        Caption,
        format::{DurationStyle, duration, percent},
        tabular,
    },
};

/// A shot of a generated video: its key, its prompt, how long it runs, and how it came out.
#[derive(Clone, Debug, PartialEq)]
pub struct Shot {
    pub key: SharedString,
    pub prompt: SharedString,
    pub length: Duration,
    pub outcome: Outcome,
}

/// A video made shot by shot, laid along its time: a ruler, each shot as wide as it runs with its poster, a breathing placeholder while it comes or a mark if it failed, and the playhead. A press or Enter chooses a shot and a line under names it; Extend asks for the next. It scrolls sideways when long.
#[derive(IntoElement)]
pub struct VideoGenerationTimeline {
    id: ElementId,
    shots: Vec<Shot>,
    at: Duration,
    selected: Option<SharedString>,
    on_select: Option<OnValue>,
    on_extend: Option<Run>,
}

impl VideoGenerationTimeline {
    /// `at` is the playhead's time.
    pub fn new(
        id: impl Into<ElementId>,
        shots: impl IntoIterator<Item = Shot>,
        at: Duration,
    ) -> Self {
        let shots: Vec<Shot> = shots.into_iter().collect();
        for shot in &shots {
            assert!(!shot.length.is_zero(), "shot {} runs no time", shot.key);
            if let Outcome::Pending(Some(share)) = shot.outcome {
                assert!(
                    (0.0..=1.0).contains(&share),
                    "shot {} at {share} of 1",
                    shot.key
                );
            }
        }
        Self {
            id: id.into(),
            shots,
            at,
            selected: None,
            on_select: None,
            on_extend: None,
        }
    }

    pub fn selected(mut self, key: impl Into<SharedString>) -> Self {
        self.selected = Some(key.into());
        self
    }

    pub fn on_select(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_select = Some(Rc::new(handler));
        self
    }

    /// Asks for one more shot after the last.
    pub fn on_extend(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_extend = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for VideoGenerationTimeline {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        if let Some(key) = &self.selected
            && !self.shots.iter().any(|shot| &shot.key == key)
        {
            log::error!("video timeline: shot {key} is not listed; none selected");
        }
        let pickable = self.on_select.is_some();
        let scroll = window
            .use_keyed_state((self.id.clone(), "scroll"), cx, |_, _| ScrollHandle::new())
            .read(cx)
            .clone();
        let revealed =
            window.use_keyed_state(
                (self.id.clone(), "revealed"),
                cx,
                |_, _| None::<SharedString>,
            );
        let focuses: Vec<_> = self
            .shots
            .iter()
            .map(|shot| {
                let focus = tab_stop(
                    (self.id.clone(), format!("focus-{}", shot.key)).into(),
                    pickable,
                    window,
                    cx,
                );
                let focused = focus.is_focused(window);
                (focus, focused)
            })
            .collect();
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let sizes = theme.generative();
        let rem = window.rem_size();
        let total: f64 = self
            .shots
            .iter()
            .map(|shot| shot.length.as_secs_f64())
            .sum();
        let second = sizes.second;
        let span = second * total as f32;
        let (_, ticks) = ruler(
            (0.0, total.max(1.0)),
            f32::from(span.to_pixels(rem)),
            f32::from(theme.chart().label.to_pixels(rem)),
        );
        let caption = (self.selected.as_ref())
            .and_then(|key| self.shots.iter().position(|shot| &shot.key == key))
            .map(|ix| {
                let shot = &self.shots[ix];
                let state = match &shot.outcome {
                    Outcome::Pending(_) => "coming".to_string(),
                    Outcome::Done(_) => duration(shot.length.as_secs(), DurationStyle::Compact),
                    Outcome::Failed(reason) => format!("failed, {reason}"),
                };
                format!("Shot {} · {state} · {}", ix + 1, shot.prompt)
            });
        let ruler_row = div()
            .relative()
            .h(theme.text_size(TextSize::Xs) * 1.6)
            .w(span)
            .border_b_1()
            .border_color(colors.border)
            .text_size(theme.text_size(TextSize::Xs))
            .text_color(colors.fg_subtle)
            .children(ticks.into_iter().map(|time| {
                tabular(div())
                    .absolute()
                    .top_0()
                    .left(relative((time / total.max(1.0)) as f32))
                    .pl_1()
                    .border_l_1()
                    .border_color(colors.border)
                    .child(duration(time as u64, DurationStyle::Clock))
            }));
        let tiles = self.shots.into_iter().zip(focuses).enumerate().map(
            |(ix, (shot, (focus, focused)))| {
                let chosen = self.selected.as_ref() == Some(&shot.key);
                let (pick, key) = (self.on_select.clone(), shot.key.clone());
                let face = match &shot.outcome {
                    Outcome::Done(poster) => {
                        Image::new((self.id.clone(), format!("poster-{ix}")), source(poster))
                            .size_full()
                            .into_any_element()
                    }
                    Outcome::Pending(share) => div()
                        .relative()
                        .size_full()
                        .child(
                            Skeleton::new((self.id.clone(), format!("pending-{ix}"))).size_full(),
                        )
                        .children(share.map(|share| {
                            div()
                                .absolute()
                                .inset_0()
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_size(theme.text_size(TextSize::Xs))
                                .text_color(colors.fg_muted)
                                .child(tabular(div()).child(percent(f64::from(share), 0, false)))
                        }))
                        .into_any_element(),
                    Outcome::Failed(_) => div()
                        .size_full()
                        .flex()
                        .items_center()
                        .justify_center()
                        .bg(colors.sunken)
                        .child(
                            Icon::new(IconName::CircleAlert)
                                .size(IconSize::Sm)
                                .color(colors.danger),
                        )
                        .into_any_element(),
                };
                div()
                    .id((self.id.clone(), format!("shot-{ix}")))
                    .flex_none()
                    .w(second * shot.length.as_secs_f32())
                    .h(sizes.shot)
                    .p_0p5()
                    .border_1()
                    .border_color(if focused {
                        colors.focus
                    } else if chosen {
                        colors.accent
                    } else {
                        transparent_black()
                    })
                    .when(pick.is_some(), |tile| {
                        let (focus, revealed, scroll, key) =
                            (focus.clone(), revealed.clone(), scroll.clone(), key.clone());
                        tile.child(
                            canvas(
                                move |bounds, window, cx| {
                                    let focused = focus.is_focused(window);
                                    let held = revealed.read(cx).as_ref() == Some(&key);
                                    if focused && !held {
                                        revealed.update(cx, |revealed, _| {
                                            *revealed = Some(key.clone())
                                        });
                                        if bring_into_view(&scroll, bounds, Axis::Horizontal) {
                                            log::info!(
                                                "video timeline: shot {key} scrolled into view"
                                            );
                                            window.request_animation_frame();
                                        }
                                    } else if !focused && held {
                                        revealed.update(cx, |revealed, _| *revealed = None);
                                    }
                                },
                                |_, _, _, _| {},
                            )
                            .absolute()
                            .top_0()
                            .left_0()
                            .size_full(),
                        )
                    })
                    .when_some(pick, |tile, pick| {
                        tile.track_focus(&focus)
                            .cursor_pointer()
                            .on_mouse_down(MouseButton::Left, |_, window, _| {
                                window.prevent_default()
                            })
                            .on_click(move |_, window, cx| {
                                if chosen {
                                    return;
                                }
                                log::info!("video timeline: chose {key}");
                                pick(&key, window, cx)
                            })
                    })
                    .child(
                        div()
                            .relative()
                            .size_full()
                            .overflow_hidden()
                            .bg(colors.sunken)
                            .child(face),
                    )
            },
        );
        let playhead = (self.at.as_secs_f64() / total.max(1.0)).clamp(0.0, 1.0) as f32;
        let extend = self.on_extend.map(|extend| {
            Button::new((self.id.clone(), "extend"), "Extend")
                .variant(ButtonVariant::Secondary)
                .size(ControlSize::Sm)
                .icon(IconName::Plus)
                .on_click(move |_, window, cx| {
                    log::info!("video timeline: extend");
                    extend(window, cx)
                })
        });
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                on_axis(div().id((self.id.clone(), "sideways")))
                    .track_scroll(&scroll)
                    .w_full()
                    .flex()
                    .overflow_x_scroll()
                    .pb_1()
                    .child(
                        div()
                            .relative()
                            .flex_none()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(ruler_row)
                            .child(div().flex().children(tiles))
                            .child(
                                div()
                                    .absolute()
                                    .top_0()
                                    .bottom_0()
                                    .left(relative(playhead))
                                    .border_l_1()
                                    .border_color(colors.accent),
                            ),
                    ),
            )
            .when(caption.is_some() || extend.is_some(), |column| {
                column.child(
                    div()
                        .flex()
                        .items_center()
                        .gap_3()
                        .child(div().flex_1().min_w_0().children(caption.map(Caption::new)))
                        .children(extend.map(|extend| div().flex_none().child(extend))),
                )
            })
    }
}
