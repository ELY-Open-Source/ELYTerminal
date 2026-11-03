use std::{collections::HashSet, rc::Rc};

use gpui::{
    AnyElement, App, Div, ElementId, Entity, InteractiveElement, IntoElement, MouseButton,
    ParentElement, Pixels, Rems, ScrollHandle, SharedString, Stateful, StatefulInteractiveElement,
    Styled, Window, div, prelude::*, transparent_black,
};

use super::{Align, Column, Row, cell::draw, model::tint, table::View};
use crate::{
    forms::{CheckState, Editing, Input, check_mark},
    layout::on_axis,
    lists::Pick,
    primitives::Disclosure,
    theme::{ActiveTheme, ControlSize, TextSize},
};

pub(super) type Select = Rc<dyn Fn(usize, Pick, &mut Window, &mut App)>;
pub(super) type Detail = Rc<dyn Fn(&SharedString, &mut Window, &mut App) -> AnyElement>;
pub(super) type OnEdit =
    Rc<dyn Fn(&SharedString, &SharedString, &SharedString, &mut Window, &mut App)>;

/// Gives a cell its column's width: fixed, or a share of what is left.
pub(super) fn sized<E: Styled>(cell: E, column: &Column, narrowest: Rems) -> E {
    let cell = match column.width {
        Some(width) => cell.w(width).flex_none(),
        None => cell.flex_1().min_w(narrowest),
    };
    if column.align == Align::End {
        cell.justify_end()
    } else {
        cell
    }
}

/// A box a table's columns scroll sideways in, tracked by `handle`; a plain wheel passes to the page.
pub(super) fn sideways(id: &ElementId, handle: &ScrollHandle) -> Stateful<Div> {
    on_axis(
        div()
            .id((id.clone(), "sideways"))
            .overflow_x_scroll()
            .track_scroll(handle),
    )
}

/// What the body draws, one after another.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Line {
    Group {
        name: SharedString,
        count: usize,
        folded: bool,
    },
    Row {
        ix: usize,
        position: usize,
    },
    Detail {
        ix: usize,
    },
}

/// What drawing lines needs, shared by pages and by a long table's list.
pub(super) struct Body {
    pub id: ElementId,
    pub view: Entity<View>,
    pub columns: Rc<Vec<Column>>,
    pub widths: Rc<Vec<Option<Pixels>>>,
    pub rows: Rc<Vec<Row>>,
    pub lines: Rc<Vec<Line>>,
    pub selected: Option<Rc<Vec<SharedString>>>,
    pub select: Option<Select>,
    pub detail: Option<Detail>,
    pub open: Rc<HashSet<SharedString>>,
    pub editing: Option<(SharedString, SharedString)>,
    pub editor: Entity<Editing>,
    pub on_edit: Option<OnEdit>,
    pub ranges: Rc<Vec<Option<(f64, f64)>>>,
    pub height: Rems,
    pub lead: Rems,
    pub narrowest: Rems,
}

impl Body {
    /// How wide the leading cells stand: a box to select, a disclosure to expand.
    pub(super) fn lead_width(&self) -> Rems {
        self.lead * (usize::from(self.select.is_some()) + usize::from(self.detail.is_some())) as f32
    }

    /// The least width columns `cols` draw in: fixed widths, the narrowest share for the rest, and the leading cells.
    pub(super) fn least(&self, cols: &[usize], lead: bool, rem: Pixels) -> Pixels {
        let start = match lead {
            true => self.lead_width().to_pixels(rem),
            false => Pixels::ZERO,
        };
        cols.iter().fold(start, |sum, col| {
            sum + match (self.widths[*col], self.columns[*col].width) {
                (Some(width), _) => width,
                (None, Some(width)) => width.to_pixels(rem),
                (None, None) => self.narrowest.to_pixels(rem),
            }
        })
    }

    /// Line `at` over columns `cols`; `lead` draws the leading cells.
    pub(super) fn line(
        &self,
        at: usize,
        cols: &[usize],
        lead: bool,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        match self.lines[at].clone() {
            Line::Group {
                name,
                count,
                folded,
            } => self.group(name, count, folded, lead, cx),
            Line::Row { ix, position } => self.row(ix, position, cols, lead, cx),
            Line::Detail { ix } => {
                let key = self.rows[ix].key.clone();
                let detail = self
                    .detail
                    .clone()
                    .expect("a detail line comes from a detail");
                let content = detail(&key, window, cx);
                let colors = &cx.theme().colors;
                div()
                    .min_w_full()
                    .pl(self.lead_width())
                    .pr_3()
                    .py_3()
                    .bg(colors.sunken)
                    .border_b_1()
                    .border_color(colors.border)
                    .child(content)
                    .into_any_element()
            }
        }
    }

    fn group(
        &self,
        name: SharedString,
        count: usize,
        folded: bool,
        lead: bool,
        cx: &App,
    ) -> AnyElement {
        let theme = cx.theme();
        let colors = &theme.colors;
        let row = div()
            .id((self.id.clone(), format!("group-{name}-{lead}")))
            .min_w_full()
            .h(self.height)
            .flex()
            .items_center()
            .gap_2()
            .px_3()
            .bg(colors.sunken)
            .border_b_1()
            .border_color(colors.border)
            .text_size(theme.text_size(TextSize::Sm));
        if !lead {
            return row.into_any_element();
        }
        let view = self.view.clone();
        let title = if name.is_empty() {
            SharedString::from("—")
        } else {
            name.clone()
        };
        row.cursor_pointer()
            .on_click(move |_, _, cx| {
                view.update(cx, |view, cx| {
                    if !view.folded.remove(&name) {
                        view.folded.insert(name.clone());
                    }
                    cx.notify();
                })
            })
            .child(Disclosure::new(
                (self.id.clone(), format!("fold-{title}")),
                !folded,
            ))
            .child(
                div()
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .text_color(colors.fg)
                    .child(title),
            )
            .child(div().text_color(colors.fg_subtle).child(count.to_string()))
            .into_any_element()
    }

