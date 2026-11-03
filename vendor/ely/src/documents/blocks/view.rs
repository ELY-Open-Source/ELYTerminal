use std::rc::Rc;

use gpui::{
    AnyElement, App, Axis, Context, ElementId, InteractiveElement, IntoElement, MouseButton,
    MouseDownEvent, ParentElement, Rems, Render, SharedString, Styled, Window, canvas, div,
    prelude::*,
};

use super::{Block, BlockData, BlockEditor, BlockKind, matching, span::Put};
use crate::{
    buttons::{ButtonVariant, IconButton},
    documents::suggest::{Offer, offers},
    forms::replace_trigger,
    forms::{
        Backspace, Choice, Copy, Cut, Delete, DeleteWordLeft, Down, Enter, Pick, Redo, Submit,
        Undo, Up,
    },
    layout::{ScrollArea, bring_into_view},
    menus::{Menu, MenuItem, OverflowMenu},
    motion::Reorder,
    primitives::IconName,
    theme::{ActiveTheme, ControlSize, Palette, Radius, TextSize},
};

/// Theme values a block reads, taken before its fields borrow the context.
pub(crate) struct Look {
    pub colors: Palette,
    pub mono: SharedString,
    pub radius: Rems,
    pub small: Rems,
    pub tiny: Rems,
    pub headings: [Rems; 3],
}

impl Look {
    fn new(cx: &App) -> Self {
        let theme = cx.theme();
        Self {
            colors: theme.colors.clone(),
            mono: theme.mono_family.clone(),
            radius: theme.radius(Radius::Md),
            small: theme.text_size(TextSize::Sm),
            tiny: theme.text_size(TextSize::Xs),
            headings: [TextSize::Xxl, TextSize::Xl, TextSize::Lg].map(|size| theme.text_size(size)),
        }
    }
}

impl BlockEditor {
    pub(crate) fn id(cx: &Context<Self>) -> ElementId {
        ("ely-blocks", cx.entity_id()).into()
    }

    /// A slash pick: an emptied block becomes the kind; one with text gets the kind below it.
    fn pick_kind(
        &mut self,
        key: u64,
        kind: BlockKind,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(ix) = self.index(key) else {
            return;
        };
        let empty = self.blocks[ix]
            .fields
            .first()
            .is_some_and(|field| field.read(cx).is_empty());
        let data = BlockData {
            texts: vec![String::new(); kind.fields()],
            kind: kind.clone(),
        };
        let placed = if empty {
            self.turn_into(key, kind.clone(), window, cx);
            key
        } else {
            self.insert(Some(key), data, window, cx)
        };
        if kind == BlockKind::Divider {
            self.insert(
                Some(placed),
                BlockData::new(BlockKind::Paragraph, [""]),
                window,
                cx,
            );
        }
    }

    /// A text field with its keys and suggests: `/` in prose opens the kinds.
    pub(crate) fn field_view(
        &self,
        block: &Block,
        ix: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let (key, field) = (block.key, block.fields[ix].clone());
        let entity = cx.entity();
        let keys = div()
            .w_full()
            .capture_action(cx.listener(move |editor, _: &Enter, window, cx| {
                if editor.collapse_span(Put::Break, window, cx).is_some() {
                    return cx.stop_propagation();
                }
                if editor.on_enter(key, ix, window, cx) {
                    cx.stop_propagation();
                }
            }))
            .capture_action(cx.listener(move |editor, _: &Backspace, window, cx| {
                if editor.collapse_span(Put::Nothing, window, cx).is_some()
                    || editor.on_backspace(key, ix, window, cx)
                {
                    cx.stop_propagation();
                }
            }))
            .capture_action(cx.listener(move |editor, _: &Up, window, cx| {
                if editor.on_vertical(key, ix, false, window, cx) {
                    cx.stop_propagation();
                }
            }))
            .capture_action(cx.listener(move |editor, _: &Down, window, cx| {
                if editor.on_vertical(key, ix, true, window, cx) {
                    cx.stop_propagation();
                }
            }))
            .capture_action(cx.listener(move |editor, _: &Submit, window, cx| {
                cx.stop_propagation();
                editor.on_submit(key, window, cx);
            }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |editor, event: &MouseDownEvent, window, cx| {
                    editor.pressed(key, ix, event, window, cx);
                    cx.stop_propagation();
                }),
            )
            .child(field.clone());
        let slash = move |query: &str, at: usize, caret: usize| -> (Vec<Choice>, Pick) {
            let kinds = matching(query);
            let rows = kinds
                .iter()
                .map(|kind| Choice::new(kind.label(), kind.label()).icon(kind.icon()))
                .collect();
            let (entity, field) = (entity.clone(), field.clone());
            let pick: Pick = Rc::new(move |pick: usize, window: &mut Window, cx: &mut App| {
                replace_trigger(&field, at, caret, "", cx);
                let kind = kinds[pick].clone();
                entity.update(cx, |editor, cx| editor.pick_kind(key, kind, window, cx));
            });
            (rows, pick)
        };
        let extra: Option<(char, Offer)> =
            (block.kind.is_prose() && ix == 0).then_some(('/', &slash));
        let id = (Self::id(cx), format!("offers-{key}-{ix}")).into();
        offers(id, &block.fields[ix], &self.people, extra, cx)
            .wrap(keys, window, cx)
            .into_any_element()
    }

