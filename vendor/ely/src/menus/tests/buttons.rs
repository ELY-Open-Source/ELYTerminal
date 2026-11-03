use gpui::{
    App, Context, FocusHandle, IntoElement, Modifiers, MouseButton, ParentElement, Render,
    SharedString, Styled, TestAppContext, VisualTestContext, Window, div, point, px,
};

use super::{press, settle};
use crate::{
    menus::{ContextMenu, Menu, MenuItem, SearchableMenu},
    primitives::{FocusScope, IconName},
    theme::Theme,
};

/// A context menu headed by clipboard buttons.
struct Clip {
    root: FocusHandle,
    ran: Vec<SharedString>,
    paste_off: bool,
    gone: Option<&'static str>,
}

impl Clip {
    fn menu(&self, cx: &mut Context<Self>) -> Menu {
        let run = |name: &'static str| {
            let view = cx.entity();
            move |_: &mut Window, cx: &mut App| {
                view.update(cx, |clip, _| clip.ran.push(name.into()))
            }
        };
        Menu::new()
            .buttons(
                [
                    MenuItem::new("Cut")
                        .icon(IconName::Scissors)
                        .on_click(run("cut")),
                    MenuItem::new("Copy")
                        .icon(IconName::Copy)
                        .disabled(true)
                        .on_click(run("copy")),
                    MenuItem::new("Paste")
                        .icon(IconName::Clipboard)
                        .keys("secondary-v")
                        .disabled(self.paste_off)
                        .on_click(run("paste")),
                    MenuItem::new("Delete")
                        .icon(IconName::Trash2)
                        .on_click(run("delete")),
                ]
                .into_iter()
                .filter(|item| Some(item.label.as_ref()) != self.gone),
            )
            .item(MenuItem::new("Select all").on_click(run("select all")))
    }
}

impl Render for Clip {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let menu = self.menu(cx);
        FocusScope::new(&self.root)
            .size_full()
            .child(SearchableMenu::new("find", "Find", menu.clone()))
            .child(ContextMenu::new("area", menu).child(div().w(px(300.0)).h(px(200.0))))
    }
}

fn clip(cx: &mut TestAppContext) -> (gpui::Entity<Clip>, &mut VisualTestContext) {
    cx.update(|cx| {
        Theme::init(cx);
        crate::forms::bind_keys(cx);
    });
    let (view, cx) = cx.add_window_view(|_, cx| Clip {
        root: cx.focus_handle(),
        ran: Vec::new(),
        paste_off: false,
        gone: None,
    });
    settle(cx);
    (view, cx)
}

fn ran(view: &gpui::Entity<Clip>, cx: &mut VisualTestContext) -> Vec<SharedString> {
    view.read_with(cx, |clip, _| clip.ran.clone())
}

fn right_click(cx: &mut VisualTestContext) {
    let at = point(px(60.0), px(120.0));
    cx.simulate_mouse_move(at, None, Modifiers::none());
    cx.simulate_mouse_down(at, MouseButton::Right, Modifiers::none());
    cx.simulate_mouse_up(at, MouseButton::Right, Modifiers::none());
    settle(cx);
}

fn press_button(selector: &'static str, cx: &mut VisualTestContext) {
    let at = cx.debug_bounds(selector).expect("the button").center();
    cx.simulate_mouse_move(at, None, Modifiers::none());
    cx.simulate_click(at, Modifiers::none());
    settle(cx);
}

#[gpui::test]
fn arrows_walk_a_button_row_past_its_disabled_button(cx: &mut TestAppContext) {
    let (view, cx) = clip(cx);
    right_click(cx);
    press("down", cx);
    press("enter", cx);
    assert_eq!(ran(&view, cx), ["cut"], "down marks the row's first button");
    right_click(cx);
    press("down", cx);
    press("right", cx);
    press("enter", cx);
    assert_eq!(ran(&view, cx), ["cut", "paste"], "right skips copy");
    right_click(cx);
    press("down", cx);
    press("left", cx);
    press("enter", cx);
    assert_eq!(ran(&view, cx), ["cut", "paste", "delete"], "left wraps");
    right_click(cx);
    press("down", cx);
    press("down", cx);
    press("enter", cx);
    assert_eq!(
        ran(&view, cx).last().map(|name| name.as_ref()),
        Some("select all")
    );
}

