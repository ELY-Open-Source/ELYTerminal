use std::rc::Rc;

use gpui::{
    App, ElementId, InteractiveElement, IntoElement, MouseButton, ParentElement, RenderOnce,
    SharedString, Styled, Window, div,
};

use super::ListItem;
use crate::{
    motion::Reorder,
    primitives::{Icon, IconName, tab_stop},
    theme::{ActiveTheme, IconSize},
};

type OnReorder = Rc<dyn Fn(usize, usize, &mut Window, &mut App)>;

/// Where the keyboard moves row `at` of `count`, or None past either end.
fn nudged(at: usize, count: usize, up: bool) -> Option<usize> {
    if up {
        at.checked_sub(1)
    } else {
        (at + 1 < count).then_some(at + 1)
    }
}

/// Rows you reorder: drag one, or with the list focused press Alt with Up or Down to move the current row. `on_reorder(from, to)` asks the owner to move row `from` to place `to`.
#[derive(IntoElement)]
pub struct SortableList {
    id: ElementId,
    rows: Vec<(SharedString, ListItem)>,
    on_reorder: Option<OnReorder>,
}

impl SortableList {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            rows: Vec::new(),
            on_reorder: None,
        }
    }

    /// A row under `key`, which follows it as it moves.
    pub fn row(mut self, key: impl Into<SharedString>, item: ListItem) -> Self {
        self.rows.push((key.into(), item));
        self
    }

    pub fn on_reorder(
        mut self,
        handler: impl Fn(usize, usize, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_reorder = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for SortableList {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let count = self.rows.len();
        let focus = tab_stop((self.id.clone(), "focus").into(), true, window, cx);
        let focused = focus.is_focused(window);
        let cursor = window.use_keyed_state((self.id.clone(), "cursor"), cx, |_, _| 0usize);
        let at = (*cursor.read(cx)).min(count.saturating_sub(1));
        let reorder: OnReorder = {
            let (id, cursor, on_reorder) = (self.id.clone(), cursor.clone(), self.on_reorder);
            Rc::new(move |from, to, window, cx| {
                log::info!("sortable list {id:?}: row {from} to {to}");
                cursor.update(cx, |cursor, cx| {
                    *cursor = to;
                    cx.notify();
                });
                if let Some(on_reorder) = &on_reorder {
                    on_reorder(from, to, window, cx);
                }
            })
        };
        let theme = cx.theme();
        let grip = theme.colors.fg_subtle;
        let dragged = reorder.clone();
        let list = self.rows.into_iter().enumerate().fold(
            Reorder::new((self.id.clone(), "rows"))
                .on_reorder(move |from, to, window, cx| dragged(from, to, window, cx)),
            |list, (ix, (key, item))| {
                let pointed = cursor.clone();
                list.row(
                    key,
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                            pointed.update(cx, |cursor, cx| {
                                *cursor = ix;
                                cx.notify();
                            })
                        })
                        .child(
                            Icon::new(IconName::GripVertical)
                                .size(IconSize::Sm)
                                .color(grip),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .child(item.current(focused && ix == at)),
                        ),
                )
            },
        );
        div()
            .id(self.id)
            .track_focus(&focus)
            .on_key_down(move |event, window, cx| {
                let up = match event.keystroke.key.as_str() {
                    "up" => true,
                    "down" => false,
                    _ => return,
                };
                cx.stop_propagation();
                let at = (*cursor.read(cx)).min(count.saturating_sub(1));
                let Some(to) = nudged(at, count, up) else {
                    return;
                };
                if event.keystroke.modifiers.alt {
                    reorder(at, to, window, cx);
                } else {
                    cursor.update(cx, |cursor, cx| {
                        *cursor = to;
                        cx.notify();
                    });
                }
            })
            .child(list)
    }
}

#[cfg(test)]
mod tests {
    use super::nudged;

    #[test]
    fn a_nudge_stops_at_either_end() {
        assert_eq!(nudged(0, 3, true), None);
        assert_eq!(nudged(1, 3, true), Some(0));
        assert_eq!(nudged(1, 3, false), Some(2));
        assert_eq!(nudged(2, 3, false), None);
    }
}