    /// Scrolls the focused caret into view once each time it moves; a wheel moves freely past a still one.
    fn reveal_caret(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some((key, ix)) = self.focused(window, cx) else {
            self.revealed = None;
            return;
        };
        let Some(block) = self.index(key).map(|at| &self.blocks[at]) else {
            return;
        };
        let field = block.fields[ix].read(cx);
        let at = (
            block.fields[ix].entity_id(),
            field.selection(),
            field.text().len(),
        );
        if self.revealed.as_ref() == Some(&at) {
            return;
        }
        let Some(caret) = field.caret_bounds() else {
            return;
        };
        self.revealed = Some(at);
        if bring_into_view(&self.scroll, caret, Axis::Vertical) {
            log::debug!("block editor: the caret came into view");
            window.request_animation_frame();
        }
    }

    /// The handle's menu: turn into another kind, duplicate or delete.
    fn menu(&self, key: u64, cx: &mut Context<Self>) -> Menu {
        let entity = cx.entity();
        let turn = super::kind::offered()
            .into_iter()
            .fold(Menu::new(), |menu, kind| {
                let entity = entity.clone();
                menu.item(MenuItem::new(kind.label()).icon(kind.icon()).on_click(
                    move |window, cx| {
                        let kind = kind.clone();
                        entity.update(cx, |editor, cx| editor.turn_into(key, kind, window, cx));
                    },
                ))
            });
        let (copy, gone) = (entity.clone(), entity);
        Menu::new()
            .item(MenuItem::submenu("Turn into", turn).icon(IconName::RefreshCw))
            .item(
                MenuItem::new("Duplicate")
                    .icon(IconName::Copy)
                    .on_click(move |window, cx| {
                        copy.update(cx, |editor, cx| editor.duplicate(key, window, cx));
                    }),
            )
            .separator()
            .item(
                MenuItem::new("Delete")
                    .icon(IconName::Trash2)
                    .on_click(move |window, cx| {
                        gone.update(cx, |editor, cx| editor.remove(key, window, cx));
                    }),
            )
    }
}

