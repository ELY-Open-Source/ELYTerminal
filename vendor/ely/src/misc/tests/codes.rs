use std::{
    cell::{Cell, RefCell},
    sync::Arc,
};

use gpui::{AnyElement, IntoElement, RenderImage, TestAppContext, VisualTestContext};

use super::{at_root, press, settle, shown, stage};
use crate::misc::{Captcha, CaptchaState, QrCodeScanner, scanner::frame_of};

thread_local! {
    static FRAME: RefCell<Option<Arc<RenderImage>>> = const { RefCell::new(None) };
    static SAID: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    static STATE: Cell<CaptchaState> = const { Cell::new(CaptchaState::Asking) };
}

fn said() -> Vec<String> {
    SAID.with(|said| said.borrow().clone())
}

fn say(text: &str) {
    SAID.with(|said| said.borrow_mut().push(text.to_string()));
}

/// Hands the stage a new frame, or state, and lets it draw and read.
fn show(cx: &mut VisualTestContext) {
    cx.update(|window, _| window.refresh());
    settle(cx);
}

fn scanner() -> AnyElement {
    let scanner = QrCodeScanner::new("scanner", 1.0, |text, _, _| say(text));
    match FRAME.with(|frame| frame.borrow().clone()) {
        Some(frame) => scanner.frame(frame),
        None => scanner,
    }
    .into_any_element()
}

/// Frames read off the main thread; a code goes to the owner once, and a new one after it.
#[gpui::test]
fn a_scanner_reads_each_code_once(cx: &mut TestAppContext) {
    FRAME.with(|frame| frame.replace(Some(frame_of("first"))));
    let (_, cx) = stage(scanner, cx);
    assert_eq!(said(), ["first"]);
    assert!(shown("scanned-first", cx));
    FRAME.with(|frame| frame.replace(Some(frame_of("first"))));
    show(cx);
    assert_eq!(
        said(),
        ["first"],
        "a new frame of the same code says nothing"
    );
    FRAME.with(|frame| frame.replace(Some(frame_of("second"))));
    show(cx);
    assert_eq!(said(), ["first", "second"]);
}

/// A frame that comes while one is read waits: the read's end redraws, and the newest frame is read next.
#[gpui::test]
fn a_frame_that_comes_mid_read_is_read_next(cx: &mut TestAppContext) {
    let (_, cx) = stage(scanner, cx);
    for text in ["first", "second"] {
        FRAME.with(|frame| frame.replace(Some(frame_of(text))));
        cx.update(|window, _| window.refresh());
    }
    assert!(said().is_empty(), "both frames drew before any read ended");
    cx.run_until_parked();
    assert_eq!(said(), ["first", "second"], "only the read's end redrew");
}

fn captcha() -> AnyElement {
    Captcha::new("captcha", frame_of("W7XK"), 1.0, |text, _, _| say(text))
        .state(STATE.get())
        .into_any_element()
}

/// Enter hands the answer over and empties the field, which keeps focus through the owner's verdict.
#[gpui::test]
fn a_captcha_hands_over_answers_and_keeps_focus(cx: &mut TestAppContext) {
    let (_, cx) = stage(captcha, cx);
    press("tab", cx);
    press("enter", cx);
    assert!(said().is_empty(), "nothing typed, nothing handed over");
    cx.simulate_input(" w7xk ");
    press("enter", cx);
    assert_eq!(said(), ["w7xk"]);
    STATE.set(CaptchaState::Wrong);
    show(cx);
    assert!(shown("captcha-wrong", cx));
    cx.simulate_input("W7XK");
    press("enter", cx);
    assert_eq!(
        said(),
        ["w7xk", "W7XK"],
        "the field emptied for the next try"
    );
    STATE.set(CaptchaState::Passed);
    show(cx);
    assert!(
        shown("captcha-verified-focused", cx),
        "the resting field handed its focus to the mark"
    );
    cx.simulate_input("again");
    press("enter", cx);
    assert_eq!(said().len(), 2, "a passed captcha takes no more answers");
}

/// With the field emptied, verdicts and a new ask turn Verify and its mark into each other on one focus handle.
#[gpui::test]
fn the_owners_verdicts_leave_focus_in_place(cx: &mut TestAppContext) {
    let (_, cx) = stage(captcha, cx);
    press("tab", cx);
    cx.simulate_input("W7XK");
    press("enter", cx);
    STATE.set(CaptchaState::Checking);
    show(cx);
    press("tab", cx);
    STATE.set(CaptchaState::Wrong);
    show(cx);
    assert!(!at_root(cx), "Verify stayed a stop through the verdict");
    STATE.set(CaptchaState::Passed);
    show(cx);
    STATE.set(CaptchaState::Asking);
    show(cx);
    assert!(!at_root(cx), "the mark's focus went back to Verify");
}

/// Enter hands nothing over while the owner checks.
#[gpui::test]
fn enter_waits_while_the_owner_checks(cx: &mut TestAppContext) {
    let (_, cx) = stage(captcha, cx);
    press("tab", cx);
    cx.simulate_input("W7XK");
    press("enter", cx);
    STATE.set(CaptchaState::Checking);
    show(cx);
    cx.simulate_input("again");
    press("enter", cx);
    assert_eq!(said(), ["W7XK"]);
}

/// Verify with nothing typed sends focus to the field.
#[gpui::test]
fn an_empty_verify_sends_focus_to_the_field(cx: &mut TestAppContext) {
    let (_, cx) = stage(captcha, cx);
    press("tab", cx);
    press("tab", cx);
    press("enter", cx);
    assert!(said().is_empty());
    cx.simulate_input("W7XK");
    press("enter", cx);
    assert_eq!(said(), ["W7XK"], "the typing reached the field");
}

/// Enter on Verify hands the answer over and focus to the emptied field.
#[gpui::test]
fn verify_hands_focus_to_the_field(cx: &mut TestAppContext) {
    let (_, cx) = stage(captcha, cx);
    press("tab", cx);
    cx.simulate_input("W7XK");
    press("tab", cx);
    press("enter", cx);
    assert_eq!(said(), ["W7XK"]);
    cx.simulate_input("again");
    press("enter", cx);
    assert_eq!(said(), ["W7XK", "again"], "the typing reached the field");
}

/// A passed captcha rests: Tab finds no stop in it and typing reaches nothing.
#[gpui::test]
fn a_passed_captcha_rests(cx: &mut TestAppContext) {
    STATE.set(CaptchaState::Passed);
    let (_, cx) = stage(captcha, cx);
    press("tab", cx);
    cx.simulate_input("again");
    press("enter", cx);
    assert!(said().is_empty());
    assert!(
        !shown("captcha-verified-focused", cx),
        "Tab found no stop in the resting captcha"
    );
}

/// Verify spins while the owner checks, and its mark keeps its focus once passed.
#[gpui::test]
fn verify_keeps_its_focus_through_the_check(cx: &mut TestAppContext) {
    let (_, cx) = stage(captcha, cx);
    press("tab", cx);
    cx.simulate_input("W7XK");
    press("tab", cx);
    STATE.set(CaptchaState::Checking);
    show(cx);
    assert!(!at_root(cx), "a spinning Verify stays a stop");
    STATE.set(CaptchaState::Passed);
    show(cx);
    assert!(!at_root(cx), "the Verified mark took Verify's focus");
}
