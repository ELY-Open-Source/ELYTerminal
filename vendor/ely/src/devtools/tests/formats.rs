use gpui::{
    AnyElement, App, AppContext as _, Entity, Focusable as _, IntoElement, TestAppContext, Window,
    div,
};

use super::{Bench, bench, settle, tab, tap};
use crate::{
    devtools::{JsonEditor, JsonViewer},
    editor::CodeEditor,
    forms::TextInput,
};

fn viewer(_: &Bench, window: &mut Window, cx: &mut App, _: Entity<Bench>) -> AnyElement {
    let search = window.use_keyed_state("search", cx, TextInput::new);
    JsonViewer::new(
        "viewer",
        r#"{"spec": {"replicas": 3}, "kind": "Deployment"}"#,
        &search,
    )
    .into_any_element()
}

/// Stops: the find field, then the tree, whose cursor starts on the root.
#[gpui::test]
fn enter_on_a_value_copies_its_path(cx: &mut TestAppContext) {
    let (_, cx) = bench(viewer, cx);
    tab(2, cx);
    for key in ["down", "right", "down", "enter"] {
        tap(key, cx);
    }
    let copied = cx
        .update(|_, cx| cx.read_from_clipboard())
        .and_then(|item| item.text());
    assert_eq!(copied.as_deref(), Some("$.spec.replicas"));
}

fn editor(bench: &Bench, _: &mut Window, _: &mut App, _: Entity<Bench>) -> AnyElement {
    match &bench.editor {
        Some(editor) => JsonEditor::new("json", editor).into_any_element(),
        None => div().into_any_element(),
    }
}

/// Stops: Format, then Minify.
#[gpui::test]
fn format_indents_the_json_and_minify_folds_it(cx: &mut TestAppContext) {
    let (host, cx) = bench(editor, cx);
    let editor = cx.update(|window, cx| cx.new(|cx| CodeEditor::new(r#"{"a":[1,2]}"#, window, cx)));
    host.update(cx, |bench, cx| {
        bench.editor = Some(editor.clone());
        cx.notify();
    });
    settle(cx);
    tab(1, cx);
    tap("space", cx);
    assert_eq!(
        editor.read_with(cx, |editor, _| editor.text().to_string()),
        "{\n  \"a\": [\n    1,\n    2\n  ]\n}"
    );
    tab(2, cx);
    tap("space", cx);
    assert_eq!(
        editor.read_with(cx, |editor, _| editor.text().to_string()),
        r#"{"a":[1,2]}"#
    );
}

/// Stops: the editor alone, the buttons resting while the JSON does not read.
#[gpui::test]
fn format_and_minify_rest_while_the_json_does_not_read(cx: &mut TestAppContext) {
    let (host, cx) = bench(editor, cx);
    let editor = cx.update(|window, cx| cx.new(|cx| CodeEditor::new("{", window, cx)));
    host.update(cx, |bench, cx| {
        bench.editor = Some(editor.clone());
        cx.notify();
    });
    settle(cx);
    tab(1, cx);
    assert!(
        cx.update(|window, cx| editor.focus_handle(cx).is_focused(window)),
        "Tab passes Format and Minify"
    );
}
