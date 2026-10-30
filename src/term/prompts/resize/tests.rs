use alacritty_terminal::{
    event::VoidListener,
    index::Point,
    term::{Config, test::TermSize},
    vte::ansi::Processor,
};

use super::*;
use crate::term::prompts::{Part, Prompts, start};

fn parsed(bytes: &[u8], columns: usize, lines: usize) -> (Term<VoidListener>, Prompts) {
    let mut term = Term::new(
        Config::default(),
        &TermSize::new(columns, lines),
        VoidListener,
    );
    let mut prompts = Prompts::default();
    let mut parser: Processor = Processor::new();
    parser.advance(&mut term, prompts.pass(bytes));
    (term, prompts)
}

/// Resizes as `Pty::resize` does.
fn resize(term: &mut Term<VoidListener>, columns: usize, lines: usize) -> bool {
    let kept = keep(term, columns);
    let was = kept.is_some();
    term.resize(TermSize::new(columns, lines));
    if let Some(kept) = kept {
        restore(term, kept);
    }
    was
}

/// Every row, history first, its blanks at the end trimmed.
fn rows(term: &Term<VoidListener>) -> Vec<String> {
    (term.topmost_line().0..=term.bottommost_line().0)
        .map(|line| {
            let row: String = term.grid()[Line(line)]
                .into_iter()
                .map(|cell| cell.c)
                .collect();
            row.trim_end().to_string()
        })
        .collect()
}

#[test]
fn a_prompt_redrawn_keeps_its_rows_unwrapped_as_output_reflows() {
    let (mut term, _) = parsed(
        b"OUTPUT123456\r\n\x1b]133;A;redraw=1\x07PROMPT123456\r\n> \x1b]133;B\x07",
        12,
        8,
    );
    assert!(resize(&mut term, 6, 8));
    // Output wraps; the prompt's rows are cut, not wrapped, for its shell to draw again.
    assert_eq!(rows(&term)[..4], ["OUTPUT", "123456", "PROMPT", ">"]);
    // alacritty keeps the cursor's line; the wrapped output goes up into the history.
    assert_eq!(term.grid().cursor.point, Point::new(Line(2), Column(2)));
    assert_eq!(start(&term, 2), Some(1));
}

#[test]
fn a_wrapped_input_is_not_joined_when_growing() {
    let (mut term, _) = parsed(
        b"\x1b]133;A;redraw=1\x07PROMPT\r\n> \x1b]133;B\x07abcdefghijklmnop\x1b[1A\x1b[4G",
        8,
        8,
    );
    let before = term.grid().cursor.point;
    assert!(resize(&mut term, 12, 8));
    assert_eq!(rows(&term)[..4], ["PROMPT", "> abcdef", "ghijklmn", "op"]);
    assert_eq!(term.grid().cursor.point, before);
}

#[test]
fn consecutive_resizes_find_the_kept_prompt() {
    let (mut term, _) = parsed(
        b"\x1b]133;A;redraw=1\x07PROMPT\r\n> \x1b]133;B\x07abcdefghijk",
        8,
        8,
    );
    assert!(resize(&mut term, 6, 8));
    assert!(resize(&mut term, 4, 8));
    assert!(resize(&mut term, 12, 8));
    assert_eq!(start(&term, term.grid().cursor.point.line.0), Some(0));
    assert_eq!(term.grid().history_size(), 0);
}

#[test]
fn a_cursor_past_the_new_width_stays_on_its_row() {
    let (mut term, _) = parsed(b"\x1b]133;A;redraw=1\x07> \x1b]133;B\x07abcdefghij", 12, 8);
    assert!(term.grid().cursor.input_needs_wrap);
    assert!(resize(&mut term, 6, 8));
    assert_eq!(term.grid().cursor.point, Point::new(Line(0), Column(5)));
    assert!(!term.grid().cursor.input_needs_wrap);
}

#[test]
fn an_empty_prompt_leaves_the_command_before_it() {
    let (mut term, _) = parsed(
        b"\x1b]133;A;redraw=1\x07$ \x1b]133;B\x07echo OUTPUT\x1b]133;C\x07\r\nOUTPUT\r\n\x1b]133;D;0\x07\x1b]133;A;redraw=1\x07\x1b]133;B\x07",
        16,
        8,
    );
    assert!(!resize(&mut term, 8, 8));
    assert_eq!(rows(&term)[..3], ["$ echo O", "UTPUT", "OUTPUT"]);
}

#[test]
fn output_before_a_prompt_on_its_row_reflows_with_it() {
    let (mut term, _) = parsed(b"OUTPUT\x1b]133;A;redraw=1\x07$ \x1b]133;B\x07abc", 16, 8);
    assert!(!resize(&mut term, 8, 8));
    assert_eq!(rows(&term)[..2], ["OUTPUT$", "abc"]);
}

