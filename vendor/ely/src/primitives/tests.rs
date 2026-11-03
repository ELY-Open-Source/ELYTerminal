use std::{cell::RefCell, rc::Rc, time::Duration};

use gpui::{
    AnyView, App, Context, Entity, FocusHandle, IntoElement, KeyBinding, Modifiers, ParentElement,
    Render, RenderOnce, Styled, TestAppContext, Window, actions, anchored, div, point, prelude::*,
    px,
};

use super::{FocusScope, Tooltip, raise};
use crate::{i18n::I18n, theme::Theme};

/// A raised square holding a raised button, under a cover drawn after it in the same layer.
struct Stack {
    heard: Vec<&'static str>,
}

impl Render for Stack {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (inner, cover) = (cx.entity(), cx.entity());
        let button = div()
            .id("inner")
            .size(px(100.0))
            .occlude()
            .on_click(move |_, _, cx| inner.update(cx, |stack, _| stack.heard.push("inner")));
        let sheet = div()
            .id("cover")
            .absolute()
            .top_0()
            .left_0()
            .size(px(200.0))
            .on_click(move |_, _, cx| cover.update(cx, |stack, _| stack.heard.push("cover")));
        div().size_full().child(raise(
            "sheet",
            anchored().position(point(px(0.0), px(0.0))).child(
                div()
                    .relative()
                    .size(px(200.0))
                    .child(raise(
                        "button",
                        anchored().position(point(px(50.0), px(50.0))).child(button),
                    ))
                    .child(sheet),
            ),
        ))
    }
}

#[gpui::test]
fn what_is_raised_inside_a_raise_lies_over_it(cx: &mut TestAppContext) {
    cx.update(Theme::init);
    let (view, cx) = cx.add_window_view(|_, _| Stack { heard: Vec::new() });
    cx.run_until_parked();
    cx.simulate_click(point(px(100.0), px(100.0)), Modifiers::none());
    assert_eq!(
        view.read_with(cx, |stack, _| stack.heard.clone()),
        ["inner"]
    );
}

/// A focusable box whose focus handle lives in keyed state; each draw tells the test which one it drew.
#[derive(IntoElement)]
struct Held {
    seen: Rc<RefCell<Vec<Entity<FocusHandle>>>>,
}

impl RenderOnce for Held {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let kept = window.use_keyed_state("kept", cx, |_, cx| cx.focus_handle());
        self.seen.borrow_mut().push(kept.clone());
        let handle = kept.read(cx).clone();
        div().id("held").size(px(40.0)).track_focus(&handle)
    }
}

/// A raised sheet holding a raised tip that comes and goes, then a raised box that must stay itself.
struct Nest {
    tip: bool,
    seen: Rc<RefCell<Vec<Entity<FocusHandle>>>>,
}

impl Render for Nest {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let tip = self.tip.then(|| {
            raise(
                "tip",
                anchored()
                    .position(point(px(0.0), px(0.0)))
                    .child(div().child("tip")),
            )
        });
        let held = Held {
            seen: self.seen.clone(),
        };
        div().size_full().child(raise(
            "sheet",
            anchored().position(point(px(0.0), px(0.0))).child(
                div().size(px(200.0)).children(tip).child(raise(
                    "held",
                    anchored().position(point(px(50.0), px(50.0))).child(held),
                )),
            ),
        ))
    }
}

#[gpui::test]
fn a_raise_keeps_its_state_while_another_comes_before_it(cx: &mut TestAppContext) {
    cx.update(Theme::init);
    let seen = Rc::new(RefCell::new(Vec::new()));
    let (view, cx) = cx.add_window_view(|_, _| Nest {
        tip: false,
        seen: seen.clone(),
    });
    cx.run_until_parked();
    let first = seen.borrow().last().cloned().expect("the box drew");
    let handle = first.read_with(cx, |handle, _| handle.clone());
    cx.update(|window, cx| window.focus(&handle, cx));
    view.update(cx, |nest, cx| {
        nest.tip = true;
        cx.notify();
    });
    cx.run_until_parked();
    let last = seen.borrow().last().cloned().expect("the box drew again");
    assert_eq!(
        last.entity_id(),
        first.entity_id(),
        "the box kept its state"
    );
    assert!(
        cx.update(|window, _| handle.is_focused(window)),
        "the box kept focus"
    );
}

