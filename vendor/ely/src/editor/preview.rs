use std::rc::Rc;

use gpui::{
    App, ElementId, FontWeight, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    SharedString, StatefulInteractiveElement, Styled, Window, div, prelude::*,
};

use crate::{
    primitives::{Icon, IconName},
    theme::{ActiveTheme, IconSize, Palette, Radius, TextSize},
};

type OnPick = Rc<dyn Fn(&mut Window, &mut App)>;

/// A theme at a glance: a small editor painted in its own palette, whatever the app's theme; a press picks it.
#[derive(IntoElement)]
pub struct ThemePreview {
    id: ElementId,
    name: SharedString,
    palette: Palette,
    selected: bool,
    on_pick: Option<OnPick>,
}

impl ThemePreview {
    pub fn new(id: impl Into<ElementId>, name: impl Into<SharedString>, palette: Palette) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            palette,
            selected: false,
            on_pick: None,
        }
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn on_pick(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_pick = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for ThemePreview {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let ring = theme.colors.focus;
        let shown = self.palette;
        let syntax = shown.syntax;
        let line = |parts: Vec<(&'static str, gpui::Hsla)>, indent: usize| {
            div()
                .flex()
                .pl(theme.text_size(TextSize::Xs) * indent as f32)
                .children(
                    parts
                        .into_iter()
                        .map(|(words, color)| div().text_color(color).child(words)),
                )
        };
        let code = div()
            .flex_1()
            .p_2()
            .flex()
            .flex_col()
            .gap_0p5()
            .font_family(theme.mono_family.clone())
            .text_size(theme.text_size(TextSize::Xs))
            .child(line(
                vec![
                    ("fn ", syntax.keyword),
                    ("paint", syntax.function),
                    ("(", syntax.punctuation),
                    ("mode", syntax.variable),
                    (": ", syntax.punctuation),
                    ("Mode", syntax.type_name),
                    (") {", syntax.punctuation),
                ],
                0,
            ))
            .child(line(vec![("// quiet by default", syntax.comment)], 2))
            .child(line(
                vec![
                    ("let ", syntax.keyword),
                    ("gap", syntax.variable),
                    (" = ", syntax.operator),
                    ("12", syntax.number),
                    (";", syntax.punctuation),
                ],
                2,
            ))
            .child(line(
                vec![
                    ("draw", syntax.function),
                    ("(", syntax.punctuation),
                    ("\"surface\"", syntax.string),
                    (", ", syntax.punctuation),
                    ("true", syntax.constant),
                    (");", syntax.punctuation),
                ],
                2,
            ))
            .child(line(vec![("}", syntax.punctuation)], 0));
        let pick = self.on_pick;
        div()
            .id(self.id)
            .flex()
            .flex_col()
            .gap_2()
            .cursor_pointer()
            .when_some(pick, |card, pick| {
                card.on_click(move |_, window, cx| pick(window, cx))
            })
            .child(
                div()
                    .h(theme.label_width() * 1.1)
                    .flex()
                    .flex_col()
                    .rounded(theme.radius(Radius::Md))
                    .border_2()
                    .border_color(if self.selected { ring } else { shown.border })
                    .bg(shown.bg)
                    .overflow_hidden()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .px_2()
                            .py_1()
                            .bg(shown.surface)
                            .border_b_1()
                            .border_color(shown.border)
                            .children(
                                [shown.danger, shown.warning, shown.success]
                                    .map(|dot| div().size_1p5().rounded_full().bg(dot)),
                            ),
                    )
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .child(
                                div()
                                    .w_6()
                                    .bg(shown.sunken)
                                    .border_r_1()
                                    .border_color(shown.border),
                            )
                            .child(code),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .px_2()
                            .h_3()
                            .bg(shown.surface)
                            .border_t_1()
                            .border_color(shown.border)
                            .child(div().w_4().h_1().rounded_full().bg(shown.accent)),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1p5()
                    .text_size(theme.text_size(TextSize::Sm))
                    .font_weight(if self.selected {
                        FontWeight::SEMIBOLD
                    } else {
                        FontWeight::NORMAL
                    })
                    .text_color(theme.colors.fg)
                    .when(self.selected, |label| {
                        label.child(Icon::new(IconName::Check).size(IconSize::Sm).color(ring))
                    })
                    .child(self.name),
            )
    }
}