#[test]
fn a_prompt_half_printed_is_kept_before_its_input_mark() {
    let (mut term, _) = parsed(b"OUTPUT\r\n\x1b]133;A;redraw=1\x07PROMPT123456", 12, 8);
    assert_eq!(part(&term.grid().cursor.template), Some(Part::Prompt(1)));
    assert!(resize(&mut term, 6, 8));
    assert_eq!(rows(&term)[..2], ["OUTPUT", "PROMPT"]);
    assert_eq!(term.grid().cursor.point.line, Line(1));
}

#[test]
fn a_prompt_reaching_into_the_history_reflows_as_output() {
    let (mut term, _) = parsed(
        b"OUTPUT\r\n\x1b]133;A;redraw=1\x07PROMPT12\r\n> \x1b]133;B\x07abcdefghijklmnopqrstuvwxyz",
        8,
        3,
    );
    assert!(start(&term, term.grid().cursor.point.line.0).unwrap() < 0);
    assert!(!resize(&mut term, 4, 3));
    let joined = rows(&term).concat();
    assert!(
        joined.contains("PROMPT12> abcdefghijklmnopqrstuvwxyz"),
        "{joined}"
    );
}

#[test]
fn a_height_and_width_change_keeps_the_editor_s_rows_from_the_prompt() {
    let (mut term, _) = parsed(
        b"OUTPUT\r\n\x1b]133;A;redraw=1\x07PROMPT123456\r\n> \x1b]133;B\x07abcdefghijklmnop",
        12,
        8,
    );
    let before = term.grid().cursor.point.line.0 - start(&term, 7).unwrap();
    assert!(resize(&mut term, 6, 3));
    let cursor = term.grid().cursor.point.line.0;
    assert_eq!(cursor - start(&term, cursor).unwrap(), before);
}

#[test]
fn only_a_prompt_promising_a_redraw_is_kept() {
    for bytes in [
        &b"PROMPT\r\n> abc"[..],
        &b"\x1b]133;A\x07PROMPT\r\n> \x1b]133;B\x07abc"[..],
        &b"\x1b]133;A;redraw=1\x07PROMPT\r\n> \x1b]133;B\x07abc\x1b]133;C\x07"[..],
        &b"\x1b]133;A;redraw=1\x07PROMPT\r\n> \x1b]133;B\x07abc\x1b[?1049h"[..],
    ] {
        let (mut term, _) = parsed(bytes, 12, 8);
        assert!(!resize(&mut term, 6, 8), "{bytes:?}");
    }
}

#[test]
fn a_secondary_prompt_is_kept_alone_and_the_lines_before_it_reflow() {
    let (mut term, _) = parsed(
        b"\x1b]133;A;redraw=1\x07% \x1b]133;B\x07for x in one two; do\r\n\
          \x1b]133;P;k=s;redraw=1\x07for> \x1b]133;B\x07echo $x",
        24,
        8,
    );
    assert!(resize(&mut term, 12, 8));
    // The first line wraps as output does; only the line being edited is kept.
    assert_eq!(
        rows(&term)[..3],
        ["% for x in o", "ne two; do", "for> echo $x"]
    );
    assert_eq!(term.grid().cursor.point, Point::new(Line(1), Column(11)));
    assert_eq!(
        start(&term, 1),
        Some(-1),
        "both lines are one prompt to jump to"
    );
}

#[test]
fn output_inside_a_kept_prompt_stays() {
    // A background job printed on the prompt's line.
    let (mut term, _) = parsed(
        b"\x1b]133;A;redraw=1\x07TOP\r\n> \x1b]133;B\x07ASYNC_PAYLOAD\r\n",
        20,
        8,
    );
    assert!(resize(&mut term, 16, 8));
    assert_eq!(rows(&term)[..2], ["TOP", "> ASYNC_PAYLOAD"]);
}

#[test]
fn a_cut_wide_character_goes_whole() {
    let (mut term, _) = parsed("\x1b]133;A;redraw=1\x07> 中\x1b]133;B\x07".as_bytes(), 8, 4);
    assert!(resize(&mut term, 3, 4));
    assert_eq!(rows(&term)[0], ">");
}

#[test]
fn output_a_prompt_continues_reflows_apart_from_it() {
    for columns in [16, 6] {
        let (mut term, _) = parsed(
            b"OUTPUT123456\x1b]133;A;redraw=1\x07> \x1b]133;B\x07",
            12,
            8,
        );
        assert!(resize(&mut term, columns, 8));
        let shown = rows(&term).join("|");
        assert!(
            shown.replace('|', "").contains("OUTPUT123456"),
            "{columns}: {shown}"
        );
        assert!(shown.contains(">"), "{columns}: {shown}");
    }
}

#[test]
fn a_prompt_above_the_screen_past_a_blank_line_reflows_as_output() {
    let (mut term, _) = parsed(
        b"\x1b]133;A;redraw=1\x07TOP\r\n\r\n> \x1b]133;B\x07abcdefgh",
        8,
        3,
    );
    assert_eq!(rows(&term), ["TOP", "", "> abcdef", "gh"]);
    assert!(!resize(&mut term, 4, 3));
}
