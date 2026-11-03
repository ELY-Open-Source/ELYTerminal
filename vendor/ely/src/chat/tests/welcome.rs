use std::{cell::Cell, rc::Rc};

use gpui::{
    Context, IntoElement, ParentElement, Pixels, Render, Styled, TestAppContext, Window, canvas,
    div, px,
};

use super::settle;
use crate::{chat::WelcomeScreen, theme::Theme};

/// A welcome screen of `height` over a tall body, noting where its first child sits.
struct Welcome {
    height: Pixels,
    top: Rc<Cell<Pixels>>,
}

impl Render for Welcome {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let top = self.top.clone();
        div().w(px(600.0)).h(self.height).child(
            WelcomeScreen::new("welcome", 9)
                .child(
                    canvas(move |bounds, _, _| top.set(bounds.top()), |_, _, _, _| {}).h(px(10.0)),
                )
                .child(div().h(px(400.0))),
        )
    }
}

#[gpui::test]
fn the_welcome_centers_what_fits_and_scrolls_what_does_not(cx: &mut TestAppContext) {
    cx.update(Theme::init);
    let top = Rc::new(Cell::new(Pixels::ZERO));
    let seen = top.clone();
    let (view, cx) = cx.add_window_view(|_, _| Welcome {
        height: px(1200.0),
        top: seen,
    });
    settle(cx);
    assert!(
        top.get() > px(300.0),
        "centered in a tall space, not at {:?}",
        top.get()
    );
    view.update(cx, |view, cx| {
        view.height = px(200.0);
        cx.notify();
    });
    settle(cx);
    assert!(
        top.get() > Pixels::ZERO,
        "a short space keeps the start in reach, not at {:?}",
        top.get()
    );
}
