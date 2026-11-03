use std::rc::Rc;

use gpui::{
    AnyElement, App, ElementId, ImageSource, IntoElement, ParentElement, Pixels, RenderOnce,
    SharedString, Styled, Window, canvas, div, img, prelude::*, rems,
};
use smallvec::SmallVec;

use crate::{
    buttons::{Button, ButtonVariant},
    data_display::Avatar,
    forms::Run,
    layout::columns_for,
    media::name_chip,
    primitives::{Icon, IconName, checked_ratio, framed},
    theme::{ActiveTheme, AvatarSize, ControlSize, IconSize, Radius, TextSize},
    typography::Ellipsis,
};

/// Someone in a call, in the call's frame shape: their camera's frame from the host, or their initials while it is off; their name in the lower corner with a mark when muted; a ring while they speak.
#[derive(IntoElement)]
pub struct ParticipantTile {
    id: ElementId,
    name: SharedString,
    ratio: f32,
    frame: Option<ImageSource>,
    picture: Option<ImageSource>,
    muted: bool,
    speaking: bool,
}

impl ParticipantTile {
    /// `ratio` is the call's frame width over its height.
    pub fn new(id: impl Into<ElementId>, name: impl Into<SharedString>, ratio: f32) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            ratio: checked_ratio(ratio),
            frame: None,
            picture: None,
            muted: false,
            speaking: false,
        }
    }

    /// The camera's frame to show now.
    pub fn frame(mut self, source: impl Into<ImageSource>) -> Self {
        self.frame = Some(source.into());
        self
    }

    /// Their picture, for while the camera is off.
    pub fn picture(mut self, source: impl Into<ImageSource>) -> Self {
        self.picture = Some(source.into());
        self
    }

    pub fn muted(mut self, muted: bool) -> Self {
        self.muted = muted;
        self
    }

    pub fn speaking(mut self, speaking: bool) -> Self {
        self.speaking = speaking;
        self
    }
}

impl RenderOnce for ParticipantTile {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = &theme.colors;
        let round = theme.radius(Radius::Lg);
        let stage = match self.frame {
            Some(frame) => img(frame)
                .id((self.id.clone(), "frame"))
                .size_full()
                .rounded(round)
                .into_any_element(),
            None => {
                let avatar = Avatar::new((self.id.clone(), "avatar"), self.name.clone())
                    .size(AvatarSize::Xl);
                let avatar = match self.picture {
                    Some(picture) => avatar.image(picture),
                    None => avatar,
                };
                div()
                    .absolute()
                    .inset_0()
                    .pb_7()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(avatar)
                    .into_any_element()
            }
        };
        let (muted_id, speaking_id) = (self.id.clone(), self.id.clone());
        let muted = self.muted.then(|| {
            div()
                .debug_selector(move || format!("tile-muted {muted_id}"))
                .flex_none()
                .child(
                    Icon::new(IconName::MicOff)
                        .size(IconSize::Xs)
                        .color(colors.on_media),
                )
        });
        let speaking = self.speaking.then(|| {
            div()
                .debug_selector(move || format!("tile-speaking {speaking_id}"))
                .absolute()
                .inset_0()
                .rounded(round)
                .border_2()
                .border_color(colors.success)
        });
        framed(self.ratio, cx)
            .debug_selector(|| "participant-tile".into())
            .rounded(round)
            .bg(colors.sunken)
            .child(stage)
            .child(
                name_chip(cx)
                    .children(muted)
                    .child(div().min_w_0().child(Ellipsis::new(self.name))),
            )
            .children(speaking)
    }
}

/// How many columns hold `count` tiles: the least square that fits them.
fn columns(count: usize) -> usize {
    (1..=count)
        .find(|columns| columns * columns >= count)
        .expect("a call holds a tile")
}

/// Everyone in a call, as tiles in even rows: the columns grow with the count as far as tiles fit across, and a short last row centers at the same width. It holds a tile at least.
#[derive(IntoElement)]
pub struct VideoCallGrid {
    id: ElementId,
    tiles: SmallVec<[AnyElement; 8]>,
}

impl VideoCallGrid {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            tiles: SmallVec::new(),
        }
    }
}

impl ParentElement for VideoCallGrid {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.tiles.extend(elements);
    }
}