actions!(host, [Indent]);

/// A root scope around a field whose own context binds Tab.
struct Field {
    root: FocusHandle,
    field: FocusHandle,
    indents: usize,
}

impl Render for Field {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        FocusScope::new(&self.root).root().size_full().child(
            div()
                .key_context("HostField")
                .track_focus(&self.field)
                .size(px(100.0))
                .on_action(move |_: &Indent, _, cx| view.update(cx, |field, _| field.indents += 1)),
        )
    }
}

#[gpui::test]
fn a_deeper_context_keeps_its_own_tab_whatever_init_ran_last(cx: &mut TestAppContext) {
    cx.update(|cx| {
        cx.bind_keys([KeyBinding::new("tab", Indent, Some("HostField"))]);
        crate::init_for_tests(cx);
    });
    let (view, cx) = cx.add_window_view(|_, cx| Field {
        root: cx.focus_handle(),
        field: cx.focus_handle(),
        indents: 0,
    });
    cx.update(|window, cx| {
        let field = view.read(cx).field.clone();
        window.focus(&field, cx);
    });
    cx.simulate_keystrokes("tab");
    assert_eq!(view.read_with(cx, |field, _| field.indents), 1);
}

/// A field that keeps Tab, and a stop after it.
struct Kept {
    root: FocusHandle,
    field: FocusHandle,
    after: FocusHandle,
    indents: usize,
}

impl Render for Kept {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        FocusScope::new(&self.root)
            .root()
            .size_full()
            .child(
                div()
                    .key_context("HostField")
                    .track_focus(&self.field)
                    .size(px(100.0))
                    .on_action(move |_: &Indent, _, cx| {
                        view.update(cx, |kept, _| kept.indents += 1)
                    }),
            )
            .child(div().track_focus(&self.after).size(px(100.0)))
    }
}

#[gpui::test]
fn f6_leaves_a_field_that_keeps_tab(cx: &mut TestAppContext) {
    cx.update(|cx| {
        cx.bind_keys([KeyBinding::new("tab", Indent, Some("HostField"))]);
        crate::init_for_tests(cx);
    });
    let (view, cx) = cx.add_window_view(|_, cx| Kept {
        root: cx.focus_handle(),
        field: cx.focus_handle().tab_stop(true),
        after: cx.focus_handle().tab_stop(true),
        indents: 0,
    });
    cx.update(|window, cx| {
        let field = view.read(cx).field.clone();
        window.focus(&field, cx);
    });
    cx.simulate_keystrokes("tab");
    let after = view.read_with(cx, |kept, _| kept.after.clone());
    cx.simulate_keystrokes("f6");
    assert_eq!(
        view.read_with(cx, |kept, _| kept.indents),
        1,
        "Tab stayed in the field"
    );
    cx.update(|window, _| assert!(after.is_focused(window), "F6 moved on"));
    cx.simulate_keystrokes("shift-f6");
    let field = view.read_with(cx, |kept, _| kept.field.clone());
    cx.update(|window, _| assert!(field.is_focused(window), "Shift-F6 came back"));
}

/// A view that paints one image until told to retire it.
struct Painted {
    image: Option<std::sync::Arc<gpui::RenderImage>>,
    retired: super::Retired,
}

impl Render for Painted {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        gpui::canvas(
            |_, _, _| {},
            move |bounds, _, window, cx| {
                view.update(cx, |painted, _| {
                    painted.retired.release(window);
                    if let Some(image) = painted.image.clone() {
                        window
                            .paint_image(bounds, bounds, Default::default(), image, 0, false)
                            .expect("paints");
                    }
                })
            },
        )
        .size(px(10.0))
    }
}

