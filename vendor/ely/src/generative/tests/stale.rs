use std::{cell::RefCell, rc::Rc, time::Duration};

use gpui::{
    AppContext as _, Context, Entity, EntityInputHandler as _, Focusable as _, IntoElement,
    Modifiers, MouseButton, ParentElement, Render, Styled, TestAppContext, Window, div, point, px,
};

use super::{press, settle, setup, tab};
use crate::forms::TextInput;
use crate::generative::{
    ABCompareView, AspectRatioPicker, Embedded, EmbeddingVisualizer, FineTuneJobCard,
    InpaintCanvas, MaskStroke, ModelState, ModelStatus, Outcome, PromptVersionHistory, SeedInput,
    Shot, StylePreset, StylePresetPicker, TTSVoicePicker, TunePhase, VariationPicker, Verdict,
    VideoGenerationTimeline, Voice,
};

/// Each picker asked for what its list no longer holds, and counts past their whole.
struct Stale;

impl Render for Stale {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let preset = StylePreset {
            key: "ink".into(),
            name: "Ink".into(),
            picture: None,
        };
        let voice = Voice {
            key: "iris".into(),
            name: "Iris".into(),
            about: "a voice".into(),
            tags: Vec::new(),
        };
        let shot = Shot {
            key: "a".into(),
            prompt: "a pan".into(),
            length: Duration::from_secs(4),
            outcome: Outcome::Done("missing.jpg".into()),
        };
        let point = |group| Embedded {
            key: format!("in-{group}").into(),
            label: "a point".into(),
            group,
            at: [0.0, 0.0, 0.0],
        };
        div()
            .w(px(640.0))
            .child(
                ABCompareView::new("compare")
                    .side("Orchid", "one answer")
                    .side("Juniper", "another")
                    .verdict(Verdict::Side(5)),
            )
            .child(AspectRatioPicker::new("ratios", [(1, 1), (4, 3)], (16, 9)))
            .child(StylePresetPicker::new("styles", [preset], "gone"))
            .child(
                VideoGenerationTimeline::new("timeline", [shot], Duration::ZERO).selected("gone"),
            )
            .child(TTSVoicePicker::new("voices", [voice]).selected("gone"))
            .child(VariationPicker::new("variations", ["a.jpg"], 1.5, 4))
            .child(PromptVersionHistory::new("history", [], 0))
            .child(FineTuneJobCard::new(
                "tune",
                "Orchid tuned",
                "Orchid",
                "notes.jsonl",
                TunePhase::Running(7, 3, 0.5),
            ))
            .child(ModelStatus::new("model", "Orchid", ModelState::Ready).memory(5, 4))
            .child(div().h(px(320.0)).child(EmbeddingVisualizer::new(
                "space",
                [point(0), point(9)],
                ["Docs"],
            )))
    }
}

#[gpui::test]
fn gone_choices_mark_none_and_overruns_peg(cx: &mut TestAppContext) {
    setup(cx);
    let (_, cx) = cx.add_window_view(|_, _| Stale);
    settle(cx);
}

/// A seed field the owner may swap for another.
struct Seeded(Entity<TextInput>);

impl Render for Seeded {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().w(px(420.0)).child(SeedInput::new("seed", &self.0))
    }
}

#[gpui::test]
fn a_seed_field_fits_whatever_it_held_and_each_new_field(cx: &mut TestAppContext) {
    setup(cx);
    let (view, cx) = cx.add_window_view(|window, cx| {
        let field = cx.new(|cx| TextInput::new(window, cx));
        field.update(cx, |input, cx| {
            input.set_text("4294967296", cx);
            input.insert("7", cx);
        });
        Seeded(field)
    });
    cx.update(|window, _| window.activate_window());
    settle(cx);
    view.read_with(cx, |view, cx| {
        assert_eq!(SeedInput::read(&view.0, cx), None)
    });
    cx.update(|window, cx| {
        let field = view.read(cx).0.clone();
        window.focus(&field.focus_handle(cx), cx);
    });
    cx.simulate_keystrokes("secondary-z");
    settle(cx);
    view.read_with(cx, |view, cx| {
        assert_eq!(
            SeedInput::read(&view.0, cx),
            None,
            "undo reaches no unfit text"
        )
    });
    let field = cx.update(|window, cx| cx.new(|cx| TextInput::new(window, cx)));
    view.update(cx, |view, cx| {
        view.0 = field.clone();
        cx.notify();
    });
    settle(cx);
    cx.update(|window, cx| window.focus(&field.focus_handle(cx), cx));
    cx.simulate_input("12a3");
    settle(cx);
    view.read_with(cx, |view, cx| {
        assert_eq!(SeedInput::read(&view.0, cx), Some(123))
    });
    cx.update(|window, cx| {
        field.update(cx, |input, cx| {
            input.replace_and_mark_text_in_range(None, "ni", None, window, cx)
        })
    });
    settle(cx);
    assert_eq!(
        field.read_with(cx, |input, _| input.text().to_string()),
        "123ni"
    );
    view.read_with(cx, |view, cx| {
        assert_eq!(
            SeedInput::read(&view.0, cx),
            Some(123),
            "a composition is not yet the seed"
        )
    });
    cx.simulate_keystrokes("secondary-z secondary-shift-z");
    settle(cx);
    view.read_with(cx, |view, cx| {
        assert_eq!(
            SeedInput::read(&view.0, cx),
            Some(123),
            "redo brings back what was committed"
        )
    });
    cx.update(|window, cx| {
        field.update(cx, |input, cx| {
            input.replace_and_mark_text_in_range(None, "ni", None, window, cx);
            input.set_text("4294967296", cx);
        })
    });
    settle(cx);
    view.read_with(cx, |view, cx| {
        assert_eq!(
            SeedInput::read(&view.0, cx),
            Some(123),
            "a refused text keeps what was committed"
        )
    });
}