impl RenderOnce for VideoCallGrid {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let width = window.use_keyed_state((self.id.clone(), "width"), cx, |_, _| None::<Pixels>);
        let count = self.tiles.len();
        let rem = window.rem_size();
        let cell = cx.theme().messaging().tile.to_pixels(rem) + rems(0.5).to_pixels(rem);
        let fit = match *width.read(cx) {
            Some(width) => columns_for(width, cell, Pixels::ZERO) as usize,
            None => count,
        };
        let across = columns(count).min(fit);
        let mut tiles = self.tiles.into_iter();
        let rows = (0..count.div_ceil(across)).map(|_| {
            let row: Vec<AnyElement> = tiles.by_ref().take(across).collect();
            let spare = (across - row.len()) as f32 / 2.0;
            let spacer = || {
                let mut spacer = div().flex_basis(gpui::relative(0.));
                spacer.style().flex_grow = Some(spare);
                spacer
            };
            div()
                .flex()
                .when(spare > 0.0, |line| line.child(spacer()))
                .children(
                    row.into_iter()
                        .map(|tile| div().flex_1().min_w_0().child(div().p_1().child(tile))),
                )
                .when(spare > 0.0, |line| line.child(spacer()))
        });
        let measured = width.clone();
        div()
            .debug_selector(|| "video-call-grid".into())
            .relative()
            .flex()
            .flex_col()
            .children(rows)
            .child(
                canvas(
                    move |bounds, window, cx| {
                        if *measured.read(cx) != Some(bounds.size.width) {
                            measured.update(cx, |width, cx| {
                                *width = Some(bounds.size.width);
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
                .size_full(),
            )
    }
}

/// A shared screen in its shape, from frames the host hands in, under a bar that names who presents; your own share, the one with a stop, offers Stop presenting. The call's tiles can run beside it, or below on a narrow window.
#[derive(IntoElement)]
pub struct ScreenShareView {
    id: ElementId,
    presenter: SharedString,
    ratio: f32,
    frame: Option<ImageSource>,
    on_stop: Option<Run>,
    tiles: SmallVec<[AnyElement; 4]>,
}

impl ScreenShareView {
    /// `ratio` is the shared screen's width over its height.
    pub fn new(id: impl Into<ElementId>, presenter: impl Into<SharedString>, ratio: f32) -> Self {
        Self {
            id: id.into(),
            presenter: presenter.into(),
            ratio: checked_ratio(ratio),
            frame: None,
            on_stop: None,
            tiles: SmallVec::new(),
        }
    }

    /// The screen's frame to show now.
    pub fn frame(mut self, source: impl Into<ImageSource>) -> Self {
        self.frame = Some(source.into());
        self
    }

    /// Offers Stop presenting, for your own share.
    pub fn on_stop(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_stop = Some(Rc::new(handler));
        self
    }
}

impl ParentElement for ScreenShareView {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.tiles.extend(elements);
    }
}

impl RenderOnce for ScreenShareView {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = &theme.colors;
        let round = theme.radius(Radius::Lg);
        let presenter = self.presenter.clone();
        let (bar, waiting) = match self.on_stop {
            Some(_) => (
                "You are presenting".to_string(),
                "Waiting for your screen".to_string(),
            ),
            None => (
                format!("{} is presenting", self.presenter),
                format!("Waiting for {}'s screen", self.presenter),
            ),
        };
        let stop = self.on_stop.map(|run| {
            Button::new((self.id.clone(), "stop"), "Stop presenting")
                .icon(IconName::ScreenShareOff)
                .variant(ButtonVariant::Danger)
                .size(ControlSize::Sm)
                .on_click(move |_, window, cx| {
                    log::info!("screen share {presenter}: stop");
                    run(window, cx)
                })
        });
        let screen = match self.frame {
            Some(frame) => img(frame)
                .id((self.id.clone(), "frame"))
                .size_full()
                .rounded(round)
                .into_any_element(),
            None => div()
                .debug_selector(|| "screen-waiting".into())
                .absolute()
                .inset_0()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap_2()
                .px_4()
                .text_color(colors.fg_muted)
                .child(
                    Icon::new(IconName::ScreenShare)
                        .size(IconSize::Lg)
                        .color(colors.fg_subtle),
                )
                .child(div().max_w_full().child(Ellipsis::new(waiting)))
                .into_any_element(),
        };
        let strip = (!self.tiles.is_empty()).then(|| {
            div()
                .flex_none()
                .w(theme.messaging().tiles)
                .flex()
                .flex_col()
                .gap_2()
                .children(self.tiles)
        });
        div()
            .debug_selector(|| "screen-share".into())
            .flex()
            .flex_col()
            .gap_2()
            .text_size(theme.text_size(TextSize::Sm))
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .child(
                        div()
                            .flex_1()
                            .min_w(theme.label_width())
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                Icon::new(IconName::ScreenShare)
                                    .size(IconSize::Sm)
                                    .color(colors.fg_muted),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .text_color(colors.fg)
                                    .child(Ellipsis::new(bar)),
                            ),
                    )
                    .children(stop.map(|stop| div().flex_none().child(stop))),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_start()
                    .gap_2()
                    .child(
                        div().flex_1().min_w(theme.label_width()).child(
                            framed(self.ratio, cx)
                                .rounded(round)
                                .bg(colors.sunken)
                                .child(screen),
                        ),
                    )
                    .children(strip),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::columns;

    #[test]
    fn columns_are_the_least_square_that_holds_the_tiles() {
        let wanted = [1, 2, 2, 2, 3, 3, 3, 3, 3, 4];
        for (count, columns_wanted) in (1..=10).zip(wanted) {
            assert_eq!(columns(count), columns_wanted, "{count} tiles");
        }
    }
}
