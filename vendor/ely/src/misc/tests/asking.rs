use std::cell::{Cell, RefCell};

use gpui::{AnyElement, IntoElement, Modifiers, TestAppContext, VisualTestContext, point, px};

use super::{at_root, press, settle, shown, stage};
use crate::misc::{Answer, Flashcards, Poll, Question, Quiz, QuizQuestion, Survey};

thread_local! {
    static VOTED: Cell<Option<usize>> = const { Cell::new(None) };
    static SAID: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

fn said() -> Vec<String> {
    SAID.with(|said| said.borrow().clone())
}

fn say(text: String) {
    SAID.with(|said| said.borrow_mut().push(text));
}

/// Presses just inside the corner of what `selector` marks, where its button starts, as a pointer does.
fn click(selector: &'static str, cx: &mut VisualTestContext) {
    let corner = cx.debug_bounds(selector).expect("a marked element").origin;
    cx.simulate_click(corner + point(px(4.0), px(4.0)), Modifiers::none());
    settle(cx);
}

fn show(cx: &mut VisualTestContext) {
    cx.update(|window, _| window.refresh());
    settle(cx);
}

fn poll() -> AnyElement {
    let voted = VOTED.get();
    let poll = Poll::new("poll", "Lunch?", ["Soup", "Tea", "Toast"], |vote, _, _| {
        say(format!("{vote:?}"))
    })
    .votes([1, u64::from(voted == Some(1)) + 2, 1]);
    match voted {
        Some(vote) => poll.voted(vote),
        None => poll,
    }
    .into_any_element()
}

/// Arrows pick, Vote hands it over; the results show each share, and Change vote takes it back on the same stop.
#[gpui::test]
fn a_poll_votes_shows_results_and_takes_the_vote_back(cx: &mut TestAppContext) {
    let (_, cx) = stage(poll, cx);
    press("tab", cx);
    press("down", cx);
    press("tab", cx);
    press("enter", cx);
    assert_eq!(said(), ["Some(1)"]);
    VOTED.set(Some(1));
    show(cx);
    assert!(shown("poll-Tea-60%-voted", cx) && shown("poll-Soup-20%", cx));
    press("enter", cx);
    assert_eq!(said(), ["Some(1)", "None"], "Change vote held Vote's focus");
    VOTED.set(None);
    show(cx);
    press("enter", cx);
    assert_eq!(
        said().last().map(String::as_str),
        Some("Some(1)"),
        "the pick stood"
    );
    assert!(!at_root(cx));
}

/// With no pick, Vote is no stop: Shift-Tab from the radios comes round to them.
#[gpui::test]
fn a_poll_rests_until_picked(cx: &mut TestAppContext) {
    let (_, cx) = stage(poll, cx);
    press("tab", cx);
    press("shift-tab", cx);
    press("enter", cx);
    assert!(said().is_empty());
}

/// A vote the owner holds seeds the pick, and a pointer's Vote hands focus to the button the results keep.
#[gpui::test]
fn a_poll_starts_from_the_owners_vote(cx: &mut TestAppContext) {
    VOTED.set(Some(2));
    let (_, cx) = stage(poll, cx);
    press("tab", cx);
    press("enter", cx);
    VOTED.set(None);
    show(cx);
    press("shift-tab", cx);
    press("down", cx);
    click("poll-action", cx);
    assert_eq!(said(), ["None", "Some(0)"], "the pick started on Toast");
    VOTED.set(Some(0));
    show(cx);
    assert!(
        !at_root(cx),
        "the radios left, and focus went to Change vote"
    );
}

fn survey() -> AnyElement {
    Survey::new(
        "survey",
        [
            Question::Choice("How often?".into(), vec!["Daily".into(), "Weekly".into()]),
            Question::Several("Which parts?".into(), vec!["Forms".into(), "Charts".into()]),
            Question::Text("Anything else?".into()),
        ],
        |answers, _, _| say(format!("{answers:?}")),
    )
    .into_any_element()
}

/// Submit rests until each question is answered, hands the answers over, and the thanks take its focus.
#[gpui::test]
fn a_survey_sends_every_answer(cx: &mut TestAppContext) {
    let (_, cx) = stage(survey, cx);
    press("tab", cx);
    press("down", cx);
    press("tab", cx);
    press("space", cx);
    press("tab", cx);
    press("tab", cx);
    cx.simulate_input("More charts");
    press("tab", cx);
    press("enter", cx);
    let sent = vec![
        Answer::Choice(1),
        Answer::Several(vec![0]),
        Answer::Text("More charts".into()),
    ];
    assert_eq!(said(), [format!("{sent:?}")]);
    assert!(shown("survey-sent", cx) && !at_root(cx));
}

/// Submit asks for what is missing: words of only spaces, or a box ticked and unticked; a pointer's Submit hands focus to the thanks.
#[gpui::test]
fn a_survey_waits_for_every_answer(cx: &mut TestAppContext) {
    let (_, cx) = stage(survey, cx);
    press("tab", cx);
    press("down", cx);
    press("tab", cx);
    press("space", cx);
    press("tab", cx);
    press("tab", cx);
    cx.simulate_input("   ");
    press("tab", cx);
    press("enter", cx);
    assert!(
        said().is_empty() && shown("survey-answer-first", cx),
        "blank words are no answer"
    );
    press("shift-tab", cx);
    cx.simulate_input("Fine");
    press("shift-tab", cx);
    press("shift-tab", cx);
    press("space", cx);
    for _ in 0..3 {
        press("tab", cx);
    }
    press("enter", cx);
    assert!(said().is_empty(), "no box ticked is no answer");
    assert!(!at_root(cx), "Submit kept its focus");
    press("shift-tab", cx);
    press("shift-tab", cx);
    press("shift-tab", cx);
    press("space", cx);
    click("survey-action", cx);
    assert_eq!(said().len(), 1);
    assert!(!at_root(cx), "the field left, and focus went to the thanks");
}

fn quiz() -> AnyElement {
    Quiz::new(
        "quiz",
        [
            QuizQuestion::new("2 + 2?", ["3", "4"], 1),
            QuizQuestion::new("Blue and yellow?", ["Green", "Red"], 0),
        ],
    )
    .on_score(|right, count, _, _| say(format!("{right} of {count}")))
    .into_any_element()
}

/// One button checks, goes on, scores and starts again, and asks for a pick when none is made.
#[gpui::test]
fn a_quiz_checks_each_answer_and_scores(cx: &mut TestAppContext) {
    let (_, cx) = stage(quiz, cx);
    press("tab", cx);
    press("tab", cx);
    press("enter", cx);
    assert!(shown("quiz-pick-first", cx));
    press("shift-tab", cx);
    press("down", cx);
    press("tab", cx);
    press("enter", cx);
    assert!(shown("quiz-row-4-true", cx), "the right answer shows");
    press("enter", cx);
    press("shift-tab", cx);
    press("down", cx);
    press("tab", cx);
    press("enter", cx);
    assert!(shown("quiz-row-Green-true", cx) && shown("quiz-row-Red-false", cx));
    press("enter", cx);
    assert!(shown("quiz-score-1-2", cx));
    assert_eq!(said(), ["1 of 2"]);
    press("enter", cx);
    assert!(!at_root(cx), "Try again kept the button's focus");
}

/// A pointer's Check hands focus from the radios, which leave, to the one button.
#[gpui::test]
fn a_pointers_check_keeps_focus_in_the_quiz(cx: &mut TestAppContext) {
    let (_, cx) = stage(quiz, cx);
    press("tab", cx);
    press("down", cx);
    click("quiz-action", cx);
    assert!(shown("quiz-row-4-true", cx) && !at_root(cx));
}

fn cards() -> AnyElement {
    Flashcards::new("cards", [("A", "a"), ("B", "b"), ("C", "c")]).into_any_element()
}

/// A press turns the card; Next and Previous go round the deck, and each card they reach shows its front.
#[gpui::test]
fn flashcards_turn_and_go_round(cx: &mut TestAppContext) {
    let (_, cx) = stage(cards, cx);
    assert!(shown("flashcard-front-A", cx));
    for _ in 0..3 {
        press("tab", cx);
    }
    press("enter", cx);
    press("enter", cx);
    assert!(shown("flashcard-front-C", cx));
    press("enter", cx);
    press("shift-tab", cx);
    press("shift-tab", cx);
    press("enter", cx);
    assert!(
        shown("flashcard-back-a", cx) && !shown("flashcard-back-c", cx),
        "Next went round from the last card to the first, and the press turned it"
    );
    press("tab", cx);
    press("enter", cx);
    assert!(
        !shown("flashcard-back-c", cx),
        "the card Previous reached shows its front"
    );
    press("shift-tab", cx);
    press("enter", cx);
    assert!(
        shown("flashcard-back-c", cx),
        "Previous went round from the first card to the last"
    );
}