#[gpui::test]
fn a_pressed_button_runs_and_a_disabled_one_does_not(cx: &mut TestAppContext) {
    let (view, cx) = clip(cx);
    right_click(cx);
    press_button("menu-button-0-1", cx);
    assert!(ran(&view, cx).is_empty(), "copy is off");
    press_button("menu-button-0-2", cx);
    assert_eq!(ran(&view, cx), ["paste"]);
    assert!(
        cx.debug_bounds("menu-button-0-0").is_none(),
        "the menu closed after paste"
    );
}

#[gpui::test]
fn a_move_after_a_button_closed_the_menu_does_nothing(cx: &mut TestAppContext) {
    let (view, cx) = clip(cx);
    right_click(cx);
    let at = cx.debug_bounds("menu-button-0-2").expect("paste").center();
    // Both events land before any repaint.
    cx.update(|window, cx| {
        let down = gpui::MouseDownEvent {
            position: at,
            button: MouseButton::Left,
            modifiers: Modifiers::none(),
            click_count: 1,
            first_mouse: false,
        };
        let moved = gpui::MouseMoveEvent {
            position: at + point(px(1.0), px(0.0)),
            pressed_button: Some(MouseButton::Left),
            modifiers: Modifiers::none(),
        };
        window.dispatch_event(gpui::PlatformInput::MouseDown(down), cx);
        window.dispatch_event(gpui::PlatformInput::MouseMove(moved), cx);
    });
    settle(cx);
    assert_eq!(ran(&view, cx), ["paste"]);
}

#[gpui::test]
fn the_filter_finds_a_button_by_its_name(cx: &mut TestAppContext) {
    let (view, cx) = clip(cx);
    cx.update(|window, cx| window.focus_next(cx));
    press("enter", cx);
    cx.simulate_input("pas");
    settle(cx);
    press("enter", cx);
    assert_eq!(ran(&view, cx), ["paste"]);
}

#[gpui::test]
fn arrows_cross_buttons_while_the_filter_holds_focus(cx: &mut TestAppContext) {
    let (view, cx) = clip(cx);
    cx.update(|window, cx| window.focus_next(cx));
    press("enter", cx);
    press("right", cx);
    press("enter", cx);
    assert_eq!(ran(&view, cx), ["paste"], "right skips copy");
}

#[gpui::test]
fn a_marked_button_turned_off_or_gone_runs_nothing(cx: &mut TestAppContext) {
    for change in [
        |clip: &mut Clip| clip.paste_off = true,
        |clip: &mut Clip| clip.gone = Some("Paste"),
    ] {
        let (view, cx) = clip(cx);
        right_click(cx);
        press("down", cx);
        press("right", cx);
        view.update(cx, |clip, cx| {
            change(clip);
            cx.notify();
        });
        settle(cx);
        press("enter", cx);
        assert!(
            ran(&view, cx).is_empty(),
            "another button ran: {:?}",
            ran(&view, cx)
        );
    }
}

#[gpui::test]
fn a_disabled_button_says_it_is_unavailable(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        let off = MenuItem::new("Copy").icon(IconName::Copy).disabled(true);
        let on = MenuItem::new("Cut").icon(IconName::Scissors);
        let read =
            |item: &MenuItem, cx: &mut App| node(item, false, cx).description().map(str::to_owned);
        let expected = crate::i18n::text(cx, "state.unavailable", &[]).to_string();
        assert_eq!(read(&off, cx), Some(expected));
        assert_eq!(read(&on, cx), None);
    });
}

/// The accessibility node `item` draws as a menu button.
fn node(item: &MenuItem, current: bool, cx: &mut App) -> gpui::accesskit::Node {
    let mut node = gpui::accesskit::Node::new(gpui::Role::Button);
    let press: crate::forms::Run = std::rc::Rc::new(|_, _| {});
    let drawn = super::super::draw::button(item, (0, 0), current, press, cx);
    gpui::Element::write_a11y_info(&drawn, &mut node);
    node
}

#[gpui::test]
fn a_button_offers_its_press_and_says_it_is_marked(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        let cut = MenuItem::new("Cut").icon(IconName::Scissors);
        let off = MenuItem::new("Copy").icon(IconName::Copy).disabled(true);
        let click = gpui::accesskit::Action::Click;
        let marked = node(&cut, true, cx);
        assert!(marked.supports_action(click), "a reader can press it");
        assert_eq!(marked.is_selected(), Some(true));
        assert_eq!(node(&cut, false, cx).is_selected(), Some(false));
        assert!(!node(&off, false, cx).supports_action(click), "off");
    });
}
