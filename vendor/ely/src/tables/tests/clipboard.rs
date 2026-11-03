use std::{cell::RefCell, rc::Rc};

use gpui::{ClipboardItem, Modifiers, TestAppContext, point, px};

use super::{Grid, settle};
use crate::theme::Theme;

#[gpui::test]
fn a_paste_fills_cells_from_the_cursor(cx: &mut TestAppContext) {
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
    cx.update(|_, cx| cx.write_to_clipboard(ClipboardItem::new_string("x\ty\nz\tw".into())));
    cx.simulate_keystrokes("secondary-v");
    settle(cx);
    let cells = |row, col, text: &str| (row, col, text.to_string());
    assert_eq!(
        *heard.borrow(),
        [
            cells(0, 0, "x"),
            cells(0, 1, "y"),
            cells(1, 0, "z"),
            cells(1, 1, "w")
        ]
    );
}
