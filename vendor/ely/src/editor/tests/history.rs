use gpui::{EntityInputHandler, TestAppContext};

use super::editor;

/// Bytes of text all undo and redo steps hold.
fn held(editor: &super::CodeEditor) -> usize {
    editor.history.held(|step| step.text.len())
}

#[gpui::test]
fn compositions_keep_undo_within_its_budget(cx: &mut TestAppContext) {
    let (editor, cx) = editor(&"x".repeat(100), cx);
    editor.update_in(cx, |editor, window, cx| {
        editor.history_bytes = 250;
        for word in ["a", "b", "c", "d"] {
            editor.replace_and_mark_text_in_range(None, word, None, window, cx);
            editor.unmark_text(window, cx);
        }
        assert!(held(editor) <= 250, "{} bytes held", held(editor));
        assert!(editor.history.can_undo(), "the nearest steps stay");
    });
}

#[gpui::test]
fn undo_keeps_redo_within_its_budget(cx: &mut TestAppContext) {
    let (editor, cx) = editor("small", cx);
    editor.update(cx, |editor, cx| {
        editor.history_bytes = 50;
        editor.edit([(0..5, "y".repeat(80))], cx);
        editor.undo(cx);
        assert_eq!(editor.text(), "small");
        assert!(!editor.history.can_redo(), "a redo past the budget went");
    });
}
