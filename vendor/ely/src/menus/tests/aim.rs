//! Hover intent: delayed submenus, the aim triangle, flips, occlusion.

use gpui::{
    Bounds, Context, FocusHandle, IntoElement, Modifiers, MouseButton, ParentElement, Pixels,
    Render, Styled, TestAppContext, VisualTestContext, Window, div, point, px, size,
};

use super::settle;
use crate::{
    menus::{ContextMenu, Menu, MenuItem, aim},
    primitives::{FocusScope, IconName},
    theme::Theme,
};

/// Two submenus and a plain row.
fn menu() -> Menu {
    let insert = Menu::new()
        .item(MenuItem::new("Row above"))
        .item(MenuItem::new("Row below"))
        .item(MenuItem::new("Column left"));
    let delete = Menu::new()
        .item(MenuItem::new("Delete row"))
        .item(MenuItem::new("Delete column"));
    Menu::new()
        .item(MenuItem::submenu("Insert", insert))
        .item(MenuItem::submenu("Delete", delete))
        .item(MenuItem::new("Plain"))
}

/// A submenu over icon buttons and a disabled row.
fn tools() -> Menu {
    let insert = Menu::new()
        .item(MenuItem::new("Row above"))
        .item(MenuItem::new("Row below"))
        .item(MenuItem::new("Column left"))
        .item(MenuItem::new("Column right"));
    Menu::new()
        .item(MenuItem::submenu("Insert", insert))
        .buttons([
            MenuItem::new("Cut").icon(IconName::Scissors),
            MenuItem::new("Copy").icon(IconName::Copy),
            MenuItem::new("Paste").icon(IconName::Clipboard),
        ])
        .item(MenuItem::new("Gone").disabled(true))
        .item(MenuItem::new("Plain"))
}

/// A region whose right click opens `menu`.
struct Region {
    root: FocusHandle,
    menu: fn() -> Menu,
}

impl Render for Region {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        FocusScope::new(&self.root).size_full().child(
            ContextMenu::new("region", (self.menu)())
                .size_full()
                .child(div().size_full()),
        )
    }
}

/// A window `width` wide, `menu` opened at `x`.
fn region(width: f32, x: f32, cx: &mut TestAppContext) -> &mut VisualTestContext {
    region_of(menu, width, x, cx)
}

fn region_of(
    menu: fn() -> Menu,
    width: f32,
    x: f32,
    cx: &mut TestAppContext,
) -> &mut VisualTestContext {
    cx.update(|cx| {
        Theme::init(cx);
        crate::forms::bind_keys(cx);
    });
    let (_, cx) = cx.add_window_view(|_, cx| Region {
        root: cx.focus_handle(),
        menu,
    });
    cx.simulate_resize(size(px(width), px(600.0)));
    settle(cx);
    right_click(point(px(x), px(20.0)), cx);
    cx
}

fn right_click(at: gpui::Point<Pixels>, cx: &mut VisualTestContext) {
    cx.simulate_mouse_move(at, None, Modifiers::none());
    cx.simulate_mouse_down(at, MouseButton::Right, Modifiers::none());
    cx.simulate_mouse_up(at, MouseButton::Right, Modifiers::none());
    settle(cx);
}

fn bounds(name: &str, cx: &mut VisualTestContext) -> Option<Bounds<Pixels>> {
    cx.debug_bounds(Box::leak(name.to_string().into_boxed_str()))
}

fn row(label: &str, cx: &mut VisualTestContext) -> Bounds<Pixels> {
    bounds(&format!("menu-row-{label}"), cx).expect("the row is drawn")
}

fn shown(label: &str, cx: &mut VisualTestContext) -> bool {
    bounds(&format!("menu-row-{label}"), cx).is_some()
}

fn move_to(at: gpui::Point<Pixels>, cx: &mut VisualTestContext) {
    cx.simulate_mouse_move(at, None, Modifiers::none());
    settle(cx);
}

fn wait(time: std::time::Duration, cx: &mut VisualTestContext) {
    cx.executor().advance_clock(time);
    settle(cx);
}

/// Opens Insert's submenu by resting on it.
fn rest_on_insert(cx: &mut VisualTestContext) {
    move_to(row("Insert", cx).center(), cx);
    wait(aim::OPEN, cx);
    assert!(shown("Row above", cx), "Insert's submenu opened");
}

#[gpui::test]
fn a_submenu_opens_after_the_pointer_rests(cx: &mut TestAppContext) {
    let cx = region(1000.0, 40.0, cx);
    move_to(row("Insert", cx).center(), cx);
    assert!(!shown("Row above", cx), "not yet");
    wait(aim::OPEN, cx);
    assert!(shown("Row above", cx));
}

#[gpui::test]
fn aiming_at_the_submenu_holds_it_across_rows(cx: &mut TestAppContext) {
    let cx = region(1000.0, 40.0, cx);
    rest_on_insert(cx);
    let delete = row("Delete", cx);
    move_to(point(delete.right() - px(16.0), delete.center().y), cx);
    assert!(
        shown("Row above", cx),
        "crossing Delete keeps Insert's submenu"
    );
    wait(aim::HOLD / 2, cx);
    assert!(shown("Row above", cx));
    move_to(row("Row below", cx).center(), cx);
    wait(aim::HOLD, cx);
    assert!(shown("Row above", cx), "the submenu was reached");
    assert!(!shown("Delete row", cx));
}

#[gpui::test]
fn resting_on_a_crossed_row_switches_after_the_hold(cx: &mut TestAppContext) {
    let cx = region(1000.0, 40.0, cx);
    rest_on_insert(cx);
    let delete = row("Delete", cx);
    move_to(point(delete.right() - px(16.0), delete.center().y), cx);
    wait(aim::HOLD, cx);
    assert!(shown("Delete row", cx), "Delete's submenu replaced it");
    assert!(!shown("Row above", cx));
}

