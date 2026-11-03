use std::{
    cell::Cell,
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use anyhow::anyhow;
use gpui::{
    AppContext as _, Context, Entity, FocusHandle, IntoElement, ParentElement, Render, Styled,
    TestAppContext, VisualTestContext, Window, div,
};

use super::{press, settle, setup};
use crate::{feedback::AsyncView, primitives::FocusScope};

/// An async view whose first load fails when told to, counting loads and shows.
struct Report {
    root: FocusHandle,
    view: Entity<AsyncView<u32>>,
}

impl Render for Report {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        FocusScope::new(&self.root)
            .root()
            .size_full()
            .child(self.view.clone())
    }
}

fn report(
    fail_first: bool,
    cx: &mut TestAppContext,
) -> (Arc<AtomicUsize>, Rc<Cell<usize>>, &mut VisualTestContext) {
    setup(cx);
    let (loads, shows) = (Arc::new(AtomicUsize::new(0)), Rc::new(Cell::new(0)));
    let (counted, seen) = (loads.clone(), shows.clone());
    let (_, cx) = cx.add_window_view(move |_, cx| {
        let view = cx.new(|cx| {
            AsyncView::new(
                "report",
                move || {
                    let turn = counted.fetch_add(1, Ordering::SeqCst);
                    async move {
                        if fail_first && turn == 0 {
                            Err(anyhow!("the service answered 503"))
                        } else {
                            Ok(42)
                        }
                    }
                },
                move |value, _, _| {
                    seen.set(seen.get() + 1);
                    div().child(value.to_string())
                },
                cx,
            )
        });
        Report {
            root: cx.focus_handle(),
            view,
        }
    });
    settle(cx);
    (loads, shows, cx)
}

#[gpui::test]
fn an_async_view_shows_its_value_once_loaded(cx: &mut TestAppContext) {
    let (loads, shows, _cx) = report(false, cx);
    assert_eq!(loads.load(Ordering::SeqCst), 1);
    assert!(shows.get() >= 1, "the value was shown");
}

#[gpui::test]
fn a_failed_load_tries_again_from_its_button(cx: &mut TestAppContext) {
    let (loads, shows, cx) = report(true, cx);
    assert_eq!((loads.load(Ordering::SeqCst), shows.get()), (1, 0));
    cx.update(|window, cx| window.focus_next(cx));
    press("enter", cx);
    settle(cx);
    assert_eq!(loads.load(Ordering::SeqCst), 2);
    assert!(shows.get() >= 1, "the second load was shown");
}