    fn row(&self, ix: usize, position: usize, cols: &[usize], lead: bool, cx: &App) -> AnyElement {
        let key = self.rows[ix].key.clone();
        let on = self
            .selected
            .as_ref()
            .is_some_and(|selected| selected.contains(&key));
        let theme = cx.theme();
        let colors = &theme.colors;
        let press = self.select.clone();
        let open = self.open.contains(&key);
        let named = key.clone();
        div()
            .id((self.id.clone(), format!("row-{key}-{lead}")))
            .debug_selector(move || format!("table-row {named}"))
            .flex()
            .items_center()
            .min_w_full()
            .h(self.height)
            .border_b_1()
            .border_color(colors.border)
            .text_size(theme.text_size(TextSize::Sm))
            .text_color(colors.fg)
            .when(on, |row| row.bg(colors.active))
            .when(!on, |row| row.hover(|style| style.bg(colors.hover)))
            .when_some(self.select.clone().filter(|_| lead), |row, select| {
                row.child(
                    div()
                        .flex_none()
                        .w(self.lead)
                        .flex()
                        .justify_center()
                        .cursor_pointer()
                        .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                            cx.stop_propagation();
                            select(position, Pick::Toggle, window, cx);
                        })
                        .child(check_mark(
                            if on { CheckState::On } else { CheckState::Off },
                            false,
                            false,
                            0,
                            cx,
                        )),
                )
            })
            .when(self.detail.is_some() && lead, |row| {
                let (view, key, name) = (self.view.clone(), key.clone(), key.clone());
                row.child(
                    div()
                        .flex_none()
                        .w(self.lead)
                        .flex()
                        .justify_center()
                        .cursor_pointer()
                        .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                            cx.stop_propagation();
                            view.update(cx, |view, cx| {
                                if !view.open.remove(&key) {
                                    view.open.insert(key.clone());
                                }
                                cx.notify();
                            })
                        })
                        .child(Disclosure::new(
                            (self.id.clone(), format!("open-{name}")),
                            open,
                        )),
                )
            })
            .when_some(press, |row, press| {
                row.cursor_pointer().on_click(move |event, window, cx| {
                    let held = event.modifiers();
                    let how = match (held.shift, held.platform) {
                        (true, _) => Pick::Range,
                        (false, true) => Pick::Toggle,
                        _ => Pick::One,
                    };
                    press(position, how, window, cx);
                })
            })
            .children(cols.iter().map(|col| self.cell(ix, *col, cx)))
            .into_any_element()
    }

    fn cell(&self, ix: usize, col: usize, cx: &App) -> AnyElement {
        let (row, column) = (&self.rows[ix], &self.columns[col]);
        let cell = &row.cells[col];
        let colors = &cx.theme().colors;
        let editing = self
            .editing
            .as_ref()
            .is_some_and(|(key, at)| *key == row.key && *at == column.key);
        let tinted = self.ranges[col]
            .zip(cell.number())
            .map(|(range, value)| tint(value, range, colors));
        let frame = div()
            .id((self.id.clone(), format!("cell-{}-{col}", row.key)))
            .h_full()
            .flex()
            .items_center()
            .px_3()
            .border_x_1()
            .border_color(transparent_black());
        let frame = match self.widths[col] {
            Some(width) => frame
                .w(width)
                .flex_none()
                .when(column.align == Align::End, |cell| cell.justify_end()),
            None => sized(frame, column, self.narrowest),
        };
        let content = match editing.then(|| self.editor.read(cx).field()).flatten() {
            Some(field) => {
                let editor = self.editor.clone();
                div()
                    .w_full()
                    .on_key_down(move |event, window, cx| {
                        if event.keystroke.key == "escape" {
                            cx.stop_propagation();
                            editor.update(cx, |editing, cx| editing.finish(true, window, cx));
                        }
                    })
                    .child(Input::new(&field).size(ControlSize::Sm))
                    .into_any_element()
            }
            None => draw(
                cell,
                column,
                (self.id.clone(), format!("draw-{}-{col}", row.key)).into(),
                cx,
            ),
        };
        frame
            .when_some(tinted, |cell, bg| cell.bg(bg))
            .when(
                column.editable && !editing && self.on_edit.is_some(),
                |frame| {
                    let (view, editor, on_edit) = (
                        self.view.clone(),
                        self.editor.clone(),
                        self.on_edit.clone().expect("checked"),
                    );
                    let (key, at, words) = (row.key.clone(), column.key.clone(), cell.words());
                    frame.cursor_text().on_click(move |event, window, cx| {
                        if event.click_count() != 2 {
                            return;
                        }
                        cx.stop_propagation();
                        let (key, at, on_edit) = (key.clone(), at.clone(), on_edit.clone());
                        view.update(cx, |view, _| view.editing = Some((key.clone(), at.clone())));
                        editor.update(cx, |editing, _| {
                            editing.on_commit = Some(Rc::new(move |text, window, cx| {
                                on_edit(&key, &at, text, window, cx)
                            }))
                        });
                        Editing::begin(&editor, words.to_string(), window, cx);
                    })
                },
            )
            .child(content)
            .into_any_element()
    }
}
