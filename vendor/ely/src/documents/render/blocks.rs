use gpui::{
    AnyElement, App, FontWeight, IntoElement, MouseButton, ParentElement, Styled, StyledText, div,
    prelude::*,
};

use super::{Draw, point::point_lines};
use crate::{
    documents::tree::{AlertKind, DocumentHeading, Node},
    primitives::{FocusRing as _, Icon, IconName},
    theme::{ActiveTheme, IconSize, Radius, TextSize},
};

impl Draw<'_> {
    /// A list; bullets turn with depth, as browsers draw them.
    pub(super) fn list(
        &self,
        start: &Option<u64>,
        items: &[(Option<bool>, Vec<Node>)],
        cx: &App,
    ) -> AnyElement {
        let colors = cx.theme().colors.clone();
        let depth = self.lists.get();
        self.lists.set(depth + 1);
        let bullet = match depth {
            0 => "•",
            1 => "◦",
            _ => "▪",
        };
        let list = div()
            .flex()
            .flex_col()
            .gap_1()
            .children(items.iter().enumerate().map(|(ix, (task, body))| {
                let marker: AnyElement = match (task, start) {
                    (Some(done), _) => Icon::new(if *done {
                        IconName::SquareCheck
                    } else {
                        IconName::Square
                    })
                    .size(IconSize::Sm)
                    .color(if *done {
                        colors.focus
                    } else {
                        colors.fg_subtle
                    })
                    .into_any_element(),
                    (None, Some(first)) => div()
                        .text_color(colors.fg_muted)
                        .child(format!("{}.", first + ix as u64))
                        .into_any_element(),
                    (None, None) => div()
                        .debug_selector(move || format!("markdown-bullet-{depth}"))
                        .text_color(colors.fg_muted)
                        .child(bullet)
                        .into_any_element(),
                };
                div()
                    .flex()
                    .gap_2()
                    .child(
                        div()
                            .flex_none()
                            .min_w_5()
                            .flex()
                            .justify_end()
                            .child(marker),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .when(*task == Some(true), |body| {
                                body.text_color(colors.fg_subtle)
                            })
                            .children(self.nodes(body, cx)),
                    )
            }))
            .into_any_element();
        self.lists.set(depth);
        list
    }

    /// A GitHub alert: a colored rule, icon, title and body.
    pub(super) fn alert(&self, kind: AlertKind, body: &[Node], cx: &App) -> AnyElement {
        let colors = &cx.theme().colors;
        let (tone, icon) = match kind {
            AlertKind::Note => (colors.info, IconName::Info),
            AlertKind::Tip => (colors.success, IconName::Lightbulb),
            // The palette's violet, read at 4.5:1.
            AlertKind::Important => (colors.syntax.keyword, IconName::MessageSquareWarning),
            AlertKind::Warning => (colors.warning, IconName::TriangleAlert),
            AlertKind::Caution => (colors.danger, IconName::OctagonAlert),
        };
        div()
            .flex()
            .flex_col()
            .gap_1()
            .py_1()
            .pl_4()
            .border_l_2()
            .border_color(tone)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(tone)
                    .child(Icon::new(icon).size(IconSize::Sm).color(tone))
                    .child(kind.title()),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .text_color(colors.fg)
                    .children(self.nodes(body, cx)),
            )
            .into_any_element()
    }

    /// `[toc]`: each heading by depth; a press reveals it.
    pub(super) fn toc(&self, headings: &[DocumentHeading], cx: &App) -> AnyElement {
        let colors = cx.theme().colors.clone();
        let top = headings.iter().map(|heading| heading.depth).min();
        div()
            .flex()
            .flex_col()
            .gap_1()
            .children(headings.iter().enumerate().map(|(ix, heading)| {
                let entry = div()
                    .id(self.key("toc"))
                    .debug_selector(move || format!("markdown-toc-{ix}"))
                    .child(heading.title.clone());
                let entry = match heading.depth - top.expect("a heading is listed") {
                    0 => entry,
                    1 => entry.pl_4(),
                    2 => entry.pl_8(),
                    3 => entry.pl_12(),
                    4 => entry.pl_16(),
                    _ => entry.pl_20(),
                };
                let Some(reveal) = self.hooks.on_heading.clone() else {
                    return entry.text_color(colors.fg_muted).into_any_element();
                };
                let (at, pressed) = (heading.at, reveal.clone());
                entry
                    .text_color(colors.link)
                    .tab_index(0)
                    .focus_ring(cx)
                    .cursor_pointer()
                    // On press: the layout may move before release.
                    .on_mouse_down(MouseButton::Left, move |event, window, cx| {
                        if event.modifiers.secondary() {
                            window.prevent_default();
                            cx.stop_propagation();
                            pressed(at, window, cx);
                        }
                    })
                    .on_click(move |event, window, cx| {
                        if !event.modifiers().secondary() {
                            reveal(at, window, cx);
                        }
                    })
                    .into_any_element()
            }))
            .into_any_element()
    }

    /// Front matter as written, inside its box.
    pub(super) fn front_matter(&self, text: &str, from: usize, cx: &App) -> AnyElement {
        let theme = cx.theme();
        let drawn = StyledText::new(text.to_string());
        let layout = drawn.layout().clone();
        let block = div()
            .p_3()
            .rounded(theme.radius(Radius::Md))
            .bg(theme.colors.sunken)
            .font_family(theme.mono_family.clone())
            .text_size(theme.text_size(TextSize::Sm) * self.scale)
            .text_color(theme.colors.fg_muted)
            .child(drawn)
            .into_any_element();
        point_lines(
            block,
            layout,
            text,
            from,
            self.hooks.on_point,
            (false, false),
        )
    }
}
