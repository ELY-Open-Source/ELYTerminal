use gpui::{Entity, EntityInputHandler, TestAppContext};

use super::editor;
use crate::{
    editor::{CodeEditor, GhostText, InlayHint},
    theme::Theme,
};

#[gpui::test]
fn ranges_from_an_older_text_are_left(cx: &mut TestAppContext) {
    let (editor, cx) = editor("héllo", cx);
    editor.update(cx, |editor, cx| {
        let caret = 1..1;
        editor.select([caret], cx);
        editor.select([0..2, 0..99], cx);
        assert_eq!(
            editor.primary().head,
            1,
            "a stale selection leaves the last"
        );
        editor.edit([(0..2, "x".to_string())], cx);
        editor.edit([(0..3, "x".to_string()), (1..4, "y".to_string())], cx);
        editor.edit(
            [(std::ops::Range { start: 3, end: 1 }, "x".to_string())],
            cx,
        );
        assert_eq!(
            editor.text(),
            "héllo",
            "a stale, crossing or backward batch makes no edit"
        );
        editor.edit([(0..1, "j".to_string())], cx);
        assert_eq!(editor.text(), "jéllo");
    });
}

/// An output panel whose shown channel left its list.
struct Output;

impl gpui::Render for Output {
    fn render(
        &mut self,
        _: &mut gpui::Window,
        _: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        crate::editor::OutputPanel::new("output", ["Build", "Tests"], "Lint", ["one line"])
    }
}

#[gpui::test]
fn an_output_channel_gone_from_its_list_draws(cx: &mut TestAppContext) {
    cx.update(Theme::init);
    let (_, cx) = cx.add_window_view(|_, _| Output);
    cx.run_until_parked();
}

#[test]
fn a_glob_too_big_for_a_regex_matches_nothing() {
    let huge = "?".repeat(500_000);
    assert!(!crate::editor::passes("src/a.rs", &huge, ""));
    assert!(crate::editor::passes("src/a.rs", "", &huge));
}

#[gpui::test]
fn new_text_drops_what_named_the_old(cx: &mut TestAppContext) {
    let (editor, cx) = editor("abcd", cx);
    editor.update_in(cx, |editor, window, cx| {
        let end = 4..4;
        editor.select([end], cx);
        editor.replace_and_mark_text_in_range(None, "ni", None, window, cx);
        editor.set_inlay_hints(
            vec![InlayHint {
                offset: 1,
                text: ": i32".into(),
            }],
            cx,
        );
        editor.set_backgrounds(vec![(1..2, gpui::black())], cx);
        editor.set_text("é", cx);
        editor.replace_text_in_range(None, "你", window, cx);
        editor.set_ghost_text(
            Some(GhostText {
                offset: 4,
                text: "late".into(),
            }),
            cx,
        );
        editor.indent(cx);
    });
    cx.run_until_parked();
    assert!(editor.read_with(cx, |editor, _| editor.marks.hints.is_empty()));
}

#[gpui::test]
fn a_diagnostic_rides_the_typing_before_it(cx: &mut TestAppContext) {
    let (editor, cx) = editor("ab", cx);
    editor.update(cx, |editor, cx| {
        let note = |range| crate::editor::Diagnostic {
            range,
            severity: crate::primitives::Severity::Danger,
            message: "note".into(),
        };
        editor.set_diagnostics(vec![note(1..2)], cx);
        let start = 0..0;
        editor.select([start], cx);
        editor.type_text("é", cx);
        assert_eq!(editor.marks.diagnostics[0].range, 3..4);
        let whole = 0..4;
        editor.select([whole], cx);
        editor.type_text("x", cx);
        assert!(
            editor.marks.diagnostics.is_empty(),
            "an edit over it takes it"
        );
    });
    cx.run_until_parked();
}

/// A sticky editor in a box short enough to scroll.
struct Boxed(Entity<CodeEditor>);

impl gpui::Render for Boxed {
    fn render(
        &mut self,
        _: &mut gpui::Window,
        _: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        use gpui::{ParentElement as _, Styled as _};
        gpui::div()
            .w(gpui::px(480.0))
            .h(gpui::px(240.0))
            .child(self.0.clone())
    }
}