impl Render for BlockEditor {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let id = Self::id(cx);
        let look = Look::new(cx);
        let colors = look.colors.clone();
        let focused = self.focused(window, cx).map(|(key, _)| key);
        let washed = self.washed().unwrap_or_default();
        let mut number = 0;
        let mut rows: Vec<(SharedString, AnyElement)> = Vec::new();
        for ix in 0..self.blocks.len() {
            let block = &self.blocks[ix];
            number = if block.kind == BlockKind::Numbered {
                number + 1
            } else {
                0
            };
            let key = block.key;
            let group = SharedString::from(format!("block-{key}"));
            let entity = cx.entity();
            let plus = IconButton::new((id.clone(), format!("plus-{key}")), IconName::Plus)
                .variant(ButtonVariant::Ghost)
                .size(ControlSize::Sm)
                .tooltip("Add a block below")
                .on_click(move |_, window, cx| {
                    entity.update(cx, |editor, cx| {
                        editor.insert(
                            Some(key),
                            BlockData::new(BlockKind::Paragraph, ["/"]),
                            window,
                            cx,
                        );
                    })
                });
            let handle =
                OverflowMenu::new((id.clone(), format!("handle-{key}")), self.menu(key, cx))
                    .icon(IconName::GripVertical)
                    .tooltip("Drag to move, press for more");
            let content = self.content(&self.blocks[ix], number, &look, window, cx);
            let row = div()
                .id((id.clone(), format!("row-{key}")))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |editor, _, window, cx| {
                        let Some(ix) = editor.index(key) else {
                            return;
                        };
                        let end = editor.blocks[ix]
                            .fields
                            .first()
                            .map(|field| field.read(cx).text().len());
                        if end.is_some() {
                            editor.focus_field(key, 0, end, window, cx);
                        }
                    }),
                )
                .group(group.clone())
                .relative()
                .pl_16()
                .when(washed.contains(&ix), |row| {
                    row.bg(colors.selection).rounded(look.radius)
                })
                .py_0p5()
                .child(content)
                .child(
                    div()
                        .absolute()
                        .top_0()
                        .left_0()
                        .flex()
                        .opacity(if focused == Some(key) { 1.0 } else { 0.0 })
                        .group_hover(group, |gutter| gutter.opacity(1.0))
                        .child(plus)
                        .child(handle),
                );
            rows.push((format!("{key}").into(), row.into_any_element()));
        }
        let entity = cx.entity();
        let list = rows.into_iter().fold(
            Reorder::new((id.clone(), "blocks")).on_reorder(move |from, to, _, cx| {
                entity.update(cx, |editor, cx| editor.move_block(from, to, cx))
            }),
            |list, (key, row)| list.row(key, row),
        );
        let (start, editor) = (cx.entity(), cx.entity());
        let caret = canvas(
            move |_, window, cx| editor.update(cx, |editor, cx| editor.reveal_caret(window, cx)),
            |_, _, _, _| {},
        )
        .absolute();
        let column = div()
            .flex()
            .flex_col()
            .capture_action(cx.listener(|editor, _: &Copy, _, cx| {
                if editor.copy_span(cx) {
                    cx.stop_propagation();
                }
            }))
            .capture_action(cx.listener(|editor, _: &Cut, window, cx| {
                if editor.copy_span(cx) {
                    cx.stop_propagation();
                    editor.collapse_span(Put::Nothing, window, cx);
                }
            }))
            .capture_action(cx.listener(|editor, _: &Delete, window, cx| {
                if editor.collapse_span(Put::Nothing, window, cx).is_some() {
                    cx.stop_propagation();
                }
            }))
            .capture_action(cx.listener(|editor, _: &DeleteWordLeft, window, cx| {
                if editor.collapse_span(Put::Nothing, window, cx).is_some() {
                    cx.stop_propagation();
                }
            }))
            .capture_action(cx.listener(|editor, _: &Undo, window, cx| {
                cx.stop_propagation();
                editor.undo(window, cx);
            }))
            .capture_action(cx.listener(|editor, _: &Redo, window, cx| {
                cx.stop_propagation();
                editor.redo(window, cx);
            }))
            .child(list)
            .child(
                div()
                    .id((id.clone(), "end"))
                    .min_h_8()
                    .cursor_text()
                    .text_color(colors.fg_subtle)
                    .when(self.blocks.is_empty(), |end| end.child("Press to write"))
                    .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                        window.prevent_default();
                        start.update(cx, |editor, cx| {
                            let last = editor.blocks.last().map(|block| block.key);
                            editor.insert(
                                last,
                                BlockData::new(BlockKind::Paragraph, [""]),
                                window,
                                cx,
                            );
                        });
                    }),
            )
            .child(caret)
            .when(self.pressing(), |column| {
                column.child(self.drag_listener(cx))
            });
        ScrollArea::new(id)
            .size_full()
            .track_scroll(&self.scroll)
            .child(column)
    }
}
