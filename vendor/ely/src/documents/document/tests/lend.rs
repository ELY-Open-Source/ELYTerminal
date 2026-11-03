use gpui::{
    AppContext as _, Context, Entity, FocusHandle, InteractiveElement as _, IntoElement,
    KeyUpEvent, Keystroke, Menu, MenuItem, Modifiers, ParentElement as _, Render, Styled as _,
    TestAppContext, VisualTestContext, Window, actions, div, point, px,
};

use super::{MarkdownMode, opened, press, text};
use crate::{
    documents::{InlineFormat, MarkdownDocument},
    menus::MenuBar,
};

actions!(lend_tests, [Bold, Away]);

/// A menu bar over a document.
struct Host {
    document: Entity<MarkdownDocument>,
    away: FocusHandle,
}

impl Render for Host {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .on_action(cx.listener(|host, _: &Bold, _, cx| {
                (host
                    .document
                    .update(cx, |document, cx| document.format(InlineFormat::Strong, cx)))
                .expect("formatted");
            }))
            .on_action(cx.listener(|host, _: &Away, window, cx| window.focus(&host.away, cx)))
            .child(MenuBar::new("bar", cx.get_menus().expect("menus")))
            .child(div().track_focus(&self.away).h(px(20.0)))
            .child(self.document.clone())
    }
}

fn host(cx: &mut TestAppContext) -> (Entity<MarkdownDocument>, &mut VisualTestContext) {
    cx.update(crate::init_for_tests);
    cx.update(|cx| {
        cx.set_menus(vec![Menu {
            name: "Format".into(),
            items: vec![
                MenuItem::action("Strong", Bold),
                MenuItem::action("Away", Away),
            ],
            disabled: false,
        }])
    });
    let (host, cx) = cx.add_window_view(|window, cx| Host {
        document: cx
            .new(|cx| MarkdownDocument::new("alpha beta\n", MarkdownMode::Visual, window, cx)),
        away: cx.focus_handle(),
    });
    cx.update(|window, _| window.activate_window());
    cx.run_until_parked();
    let document = host.read_with(cx, |host, _| host.document.clone());
    press(0, Modifiers::none(), cx);
    document.update(cx, |document, cx| {
        let input = document.open.as_ref().expect("opened").input.clone();
        input.update(cx, |input, cx| input.select(0..5, cx));
    });
    (document, cx)
}

/// Opens the bar's only menu and runs row `ix`.
fn choose(ix: usize, cx: &mut VisualTestContext) {
    cx.simulate_click(point(px(12.0), px(10.0)), Modifiers::none());
    cx.run_until_parked();
    for _ in 0..=ix {
        cx.simulate_keystrokes("down");
        cx.run_until_parked();
    }
    cx.simulate_keystrokes("enter");
    cx.simulate_event(KeyUpEvent {
        keystroke: Keystroke::parse("enter").expect("a key"),
    });
    cx.run_until_parked();
}

#[gpui::test]
fn a_menu_acts_on_the_open_block_selection(cx: &mut TestAppContext) {
    let (document, cx) = host(cx);
    choose(0, cx);
    assert_eq!(opened(&document, cx), "**alpha** beta");
}

#[gpui::test]
fn a_menu_that_moves_focus_away_closes_the_block(cx: &mut TestAppContext) {
    let (document, cx) = host(cx);
    choose(1, cx);
    let open = document.read_with(cx, |document, _| document.open.is_some());
    assert!(!open, "the block wrote back");
    assert_eq!(text(&document, cx), "alpha beta\n");
}