#[gpui::test]
fn a_scrolled_sticky_editor_takes_a_shorter_text(cx: &mut TestAppContext) {
    use gpui::AppContext as _;
    cx.update(Theme::init);
    let text = format!("outer\n  inner\n{}\nend", vec!["    child"; 100].join("\n"));
    let (view, cx) = cx.add_window_view(move |window, cx| {
        Boxed(cx.new(|cx| CodeEditor::new(text, window, cx).sticky_scroll()))
    });
    let editor = view.read_with(cx, |root, _| root.0.clone());
    cx.run_until_parked();
    editor.update(cx, |editor, cx| {
        let at = editor.buffer.offset(50, 0);
        let caret = at..at;
        editor.select([caret], cx);
    });
    for _ in 0..3 {
        cx.run_until_parked();
        cx.update(|window, _| window.refresh());
    }
    let offset = editor.read_with(cx, |editor, _| {
        editor.scroll.0.borrow().base_handle.offset()
    });
    assert!(
        offset.y < gpui::Pixels::ZERO,
        "the editor scrolled: {offset:?}"
    );
    editor.update(cx, |editor, cx| editor.set_text("x", cx));
    cx.run_until_parked();
}

#[gpui::test]
fn a_point_on_a_row_the_text_lost_has_no_offset(cx: &mut TestAppContext) {
    let (editor, cx) = editor("first\nsecond\nthird", cx);
    cx.run_until_parked();
    editor.update_in(cx, |editor, window, cx| {
        let position = gpui::point(
            editor.metrics.left,
            editor.metrics.top + editor.metrics.line * 1.5,
        );
        assert_eq!(
            editor.character_index_for_point(position, window, cx),
            Some(6)
        );
        editor.set_text("x", cx);
        assert_eq!(editor.character_index_for_point(position, window, cx), None);
    });
    cx.run_until_parked();
}

/// A press at a window point, sent as the platform would.
fn press(position: gpui::Point<gpui::Pixels>) -> gpui::PlatformInput {
    gpui::PlatformInput::MouseDown(gpui::MouseDownEvent {
        position,
        button: gpui::MouseButton::Left,
        modifiers: gpui::Modifiers::none(),
        click_count: 1,
        first_mouse: false,
    })
}

#[gpui::test]
fn a_press_or_drag_on_a_row_the_text_lost_is_dropped(cx: &mut TestAppContext) {
    let (editor, cx) = editor("first\nsecond\nthird", cx);
    cx.run_until_parked();
    let position = editor.read_with(cx, |editor, _| {
        gpui::point(
            editor.metrics.left + editor.metrics.advance,
            editor.metrics.top + editor.metrics.line * 1.5,
        )
    });
    cx.simulate_mouse_down(position, gpui::MouseButton::Left, gpui::Modifiers::none());
    assert_eq!(editor.read_with(cx, |editor, _| editor.position().0), 2);
    cx.update(|window, cx| {
        editor.update(cx, |editor, cx| editor.set_text("x", cx));
        let held = gpui::PlatformInput::MouseMove(gpui::MouseMoveEvent {
            position,
            pressed_button: Some(gpui::MouseButton::Left),
            modifiers: gpui::Modifiers::none(),
        });
        window.dispatch_event(held, cx);
        window.dispatch_event(press(position), cx);
    });
    cx.run_until_parked();
    assert_eq!(editor.read_with(cx, |editor, _| editor.position().0), 1);
}

#[gpui::test]
fn a_sticky_press_on_a_line_the_text_lost_is_dropped(cx: &mut TestAppContext) {
    use gpui::AppContext as _;
    cx.update(Theme::init);
    let text = format!("outer\n  inner\n{}\nend", vec!["    child"; 100].join("\n"));
    let (view, cx) = cx.add_window_view(move |window, cx| {
        Boxed(cx.new(|cx| CodeEditor::new(text, window, cx).sticky_scroll()))
    });
    let editor = view.read_with(cx, |root, _| root.0.clone());
    cx.run_until_parked();
    editor.update(cx, |editor, cx| {
        let at = editor.buffer.offset(50, 0);
        let caret = at..at;
        editor.select([caret], cx);
    });
    for _ in 0..3 {
        cx.run_until_parked();
        cx.update(|window, _| window.refresh());
    }
    let position = editor.read_with(cx, |editor, _| {
        gpui::point(
            gpui::px(200.0),
            editor.metrics.top + editor.metrics.line * 1.5,
        )
    });
    cx.update(|window, cx| {
        editor.update(cx, |editor, cx| editor.set_text("x", cx));
        window.dispatch_event(press(position), cx);
    });
    cx.run_until_parked();
}
