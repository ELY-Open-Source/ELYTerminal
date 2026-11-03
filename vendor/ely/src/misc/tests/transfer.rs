use std::cell::{Cell, RefCell};

use gpui::{
    AnyElement, App, Context, FocusHandle, IntoElement, Modifiers, ParentElement, Render, Styled,
    TestAppContext, VisualTestContext, Window, div, point, px,
};

use super::{press, settle, shown, stage};
use crate::misc::{CsvImporter, ExportDialog, ExportFormat, ImportDialog, ImportField};
use crate::{buttons::Button, primitives::FocusScope, theme::Theme};

thread_local! {
    static SAID: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    static OPEN: Cell<bool> = const { Cell::new(true) };
    static TEXT: RefCell<&'static str> = const { RefCell::new("") };
}

fn said() -> Vec<String> {
    SAID.with(|said| said.borrow().clone())
}

/// Closes the dialog and notes the answer.
fn say(text: impl Into<String>) {
    OPEN.set(false);
    SAID.with(|said| said.borrow_mut().push(text.into()));
}

fn export() -> AnyElement {
    match OPEN.get() {
        true => ExportDialog::new(
            "export",
            "Export",
            [ExportFormat::Pdf, ExportFormat::Png, ExportFormat::Csv],
            |format, _, _| say(format!("{format:?}")),
            |_, _| say("cancelled"),
        )
        .into_any_element(),
        false => div().into_any_element(),
    }
}

/// A card picks the format and Export hands it over, once.
#[gpui::test]
fn export_hands_over_the_chosen_format(cx: &mut TestAppContext) {
    let (_, cx) = stage(export, cx);
    press("tab", cx);
    press("tab", cx);
    press("space", cx);
    for _ in 0..3 {
        press("tab", cx);
    }
    press("enter", cx);
    assert_eq!(said(), ["Png"]);
}

/// Escape cancels, once.
#[gpui::test]
fn escape_cancels_the_export(cx: &mut TestAppContext) {
    let (_, cx) = stage(export, cx);
    press("tab", cx);
    press("escape", cx);
    assert_eq!(said(), ["cancelled"]);
}

fn import() -> AnyElement {
    match OPEN.get() {
        true => ImportDialog::new(
            "import",
            "Import people",
            ["Mail", "Who"],
            [
                ImportField::new("name").required(),
                ImportField::new("email"),
            ],
            |mapping, _, _| say(format!("{mapping:?}")),
            |_, _| say("cancelled"),
        )
        .preview([vec!["ada@x.io".into(), "Ada".into()]])
        .into_any_element(),
        false => div().into_any_element(),
    }
}

/// Import asks for each required field's column, then hands the mapping over.
#[gpui::test]
fn import_asks_for_required_columns(cx: &mut TestAppContext) {
    let (_, cx) = stage(import, cx);
    assert!(shown("import-preview", cx));
    for _ in 0..4 {
        press("tab", cx);
    }
    press("enter", cx);
    assert!(said().is_empty() && shown("import-missing-name", cx));
    for _ in 0..3 {
        press("shift-tab", cx);
    }
    for key in ["down", "down", "enter"] {
        press(key, cx);
    }
    for _ in 0..3 {
        press("tab", cx);
    }
    press("enter", cx);
    assert_eq!(said(), ["[Some(1), None]"]);
}

fn csv() -> AnyElement {
    match OPEN.get() {
        true => CsvImporter::new(
            "csv",
            "Import people",
            TEXT.with(|text| *text.borrow()),
            [
                ImportField::new("name").required(),
                ImportField::new("email"),
            ],
            |records, _, _| say(format!("{records:?}")),
            |_, _| say("cancelled"),
        )
        .into_any_element(),
        false => div().into_any_element(),
    }
}

