use std::{cell::RefCell, ffi::OsStr, os::unix::ffi::OsStrExt, path::PathBuf, rc::Rc};

use gpui::{
    AppContext as _, Context, Entity, IntoElement, KeyUpEvent, Keystroke, Render, TestAppContext,
    VisualTestContext, Window,
};

use super::setup;
use crate::forms::{PathInput, TextInput};

/// A folder field, and the paths its `on_choose` was handed.
struct Picker {
    field: Entity<TextInput>,
    handed: Option<Rc<RefCell<Vec<PathBuf>>>>,
}

impl Render for Picker {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let path = PathInput::new("path", &self.field).directories();
        match self.handed.clone() {
            Some(handed) => path.on_choose(move |path, _, _| handed.borrow_mut().push(path)),
            None => path,
        }
    }
}

/// Presses Browse and answers its dialog with `chosen`.
fn browse(chosen: PathBuf, cx: &mut VisualTestContext) {
    cx.update(|window, cx| {
        window.focus_next(cx);
        window.focus_next(cx);
    });
    cx.simulate_keystrokes("enter");
    cx.simulate_event(KeyUpEvent {
        keystroke: Keystroke::parse("enter").expect("a key"),
    });
    cx.run_until_parked();
    cx.simulate_path_prompt_response(move |_| Some(vec![chosen]));
    cx.run_until_parked();
}

fn text(field: &Entity<TextInput>, cx: &mut VisualTestContext) -> String {
    field.read_with(cx, |input, _| input.text().to_string())
}

#[gpui::test]
fn a_path_not_in_utf_8_never_lands_in_the_field_as_another_s_name(cx: &mut TestAppContext) {
    setup(cx);
    let raw = PathBuf::from(OsStr::from_bytes(b"/tmp/chosen-\xff"));
    let (picker, cx) = cx.add_window_view(|window, cx| Picker {
        field: cx.new(|cx| TextInput::new(window, cx)),
        handed: None,
    });
    let field = picker.read_with(cx, |picker, _| picker.field.clone());
    browse(raw.clone(), cx);
    assert_eq!(text(&field, cx), "", "the field keeps its text");
    browse(PathBuf::from("/tmp/chosen"), cx);
    assert_eq!(text(&field, cx), "/tmp/chosen");
    let handed = Rc::new(RefCell::new(Vec::new()));
    picker.update(cx, |picker, cx| {
        picker.handed = Some(handed.clone());
        cx.notify();
    });
    cx.run_until_parked();
    browse(raw.clone(), cx);
    assert_eq!(*handed.borrow(), [raw], "on_choose takes it whole");
    assert_eq!(text(&field, cx), "/tmp/chosen", "and the field is its own");
}
