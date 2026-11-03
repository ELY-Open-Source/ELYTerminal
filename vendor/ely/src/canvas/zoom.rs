use std::rc::Rc;

use gpui::{App, ElementId, IntoElement, ParentElement, RenderOnce, Styled, Window, div};

use super::view::Viewport;
use crate::{
    buttons::{Button, ButtonVariant, IconButton},
    forms::Run,
    primitives::IconName,
    theme::ControlSize,
};

type OnZoom = Rc<dyn Fn(f32, &mut Window, &mut App)>;

/// The zooms the buttons step through.
pub(crate) const STEPS: [f32; 11] = [0.1, 0.25, 0.5, 0.75, 1.0, 1.5, 2.0, 3.0, 4.0, 6.0, 8.0];

/// The step past `zoom` going in, or out, if there is one.
pub(crate) fn stepped(zoom: f32, inward: bool) -> Option<f32> {
    match inward {
        true => STEPS.into_iter().find(|step| *step > zoom + 0.001),
        false => STEPS.into_iter().rev().find(|step| *step < zoom - 0.001),
    }
}

/// A zoom in percent; a press sets it back to 100%.
#[derive(IntoElement)]
pub struct ZoomIndicator {
    id: ElementId,
    zoom: f32,
    on_zoom: Option<OnZoom>,
}

impl ZoomIndicator {
    pub fn new(id: impl Into<ElementId>, zoom: f32) -> Self {
        let (least, most) = Viewport::ZOOMS;
        assert!((least..=most).contains(&zoom), "zoom {zoom}");
        Self {
            id: id.into(),
            zoom,
            on_zoom: None,
        }
    }

    /// Gets the zoom asked for, one on a press.
    pub fn on_zoom(mut self, handler: impl Fn(f32, &mut Window, &mut App) + 'static) -> Self {
        self.on_zoom = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for ZoomIndicator {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let words = format!("{}%", (self.zoom * 100.0).round());
        let on_zoom = self.on_zoom;
        Button::new(self.id, words)
            .variant(ButtonVariant::Ghost)
            .size(ControlSize::Sm)
            .on_click(move |_, window, cx| {
                log::info!("zoom indicator: back to 100%");
                if let Some(on_zoom) = &on_zoom {
                    on_zoom(1.0, window, cx);
                }
            })
    }
}

/// Out, the zoom in percent, and in, through fixed steps; with `on_fit`, a button that fits what lies on the canvas.
#[derive(IntoElement)]
pub struct ZoomControls {
    id: ElementId,
    zoom: f32,
    on_zoom: Option<OnZoom>,
    on_fit: Option<Run>,
}

impl ZoomControls {
    pub fn new(id: impl Into<ElementId>, zoom: f32) -> Self {
        Self {
            id: id.into(),
            zoom,
            on_zoom: None,
            on_fit: None,
        }
    }

    /// Gets each zoom asked for.
    pub fn on_zoom(mut self, handler: impl Fn(f32, &mut Window, &mut App) + 'static) -> Self {
        self.on_zoom = Some(Rc::new(handler));
        self
    }

    pub fn on_fit(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_fit = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for ZoomControls {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let (id, zoom) = (self.id.clone(), self.zoom);
        let button = |inward: bool| {
            let next = stepped(zoom, inward);
            let on_zoom = self.on_zoom.clone();
            let (icon, words) = match inward {
                true => (IconName::ZoomIn, "Zoom in"),
                false => (IconName::ZoomOut, "Zoom out"),
            };
            IconButton::new((id.clone(), words), icon)
                .variant(ButtonVariant::Ghost)
                .size(ControlSize::Sm)
                .tooltip(words)
                .disabled(next.is_none())
                .on_click(move |_, window, cx| {
                    let next = next.expect("a step lies that way");
                    log::info!("zoom controls: {}%", (next * 100.0).round());
                    if let Some(on_zoom) = &on_zoom {
                        on_zoom(next, window, cx);
                    }
                })
        };
        let indicator = {
            let on_zoom = self.on_zoom.clone();
            ZoomIndicator::new((id.clone(), "level"), zoom).on_zoom(move |zoom, window, cx| {
                if let Some(on_zoom) = &on_zoom {
                    on_zoom(zoom, window, cx);
                }
            })
        };
        let fit = self.on_fit.map(|on_fit| {
            IconButton::new((id.clone(), "fit"), IconName::Maximize2)
                .variant(ButtonVariant::Ghost)
                .size(ControlSize::Sm)
                .tooltip("Fit to screen")
                .on_click(move |_, window, cx| on_fit(window, cx))
        });
        div()
            .flex()
            .items_center()
            .gap_0p5()
            .child(button(false))
            .child(indicator)
            .child(button(true))
            .children(fit)
    }
}

#[cfg(test)]
mod tests {
    use super::stepped;

    #[test]
    fn zoom_steps_go_to_the_next_stop_and_end_at_the_last() {
        assert_eq!(stepped(1.0, true), Some(1.5));
        assert_eq!(stepped(1.2, false), Some(1.0));
        assert_eq!(stepped(8.0, true), None);
        assert_eq!(stepped(0.1, false), None);
    }
}
