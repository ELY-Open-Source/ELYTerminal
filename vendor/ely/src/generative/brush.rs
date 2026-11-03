use std::rc::Rc;

use gpui::{App, ElementId, IntoElement, ParentElement, RenderOnce, Styled, Window, div};

use super::mask::{OnBrush, REACH};
use crate::{
    buttons::{IconButton, SegmentedControl},
    forms::{Run, Slider},
    primitives::IconName,
    theme::{ActiveTheme, ControlSize, TextSize},
    typography::{format::percent, tabular},
};

/// The mask brush: paint or erase, its size, and a way back: Undo takes the last stroke, Clear takes them all.
#[derive(IntoElement)]
pub struct MaskBrush {
    id: ElementId,
    radius: f32,
    erase: bool,
    strokes: usize,
    on_brush: Option<OnBrush>,
    on_undo: Option<Run>,
    on_clear: Option<Run>,
}

impl MaskBrush {
    /// `radius` is a share of the picture's width; `strokes` counts the mask's strokes, which Undo and Clear wait for.
    pub fn new(id: impl Into<ElementId>, radius: f32, erase: bool, strokes: usize) -> Self {
        assert!(
            (REACH.0..=REACH.1).contains(&radius),
            "a brush radius of {radius}"
        );
        Self {
            id: id.into(),
            radius,
            erase,
            strokes,
            on_brush: None,
            on_undo: None,
            on_clear: None,
        }
    }

    /// Gets the brush's radius and whether it erases.
    pub fn on_brush(
        mut self,
        handler: impl Fn(f32, bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_brush = Some(Rc::new(handler));
        self
    }

    pub fn on_undo(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_undo = Some(Rc::new(handler));
        self
    }

    pub fn on_clear(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_clear = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for MaskBrush {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let (radius, erase, empty) = (self.radius, self.erase, self.strokes == 0);
        let (mode, size) = (self.on_brush.clone(), self.on_brush);
        let back = |key: &'static str, icon: IconName, tip: &'static str, run: Option<Run>| {
            run.map(|run| {
                IconButton::new((self.id.clone(), key), icon)
                    .size(ControlSize::Sm)
                    .tooltip(tip)
                    .disabled(empty)
                    .on_click(move |_, window, cx| {
                        log::info!("mask brush: {key}");
                        run(window, cx)
                    })
            })
        };
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .child(
                        SegmentedControl::new(
                            (self.id.clone(), "mode"),
                            if erase { "erase" } else { "paint" },
                        )
                        .segment("paint", "Paint", Some(IconName::Brush))
                        .segment("erase", "Erase", Some(IconName::Eraser))
                        .size(ControlSize::Sm)
                        .on_change(move |value, window, cx| {
                            if let Some(mode) = &mode {
                                mode(radius, value == "erase", window, cx);
                            }
                        }),
                    )
                    .child(
                        div()
                            .flex()
                            .gap_1()
                            .children(back(
                                "undo",
                                IconName::Undo2,
                                "Undo the last stroke",
                                self.on_undo,
                            ))
                            .children(back(
                                "clear",
                                IconName::Trash2,
                                "Clear the mask",
                                self.on_clear,
                            )),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .text_size(theme.text_size(TextSize::Sm))
                    .child(div().flex_none().text_color(colors.fg_muted).child("Size"))
                    .child(
                        div().flex_1().min_w_0().child(
                            Slider::new((self.id.clone(), "size"), f64::from(radius))
                                .range(f64::from(REACH.0), f64::from(REACH.1))
                                .step(0.005)
                                .on_change(move |value, window, cx| {
                                    if let Some(size) = &size {
                                        size(value as f32, erase, window, cx);
                                    }
                                }),
                        ),
                    )
                    .child(
                        tabular(div())
                            .flex_none()
                            .text_color(colors.fg_muted)
                            .child(percent(f64::from(radius), 1, false)),
                    ),
            )
    }
}