type Heard = Rc<RefCell<Vec<String>>>;

/// Variations whose chosen one may leave, and the actions asked.
struct Varied {
    chosen: usize,
    heard: Heard,
}

impl Render for Varied {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let (vary, upscale) = (self.heard.clone(), self.heard.clone());
        div().w(px(420.0)).child(
            VariationPicker::new("variations", ["a.jpg"], 1.5, self.chosen)
                .on_vary(move |strong, _, _| vary.borrow_mut().push(format!("vary {strong}")))
                .on_upscale(move |_, _| upscale.borrow_mut().push("upscale".into())),
        )
    }
}

#[gpui::test]
fn no_picture_chosen_offers_no_way_on(cx: &mut TestAppContext) {
    setup(cx);
    let heard = Heard::default();
    let kept = heard.clone();
    let (view, cx) = cx.add_window_view(|_, _| Varied {
        chosen: 0,
        heard: kept,
    });
    cx.update(|window, _| window.activate_window());
    settle(cx);
    tab(2, cx);
    press("enter", cx);
    assert_eq!(*heard.borrow(), ["vary false"]);
    heard.borrow_mut().clear();
    view.update(cx, |view, cx| {
        view.chosen = 4;
        cx.notify();
    });
    settle(cx);
    cx.update(|window, cx| window.blur(cx));
    for _ in 0..4 {
        tab(1, cx);
        press("enter", cx);
    }
    assert_eq!(*heard.borrow(), Vec::<String>::new());
}

/// A canvas the owner narrows, keeping every stroke it hands over.
struct Painting {
    width: f32,
    strokes: Vec<MaskStroke>,
}

impl Render for Painting {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let owner = cx.entity();
        div().w(px(self.width)).child(
            InpaintCanvas::new("canvas", "missing.jpg", 2.0, self.strokes.clone()).on_stroke(
                move |stroke, _, cx| {
                    owner.update(cx, |view, cx| {
                        view.strokes.push(stroke);
                        cx.notify();
                    })
                },
            ),
        )
    }
}

#[gpui::test]
fn a_canvas_that_shrinks_under_the_pen_ends_the_stroke_on_it(cx: &mut TestAppContext) {
    setup(cx);
    let (view, cx) = cx.add_window_view(|_, _| Painting {
        width: 400.0,
        strokes: Vec::new(),
    });
    cx.update(|window, _| window.activate_window());
    settle(cx);
    let none = Modifiers::none();
    cx.simulate_mouse_down(point(px(300.0), px(100.0)), MouseButton::Left, none);
    cx.simulate_mouse_move(point(px(320.0), px(100.0)), MouseButton::Left, none);
    cx.simulate_mouse_move(point(px(340.0), px(100.0)), MouseButton::Left, none);
    settle(cx);
    view.update(cx, |view, cx| {
        view.width = 200.0;
        cx.notify();
    });
    settle(cx);
    cx.simulate_mouse_up(point(px(160.0), px(50.0)), MouseButton::Left, none);
    settle(cx);
    let strokes = view.read_with(cx, |view, _| view.strokes.clone());
    assert_eq!(strokes.len(), 1);
    let on = |share: f32| (0.0..=1.0).contains(&share);
    assert!(strokes[0].points.iter().all(|&(x, y)| on(x) && on(y)));
}