#[gpui::test]
fn a_retired_image_is_freed_at_the_next_paint(cx: &mut TestAppContext) {
    cx.update(Theme::init);
    let pixels = image::RgbaImage::from_pixel(2, 2, image::Rgba([1, 2, 3, 255]));
    let image = std::sync::Arc::new(gpui::RenderImage::new([image::Frame::new(pixels)]));
    let (view, cx) = cx.add_window_view(|_, _| Painted {
        image: Some(image),
        retired: super::Retired::default(),
    });
    cx.run_until_parked();
    view.update(cx, |painted, cx| {
        let old = painted.image.take().expect("painted");
        painted.retired.push(old);
        cx.notify();
    });
    cx.run_until_parked();
    assert!(
        view.read_with(cx, |painted, _| painted.retired.is_empty()),
        "the old image was freed"
    );
    // One still held elsewhere waits until it is let go.
    let pixels = image::RgbaImage::from_pixel(2, 2, image::Rgba([4, 5, 6, 255]));
    let shared = std::sync::Arc::new(gpui::RenderImage::new([image::Frame::new(pixels)]));
    view.update(cx, |painted, cx| {
        painted.retired.push(shared.clone());
        cx.notify();
    });
    cx.run_until_parked();
    assert!(
        !view.read_with(cx, |painted, _| painted.retired.is_empty()),
        "a held image waits"
    );
    drop(shared);
    view.update(cx, |_, cx| cx.notify());
    cx.run_until_parked();
    assert!(
        view.read_with(cx, |painted, _| painted.retired.is_empty()),
        "then it is freed"
    );
}

/// Holds an open tooltip's body, as gpui does.
struct Open {
    tip: AnyView,
}

impl Render for Open {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(self.tip.clone())
    }
}

#[gpui::test]
fn an_open_tooltip_withdraws_when_the_language_changes(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        cx.set_global(
            I18n::new("en-US")
                .catalog("en-US", &[])
                .catalog("zh-CN", &[]),
        );
    });
    let (_, cx) = cx.add_window_view(|window, cx| {
        let body = |_: &mut Window, _: &mut App| {
            let words = div().debug_selector(|| "tip".into()).child("Close");
            words.into_any_element()
        };
        Open {
            tip: Tooltip::rich(body)(window, cx),
        }
    });
    cx.run_until_parked();
    assert!(
        cx.debug_bounds("tip").is_some(),
        "it shows in its own language"
    );
    cx.update(|_, cx| I18n::set_locale("zh-CN", cx));
    cx.run_until_parked();
    assert!(cx.debug_bounds("tip").is_none(), "it withdraws in another");
}

/// A button whose tooltip speaks its drawing locale.
struct Worded;

impl Render for Worded {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let words = cx.global::<I18n>().text("label", &[]);
        let english = words.as_ref() == "English";
        let body = move |_: &mut Window, _: &mut App| {
            let shown = if english { "english" } else { "chinese" };
            let tip = div().debug_selector(move || shown.into());
            tip.child(words.clone()).into_any_element()
        };
        let button = div()
            .id("button")
            .size(px(80.0))
            .tooltip(Tooltip::rich(body));
        div().size_full().child(button)
    }
}

#[gpui::test]
fn a_tooltip_worded_before_a_language_change_does_not_open_after_it(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        let english = I18n::new("en-US").catalog("en-US", &[("label", "English")]);
        cx.set_global(english.catalog("zh-CN", &[("label", "中文")]));
    });
    let (_, cx) = cx.add_window_view(|_, _| Worded);
    cx.run_until_parked();
    cx.simulate_mouse_move(point(px(20.0), px(20.0)), None, Modifiers::none());
    cx.run_until_parked();
    // gpui delays a tooltip; the language changes meanwhile.
    cx.executor().advance_clock(Duration::from_millis(120));
    cx.update(|_, cx| I18n::set_locale("zh-CN", cx));
    cx.run_until_parked();
    cx.executor().advance_clock(Duration::from_millis(600));
    cx.run_until_parked();
    assert!(
        cx.debug_bounds("english").is_none(),
        "no English opens after the change"
    );
    cx.simulate_mouse_move(point(px(200.0), px(200.0)), None, Modifiers::none());
    cx.run_until_parked();
    cx.simulate_mouse_move(point(px(20.0), px(20.0)), None, Modifiers::none());
    cx.run_until_parked();
    cx.executor().advance_clock(Duration::from_millis(600));
    cx.run_until_parked();
    assert!(
        cx.debug_bounds("chinese").is_some(),
        "a new hover opens in Chinese"
    );
}
