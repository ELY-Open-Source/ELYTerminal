use std::rc::Rc;

use gpui::{
    App, ElementId, InteractiveElement, IntoElement, MouseButton, ParentElement, RenderOnce,
    SharedString, StatefulInteractiveElement, Styled, Window, div,
};

use super::NumberInput;
use crate::theme::{ActiveTheme, Radius};

type OnUnit = Rc<dyn Fn(&SharedString, &mut Window, &mut App)>;

/// A number with a unit you click through, such as px, % and em.
#[derive(IntoElement)]
pub struct UnitInput {
    field: NumberInput,
    units: Vec<SharedString>,
    unit: SharedString,
    on_unit: Option<OnUnit>,
}

impl UnitInput {
    pub fn new(
        id: impl Into<ElementId>,
        value: f64,
        units: impl IntoIterator<Item = impl Into<SharedString>>,
        unit: impl Into<SharedString>,
    ) -> Self {
        let units: Vec<SharedString> = units.into_iter().map(Into::into).collect();
        let unit = unit.into();
        assert!(!units.is_empty(), "unit input has no units");
        Self {
            field: NumberInput::new(id, value),
            units,
            unit,
            on_unit: None,
        }
    }

    pub fn range(mut self, min: f64, max: f64) -> Self {
        self.field = self.field.range(min, max);
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

    /// Runs with the next unit when the unit is clicked.
    pub fn on_unit(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_unit = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for UnitInput {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let next = match self.units.iter().position(|unit| *unit == self.unit) {
            Some(at) => self.units[(at + 1) % self.units.len()].clone(),
            None => {
                log::error!("unit input: {} is not among {:?}", self.unit, self.units);
                self.units[0].clone()
            }
        };
        let on_unit = self.on_unit;
        let hover = theme.colors.hover;
        let unit = div()
            .id((self.field.id.clone(), "unit"))
            .px_1p5()
            .rounded(theme.radius(Radius::Sm))
            .text_color(theme.colors.fg_muted)
            .cursor_pointer()
            .hover(|style| style.bg(hover))
            .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
            .on_click(move |_, window, cx| {
                log::info!("unit input: {next}");
                if let Some(on_unit) = &on_unit {
                    on_unit(&next, window, cx);
                }
            })
            .child(self.unit);
        self.field.suffix(unit)
    }
}
