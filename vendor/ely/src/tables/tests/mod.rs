use std::{
    cell::{Cell as Count, RefCell},
    collections::HashMap,
    rc::Rc,
};

use gpui::{
    Context, FocusHandle, IntoElement, KeyBinding, Modifiers, ParentElement, Render, SharedString,
    Styled, TestAppContext, VisualTestContext, Window, div, point, px,
};

mod chinese;
mod clipboard;
mod columns;
mod stale;

use super::{Column, DataTable, FilterBuilder, FilterRule, Row};
use crate::{
    primitives::{FocusNext, FocusPrev, FocusScope},
    theme::Theme,
};

/// Five selectable rows, 36 tall under a 36 tall header; it keeps what the table reports.
struct Picks(Rc<RefCell<Vec<SharedString>>>);

impl Render for Picks {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let (now, store) = (self.0.borrow().clone(), self.0.clone());
        DataTable::new("picks", [Column::new("name", "Name")])
            .rows(
                (0..5)
                    .map(|ix| Row::new(format!("r{ix}"), [format!("Row {ix}").into()]))
                    .collect::<Vec<_>>(),
            )
            .selected(now)
            .on_select(move |keys, _, _| *store.borrow_mut() = keys.to_vec())
            .w(px(400.0))
    }
}

fn settle(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.update(|window, _| window.refresh());
    cx.run_until_parked();
}

/// Edits the cell at `at`: a double press, then `text` and Enter.
fn edit_at(at: gpui::Point<gpui::Pixels>, text: &str, cx: &mut VisualTestContext) {
    for count in [1, 2] {
        cx.simulate_event(gpui::MouseDownEvent {
            button: gpui::MouseButton::Left,
            position: at,
            modifiers: Modifiers::none(),
            click_count: count,
            first_mouse: false,
        });
        cx.simulate_event(gpui::MouseUpEvent {
            button: gpui::MouseButton::Left,
            position: at,
            modifiers: Modifiers::none(),
            click_count: count,
        });
    }
    settle(cx);
    cx.simulate_input(text);
    cx.simulate_keystrokes("enter");
    settle(cx);
}

fn picks(seen: &Rc<RefCell<Vec<SharedString>>>) -> Vec<String> {
    seen.borrow().iter().map(|key| key.to_string()).collect()
}

#[gpui::test]
fn boxes_toggle_shift_takes_a_range_and_the_header_takes_all(cx: &mut TestAppContext) {
    cx.update(Theme::init);
    let seen = Rc::new(RefCell::new(Vec::new()));
    let store = seen.clone();
    let (_, cx) = cx.add_window_view(|_, _| Picks(store));
    settle(cx);
    let row = |ix: usize, x: f32| point(px(x), px(54.0 + 36.0 * ix as f32));
    cx.simulate_click(row(0, 14.0), Modifiers::none());
    settle(cx);
    assert_eq!(picks(&seen), ["r0"]);
    cx.simulate_click(
        row(3, 200.0),
        Modifiers {
            shift: true,
            ..Modifiers::none()
        },
    );
    settle(cx);
    assert_eq!(picks(&seen), ["r0", "r1", "r2", "r3"]);
    cx.simulate_click(point(px(14.0), px(18.0)), Modifiers::none());
    settle(cx);
    assert_eq!(
        picks(&seen),
        ["r0", "r1", "r2", "r3", "r4"],
        "a mixed header box takes all"
    );
    cx.simulate_click(point(px(14.0), px(18.0)), Modifiers::none());
    settle(cx);
    assert!(picks(&seen).is_empty(), "a full one clears");
}

/// Six rows in two groups, measured as they draw.
struct Grouped(Rc<std::cell::Cell<gpui::Pixels>>);

