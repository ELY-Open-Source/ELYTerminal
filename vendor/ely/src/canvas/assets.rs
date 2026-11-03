use std::rc::Rc;

use gpui::{
    AnyElement, App, ElementId, IntoElement, ParentElement, RenderOnce, SharedString, Styled,
    Window, canvas, div,
};

use super::{
    paint::paint_shape,
    shape::{Shape, ShapeKind},
    view::Viewport,
};
use crate::{
    forms::{Grid, finder, grid, rows},
    theme::{ActiveTheme, TextSize},
};

type OnShape = Rc<dyn Fn(&Shape, &mut Window, &mut App)>;

const COLUMNS: usize = 4;

/// Shapes to place, in titled groups, each drawn to fit its tile and found by name. A press or Enter hands the owner the shape.
#[derive(IntoElement)]
pub struct AssetPanel {
    id: ElementId,
    groups: Vec<(SharedString, Vec<Shape>)>,
    on_pick: Option<OnShape>,
}

impl AssetPanel {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            groups: Vec::new(),
            on_pick: None,
        }
    }

    pub fn group(
        mut self,
        title: impl Into<SharedString>,
        shapes: impl IntoIterator<Item = Shape>,
    ) -> Self {
        self.groups
            .push((title.into(), shapes.into_iter().collect()));
        self
    }

    pub fn on_pick(mut self, handler: impl Fn(&Shape, &mut Window, &mut App) + 'static) -> Self {
        self.on_pick = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for AssetPanel {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let finder = finder(&self.id, "Search assets", window, cx);
        let query = finder.read(cx).query.clone();
        let found: Vec<(SharedString, Vec<Shape>)> = self
            .groups
            .into_iter()
            .map(|(title, shapes)| {
                let named = shapes
                    .into_iter()
                    .filter(|shape| shape.name.to_lowercase().contains(&query))
                    .collect();
                (title, named)
            })
            .collect();
        let layout = if query.is_empty() {
            rows(
                found
                    .iter()
                    .map(|(title, shapes)| (Some(title.clone()), shapes.len())),
                COLUMNS,
            )
        } else {
            rows(
                [(None, found.iter().map(|(_, shapes)| shapes.len()).sum())],
                COLUMNS,
            )
        };
        let shapes: Rc<Vec<Shape>> =
            Rc::new(found.into_iter().flat_map(|(_, shapes)| shapes).collect());
        let (painted, picked, on_pick) = (shapes.clone(), shapes.clone(), self.on_pick);
        let cell = cx.theme().canvas().asset;
        grid(
            Grid {
                id: self.id,
                names: Rc::new(shapes.iter().map(|shape| shape.name.clone()).collect()),
                rows: Rc::new(layout),
                selected: None,
                draw: Rc::new(move |ix, cx| thumbnail(painted[ix].clone(), cx)),
                pick: Rc::new(move |ix, window, cx| {
                    log::info!("asset panel: {}", picked[ix].key);
                    if let Some(on_pick) = &on_pick {
                        on_pick(&picked[ix], window, cx);
                    }
                }),
                none: "No assets found",
                columns: COLUMNS,
                cell,
            },
            &finder,
            window,
            cx,
        )
    }
}

/// A shape drawn to fit its tile. Text drawn so small would not read, so a text tile shows a sample of letters.
fn thumbnail(shape: Shape, cx: &App) -> AnyElement {
    if let ShapeKind::Text(_) = shape.kind {
        let theme = cx.theme();
        return div()
            .text_size(theme.text_size(TextSize::Lg))
            .text_color(theme.colors.fg)
            .child("Aa")
            .into_any_element();
    }
    canvas(
        |_, _, _| {},
        move |bounds, _, window, cx| {
            let theme = cx.theme();
            let rem = window.rem_size();
            let sizes = theme.canvas();
            let inset = f32::from(sizes.asset_inset.to_pixels(rem));
            let room = (f32::from(bounds.size.width), f32::from(bounds.size.height));
            let view = Viewport::fitting(shape.frame, room, inset);
            let stroke = sizes.stroke.to_pixels(rem);
            paint_shape(&shape, &view, bounds.origin, &theme.colors, stroke, window);
        },
    )
    .size_full()
    .into_any_element()
}
