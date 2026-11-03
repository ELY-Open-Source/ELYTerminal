use std::path::Path;

use gpui::{
    AppContext as _, Bounds, Context, Entity, FocusHandle, InteractiveElement, IntoElement,
    KeyBinding, ParentElement, Render, Styled, TestAppContext, VisualTestContext, Window, div,
    point, px, size,
};

use super::{click, press, settle};
use crate::{
    buttons::{Button, IconButton},
    forms::{Input, TextInput},
    overlays::{FloatingToolbar, Lightbox, Peek, Slide, Spotlight, Tour, TourStep},
    primitives::{FocusNext, FocusScope, IconName},
    theme::Theme,
};

#[derive(Clone, Copy, Debug, PartialEq)]
enum Open {
    Lightbox,
    Spotlight,
    SpotField,
    Tour,
    Peek,
}

/// Opens one guide at a time over a page with a target button, recording what happens.
struct Guides {
    root: FocusHandle,
    before: FocusHandle,
    field: Entity<TextInput>,
    open: Option<Open>,
    at: usize,
    log: Vec<String>,
    lower: bool,
    pictures: usize,
}

fn target() -> Bounds<gpui::Pixels> {
    Bounds::new(point(px(0.0), px(0.0)), size(px(90.0), px(28.0)))
}

impl Render for Guides {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let note = |what: &'static str, cx: &mut Context<Self>| {
            let view = cx.entity();
            move |_: &mut Window, cx: &mut gpui::App| {
                view.update(cx, |guides, _| guides.log.push(what.into()))
            }
        };
        let (shut, stepped) = (cx.entity(), cx.entity());
        let shut = move |_: &mut Window, cx: &mut gpui::App| {
            shut.update(cx, |guides, cx| {
                guides.open = None;
                cx.notify();
            })
        };
        let step = move |to: usize, _: &mut Window, cx: &mut gpui::App| {
            stepped.update(cx, |guides, cx| {
                guides.at = to;
                cx.notify();
            })
        };
        let slides = ["a", "b", "c"]
            .into_iter()
            .take(self.pictures)
            .map(|name| Slide::new(Path::new("/missing/ely.png"), name));
        let steps = [
            TourStep::new(target(), "Here", "The first stop"),
            TourStep::new(
                Bounds::new(point(px(200.0), px(300.0)), size(px(90.0), px(28.0))),
                "There",
                "The second stop",
            ),
        ];
        let (pressed, bold) = (note("target", cx), note("bold", cx));
        let overlay = self.open.map(|open| match open {
            Open::Lightbox => Lightbox::new("light", slides, self.at, shut)
                .on_step(step)
                .into_any_element(),
            Open::Spotlight => Spotlight::new("spot", steps[0].clone(), shut).into_any_element(),
            Open::SpotField => Spotlight::new(
                "spot",
                TourStep::new(
                    Bounds::new(point(px(0.0), px(188.0)), size(px(300.0), px(28.0))),
                    "Type here",
                    "The field takes focus",
                ),
                shut,
            )
            .into_any_element(),
            Open::Tour => Tour::new("tour", steps, self.at, shut)
                .on_step(step)
                .into_any_element(),
            Open::Peek => Peek::new("peek", "Definition", true, shut)
                .child("fn peek()")
                .into_any_element(),
        });
        FocusScope::new(&self.root)
            .size_full()
            .child(
                Button::new("target", "Target").on_click(move |_, window, cx| pressed(window, cx)),
            )
            .child(div().id("before").track_focus(&self.before))
            .child(div().h(px(if self.lower { 260.0 } else { 160.0 })))
            .child(
                div().w(px(300.0)).child(Input::new(&self.field)).child(
                    FloatingToolbar::new("tools", &self.field).child(
                        IconButton::new("bold", IconName::Bold)
                            .on_click(move |_, window, cx| bold(window, cx)),
                    ),
                ),
            )
            .children(overlay)
    }
}

fn guides(cx: &mut TestAppContext) -> (Entity<Guides>, &mut VisualTestContext) {
    cx.update(|cx| {
        Theme::init(cx);
        crate::forms::bind_keys(cx);
        cx.bind_keys([KeyBinding::new("tab", FocusNext, None)]);
    });
    let (view, cx) = cx.add_window_view(|window, cx| Guides {
        root: cx.focus_handle(),
        before: cx.focus_handle(),
        field: cx.new(|cx| {
            let mut input = TextInput::new(window, cx);
            input.set_text("hello world", cx);
            input
        }),
        open: None,
        at: 0,
        log: Vec::new(),
        lower: false,
        pictures: 3,
    });
    settle(cx);
    (view, cx)
}

