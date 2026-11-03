use super::*;
use crate::{term::links::hit_at, themes};
use alacritty_terminal::{
    event::VoidListener,
    grid::Scroll,
    index::{Column, Line},
    term::{Config, test::TermSize},
    vte::ansi::Processor,
};

fn term(text: &str) -> Term<VoidListener> {
    let mut term = Term::new(Config::default(), &TermSize::new(12, 3), VoidListener);
    <Processor>::new().advance(&mut term, text.as_bytes());
    term
}

fn styles(frame: &Frame) -> Vec<Style> {
    frame
        .rows
        .iter()
        .flat_map(|row| {
            row.runs
                .iter()
                .flat_map(|(bytes, style)| std::iter::repeat_n(*style, *bytes))
        })
        .collect()
}

fn shown(term: &Term<VoidListener>, hover: Option<Point>) -> Frame {
    let ink = Ink::new(themes::find("dracula").expect("a theme"), term.colors());
    let link = hover.and_then(|point| hit_at(term, point));
    frame(term, &ink, &[], link.as_ref())
}

#[test]
fn hover_underlines_the_whole_link_and_leaving_restores_its_style() {
    let term = term("(https://a.b/c) ");
    let hover = Point::new(Line(0), Column(4));
    let painted = styles(&shown(&term, Some(hover)));
    assert_eq!(painted[0].underline, Underline::None);
    assert!(
        painted[1..14]
            .iter()
            .all(|style| style.underline == Underline::Single)
    );
    assert_eq!(painted[14].underline, Underline::None);
    assert!(
        styles(&shown(&term, None))
            .iter()
            .all(|style| style.underline == Underline::None)
    );
    assert!(!term.grid()[hover].flags.contains(Flags::UNDERLINE));
}

#[test]
fn hover_preserves_explicit_underlines_and_hidden_text() {
    let term = term("\x1b[4:3m\x1b]8;;https://a.b\x07label\x1b]8;;\x07");
    let hover = Point::new(Line(0), Column(2));
    assert_eq!(shown(&term, Some(hover)).rows, shown(&term, None).rows);
    let term = self::term("\x1b[8mhttps://a.b\x1b[0m");
    assert!(
        styles(&shown(&term, Some(hover)))
            .iter()
            .all(|style| style.underline == Underline::None)
    );
}

#[test]
fn hover_follows_history_and_output_changes_without_a_mouse_move() {
    let mut term = term("https://a.b\r\nnext\r\nlast\r\nbottom");
    term.scroll_display(Scroll::Top);
    let hover = Point::new(Line(-1), Column(2));
    assert!(
        styles(&shown(&term, Some(hover)))[..11]
            .iter()
            .all(|style| style.underline == Underline::Single)
    );
    let mut term = self::term("https://a.b");
    let hover = Point::new(Line(0), Column(2));
    <Processor>::new().advance(&mut term, b"\r\x1b[2Kplain");
    assert!(
        styles(&shown(&term, Some(hover)))
            .iter()
            .all(|style| style.underline == Underline::None)
    );
}
