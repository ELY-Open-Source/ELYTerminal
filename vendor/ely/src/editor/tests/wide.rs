//! A wide character takes its laid-out width, not one column.

use gpui::{Bounds, EntityInputHandler, TestAppContext, VisualTestContext};

use super::editor;

#[gpui::test]
fn a_wide_character_moves_the_caret_by_its_width(cx: &mut TestAppContext) {
    // Test text draws "😄" two ems wide, "a" one.
    let (editor, cx) = editor("😄a", cx);
    cx.run_until_parked();
    let x = |utf16: usize, cx: &mut VisualTestContext| {
        editor.update_in(cx, |editor, window, cx| {
            let bounds = editor.bounds_for_range(utf16..utf16, Bounds::default(), window, cx);
            bounds.expect("a drawn line").origin.x
        })
    };
    let (start, wide, narrow) = (x(0, cx), x(2, cx), x(3, cx));
    let off = (wide - start) - (narrow - wide) * 2.0;
    assert!(off.abs() < gpui::px(0.01), "{start:?} {wide:?} {narrow:?}");
    let at = editor.update_in(cx, |editor, window, cx| {
        let left = editor.metrics.left;
        editor.offset_in(0, left + (wide - start) * 0.9, window, cx)
    });
    assert_eq!(at, Some("😄".len()), "a press near its end lands after it");
}

#[gpui::test]
fn a_press_on_the_last_character_takes_its_nearer_edge(cx: &mut TestAppContext) {
    for (text, last) in [("ab", "a".len()), ("测", 0)] {
        let (editor, cx) = editor(text, cx);
        cx.run_until_parked();
        let at = |share: f32, cx: &mut VisualTestContext| {
            editor.update_in(cx, |editor, window, cx| {
                let mut x = |utf16: usize| {
                    let bounds =
                        editor.bounds_for_range(utf16..utf16, Bounds::default(), window, cx);
                    bounds.expect("a drawn line").origin.x
                };
                let (start, from, to) = (x(0), x(last), x(text.encode_utf16().count()));
                let left = editor.metrics.left;
                editor.offset_in(0, left + from - start + (to - from) * share, window, cx)
            })
        };
        assert_eq!(at(0.1, cx), Some(last), "{text}: near its start");
        assert_eq!(at(0.9, cx), Some(text.len()), "{text}: near its end");
    }
}

#[gpui::test]
fn an_input_method_edit_measures_before_the_next_draw(cx: &mut TestAppContext) {
    let (editor, cx) = editor("(a)", cx);
    cx.run_until_parked();
    editor.update_in(cx, |editor, window, cx| {
        let start = 0..0;
        editor.select([start], cx);
        editor.replace_and_mark_text_in_range(None, "测", None, window, cx);
        let text = editor.row_text(0).to_string();
        for (range, _) in editor.styles(0, cx) {
            let whole = text.is_char_boundary(range.start) && text.is_char_boundary(range.end);
            assert!(whole, "{range:?} splits a character of {text}");
        }
        let bounds = editor.bounds_for_range(1..1, Bounds::default(), window, cx);
        assert!(bounds.is_some(), "the caret after it is measured");
    });
}