fn open(view: &Entity<Guides>, which: Open, cx: &mut VisualTestContext) {
    let before = view.read_with(cx, |guides, _| guides.before.clone());
    cx.update(|window, cx| window.focus(&before, cx));
    view.update(cx, |guides, cx| {
        guides.open = Some(which);
        guides.at = 0;
        cx.notify();
    });
    settle(cx);
}

fn state(view: &Entity<Guides>, cx: &mut VisualTestContext) -> (Option<Open>, usize, Vec<String>) {
    view.read_with(cx, |guides, _| (guides.open, guides.at, guides.log.clone()))
}

fn before_focused(view: &Entity<Guides>, cx: &mut VisualTestContext) -> bool {
    let before = view.read_with(cx, |guides, _| guides.before.clone());
    cx.update(|window, _| before.is_focused(window))
}

#[gpui::test]
fn the_lightbox_steps_with_arrows_and_escape_hands_focus_back(cx: &mut TestAppContext) {
    let (view, cx) = guides(cx);
    open(&view, Open::Lightbox, cx);
    for key in ["right", "right", "right", "left"] {
        press(key, cx);
    }
    assert_eq!(state(&view, cx).1, 1);
    press("escape", cx);
    assert!(state(&view, cx).0.is_none());
    assert!(before_focused(&view, cx));
}

#[gpui::test]
fn a_lightbox_whose_pictures_go_away_closes_and_hands_focus_back(cx: &mut TestAppContext) {
    let (view, cx) = guides(cx);
    open(&view, Open::Lightbox, cx);
    assert!(!before_focused(&view, cx), "the lightbox took focus");
    view.update(cx, |guides, cx| {
        guides.pictures = 0;
        cx.notify();
    });
    settle(cx);
    assert!(state(&view, cx).0.is_none(), "it closed");
    assert!(before_focused(&view, cx), "focus went back");
}

#[gpui::test]
fn the_dark_closes_the_lightbox_and_the_picture_does_not(cx: &mut TestAppContext) {
    let (view, cx) = guides(cx);
    open(&view, Open::Lightbox, cx);
    let middle = cx.update(|window, _| window.viewport_size());
    click(
        f32::from(middle.width) / 2.0,
        f32::from(middle.height) / 2.0,
        cx,
    );
    assert_eq!(state(&view, cx).0, Some(Open::Lightbox));
    click(4.0, f32::from(middle.height) - 4.0, cx);
    assert!(state(&view, cx).0.is_none());
}

#[gpui::test]
fn the_tour_walks_by_arrows_and_buttons_and_done_closes(cx: &mut TestAppContext) {
    let (view, cx) = guides(cx);
    open(&view, Open::Tour, cx);
    press("right", cx);
    assert_eq!(state(&view, cx).1, 1);
    press("left", cx);
    assert_eq!(state(&view, cx).1, 0);
    press("tab", cx);
    press("tab", cx);
    press("enter", cx);
    assert_eq!(state(&view, cx).1, 1, "Next moved on");
    press("enter", cx);
    assert!(
        state(&view, cx).0.is_none(),
        "focus stayed on the button, now Done"
    );
    assert!(before_focused(&view, cx));
}

#[gpui::test]
fn the_spotlight_passes_presses_through_its_hole_only(cx: &mut TestAppContext) {
    let (view, cx) = guides(cx);
    open(&view, Open::Spotlight, cx);
    click(40.0, 14.0, cx);
    assert_eq!(
        state(&view, cx).2,
        ["target"],
        "the lit target took the press"
    );
    click(40.0, 200.0, cx);
    assert_eq!(state(&view, cx).0, Some(Open::Spotlight), "the dim held");
    press("escape", cx);
    assert!(state(&view, cx).0.is_none());
}