#[gpui::test]
fn moving_away_from_the_submenu_switches_at_once(cx: &mut TestAppContext) {
    let cx = region(1000.0, 40.0, cx);
    rest_on_insert(cx);
    let insert = row("Insert", cx).center();
    move_to(point(insert.x, row("Delete", cx).center().y), cx);
    assert!(!shown("Row above", cx), "straight down left Insert");
    wait(aim::OPEN, cx);
    assert!(shown("Delete row", cx));
}

#[gpui::test]
fn a_submenu_without_room_on_the_right_opens_on_the_left(cx: &mut TestAppContext) {
    let cx = region(800.0, 790.0, cx);
    rest_on_insert(cx);
    let parent = bounds("menu-panel-0", cx).expect("the menu");
    let sub = bounds("menu-panel-1", cx).expect("the submenu");
    // It meets the row's edge, as on the right.
    let edge = row("Insert", cx).left() + crate::motion::NUDGE;
    assert!(sub.left() < parent.left(), "{sub:?} left of {parent:?}");
    assert!(sub.right() <= edge, "{sub:?} leaves the rows of {parent:?}");
}

#[gpui::test]
fn rows_under_a_submenu_take_no_hover(cx: &mut TestAppContext) {
    let cx = region(320.0, 10.0, cx);
    rest_on_insert(cx);
    let (plain, sub) = (
        row("Plain", cx),
        bounds("menu-panel-1", cx).expect("submenu"),
    );
    assert!(sub.left() < plain.right(), "the submenu covers the menu");
    let under = point((sub.left() + plain.right()) / 2.0, plain.center().y);
    assert!(sub.contains(&under), "the point is on the submenu");
    move_to(under, cx);
    wait(aim::HOLD + aim::OPEN, cx);
    assert!(
        shown("Row above", cx),
        "Plain under the submenu stayed unmarked"
    );
}

#[gpui::test]
fn leaving_a_row_drops_its_delayed_open(cx: &mut TestAppContext) {
    let cx = region(1000.0, 40.0, cx);
    let insert = row("Insert", cx);
    move_to(insert.center(), cx);
    wait(aim::OPEN / 2, cx);
    let panel = bounds("menu-panel-0", cx).expect("the menu");
    move_to(point(panel.left() + px(1.0), insert.center().y), cx);
    wait(aim::OPEN, cx);
    assert!(!shown("Row above", cx), "padding is not the row");
    move_to(insert.center(), cx);
    wait(aim::OPEN / 2, cx);
    assert!(!shown("Row above", cx), "re-entry waits the full pause");
    wait(aim::OPEN / 2, cx);
    assert!(shown("Row above", cx));
}

#[gpui::test]
fn leaving_the_menu_drops_the_delayed_open(cx: &mut TestAppContext) {
    let cx = region(1000.0, 40.0, cx);
    move_to(row("Insert", cx).center(), cx);
    wait(aim::OPEN / 2, cx);
    move_to(point(px(900.0), px(500.0)), cx);
    wait(aim::OPEN, cx);
    assert!(!shown("Row above", cx));
}

#[gpui::test]
fn a_reopened_menu_forgets_the_old_hover(cx: &mut TestAppContext) {
    let cx = region(1000.0, 40.0, cx);
    move_to(row("Insert", cx).center(), cx);
    wait(aim::OPEN / 2, cx);
    right_click(point(px(500.0), px(300.0)), cx);
    wait(aim::OPEN, cx);
    assert!(!shown("Row above", cx));
}

#[gpui::test]
fn leaving_a_crossed_row_keeps_the_submenu(cx: &mut TestAppContext) {
    let cx = region(1000.0, 40.0, cx);
    rest_on_insert(cx);
    let delete = row("Delete", cx);
    move_to(point(delete.right() - px(16.0), delete.center().y), cx);
    move_to(point(px(900.0), px(500.0)), cx);
    wait(aim::HOLD, cx);
    assert!(shown("Row above", cx), "Delete was abandoned");
    assert!(!shown("Delete row", cx));
}

#[gpui::test]
fn a_disabled_row_drops_the_delayed_open(cx: &mut TestAppContext) {
    let cx = region_of(tools, 1000.0, 40.0, cx);
    move_to(row("Insert", cx).center(), cx);
    wait(aim::OPEN / 2, cx);
    move_to(row("Gone", cx).center(), cx);
    wait(aim::OPEN, cx);
    assert!(!shown("Row above", cx));
}

#[gpui::test]
fn aiming_across_icon_buttons_holds_the_submenu(cx: &mut TestAppContext) {
    let cx = region_of(tools, 1000.0, 40.0, cx);
    let insert = row("Insert", cx);
    let from = point(insert.left() + px(4.0), insert.center().y);
    move_to(from, cx);
    wait(aim::OPEN, cx);
    let sub = bounds("menu-panel-1", cx).expect("Insert's submenu");
    let paste = bounds("menu-button-1-2", cx).expect("the paste button");
    assert!(aim::toward(from, paste.center(), sub), "the crossing aims");
    move_to(paste.center(), cx);
    assert!(shown("Row above", cx), "crossing a button keeps it");
    move_to(row("Row below", cx).center(), cx);
    wait(aim::HOLD, cx);
    assert!(shown("Row above", cx), "the submenu was reached");
    move_to(from, cx);
    move_to(paste.center(), cx);
    wait(aim::HOLD, cx);
    assert!(!shown("Row above", cx), "resting on the button marks it");
}
