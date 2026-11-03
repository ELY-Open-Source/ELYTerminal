//! A number field with a label you drag.

use gpui::{App, ElementId, IntoElement, RenderOnce, SharedString, Window};

use super::NumberInput;

/// A label you drag sideways to change a number, beside its field.
#[derive(IntoElement)]
pub struct ScrubInput {
    field: NumberInput,
}

impl ScrubInput {
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>, value: f64) -> Self {
        let mut field = NumberInput::new(id, value);
        field.scrub_label = Some(label.into());
        Self { field }
    }

    pub fn range(mut self, min: f64, max: f64) -> Self {
        self.field = self.field.range(min, max);
        self
    }

    pub fn step(mut self, step: f64) -> Self {
        self.field = self.field.step(step);
        self
    }

    pub fn precision(mut self, places: usize) -> Self {
        self.field = self.field.precision(places);
        self
    }

    pub fn on_change(mut self, handler: impl Fn(f64, &mut Window, &mut App) + 'static) -> Self {
        self.field = self.field.on_change(handler);
        self
    }
}

impl RenderOnce for ScrubInput {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        self.field
    }
}
