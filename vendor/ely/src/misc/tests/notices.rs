use std::cell::{Cell, RefCell};

use gpui::{
    AnyElement, App, Context, FocusHandle, IntoElement, Modifiers, ParentElement, Render, Styled,
    TestAppContext, Window, div, point, px,
};

use super::{at_root, press, settle, shown, stage};
use crate::misc::{ConsentDialog, CookieBanner, LicenseViewer, Package};
use crate::{buttons::Button, primitives::FocusScope, theme::Theme};

thread_local! {
    static SAID: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    static OPEN: Cell<bool> = const { Cell::new(true) };
}

fn said() -> Vec<String> {
    SAID.with(|said| said.borrow().clone())
}

fn say(text: impl Into<String>) {
    SAID.with(|said| said.borrow_mut().push(text.into()));
}

fn licenses() -> AnyElement {
    LicenseViewer::new(
        "licenses",
        [
            Package::new("Alpha", "1.0", "MIT", "Permission is hereby granted"),
            Package::new("Beta", "2.3", "ISC", "Permission to use, copy, modify"),
            Package::new("Gamma", "0.9", "OFL-1.1", "Permission is hereby granted"),
        ],
    )
    .into_any_element()
}

/// Each row reads its version and license; the first license shows, and moves down the list show the next ones, the list marking each.
#[gpui::test]
fn a_license_viewer_shows_the_chosen_license(cx: &mut TestAppContext) {
    let (_, cx) = stage(licenses, cx);
    assert!(shown("license-Alpha", cx) && shown("item-description-2.3 · ISC", cx));
    press("tab", cx);
    press("down", cx);
    assert!(shown("license-Beta", cx) && shown("item-selected-Beta", cx));
    press("down", cx);
    assert!(
        shown("license-Gamma", cx),
        "the list kept its mark on Beta, so its cursor went on from there"
    );
}

fn consent() -> AnyElement {
    let answer = |word: &'static str| {
        move |_: &mut gpui::Window, _: &mut gpui::App| {
            OPEN.set(false);
            say(word)
        }
    };
    match OPEN.get() {
        true => ConsentDialog::new(
            "consent",
            "Terms",
            "Be kind.",
            answer("accepted"),
            answer("declined"),
        )
        .into_any_element(),
        false => gpui::div().into_any_element(),
    }
}

/// Accept asks for the box until it is ticked, then answers once.
#[gpui::test]
fn consent_waits_for_the_box(cx: &mut TestAppContext) {
    let (_, cx) = stage(consent, cx);
    press("tab", cx);
    press("tab", cx);
    press("tab", cx);
    press("enter", cx);
    assert!(said().is_empty() && shown("consent-tick-first", cx));
    press("shift-tab", cx);
    press("shift-tab", cx);
    press("space", cx);
    press("tab", cx);
    press("tab", cx);
    press("enter", cx);
    assert_eq!(said(), ["accepted"], "Accept answers alone");
}

/// Escape, Decline and a press on the scrim each decline, once.
#[gpui::test]
fn every_other_way_out_declines(cx: &mut TestAppContext) {
    let (_, cx) = stage(consent, cx);
    press("tab", cx);
    press("escape", cx);
    OPEN.set(true);
    cx.update(|window, _| window.refresh());
    settle(cx);
    press("tab", cx);
    press("tab", cx);
    press("enter", cx);
    OPEN.set(true);
    cx.update(|window, _| window.refresh());
    settle(cx);
    cx.simulate_click(point(px(3.0), px(3.0)), Modifiers::none());
    settle(cx);
    assert_eq!(said(), ["declined", "declined", "declined"]);
}

/// A page with a button that opens the consent dialog, which answers into `said`.
struct Opener {
    root: FocusHandle,
    button: FocusHandle,
}

impl Render for Opener {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        FocusScope::new(&self.root)
            .root()
            .size_full()
            .child(
                Button::new("open", "Terms")
                    .focus_handle(&self.button)
                    .on_click(move |_, _, cx| {
                        OPEN.set(true);
                        view.update(cx, |_, cx| cx.notify());
                    }),
            )
            .child(div().child(consent()))
    }
}

/// Accept hands focus back to the button that opened the terms.
#[gpui::test]
fn consent_hands_focus_back_to_its_opener(cx: &mut TestAppContext) {
    OPEN.set(false);
    cx.update(|cx: &mut App| {
        Theme::init(cx);
        crate::forms::bind_keys(cx);
        cx.bind_keys([
            gpui::KeyBinding::new("tab", crate::primitives::FocusNext, None),
            gpui::KeyBinding::new("shift-tab", crate::primitives::FocusPrev, None),
        ]);
    });
    let (view, cx) = cx.add_window_view(|_, cx| Opener {
        root: cx.focus_handle(),
        button: cx.focus_handle(),
    });
    let button = view.read_with(cx, |opener, _| opener.button.clone());
    cx.update(|window, cx| window.focus(&button, cx));
    settle(cx);
    press("enter", cx);
    press("tab", cx);
    press("space", cx);
    press("tab", cx);
    press("tab", cx);
    press("enter", cx);
    assert_eq!(said(), ["accepted"]);
    assert!(cx.update(|window, _| button.is_focused(window)));
}

fn cookies() -> AnyElement {
    CookieBanner::new("cookies", "This page would set cookies.", |kinds, _, _| {
        say(format!("{kinds:?}"))
    })
    .kind("Analytics", "Counts visits.")
    .kind("Marketing", "Shows offers.")
    .into_any_element()
}

/// Accept all names every kind and Reject all none; Choose turns into Save choices on its focus, which names the switches turned on.
#[gpui::test]
fn a_cookie_banner_hands_over_the_kinds_chosen(cx: &mut TestAppContext) {
    let (_, cx) = stage(cookies, cx);
    press("tab", cx);
    press("enter", cx);
    press("tab", cx);
    press("enter", cx);
    press("tab", cx);
    press("enter", cx);
    assert!(!at_root(cx), "Save choices took Choose's focus");
    assert!(
        shown("cookie-necessary-true-true", cx),
        "necessary cookies stay on"
    );
    for _ in 0..3 {
        press("shift-tab", cx);
    }
    press("space", cx);
    for _ in 0..3 {
        press("tab", cx);
    }
    press("enter", cx);
    assert_eq!(
        said(),
        [r#"["Analytics", "Marketing"]"#, "[]", r#"["Marketing"]"#]
    );
    for _ in 0..5 {
        press("shift-tab", cx);
    }
    press("space", cx);
    assert_eq!(
        said().len(),
        4,
        "Shift-Tab went round past the resting Necessary switch to Save choices"
    );
}
