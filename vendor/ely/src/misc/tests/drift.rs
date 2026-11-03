use gpui::{
    AnyElement, Context, IntoElement, ParentElement, Render, Styled, TestAppContext, Window, div,
    px,
};

use super::{press, settle, shown, stage};
use crate::{
    misc::{
        CookieBanner, ExportDialog, ExportFormat, Flashcards, ImportDialog, ImportField, Poll,
        Question, Quiz, QuizQuestion, Survey,
    },
    theme::Theme,
};

/// Parts whose lists change once `changed` turns: kinds, formats, fields, questions and a vote.
struct Drifting {
    changed: bool,
}

impl Drifting {
    fn parts(&self) -> Vec<AnyElement> {
        let changed = self.changed;
        let mut cookies =
            CookieBanner::new("cookies", "This page would set cookies.", |_, _, _| {})
                .kind("Analytics", "Counts visits.")
                .kind("Marketing", "Shows offers.");
        if changed {
            cookies = cookies.kind("Support", "Remembers chats.");
        }
        let formats = match changed {
            false => vec![ExportFormat::Pdf, ExportFormat::Png],
            true => vec![ExportFormat::Json],
        };
        let mut fields = vec![ImportField::new("name").required()];
        if changed {
            fields.push(ImportField::new("email"));
        }
        let first = match changed {
            false => Question::Choice("How often?".into(), vec!["Daily".into(), "Weekly".into()]),
            true => Question::Text("Anything else?".into()),
        };
        let poll = Poll::new("poll", "Lunch?", ["Soup", "Tea"], |_, _, _| {}).votes([1, 2]);
        vec![
            cookies.into_any_element(),
            ExportDialog::new("export", "Export", formats, |_, _, _| {}, |_, _| {})
                .into_any_element(),
            ImportDialog::new(
                "import",
                "Import",
                ["Who", "Mail"],
                fields,
                |_, _, _| {},
                |_, _| {},
            )
            .into_any_element(),
            Survey::new("survey", [first], |_, _, _| {}).into_any_element(),
            match changed {
                false => poll.into_any_element(),
                true => poll.voted(9).into_any_element(),
            },
        ]
    }
}

impl Render for Drifting {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().w(px(400.0)).children(self.parts())
    }
}

#[gpui::test]
fn parts_whose_lists_changed_fit_their_state_again(cx: &mut TestAppContext) {
    cx.update(Theme::init);
    let (view, cx) = cx.add_window_view(|_, _| Drifting { changed: false });
    settle(cx);
    view.update(cx, |view, cx| {
        view.changed = true;
        cx.notify();
    });
    settle(cx);
}

thread_local! {
    /// Whether the staged quiz and deck show their shorter lists.
    static SHORTER: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

fn quiz() -> AnyElement {
    let one = || QuizQuestion::new("One?", ["A", "B"], 0);
    let questions = match SHORTER.get() {
        false => vec![one(), QuizQuestion::new("Two?", ["C", "D"], 0)],
        true => vec![one()],
    };
    Quiz::new("quiz", questions).into_any_element()
}

#[gpui::test]
fn a_quiz_past_its_questions_starts_over(cx: &mut TestAppContext) {
    SHORTER.set(false);
    let (_, cx) = stage(quiz, cx);
    for key in ["tab", "down", "tab", "enter", "enter"] {
        press(key, cx);
    }
    SHORTER.set(true);
    settle(cx);
}

fn deck() -> AnyElement {
    let cards = match SHORTER.get() {
        false => vec![("A", "a"), ("B", "b"), ("C", "c")],
        true => vec![("A", "a")],
    };
    Flashcards::new("cards", cards).into_any_element()
}

#[gpui::test]
fn a_deck_past_its_cards_starts_over(cx: &mut TestAppContext) {
    SHORTER.set(false);
    let (_, cx) = stage(deck, cx);
    for key in ["tab", "tab", "tab", "enter", "enter"] {
        press(key, cx);
    }
    assert!(shown("flashcard-front-C", cx));
    SHORTER.set(true);
    settle(cx);
    assert!(shown("flashcard-front-A", cx));
}