impl Render for Grouped {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let tall = self.0.clone();
        let rows: Vec<Row> = (0..6)
            .map(|ix| {
                Row::new(
                    format!("r{ix}"),
                    [
                        if ix < 4 { "Europe" } else { "Asia" }.into(),
                        format!("Row {ix}").into(),
                    ],
                )
            })
            .collect();
        // gpui fills the window with an auto-sized root, so the measured box sits inside one.
        div().child(
            crate::primitives::Measure::new("measure", move |bounds, _, _| {
                tall.set(bounds.size.height)
            })
            .w(px(400.0))
            .child(
                DataTable::new(
                    "grouped",
                    [Column::new("region", "Region"), Column::new("name", "Name")],
                )
                .rows(rows)
                .group_by("region"),
            ),
        )
    }
}

#[gpui::test]
fn a_press_on_a_group_folds_its_rows(cx: &mut TestAppContext) {
    cx.update(Theme::init);
    let tall = Rc::new(std::cell::Cell::new(gpui::Pixels::ZERO));
    let seen = tall.clone();
    let (_, cx) = cx.add_window_view(|_, _| Grouped(seen));
    settle(cx);
    let open = tall.get();
    cx.simulate_click(point(px(60.0), px(54.0)), Modifiers::none());
    settle(cx);
    assert_eq!(
        open - tall.get(),
        px(36.0 * 4.0),
        "Europe's four rows fold away"
    );
}

/// Two rows whose names edit in place and whose detail opens; it keeps what it hears.
struct Editable(Rc<RefCell<Vec<String>>>);

impl Render for Editable {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let (heard, opened) = (self.0.clone(), self.0.clone());
        DataTable::new("editable", [Column::new("name", "Name").editable()])
            .rows(vec![
                Row::new("a", ["Ada".into()]),
                Row::new("b", ["Alan".into()]),
            ])
            .detail(move |key, _, _| {
                opened.borrow_mut().push(format!("detail {key}"));
                gpui::div().h(px(40.0)).into_any_element()
            })
            .on_edit(move |row, column, text, _, _| {
                heard
                    .borrow_mut()
                    .push(format!("edit {row} {column} {text}"))
            })
            .w(px(400.0))
    }
}

#[gpui::test]
fn a_row_opens_its_detail_and_a_double_press_edits_a_cell(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        crate::forms::bind_keys(cx);
    });
    let heard = Rc::new(RefCell::new(Vec::new()));
    let seen = heard.clone();
    let (_, cx) = cx.add_window_view(|_, _| Editable(seen));
    settle(cx);
    cx.simulate_click(point(px(14.0), px(54.0)), Modifiers::none());
    settle(cx);
    assert_eq!(heard.borrow().last().map(String::as_str), Some("detail a"));
    edit_at(
        point(px(120.0), px(54.0 + 36.0 + 40.0 + 24.0)),
        "Alan Kay",
        cx,
    );
    assert_eq!(
        heard
            .borrow()
            .iter()
            .filter(|line| line.starts_with("edit"))
            .cloned()
            .collect::<Vec<_>>(),
        ["edit b name Alan Kay"]
    );
}

/// A two-column grid of three rows that keeps every edit it hears.
struct Grid(Rc<RefCell<Vec<(usize, usize, String)>>>);

impl Render for Grid {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.render_grid()
    }
}

impl Grid {
    fn render_grid(&self) -> impl IntoElement + use<> {
        let heard = self.0.clone();
        let rows: Vec<Vec<SharedString>> = (0..3)
            .map(|row| vec![format!("a{row}").into(), format!("b{row}").into()])
            .collect();
        super::DataGrid::new("grid", ["A", "B"], rows)
            .on_change(move |edits, _, _| {
                heard.borrow_mut().extend(
                    edits
                        .iter()
                        .map(|(row, col, text)| (*row, *col, text.to_string())),
                )
            })
            .w(px(300.0))
            .h(px(200.0))
    }
}

