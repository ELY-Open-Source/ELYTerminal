use gpui::{AppContext as _, Focusable as _};

use super::{MarkdownEditor, MarkdownMode, TextInput, Visual, commit, joint, open, shifted};

#[cfg(feature = "test-support")]
#[gpui::test]
fn an_open_block_of_a_replaced_text_writes_nothing(cx: &mut gpui::TestAppContext) {
    cx.update(crate::theme::Theme::init);
    cx.add_empty_window().update(|window, cx| {
        let field = cx.new(|cx| TextInput::new(window, cx).multi_line(1, usize::MAX));
        field.update(cx, |field, cx| {
            field.set_text(
                "one

two", cx,
            )
        });
        let state = cx.new(|_| Visual::default());
        open(&state, &field, 0..3, false, true, window, cx);
        field.update(cx, |field, cx| field.set_text("three", cx));
        assert_eq!(commit(&state, &field, cx), None);
        assert_eq!(field.read(cx).text(), "three");
    });
}

#[test]
fn later_blocks_move_by_what_an_edit_wrote() {
    assert_eq!(shifted(10..14, &(2..6), 9), 15..19);
    assert_eq!(
        shifted(0..2, &(2..6), 9),
        0..2,
        "a block before the edit stays"
    );
}

#[test]
fn a_block_at_the_end_starts_a_paragraph() {
    assert_eq!(
        (joint(""), joint("a"), joint("a\n"), joint("a\n\n")),
        ("", "\n\n", "\n", "")
    );
}

#[cfg(feature = "test-support")]
struct Visualized(gpui::Entity<TextInput>);

#[cfg(feature = "test-support")]
impl gpui::Render for Visualized {
    fn render(
        &mut self,
        _: &mut gpui::Window,
        _: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        use gpui::{ParentElement, Styled};
        gpui::div()
            .size_full()
            .child(MarkdownEditor::new("visual", &self.0, MarkdownMode::Visual))
    }
}

#[cfg(feature = "test-support")]
#[gpui::test]
fn a_press_on_a_block_of_a_replaced_text_opens_nothing(cx: &mut gpui::TestAppContext) {
    use crate::theme::{ActiveTheme, ControlSize};
    cx.update(crate::theme::Theme::init);
    let (view, cx) = cx.add_window_view(|window, cx| {
        Visualized(cx.new(|cx| {
            let mut field = TextInput::new(window, cx).multi_line(1, usize::MAX);
            field.set_text("A long paragraph to edit.", cx);
            field
        }))
    });
    cx.update(|window, _| window.activate_window());
    cx.run_until_parked();
    let field = view.read_with(cx, |view, _| view.0.clone());
    let at = cx.update(|window, cx| {
        let bar = cx.theme().control_height(ControlSize::Lg);
        gpui::point(
            gpui::px(40.0),
            bar.to_pixels(window.rem_size()) + gpui::px(20.0),
        )
    });
    let source = |cx: &mut gpui::VisualTestContext| {
        cx.update(|window, cx| {
            window
                .focused(cx)
                .is_some_and(|f| f != field.focus_handle(cx))
        })
    };
    cx.simulate_mouse_down(at, gpui::MouseButton::Left, gpui::Modifiers::none());
    cx.run_until_parked();
    assert!(source(cx), "a press opens the block as its source");
    cx.update(|window, cx| window.focus(&field.focus_handle(cx), cx));
    cx.run_until_parked();
    cx.update(|window, cx| {
        field.update(cx, |field, cx| field.set_text("x", cx));
        let press = gpui::MouseDownEvent {
            position: at,
            button: gpui::MouseButton::Left,
            modifiers: gpui::Modifiers::none(),
            click_count: 1,
            first_mouse: false,
        };
        window.dispatch_event(gpui::PlatformInput::MouseDown(press), cx);
    });
    cx.run_until_parked();
    assert!(!source(cx), "a press on a replaced block opens nothing");
    assert_eq!(
        field.read_with(cx, |field, _| field.text().to_string()),
        "x"
    );
}

#[cfg(feature = "test-support")]
struct Previewed(gpui::Entity<TextInput>);

#[cfg(feature = "test-support")]
impl gpui::Render for Previewed {
    fn render(
        &mut self,
        _: &mut gpui::Window,
        _: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        MarkdownEditor::new("preview", &self.0, MarkdownMode::Preview)
    }
}

#[cfg(feature = "test-support")]
#[gpui::test]
fn preview_draws_the_document_and_takes_no_press(cx: &mut gpui::TestAppContext) {
    cx.update(crate::init_for_tests);
    let (view, cx) = cx.add_window_view(|window, cx| {
        Previewed(cx.new(|cx| {
            let mut field = TextInput::new(window, cx).multi_line(1, usize::MAX);
            field.set_text("# Lift", cx);
            field
        }))
    });
    cx.run_until_parked();
    let field = view.read_with(cx, |view, _| view.0.clone());
    cx.simulate_click(
        gpui::point(gpui::px(20.0), gpui::px(60.0)),
        gpui::Modifiers::none(),
    );
    assert_eq!(
        field.read_with(cx, |field, _| field.text().to_string()),
        "# Lift"
    );
    assert!(!cx.update(|window, cx| field.focus_handle(cx).is_focused(window)));
}