/// Columns named by the first row match the fields, and Import hands over a record per row.
#[gpui::test]
fn a_csv_file_imports_as_records(cx: &mut TestAppContext) {
    TEXT.with(|text| text.replace("Name,Email\nAda,ada@x.io\n\"Lin, Wei\",lin@x.io\n"));
    let (_, cx) = stage(csv, cx);
    for _ in 0..5 {
        press("tab", cx);
    }
    press("enter", cx);
    assert_eq!(
        said(),
        [r#"[["Ada", "ada@x.io"], ["Lin, Wei", "lin@x.io"]]"#]
    );
}

/// Unticked, the first row is data, its columns go by number, and the required field waits for one.
#[gpui::test]
fn the_first_row_can_be_data(cx: &mut TestAppContext) {
    TEXT.with(|text| text.replace("Name,Email\nAda,ada@x.io\n"));
    let (_, cx) = stage(csv, cx);
    press("tab", cx);
    press("space", cx);
    for _ in 0..4 {
        press("tab", cx);
    }
    press("enter", cx);
    assert!(said().is_empty() && shown("import-missing-name", cx));
}

/// Text that does not read as CSV says why, and its Cancel cancels.
#[gpui::test]
fn broken_csv_says_why(cx: &mut TestAppContext) {
    TEXT.with(|text| text.replace("a,\"b"));
    let (_, cx) = stage(csv, cx);
    assert!(shown("csv-unread", cx));
    press("tab", cx);
    press("enter", cx);
    assert_eq!(said(), ["cancelled"], "its Cancel cancels");
}

/// Text with no rows says so.
#[gpui::test]
fn empty_csv_says_why(cx: &mut TestAppContext) {
    TEXT.with(|text| text.replace(""));
    let (_, cx) = stage(csv, cx);
    assert!(shown("csv-unread", cx));
}

/// Shows the dialog again after an answer closed it.
fn reopen(cx: &mut VisualTestContext) {
    OPEN.set(true);
    cx.update(|window, _| window.refresh());
    settle(cx);
}

/// Cancel and a press on the scrim cancel each dialog once; with nothing picked, Export hands over the first format.
#[gpui::test]
fn every_way_out_answers_once(cx: &mut TestAppContext) {
    let (_, cx) = stage(export, cx);
    for _ in 0..4 {
        press("tab", cx);
    }
    press("enter", cx);
    reopen(cx);
    cx.simulate_click(point(px(3.0), px(3.0)), Modifiers::none());
    settle(cx);
    reopen(cx);
    for _ in 0..5 {
        press("tab", cx);
    }
    press("enter", cx);
    assert_eq!(said(), ["cancelled", "cancelled", "Pdf"]);
}

/// The import dialog's Cancel and scrim cancel once each.
#[gpui::test]
fn import_cancels_once(cx: &mut TestAppContext) {
    let (_, cx) = stage(import, cx);
    for _ in 0..3 {
        press("tab", cx);
    }
    press("enter", cx);
    reopen(cx);
    cx.simulate_click(point(px(3.0), px(3.0)), Modifiers::none());
    settle(cx);
    assert_eq!(said(), ["cancelled", "cancelled"]);
}

/// New text under an open importer reads again and imports, a field with no column coming empty.
#[gpui::test]
fn new_text_reads_again(cx: &mut TestAppContext) {
    TEXT.with(|text| text.replace("a,\"b"));
    let (_, cx) = stage(csv, cx);
    assert!(shown("csv-unread", cx));
    TEXT.with(|text| text.replace("Name,Team\nAda,Engines\n"));
    cx.update(|window, _| window.refresh());
    settle(cx);
    for _ in 0..5 {
        press("tab", cx);
    }
    press("enter", cx);
    assert_eq!(said(), [r#"[["Ada", ""]]"#]);
}

/// A page with a button that opens a dialog, and the dialog while it is open.
struct Opener {
    root: FocusHandle,
    button: FocusHandle,
    part: fn() -> AnyElement,
}

impl Render for Opener {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        FocusScope::new(&self.root)
            .root()
            .size_full()
            .child(
                Button::new("open", "Open")
                    .focus_handle(&self.button)
                    .on_click(move |_, _, cx| {
                        OPEN.set(true);
                        view.update(cx, |_, cx| cx.notify());
                    }),
            )
            .child(div().child((self.part)()))
    }
}

/// Opens `part` from a button, presses Tab `tabs` times and Enter, and says whether focus came back to the button.
fn answered_from_button(part: fn() -> AnyElement, tabs: usize, cx: &mut TestAppContext) -> bool {
    OPEN.set(false);
    cx.update(|cx: &mut App| {
        Theme::init(cx);
        crate::forms::bind_keys(cx);
        cx.bind_keys([
            gpui::KeyBinding::new("tab", crate::primitives::FocusNext, None),
            gpui::KeyBinding::new("shift-tab", crate::primitives::FocusPrev, None),
        ]);
    });
    let (view, cx) = cx.add_window_view(|_, cx| Opener {
        root: cx.focus_handle(),
        button: cx.focus_handle(),
        part,
    });
    let button = view.read_with(cx, |opener, _| opener.button.clone());
    cx.update(|window, cx| window.focus(&button, cx));
    settle(cx);
    press("enter", cx);
    for _ in 0..tabs {
        press("tab", cx);
    }
    press("enter", cx);
    cx.update(|window, _| button.is_focused(window))
}

/// Export hands focus back to the button that opened it.
#[gpui::test]
fn export_hands_focus_back(cx: &mut TestAppContext) {
    assert!(answered_from_button(export, 5, cx));
    assert_eq!(said(), ["Pdf"]);
}

/// Import hands focus back to the button that opened it.
#[gpui::test]
fn import_hands_focus_back(cx: &mut TestAppContext) {
    TEXT.with(|text| text.replace("Name,Email\nAda,ada@x.io\n"));
    assert!(answered_from_button(csv, 5, cx));
    assert_eq!(said(), [r#"[["Ada", "ada@x.io"]]"#]);
}
