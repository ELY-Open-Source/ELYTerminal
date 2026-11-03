use std::rc::Rc;

use gpui::{
    App, ElementId, Hsla, InteractiveElement, IntoElement, MouseButton, ParentElement, RenderOnce,
    SharedString, StatefulInteractiveElement, Styled, Window, div, prelude::*, transparent_black,
};

use crate::{
    primitives::FocusRing,
    theme::{ActiveTheme, Elevation, Radius, TextSize},
    typography::tabular,
};

type OnToggle = Rc<dyn Fn(&SharedString, &mut Window, &mut App)>;

/// A chart's key: a dot in each series' color and its name; with `on_toggle`, a press or Enter hides or shows one.
#[derive(IntoElement)]
pub struct ChartLegend {
    id: ElementId,
    entries: Vec<(Hsla, SharedString, bool)>,
    on_toggle: Option<OnToggle>,
}

impl ChartLegend {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            entries: Vec::new(),
            on_toggle: None,
        }
    }

    /// An entry; a hidden one shows faint.
    pub fn entry(mut self, color: Hsla, name: impl Into<SharedString>, hidden: bool) -> Self {
        self.entries.push((color, name.into(), hidden));
        self
    }

    pub fn on_toggle(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_toggle = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for ChartLegend {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        div()
            .flex()
            .flex_wrap()
            .gap_x_4()
            .gap_y_1()
            .text_size(theme.text_size(TextSize::Xs))
            .text_color(theme.colors.fg_muted)
            .children(
                self.entries
                    .into_iter()
                    .enumerate()
                    .map(|(ix, (color, name, hidden))| {
                        let (on_toggle, key) = (self.on_toggle.clone(), name.clone());
                        div()
                            .id((self.id.clone(), format!("entry-{ix}")))
                            .flex()
                            .when_some(on_toggle, |entry, on_toggle| {
                                entry
                                    .px_1()
                                    .rounded(theme.radius(Radius::Sm))
                                    .border_1()
                                    .border_color(transparent_black())
                                    .tab_index(0)
                                    .focus_ring(cx)
                                    .cursor_pointer()
                                    .on_mouse_down(MouseButton::Left, |_, window, _| {
                                        window.prevent_default()
                                    })
                                    .on_click(move |_, window, cx| on_toggle(&key, window, cx))
                            })
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_1p5()
                                    .when(hidden, |key| key.opacity(0.4))
                                    .child(div().size(theme.status_dot()).rounded_full().bg(color))
                                    .child(name),
                            )
                    }),
            )
    }
}

/// A chart's reading at one place: a title, then a row for each series with its dot, name and value.
#[derive(IntoElement)]
pub struct ChartTooltip {
    title: SharedString,
    rows: Vec<(Option<Hsla>, SharedString, SharedString)>,
}

impl ChartTooltip {
    pub fn new(title: impl Into<SharedString>) -> Self {
        Self {
            title: title.into(),
            rows: Vec::new(),
        }
    }

    /// A row: its dot's color if any, its name, and its value as read.
    pub fn row(
        mut self,
        color: Option<Hsla>,
        name: impl Into<SharedString>,
        value: impl Into<SharedString>,
    ) -> Self {
        self.rows.push((color, name.into(), value.into()));
        self
    }
}

impl RenderOnce for ChartTooltip {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = &theme.colors;
        div()
            .flex()
            .flex_col()
            .gap_0p5()
            .px_2()
            .py_1p5()
            .rounded(theme.radius(Radius::Md))
            .bg(colors.overlay)
            .border_1()
            .border_color(colors.border)
            .shadow(theme.elevation(Elevation::Floating))
            .text_size(theme.text_size(TextSize::Xs))
            .whitespace_nowrap()
            .child(div().text_color(colors.fg_muted).child(self.title))
            .children(self.rows.into_iter().map(|(color, name, value)| {
                div()
                    .flex()
                    .items_center()
                    .gap_1p5()
                    .children(
                        color.map(|color| div().size(theme.status_dot()).rounded_full().bg(color)),
                    )
                    .child(div().flex_1().text_color(colors.fg_muted).child(name))
                    .child(tabular(div()).pl_3().text_color(colors.fg).child(value))
            }))
    }
}
