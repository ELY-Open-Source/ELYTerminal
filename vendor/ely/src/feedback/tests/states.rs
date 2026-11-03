use std::time::Duration;

use anyhow::anyhow;
use gpui::{
    Context, Entity, FocusHandle, IntoElement, ParentElement, Pixels, Render, Styled,
    TestAppContext, VisualTestContext, Window, div, px,
};
use web_time::Instant;

use super::{press, settle, setup, wait};
use crate::{
    buttons::Button,
    feedback::{
        ConfirmationCard, Countdown, EmptyState, ErrorBoundary, ErrorView, ResultView, SaveState,
        SavingIndicator,
    },
    layout::tests::narrow_width,
    primitives::{FocusScope, IconName, Measure},
};

/// A boundary, a confirmation card, a saving line and a countdown, recording what they do.
struct Shelf {
    root: FocusHandle,
    broken: bool,
    save: SaveState,
    until: Instant,
    log: Vec<String>,
}

impl Shelf {
    fn note(
        &self,
        cx: &mut Context<Self>,
        what: &'static str,
    ) -> impl Fn(&mut Window, &mut gpui::App) + 'static + use<> {
        let view = cx.entity();
        move |_, cx| view.update(cx, |shelf, _| shelf.log.push(what.into()))
    }
}

impl Render for Shelf {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let content = if self.broken {
            Err(anyhow!("the service answered 503"))
        } else {
            Ok(div().child("fine"))
        };
        FocusScope::new(&self.root)
            .root()
            .size_full()
            .child(ErrorBoundary::new("boundary", content).on_retry(self.note(cx, "retry")))
            .child(
                ConfirmationCard::new("card", "Sure?")
                    .on_cancel(self.note(cx, "cancel"))
                    .on_confirm(self.note(cx, "confirm")),
            )
            .child(SavingIndicator::new("saving", self.save).on_retry(self.note(cx, "save again")))
            .child(Countdown::new("countdown", self.until).on_done(self.note(cx, "done")))
    }
}

fn shelf(
    broken: bool,
    save: SaveState,
    cx: &mut TestAppContext,
) -> (Entity<Shelf>, &mut VisualTestContext) {
    setup(cx);
    let (view, cx) = cx.add_window_view(|_, cx| Shelf {
        root: cx.focus_handle(),
        broken,
        save,
        until: cx.background_executor().now() + Duration::from_secs(3),
        log: Vec::new(),
    });
    settle(cx);
    let root = view.read_with(cx, |shelf, _| shelf.root.clone());
    cx.update(|window, cx| window.focus(&root, cx));
    (view, cx)
}

fn log(view: &Entity<Shelf>, cx: &mut VisualTestContext) -> Vec<String> {
    view.read_with(cx, |shelf, _| shelf.log.clone())
}

#[gpui::test]
fn a_broken_boundary_offers_to_try_again(cx: &mut TestAppContext) {
    let (view, cx) = shelf(true, SaveState::Saved, cx);
    press("tab", cx);
    press("enter", cx);
    assert_eq!(log(&view, cx), ["retry"]);
}

#[gpui::test]
fn the_card_cancels_and_confirms(cx: &mut TestAppContext) {
    let (view, cx) = shelf(false, SaveState::Saved, cx);
    press("tab", cx);
    press("enter", cx);
    press("tab", cx);
    press("enter", cx);
    assert_eq!(log(&view, cx), ["cancel", "confirm"]);
}

#[gpui::test]
fn retry_shows_only_once_saving_failed(cx: &mut TestAppContext) {
    let (view, cx) = shelf(false, SaveState::Failed, cx);
    for _ in 0..3 {
        press("tab", cx);
    }
    press("enter", cx);
    assert_eq!(log(&view, cx), ["save again"]);
    view.update(cx, |shelf, cx| {
        shelf.save = SaveState::Saved;
        cx.notify();
    });
    settle(cx);
    let root = view.read_with(cx, |shelf, _| shelf.root.clone());
    cx.update(|window, cx| window.focus(&root, cx));
    for _ in 0..3 {
        press("tab", cx);
    }
    press("enter", cx);
    assert_eq!(
        log(&view, cx),
        ["save again", "cancel"],
        "Tab wrapped past a saved line"
    );
}

#[gpui::test]
fn the_countdown_runs_on_done_once_at_zero(cx: &mut TestAppContext) {
    let (view, cx) = shelf(false, SaveState::Saved, cx);
    wait(Duration::from_millis(2_900), cx);
    assert!(log(&view, cx).is_empty());
    wait(Duration::from_millis(200), cx);
    assert_eq!(log(&view, cx), ["done"]);
    wait(Duration::from_secs(5), cx);
    assert_eq!(log(&view, cx), ["done"]);
}

/// An empty state in a narrow column, and the height it took.
struct Column {
    body: &'static str,
    height: Pixels,
}

impl Render for Column {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        div().w(px(420.0)).child(
            Measure::new("column", move |bounds, _, cx| {
                view.update(cx, |column, _| column.height = bounds.size.height)
            })
            .child(
                EmptyState::new("empty", IconName::Inbox, "Nothing yet")
                    .body(self.body)
                    .action(Button::new("act", "Act")),
            ),
        )
    }
}

fn column_height(body: &'static str, cx: &mut TestAppContext) -> Pixels {
    setup(cx);
    let (view, cx) = cx.add_window_view(|_, _| Column {
        body,
        height: Pixels::ZERO,
    });
    settle(cx);
    view.read_with(cx, |column, _| column.height)
}

#[gpui::test]
fn a_wrapped_body_pushes_the_actions_down(cx: &mut TestAppContext) {
    let short = column_height("One line.", cx);
    let long = column_height(
        "Many words that cannot fit on one line of a narrow column, so they wrap onto a second, a third and a fourth line.",
        cx,
    );
    assert!(long > short + px(24.0), "{short:?} then {long:?}");
}

/// A message too long for its box.
struct Worded;

impl Render for Worded {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .w(px(200.0))
            .child(crate::feedback::InlineMessage::new(
                crate::primitives::Severity::Danger,
                "This file does not read as CSV: line 2: a quote inside a field it did not open.",
            ))
    }
}

#[gpui::test]
fn a_long_inline_message_wraps_inside_its_box(cx: &mut TestAppContext) {
    setup(cx);
    let (_, cx) = cx.add_window_view(|_, _| Worded);
    cx.run_until_parked();
    let text = cx
        .debug_bounds("inline-message-text")
        .expect("the words draw");
    assert!(text.right() <= px(200.0), "{text:?}");
}

#[gpui::test]
fn state_views_fill_a_column_their_block_measures_by_content(cx: &mut TestAppContext) {
    setup(cx);
    let widths = [
        narrow_width(cx, "state-root", |_, _| {
            EmptyState::new("empty", IconName::Search, "Nothing here").into_any_element()
        }),
        narrow_width(cx, "state-root", |_, _| {
            ErrorView::not_found("missing").into_any_element()
        }),
        narrow_width(cx, "state-root", |_, _| {
            ResultView::success("done", "Saved").into_any_element()
        }),
    ];
    assert_eq!(
        widths,
        [px(240.0); 3],
        "each view spans the card inside its padding"
    );
}
