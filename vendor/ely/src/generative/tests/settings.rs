use std::{cell::RefCell, rc::Rc};

use gpui::{
    AppContext as _, Context, Entity, Focusable, IntoElement, ParentElement, Render, SharedString,
    Styled, TestAppContext, VisualTestContext, Window, div, px,
};

use super::{press, settle, setup, tab};
use crate::{
    forms::TextInput,
    generative::{AspectRatioPicker, PromptEnhancer, SeedInput, StylePreset, StylePresetPicker},
};

/// A prompt with its enhancer, a suggestion the test sets, and what it heard.
struct Enhancing {
    field: Entity<TextInput>,
    suggestion: Option<SharedString>,
    heard: Vec<String>,
}

impl Render for Enhancing {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (asked, resolved) = (cx.entity(), cx.entity());
        let enhancer = PromptEnhancer::new("enhancer", &self.field)
            .on_enhance(move |prompt, _, cx| {
                asked.update(cx, |view, _| view.heard.push(format!("asked {prompt}")))
            })
            .on_resolve(move |taken, _, cx| {
                resolved.update(cx, |view, _| {
                    view.heard.push(format!("taken {taken}"));
                    view.suggestion = None;
                })
            });
        let enhancer = match &self.suggestion {
            Some(text) => enhancer.suggestion(text.clone()),
            None => enhancer,
        };
        div().w(px(420.0)).child(enhancer)
    }
}

fn enhancing(cx: &mut TestAppContext) -> (Entity<Enhancing>, &mut VisualTestContext) {
    setup(cx);
    let (view, cx) = cx.add_window_view(|window, cx| Enhancing {
        field: cx.new(|cx| TextInput::new(window, cx)),
        suggestion: None,
        heard: Vec::new(),
    });
    cx.update(|window, _| window.activate_window());
    settle(cx);
    (view, cx)
}

#[gpui::test]
fn enhance_asks_with_the_trimmed_prompt_and_use_this_takes_the_offer(cx: &mut TestAppContext) {
    let (view, cx) = enhancing(cx);
    tab(1, cx);
    cx.simulate_input("  a cat  ");
    tab(1, cx);
    press("enter", cx);
    view.update(cx, |view, cx| {
        view.suggestion = Some("a fluffy cat asleep".into());
        cx.notify();
    });
    settle(cx);
    tab(1, cx);
    press("enter", cx);
    view.read_with(cx, |view, cx| {
        assert_eq!(view.heard, ["asked a cat", "taken true"]);
        assert_eq!(view.field.read(cx).text(), "a fluffy cat asleep");
    });
    assert!(field_focused(&view, cx), "the field takes focus back");
}

fn field_focused(view: &Entity<Enhancing>, cx: &mut VisualTestContext) -> bool {
    cx.update(|window, cx| view.read(cx).field.focus_handle(cx).is_focused(window))
}

#[gpui::test]
fn keep_mine_leaves_the_prompt_as_written(cx: &mut TestAppContext) {
    let (view, cx) = enhancing(cx);
    tab(1, cx);
    cx.simulate_input("a cat");
    view.update(cx, |view, cx| {
        view.suggestion = Some("a fluffy cat".into());
        cx.notify();
    });
    settle(cx);
    tab(3, cx);
    press("enter", cx);
    view.read_with(cx, |view, cx| {
        assert_eq!(view.heard, ["taken false"]);
        assert_eq!(view.field.read(cx).text(), "a cat");
    });
    assert!(field_focused(&view, cx), "the field takes focus back");
}

/// Three presets, two ratios, a seed, and every pick heard.
struct Picks {
    seed: Entity<TextInput>,
    style: SharedString,
    heard: Rc<RefCell<Vec<String>>>,
}

impl Render for Picks {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let (styled, shaped) = (self.heard.clone(), self.heard.clone());
        let preset = |key: &str| StylePreset {
            key: key.to_string().into(),
            name: key.to_string().into(),
            picture: None,
        };
        div()
            .w(px(420.0))
            .child(
                StylePresetPicker::new(
                    "styles",
                    [preset("a"), preset("b"), preset("c")],
                    self.style.clone(),
                )
                .on_select(move |key, _, _| styled.borrow_mut().push(format!("style {key}"))),
            )
            .child(
                AspectRatioPicker::new("ratios", [(1, 1), (4, 3)], (1, 1)).on_select(
                    move |ratio, _, _| {
                        shaped
                            .borrow_mut()
                            .push(format!("ratio {}:{}", ratio.0, ratio.1))
                    },
                ),
            )
            .child(SeedInput::new("seed", &self.seed))
    }
}

fn picks(
    cx: &mut TestAppContext,
) -> (
    Entity<Picks>,
    Rc<RefCell<Vec<String>>>,
    &mut VisualTestContext,
) {
    setup(cx);
    let heard = Rc::new(RefCell::new(Vec::new()));
    let store = heard.clone();
    let (view, cx) = cx.add_window_view(|window, cx| Picks {
        seed: cx.new(|cx| TextInput::new(window, cx)),
        style: "a".into(),
        heard: store,
    });
    cx.update(|window, _| window.activate_window());
    settle(cx);
    (view, heard, cx)
}

#[gpui::test]
fn tabs_reach_each_tile_and_chip_and_enter_picks_it(cx: &mut TestAppContext) {
    let (_, heard, cx) = picks(cx);
    tab(1, cx);
    press("enter", cx);
    tab(1, cx);
    press("enter", cx);
    tab(3, cx);
    press("enter", cx);
    assert_eq!(
        *heard.borrow(),
        ["style b", "ratio 4:3"],
        "the chosen tile stays quiet"
    );
}

#[gpui::test]
fn a_seed_keeps_digits_and_the_die_rolls_one(cx: &mut TestAppContext) {
    let (view, _, cx) = picks(cx);
    tab(6, cx);
    cx.simulate_input("12a3");
    view.read_with(cx, |view, cx| {
        assert_eq!(SeedInput::read(&view.seed, cx), Some(123))
    });
    tab(1, cx);
    press("enter", cx);
    view.read_with(cx, |view, cx| {
        let text = view.seed.read(cx).text().to_string();
        assert_ne!(text, "123", "the die rolls a new seed");
        assert_eq!(
            SeedInput::read(&view.seed, cx).map(|seed| seed.to_string()),
            Some(text)
        );
    });
}