#[gpui::test]
fn typing_edits_enter_keeps_and_backspace_clears_a_range(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        crate::forms::bind_keys(cx);
    });
    let heard = Rc::new(RefCell::new(Vec::new()));
    let seen = heard.clone();
    let (_, cx) = cx.add_window_view(|_, _| Grid(seen));
    settle(cx);
    cx.simulate_click(point(px(48.0), px(42.0)), Modifiers::none());
    settle(cx);
    cx.simulate_keystrokes("7");
    settle(cx);
    cx.simulate_keystrokes("enter");
    settle(cx);
    assert_eq!(*heard.borrow(), [(0, 0, "7".to_string())]);
    cx.simulate_keystrokes("shift-down backspace");
    settle(cx);
    assert_eq!(
        heard.borrow()[1..],
        [(1, 0, String::new()), (2, 0, String::new())],
        "enter moved down; the range clears"
    );
}

/// Filter rules the owner keeps, 600 across, rebuilt from what the builder reports.
struct Filters(Rc<RefCell<Vec<FilterRule>>>);

impl Render for Filters {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (rules, store, view) = (self.0.borrow().clone(), self.0.clone(), cx.entity());
        div().w(px(600.0)).child(
            FilterBuilder::new("filters", [("name", "Name"), ("region", "Region")])
                .rules(rules, false)
                .on_change(move |rules, _, _, cx| {
                    *store.borrow_mut() = rules.to_vec();
                    view.update(cx, |_, cx| cx.notify());
                }),
        )
    }
}

#[gpui::test]
fn removing_the_rule_being_typed_leaves_the_next_rule_its_own_value(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        crate::forms::bind_keys(cx);
    });
    let rule = |column: &'static str, value: &'static str| FilterRule {
        column: column.into(),
        value: value.into(),
        ..FilterRule::default()
    };
    let rules = Rc::new(RefCell::new(vec![
        rule("name", "one"),
        rule("region", "keep"),
    ]));
    let seen = rules.clone();
    let (_, cx) = cx.add_window_view(|_, _| Filters(seen));
    settle(cx);
    cx.simulate_click(point(px(330.0), px(12.0)), Modifiers::none());
    settle(cx);
    cx.simulate_input("typed");
    settle(cx);
    assert_eq!(
        rules.borrow()[0].value.as_ref(),
        "onetyped",
        "the first field took the typing"
    );
    cx.simulate_click(point(px(588.0), px(12.0)), Modifiers::none());
    settle(cx);
    cx.simulate_input("X");
    settle(cx);
    assert_eq!(
        *rules.borrow(),
        [rule("region", "keep")],
        "the removed rule's draft went with it"
    );
}

/// One rule, Change is above, in a 280px column.
struct NarrowRule;

impl Render for NarrowRule {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let rule = FilterRule {
            column: "change".into(),
            test: super::Test::Above,
            value: "".into(),
        };
        div()
            .w(px(280.0))
            .child(FilterBuilder::new("narrow", [("change", "Change")]).rules([rule], false))
    }
}

#[gpui::test]
fn a_rule_keeps_room_for_its_value_in_a_narrow_column(cx: &mut TestAppContext) {
    cx.update(Theme::init);
    let (_, cx) = cx.add_window_view(|_, _| NarrowRule);
    settle(cx);
    let text = cx.debug_bounds("input-text").expect("the value's text");
    assert_eq!(text.size.width, px(18.0), "the frame's inset to type in");
}

/// A two-column grid whose row count the test sets.
struct Shrinking(Rc<Count<usize>>);

impl Render for Shrinking {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let rows: Vec<Vec<SharedString>> = (0..self.0.get())
            .map(|row| vec![format!("a{row}").into(), format!("b{row}").into()])
            .collect();
        super::DataGrid::new("shrink", ["A", "B"], rows)
            .w(px(300.0))
            .h(px(200.0))
    }
}

fn grid_keys(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        crate::forms::bind_keys(cx);
        cx.bind_keys([
            KeyBinding::new("tab", FocusNext, None),
            KeyBinding::new("shift-tab", FocusPrev, None),
        ]);
    });
}

