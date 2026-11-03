use std::rc::Rc;

use gpui::{App, ElementId, IntoElement, ParentElement, RenderOnce, Styled, Window, div};

use super::{CoordFormat, CoordinateDisplay, LatLon, MapView};
use crate::{
    layout::seeded::use_seeded,
    primitives::{Icon, IconName},
    theme::{ActiveTheme, IconSize},
};

type OnPick = Rc<dyn Fn(LatLon, &mut Window, &mut App)>;

/// A map whose center is the pick: a crosshair marks it, the place shows below with a copy button, and the owner hears each move, by pointer or keys. It fills its box; the host gives it a height.
#[derive(IntoElement)]
pub struct LocationPicker {
    id: ElementId,
    map: MapView,
    format: CoordFormat,
    on_pick: Option<OnPick>,
}

impl LocationPicker {
    /// Takes the map's moves; a map that has its own `on_viewport` fails at render.
    pub fn new(id: impl Into<ElementId>, map: MapView) -> Self {
        Self {
            id: id.into(),
            map,
            format: CoordFormat::default(),
            on_pick: None,
        }
    }

    pub fn format(mut self, format: CoordFormat) -> Self {
        self.format = format;
        self
    }

    pub fn on_pick(mut self, handler: impl Fn(LatLon, &mut Window, &mut App) + 'static) -> Self {
        self.on_pick = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for LocationPicker {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        assert!(
            self.map.on_viewport.is_none(),
            "location picker {:?}: it hears the map's moves; pass on_pick",
            self.id
        );
        let local = use_seeded((self.id.clone(), "pick"), self.map.viewport, window, cx);
        let view = local.read(cx).value;
        let on_pick = self.on_pick;
        let mut map = self.map;
        map.viewport = view;
        map.on_viewport = Some(Rc::new(move |next, window, cx| {
            local.update(cx, |local, cx| {
                local.value = next;
                cx.notify();
            });
            if let Some(on_pick) = &on_pick {
                on_pick(next.center, window, cx);
            }
        }));
        let fg = cx.theme().colors.fg;
        div()
            .h_full()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div().relative().flex_1().min_h_0().child(map).child(
                    div()
                        .absolute()
                        .inset_0()
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(Icon::new(IconName::Crosshair).size(IconSize::Lg).color(fg)),
                ),
            )
            .child(CoordinateDisplay::new((self.id, "place"), view.center).format(self.format))
    }
}
