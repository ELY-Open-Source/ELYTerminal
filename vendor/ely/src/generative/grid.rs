use std::rc::Rc;

use gpui::{
    App, ElementId, FontWeight, InteractiveElement, IntoElement, MouseButton, ParentElement,
    RenderOnce, SharedString, StatefulInteractiveElement, Styled, Window, div, prelude::*,
    transparent_black,
};

use crate::{
    buttons::{Button, ButtonVariant},
    documents::source,
    forms::Pick,
    motion::Skeleton,
    primitives::{Icon, IconName, Image, checked_ratio, framed, tab_stop},
    theme::{ActiveTheme, ControlSize, IconSize, Radius, TextSize},
    typography::{format::percent, tabular},
};

/// One result of a run: on its way, with its share done when known; a picture; or why it failed.
#[derive(Clone, Debug, PartialEq)]
pub enum Outcome {
    Pending(Option<f32>),
    Done(SharedString),
    Failed(SharedString),
}

/// A run's results in columns, each in the run's shape: a breathing placeholder and its percent while it comes, the picture once done, or why it failed with Retry. A done picture is a Tab stop; a press or Enter opens it.
#[derive(IntoElement)]
pub struct GenerationGrid {
    id: ElementId,
    results: Vec<Outcome>,
    ratio: f32,
    columns: u16,
    on_open: Option<Pick>,
    on_retry: Option<Pick>,
}

impl GenerationGrid {
    /// `ratio` is the run's width over height.
    pub fn new(
        id: impl Into<ElementId>,
        results: impl IntoIterator<Item = Outcome>,
        ratio: f32,
    ) -> Self {
        let results: Vec<Outcome> = results.into_iter().collect();
        for result in &results {
            if let Outcome::Pending(Some(share)) = result {
                assert!((0.0..=1.0).contains(share), "a result at {share} of 1");
            }
        }
        Self {
            id: id.into(),
            results,
            ratio: checked_ratio(ratio),
            columns: 2,
            on_open: None,
            on_retry: None,
        }
    }

    pub fn columns(mut self, columns: u16) -> Self {
        assert!(columns > 0, "a grid needs a column");
        self.columns = columns;
        self
    }

    pub fn on_open(mut self, handler: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_open = Some(Rc::new(handler));
        self
    }

    pub fn on_retry(mut self, handler: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_retry = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for GenerationGrid {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let focuses: Vec<_> = (0..self.results.len())
            .map(|ix| {
                let id = (self.id.clone(), format!("focus-{ix}")).into();
                let pressable =
                    matches!(self.results[ix], Outcome::Done(_)) && self.on_open.is_some();
                let focus = tab_stop(id, pressable, window, cx);
                let focused = focus.is_focused(window);
                (focus, focused)
            })
            .collect();
        let radius = cx.theme().radius(Radius::Md);
        let tiles: Vec<_> = self
            .results
            .into_iter()
            .zip(focuses)
            .enumerate()
            .map(|(ix, (result, (focus, focused)))| {
                let theme = cx.theme();
                let colors = theme.colors.clone();
                let frame = framed(self.ratio, cx)
                    .id((self.id.clone(), format!("result-{ix}")))
                    .rounded(radius)
                    .border_1()
                    .border_color(if focused {
                        colors.focus
                    } else {
                        transparent_black()
                    });
                match result {
                    Outcome::Pending(share) => frame
                        .child(
                            Skeleton::new((self.id.clone(), format!("pending-{ix}")))
                                .size_full()
                                .rounded(radius),
                        )
                        .children(share.map(|share| {
                            div()
                                .absolute()
                                .inset_0()
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_size(theme.text_size(TextSize::Sm))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(colors.fg_muted)
                                .child(tabular(div()).child(percent(f64::from(share), 0, false)))
                        }))
                        .into_any_element(),
                    Outcome::Done(picture) => {
                        let open = self.on_open.clone();
                        frame
                            .when_some(open, |tile, open| {
                                tile.track_focus(&focus)
                                    .cursor_pointer()
                                    .on_mouse_down(MouseButton::Left, |_, window, _| {
                                        window.prevent_default()
                                    })
                                    .on_click(move |_, window, cx| {
                                        log::info!("generation grid: open {ix}");
                                        open(ix, window, cx)
                                    })
                            })
                            .child(
                                Image::new(
                                    (self.id.clone(), format!("picture-{ix}")),
                                    source(&picture),
                                )
                                .size_full()
                                .rounded(radius),
                            )
                            .into_any_element()
                    }
                    Outcome::Failed(reason) => {
                        let retry = self.on_retry.clone();
                        frame
                            .child(
                                div()
                                    .absolute()
                                    .inset_0()
                                    .p_3()
                                    .flex()
                                    .flex_col()
                                    .items_center()
                                    .justify_center()
                                    .gap_2()
                                    .text_size(theme.text_size(TextSize::Xs))
                                    .text_color(colors.fg_muted)
                                    .child(
                                        Icon::new(IconName::CircleAlert)
                                            .size(IconSize::Md)
                                            .color(colors.danger),
                                    )
                                    .child(div().text_center().child(reason))
                                    .children(retry.map(|retry| {
                                        Button::new(
                                            (self.id.clone(), format!("retry-{ix}")),
                                            "Retry",
                                        )
                                        .variant(ButtonVariant::Secondary)
                                        .size(ControlSize::Sm)
                                        .on_click(
                                            move |_, window, cx| {
                                                log::info!("generation grid: retry {ix}");
                                                retry(ix, window, cx)
                                            },
                                        )
                                    })),
                            )
                            .into_any_element()
                    }
                }
            })
            .collect();
        let columns = usize::from(self.columns);
        let mut tiles = tiles.into_iter().peekable();
        let mut rows = Vec::new();
        while tiles.peek().is_some() {
            let row: Vec<_> = tiles.by_ref().take(columns).collect();
            let short = columns - row.len();
            rows.push(
                div()
                    .flex()
                    .gap_2()
                    .children(
                        row.into_iter()
                            .map(|tile| div().flex_1().min_w_0().child(tile)),
                    )
                    .children((0..short).map(|_| div().flex_1())),
            );
        }
        div().flex().flex_col().gap_2().children(rows)
    }
}