#[gpui::test]
fn a_grid_that_shrinks_keeps_its_cursor_inside(cx: &mut TestAppContext) {
    grid_keys(cx);
    let count = Rc::new(Count::new(3));
    let (view, cx) = cx.add_window_view(|_, _| Shrinking(count.clone()));
    settle(cx);
    cx.simulate_click(point(px(146.0), px(98.0)), Modifiers::none());
    settle(cx);
    count.set(1);
    view.update(cx, |_, cx| cx.notify());
    settle(cx);
    cx.simulate_keystrokes("secondary-c");
    settle(cx);
    let copied = cx.update(|_, cx| cx.read_from_clipboard().and_then(|item| item.text()));
    assert_eq!(
        copied.as_deref(),
        Some("b0"),
        "the cursor moved to the last row left"
    );
}

/// A sheet with A1:C1 merged, 4 across, that keeps every edit it hears.
struct Merged(Rc<RefCell<Vec<(usize, usize, String)>>>);

impl Render for Merged {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let heard = self.0.clone();
        let cells: HashMap<(usize, usize), SharedString> =
            [((0, 0), "Title".into())].into_iter().collect();
        super::Spreadsheet::new("merged", 3, 4)
            .cells(cells)
            .merge((0, 0), (0, 2))
            .on_change(move |edits, _, _| {
                heard.borrow_mut().extend(
                    edits
                        .iter()
                        .map(|(row, col, text)| (*row, *col, text.to_string())),
                )
            })
            .w(px(500.0))
            .h(px(200.0))
    }
}

#[gpui::test]
fn a_merge_answers_for_every_cell_it_covers(cx: &mut TestAppContext) {
    grid_keys(cx);
    let heard = Rc::new(RefCell::new(Vec::new()));
    let seen = heard.clone();
    let (_, cx) = cx.add_window_view(|_, _| Merged(seen));
    settle(cx);
    cx.simulate_click(point(px(146.0), px(42.0)), Modifiers::none());
    settle(cx);
    cx.simulate_keystrokes("7 enter");
    settle(cx);
    cx.simulate_keystrokes("up right 8 enter");
    settle(cx);
    assert_eq!(
        *heard.borrow(),
        [(0, 0, "7".to_string()), (0, 3, "8".to_string())],
        "a press inside lands on the merge; right steps past it"
    );
}

/// A grid inside a focus scope, as apps hold it, that keeps every edit it hears.
struct Scoped(FocusHandle, Rc<RefCell<Vec<(usize, usize, String)>>>);

impl Render for Scoped {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        FocusScope::new(&self.0)
            .size_full()
            .child(Grid(self.1.clone()).render_grid())
    }
}

#[gpui::test]
fn tab_steps_across_the_grid_and_keeps_an_edit(cx: &mut TestAppContext) {
    grid_keys(cx);
    let heard = Rc::new(RefCell::new(Vec::new()));
    let seen = heard.clone();
    let (_, cx) = cx.add_window_view(|_, cx| Scoped(cx.focus_handle(), seen));
    settle(cx);
    cx.simulate_click(point(px(48.0), px(42.0)), Modifiers::none());
    settle(cx);
    cx.simulate_keystrokes("tab 7 enter");
    settle(cx);
    cx.simulate_click(point(px(48.0), px(70.0)), Modifiers::none());
    settle(cx);
    cx.simulate_keystrokes("5 tab");
    settle(cx);
    cx.simulate_keystrokes("6 enter shift-tab");
    settle(cx);
    assert_eq!(
        *heard.borrow(),
        [
            (0, 1, "7".to_string()),
            (1, 0, "5".to_string()),
            (1, 1, "6".to_string())
        ],
        "tab moves right, and keeps an open edit first"
    );
}

#[gpui::test]
fn tab_steps_past_a_merge_like_the_arrows(cx: &mut TestAppContext) {
    grid_keys(cx);
    let heard = Rc::new(RefCell::new(Vec::new()));
    let seen = heard.clone();
    let (_, cx) = cx.add_window_view(|_, _| Merged(seen));
    settle(cx);
    cx.simulate_click(point(px(146.0), px(42.0)), Modifiers::none());
    settle(cx);
    cx.simulate_keystrokes("tab 9 enter");
    settle(cx);
    assert_eq!(
        *heard.borrow(),
        [(0, 3, "9".to_string())],
        "tab leaves the merge by its far side"
    );
}
