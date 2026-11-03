use gpui::{
    AnyElement, Context, FontWeight, InteractiveElement, IntoElement, MouseButton, ParentElement,
    SharedString, StatefulInteractiveElement, Styled, Window, div, prelude::*,
};

use super::{Block, BlockEditor, BlockKind, media, table, view::Look};
use crate::{
    buttons::CopyButton,
    forms::Checkbox,
    primitives::{Disclosure, Icon, IconName},
    theme::IconSize,
    typography::Latex,
};

impl BlockEditor {
    /// What a block shows, by kind.
    pub(crate) fn content(
        &self,
        block: &Block,
        number: usize,
        look: &Look,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let colors = look.colors.clone();
        let (key, id) = (block.key, Self::id(cx));
        let marker = |text: SharedString| {
            div()
                .flex_none()
                .min_w_5()
                .text_color(colors.fg_muted)
                .child(text)
        };
        let row = |marker: AnyElement, field: AnyElement| {
            div()
                .flex()
                .gap_1p5()
                .child(marker)
                .child(div().flex_1().min_w_0().child(field))
        };
        match &block.kind {
            BlockKind::Paragraph => self.field_view(block, 0, window, cx),
            BlockKind::Heading(level) => div()
                .pt_2()
                .text_size(look.headings[(*level as usize).clamp(1, 3) - 1])
                .font_weight(FontWeight::SEMIBOLD)
                .child(self.field_view(block, 0, window, cx))
                .into_any_element(),
            BlockKind::Bullet => row(
                marker("•".into()).into_any_element(),
                self.field_view(block, 0, window, cx),
            )
            .into_any_element(),
            BlockKind::Numbered => row(
                marker(format!("{number}.").into()).into_any_element(),
                self.field_view(block, 0, window, cx),
            )
            .into_any_element(),
            BlockKind::Todo(done) => {
                let (entity, done) = (cx.entity(), *done);
                let check = Checkbox::new((id.clone(), format!("todo-{key}")), done).on_change(
                    move |on, _, cx| {
                        entity.update(cx, |editor, cx| {
                            if editor
                                .kind_of(key)
                                .is_some_and(|kind| matches!(kind, BlockKind::Todo(_)))
                            {
                                editor.set_kind(key, BlockKind::Todo(on), cx)
                            }
                        })
                    },
                );
                row(
                    check.into_any_element(),
                    self.field_view(block, 0, window, cx),
                )
                .when(done, |row| row.text_color(colors.fg_subtle))
                .into_any_element()
            }
            BlockKind::Quote => div()
                .pl_3()
                .border_l_2()
                .border_color(colors.border_strong)
                .text_color(colors.fg_muted)
                .child(self.field_view(block, 0, window, cx))
                .into_any_element(),
            BlockKind::Callout => div()
                .flex()
                .gap_2()
                .p_3()
                .rounded(look.radius)
                .bg(colors.sunken)
                .child(
                    Icon::new(IconName::Lightbulb)
                        .size(IconSize::Sm)
                        .color(colors.warning),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .child(self.field_view(block, 0, window, cx)),
                )
                .into_any_element(),
            BlockKind::Toggle(open) => {
                let (entity, open) = (cx.entity(), *open);
                let chevron = div()
                    .id((id.clone(), format!("toggle-{key}")))
                    .flex_none()
                    .cursor_pointer()
                    .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
                    .on_click(move |_, _, cx| {
                        entity.update(cx, |editor, cx| {
                            if editor.kind_of(key) == Some(&BlockKind::Toggle(open)) {
                                editor.set_kind(key, BlockKind::Toggle(!open), cx)
                            }
                        })
                    })
                    .child(Disclosure::new(
                        (id.clone(), format!("disclosure-{key}")),
                        open,
                    ));
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(row(
                        chevron.into_any_element(),
                        self.field_view(block, 0, window, cx),
                    ))
                    .when(open, |toggle| {
                        toggle.child(div().pl_6().child(self.field_view(block, 1, window, cx)))
                    })
                    .into_any_element()
            }
            BlockKind::Code => {
                let text = block.fields[0].read(cx).text().to_string();
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .p_3()
                    .rounded(look.radius)
                    .bg(colors.sunken)
                    .font_family(look.mono.clone())
                    .text_size(look.small)
                    .child(
                        div()
                            .flex()
                            .justify_between()
                            .items_center()
                            .text_size(look.tiny)
                            .text_color(colors.fg_subtle)
                            .child("Code")
                            .child(CopyButton::new((id.clone(), format!("copy-{key}")), text)),
                    )
                    .child(self.field_view(block, 0, window, cx))
                    .into_any_element()
            }
            BlockKind::Math => {
                let tex = block.fields[0].read(cx).text().to_string();
                let shown = match Latex::parse((id.clone(), format!("math-{key}")), &tex) {
                    _ if tex.trim().is_empty() => div().into_any_element(),
                    Ok(math) => div()
                        .flex()
                        .justify_center()
                        .py_2()
                        .child(math)
                        .into_any_element(),
                    Err(error) => div()
                        .text_color(colors.danger)
                        .child(error.to_string())
                        .into_any_element(),
                };
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .p_3()
                    .rounded(look.radius)
                    .border_1()
                    .border_color(colors.border)
                    .child(
                        div()
                            .font_family(look.mono.clone())
                            .text_size(look.small)
                            .text_color(colors.fg_muted)
                            .child(self.field_view(block, 0, window, cx)),
                    )
                    .child(shown)
                    .into_any_element()
            }
            BlockKind::Diagram => media::diagram(self, block, look, window, cx),
            BlockKind::Divider => div()
                .py_2()
                .child(div().h_0().border_t_1().border_color(colors.border))
                .into_any_element(),
            BlockKind::Table { .. } => table::table(self, block, look, window, cx),
            BlockKind::Image(_) | BlockKind::Video(_) | BlockKind::Embed(_) => {
                media::media(self, block, look, window, cx)
            }
            BlockKind::Columns(_) => div()
                .flex()
                .gap_4()
                .children((0..block.fields.len()).map(|ix| {
                    div()
                        .flex_1()
                        .min_w_0()
                        .when(ix > 0, |column| {
                            column.pl_4().border_l_1().border_color(colors.border)
                        })
                        .child(self.field_view(block, ix, window, cx))
                }))
                .into_any_element(),
            BlockKind::Synced(name) => div()
                .flex()
                .flex_col()
                .gap_1()
                .px_3()
                .py_2()
                .rounded(look.radius)
                .border_1()
                .border_color(colors.border)
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .text_size(look.tiny)
                        .text_color(colors.fg_subtle)
                        .child(
                            Icon::new(IconName::RefreshCw)
                                .size(IconSize::Xs)
                                .color(colors.fg_subtle),
                        )
                        .child(format!("Synced · {name}")),
                )
                .child(self.field_view(block, 0, window, cx))
                .into_any_element(),
        }
    }
}
