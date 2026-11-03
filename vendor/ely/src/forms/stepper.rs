use std::rc::Rc;

use gpui::{App, ElementId, IntoElement, ParentElement, RenderOnce, Styled, Window, div};

use super::options::OnNumber;
use crate::{
    buttons::{ButtonVariant, IconButton},
    primitives::IconName,
    theme::{ActiveTheme, ControlSize, Radius, TextSize},
    typography::format::{self, Separators},
};

/// Minus and plus around a number that cannot be typed.
#[derive(IntoElement)]
pub struct Stepper {
    id: ElementId,
    value: f64,
    min: f64,
    max: f64,
    step: f64,
    precision: usize,
    on_change: Option<OnNumber>,
}

impl Stepper {
    pub fn new(id: impl Into<ElementId>, value: f64) -> Self {
        Self {
            id: id.into(),
            value,
            min: f64::MIN,
            max: f64::MAX,
            step: 1.0,
            precision: 0,
            on_change: None,
        }
    }

    pub fn range(mut self, min: f64, max: f64) -> Self {
        assert!(min <= max, "stepper range {min}..{max} is empty");
        self.min = min;
        self.max = max;
        self
    }

    pub fn step(mut self, step: f64) -> Self {
        assert!(step > 0.0, "stepper step {step} must be positive");
        self.step = step;
        self
    }

    /// Decimal places shown.
    pub fn precision(mut self, places: usize) -> Self {
        self.precision = places;
        self
    }

    pub fn on_change(mut self, handler: impl Fn(f64, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for Stepper {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let (value, min, max) = (self.value, self.min, self.max);
        let nudge = |by: f64| {
            let (id, on_change) = (self.id.clone(), self.on_change.clone());
            move |_: &gpui::ClickEvent, window: &mut Window, cx: &mut App| {
                let next = (value + by).clamp(min, max);
                log::info!("stepper {id:?}: {next}");
                if let Some(on_change) = &on_change {
                    on_change(next, window, cx);
                }
            }
        };
        div()
            .flex()
            .items_center()
            .h(theme.control_height(ControlSize::Md))
            .p_0p5()
            .rounded(theme.radius(Radius::Md))
            .border_1()
            .border_color(theme.colors.border_strong)
            .bg(theme.colors.surface)
            .child(
                IconButton::new((self.id.clone(), "less"), IconName::Minus)
                    .variant(ButtonVariant::Ghost)
                    .size(ControlSize::Sm)
                    .disabled(value <= min)
                    .on_click(nudge(-self.step)),
            )
            .child(
                div()
                    .min_w(theme.control_height(ControlSize::Md))
                    .text_center()
                    .text_size(theme.text_size(TextSize::Base))
                    .text_color(theme.colors.fg)
                    .child(format::number(value, self.precision, Separators::EN)),
            )
            .child(
                IconButton::new((self.id.clone(), "more"), IconName::Plus)
                    .variant(ButtonVariant::Ghost)
                    .size(ControlSize::Sm)
                    .disabled(value >= max)
                    .on_click(nudge(self.step)),
            )
    }
}