#[gpui::test]
fn the_toolbar_floats_over_a_selection_only(cx: &mut TestAppContext) {
    let (view, cx) = guides(cx);
    let field = view.read_with(cx, |guides, _| guides.field.clone());
    let focus = field.read_with(cx, |input, _| input.focus().clone());
    cx.update(|window, cx| window.focus(&focus, cx));
    settle(cx);
    let start = field
        .read_with(cx, |input, _| input.bounds_for(0))
        .expect("the field laid out");
    let bold = (
        f32::from(start.left()) + 16.0,
        f32::from(start.top()) - 22.0,
    );
    click(bold.0, bold.1, cx);
    assert!(state(&view, cx).2.is_empty(), "no selection, no toolbar");
    cx.update(|window, cx| window.focus(&focus, cx));
    press("secondary-a", cx);
    click(bold.0, bold.1, cx);
    assert_eq!(state(&view, cx).2, ["bold"]);
}

#[gpui::test]
fn the_peek_closes_from_its_button(cx: &mut TestAppContext) {
    let (view, cx) = guides(cx);
    open(&view, Open::Peek, cx);
    cx.update(|window, cx| {
        window.blur(cx);
        for _ in 0..3 {
            window.focus_next(cx);
        }
    });
    press("enter", cx);
    assert!(state(&view, cx).0.is_none());
}

#[gpui::test]
fn escape_closes_a_spotlight_while_its_target_holds_focus(cx: &mut TestAppContext) {
    let (view, cx) = guides(cx);
    open(&view, Open::SpotField, cx);
    click(40.0, 200.0, cx);
    press("escape", cx);
    assert!(state(&view, cx).0.is_none());
}

#[gpui::test]
fn the_lightbox_keeps_focus_when_its_button_disables(cx: &mut TestAppContext) {
    let (view, cx) = guides(cx);
    open(&view, Open::Lightbox, cx);
    press("tab", cx);
    press("tab", cx);
    press("enter", cx);
    press("enter", cx);
    assert_eq!(state(&view, cx).1, 2, "Next walked to the end");
    press("escape", cx);
    assert!(state(&view, cx).0.is_none());
}

#[gpui::test]
fn a_press_on_a_dead_arrow_keeps_the_lightbox(cx: &mut TestAppContext) {
    let (view, cx) = guides(cx);
    open(&view, Open::Lightbox, cx);
    let screen = cx.update(|window, _| window.viewport_size());
    click(28.0, f32::from(screen.height) / 2.0, cx);
    assert_eq!(state(&view, cx).0, Some(Open::Lightbox));
}

#[gpui::test]
fn the_toolbar_follows_its_field(cx: &mut TestAppContext) {
    let (view, cx) = guides(cx);
    let field = view.read_with(cx, |guides, _| guides.field.clone());
    let focus = field.read_with(cx, |input, _| input.focus().clone());
    cx.update(|window, cx| window.focus(&focus, cx));
    press("secondary-a", cx);
    view.update(cx, |guides, cx| {
        guides.lower = true;
        cx.notify();
    });
    cx.run_until_parked();
    let start = field
        .read_with(cx, |input, _| input.bounds_for(0))
        .expect("the field laid out");
    click(
        f32::from(start.left()) + 16.0,
        f32::from(start.top()) - 22.0,
        cx,
    );
    assert_eq!(state(&view, cx).2, ["bold"]);
}

#[gpui::test]
fn back_to_the_first_step_keeps_the_tour_keys(cx: &mut TestAppContext) {
    let (view, cx) = guides(cx);
    open(&view, Open::Tour, cx);
    press("right", cx);
    press("tab", cx);
    press("tab", cx);
    press("enter", cx);
    assert_eq!(state(&view, cx).1, 0, "Back went to the first step");
    press("right", cx);
    assert_eq!(state(&view, cx).1, 1);
}

#[gpui::test]
fn left_from_back_keeps_the_tour_keys(cx: &mut TestAppContext) {
    let (view, cx) = guides(cx);
    open(&view, Open::Tour, cx);
    press("right", cx);
    press("tab", cx);
    press("tab", cx);
    press("left", cx);
    assert_eq!(state(&view, cx).1, 0);
    press("right", cx);
    assert_eq!(state(&view, cx).1, 1);
}

#[gpui::test]
fn a_lightbox_or_tour_past_its_last_closes(cx: &mut TestAppContext) {
    let (view, cx) = guides(cx);
    for open in [Open::Lightbox, Open::Tour] {
        view.update(cx, |guides, cx| {
            guides.open = Some(open);
            guides.at = 5;
            cx.notify();
        });
        settle(cx);
        assert_eq!(
            view.read_with(cx, |guides, _| guides.open),
            None,
            "{open:?}"
        );
    }
}
