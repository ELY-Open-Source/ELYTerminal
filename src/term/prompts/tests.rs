use alacritty_terminal::{
    event::VoidListener,
    index::Column,
    term::{Config, test::TermSize},
    vte::ansi::Processor,
};

use super::*;
use crate::term::marks::{self, Mark};

/// What the parser gets from `chunks`, each one read.
fn passed(chunks: &[&[u8]]) -> Vec<u8> {
    let mut prompts = Prompts::default();
    let mut out = Vec::new();
    for chunk in chunks {
        out.extend_from_slice(prompts.pass(chunk));
    }
    out
}

/// A terminal `columns` wide and `lines` high after `bytes` passed through the rewrite.
fn parsed(bytes: &[u8], columns: usize, lines: usize) -> Term<VoidListener> {
    let mut term = Term::new(
        Config::default(),
        &TermSize::new(columns, lines),
        VoidListener,
    );
    let mut parser: Processor = Processor::new();
    parser.advance(&mut term, &passed(&[bytes]));
    term
}

/// The target of the link at a cell.
fn target_of(term: &Term<VoidListener>, line: i32, column: usize) -> Option<String> {
    let cell = &term.grid()[Line(line)][Column(column)];
    cell.hyperlink().map(|link| link.uri().to_string())
}

#[test]
fn a_mark_becomes_its_prompt_s_numbered_link() {
    assert_eq!(
        passed(&[b"\x1b]133;A\x07$ \x1b]133;B\x1b\\x"]),
        b"\x1b]8;id=\x7f1;\x7f\x07$ \x1b]8;id=\x7f0i;\x7f\x07\x1b\\x"
    );
    assert_eq!(
        passed(&[b"\x1b]133;A\x07$ \x1b]133;B\x07ls\x1b]133;C\x07out"]),
        b"\x1b]8;id=\x7f1;\x7f\x07$ \x1b]8;id=\x7f0i;\x7f\x07ls\x1b]8;;\x07out",
        "the input runs from B to C"
    );
    assert_eq!(
        passed(&[b"\x1b]133;A;k=i\x07\x1b]133;D;1\x18"]),
        b"\x1b]8;id=\x7f1;\x7f\x07\x1b]8;;\x07\x18"
    );
    assert_eq!(
        passed(&[b"\x1b]133;N\x07\x1b]133;C\x07\x1b]133;A\x07"]),
        b"\x1b]8;id=\x7f1;\x7f\x07\x1b]8;;\x07\x1b]8;id=\x7f2;\x7f\x07"
    );
    assert_eq!(
        passed(&[b"\x1b]133;D;0\x07"]),
        b"\x1b]133;D;0\x07",
        "an end with no prompt open passes"
    );
}

#[test]
fn the_rewrite_reads_a_mark_as_the_session_hears_it() {
    let bodies = [
        "A", "A;k=i", "N", "B", "I", "C;x", "D;1", "D;abc", "D", "", "AB", "L", "P;k=i", "Q", "a",
    ];
    for body in bodies {
        let heard = marks::read(body.as_bytes()).map(|mark| match mark {
            Mark::Prompt(_) => Edge::Prompt,
            Mark::Input => Edge::Input,
            Mark::Output | Mark::Done(_) => Edge::End,
        });
        assert_eq!(mark(body.as_bytes()), heard, "{body}");
    }
}

#[test]
fn a_mark_split_across_reads_comes_out_whole() {
    let whole: &[u8] =
        b"a\x1b]133;A;k=i\x07\x1b]8;;file:///w\x1b\\w\x1b]8;;\x1b\\$ \x1b]133;B\x1b\\b";
    let once = passed(&[whole]);
    for cut in 0..=whole.len() {
        let (first, second) = whole.split_at(cut);
        assert_eq!(passed(&[first, second]), once, "cut at {cut}");
    }
    let bytes: Vec<&[u8]> = whole.chunks(1).collect();
    assert_eq!(passed(&bytes), once);
}

#[test]
fn other_sequences_pass_as_they_came() {
    for bytes in [
        "\x1b]133;AB\x07x",
        "\x1b]1337;A\x07",
        "\x1b]133;P;k=i\x07",
        "\x1b]133;\x07",
        "\x1b]0;133;A\x07",
        "\x1b[31m\x1bc",
        "\x1b]8;;file:///x\x07x\x1b]8;;\x07",
        "text",
    ] {
        assert_eq!(passed(&[bytes.as_bytes()]), bytes.as_bytes(), "{bytes:?}");
    }
    let mut long = b"\x1b]133;A;".to_vec();
    long.extend([b'x'; MOST]);
    long.push(0x07);
    assert_eq!(passed(&[&long]), long, "past the scanner's bound");
    let mut skipped = b"\x1b]133;A".to_vec();
    skipped.extend([0; 3 * MOST]);
    skipped.push(0x07);
    assert_eq!(
        passed(&[&skipped]),
        skipped,
        "skipped controls bound the bytes held"
    );
}

#[test]
fn a_long_escape_of_controls_goes_on_as_it_comes() {
    let mut whole = vec![ESC];
    whole.extend(vec![0; 3 << 20]);
    whole.extend_from_slice(b"[0mREADY");
    let mut prompts = Prompts::default();
    let mut out = Vec::new();
    for chunk in whole.chunks(64 << 10) {
        out.extend_from_slice(prompts.pass(chunk));
        assert!(prompts.held() <= 1, "only the ESC waits");
    }
    assert_eq!(out.len(), whole.len());
    assert!(out.ends_with(b"\x1b[0mREADY"));
}

#[test]
fn a_program_s_link_inside_a_prompt_carries_its_number() {
    let whole = parsed(
        b"\x1b]133;A\x07\x1b]8;;file:///work\x1b\\/work $ \x1b]8;;\x1b\\\x1b]133;B\x07",
        12,
        2,
    );
    assert_eq!(
        prompt(&whole.grid()[Line(0)]),
        Some(1),
        "a prompt all in a link"
    );
    assert_eq!(target_of(&whole, 0, 6).as_deref(), Some("file:///work"));
    let first = b"\x1b]133;A\x07\x1b]8;;file:///w\x1b\\/work\r\n\x1b]8;;\x1b\\$ \x1b]133;B\x07\r\no\r\no\r\no";
    let term = parsed(first, 8, 3);
    assert_eq!(jump(&term, true), Some(2), "its linked first row starts it");
    let open = parsed(
        b"\x1b]8;;file:///x\x1b\\x\x1b]133;A\x07y\x1b]133;B\x07z",
        8,
        2,
    );
    assert_eq!(
        prompt(&open.grid()[Line(0)]),
        Some(1),
        "a prompt inside a link left open"
    );
    let (y, z) = (
        &open.grid()[Line(0)][Column(1)],
        &open.grid()[Line(0)][Column(2)],
    );
    assert_eq!(part(z), Some(Part::Input(0)), "the input inside it");
    assert_eq!(
        target_of(&open, 0, 2).as_deref(),
        Some("file:///x"),
        "its own target past the prompt"
    );
    assert!(
        y.hyperlink()
            .is_some_and(|link| link.id().starts_with(NUMBERED))
    );
}

#[test]
fn a_long_target_numbers_a_prompt_in_either_order() {
    let target = format!("https://e.test/{}", "a".repeat(249));
    let open = format!("\x1b]8;;{target}\x1b\\");
    let before = format!("{open}x\x1b]133;A\x07y\x1b]133;B\x07z\x1b]8;;\x1b\\");
    let inside = format!("x\x1b]133;A\x07{open}y\x1b]133;B\x07z\x1b]8;;\x1b\\");
    for bytes in [before, inside] {
        let term = parsed(bytes.as_bytes(), 8, 2);
        let row = &term.grid()[Line(0)];
        assert_eq!(prompt(row), Some(1), "{bytes:?}");
        assert!(
            prompted(&row[Column(1)]) && !prompted(&row[Column(2)]),
            "the prompt's cell alone: {bytes:?}"
        );
        assert_eq!(part(&row[Column(2)]), Some(Part::Input(0)), "{bytes:?}");
        assert_eq!(target_of(&term, 0, 2).as_deref(), Some(target.as_str()));
    }
}

#[test]
fn controls_after_an_osc_s_closing_escape_run_where_vte_runs_them() {
    let whole: &[u8] = b"LEFT\x1b]0;title\x1b\r\n\\RIGHT";
    for cut in 0..=whole.len() {
        let (first, second) = whole.split_at(cut);
        assert_eq!(passed(&[first, second]), whole, "cut at {cut}");
    }
    let term = parsed(whole, 8, 2);
    let line = |at: i32| -> String {
        term.grid()[Line(at)]
            .into_iter()
            .map(|cell| cell.c)
            .collect()
    };
    assert_eq!((line(0).trim_end(), line(1).trim_end()), ("LEFT", "RIGHT"));
    let marked = parsed(b"\x1b]133;A\x1b\r\n\\$", 8, 2);
    assert_eq!(prompt(&marked.grid()[Line(1)]), Some(1));
}

#[test]
fn del_and_high_bytes_in_an_escape_go_on_after_it() {
    assert_eq!(
        passed(&[b"\x1b\x7f]133;A\x07$\x1b\xff", b"]133;B\x07"]),
        b"\x1b\x7f]8;id=\x7f1;\x7f\x07$\x1b\xff]8;id=\x7f0i;\x7f\x07"
    );
}

#[test]
fn controls_vte_runs_inside_an_escape_go_on_before_the_link() {
    assert_eq!(
        passed(&[b"\x1b\n\x07]133;A\x07$"]),
        b"\n\x07\x1b]8;id=\x7f1;\x7f\x07$"
    );
}

#[test]
fn a_prompt_s_cells_carry_its_link_through_sgr_0_and_its_input_its_own() {
    let term = parsed(
        b"x\r\n\x1b]133;A\x07\x1b[31m$\x1b[0m \x1b]133;B\x07ls",
        8,
        2,
    );
    assert_eq!(prompt(&term.grid()[Line(0)]), None);
    assert_eq!(prompt(&term.grid()[Line(1)]), Some(1));
    assert_eq!(target_of(&term, 1, 1).as_deref(), Some(LINK));
    assert_eq!(
        part(&term.grid()[Line(1)][Column(2)]),
        Some(Part::Input(0)),
        "input"
    );
    let ran = parsed(
        b"\x1b]133;A\x07$\x1b]133;B\x07ls\x1b]133;C\x07\r\nout",
        8,
        2,
    );
    assert_eq!(target_of(&ran, 1, 0), None, "output");
}

#[test]
fn a_program_s_own_link_and_the_prompt_s_keep_each_other() {
    let term = parsed(
        b"\x1b]133;A\x07\x1b]8;;file:///work\x1b\\/work\x1b]8;;\x1b\\ $ \x1b]133;B\x07",
        12,
        2,
    );
    assert_eq!(target_of(&term, 0, 0).as_deref(), Some("file:///work"));
    assert_eq!(target_of(&term, 0, 6).as_deref(), Some(LINK));
    assert_eq!(prompt(&term.grid()[Line(0)]), Some(1));
    let term = parsed(
        b"\x1b]8;;file:///x\x1b\\x\x1b]133;A\x07y\x1b]8;;\x1b\\z\x1b]133;B\x07",
        12,
        2,
    );
    assert_eq!(target_of(&term, 0, 0).as_deref(), Some("file:///x"));
    assert_eq!(
        target_of(&term, 0, 1).as_deref(),
        Some("file:///x"),
        "a prompt inside a link keeps it"
    );
    assert_eq!(target_of(&term, 0, 2).as_deref(), Some(LINK));
}

/// Prompts on lines 0, 3 and 6 of a history four lines high, the last on the screen.
fn history() -> Term<VoidListener> {
    let mut bytes = Vec::new();
    for _ in 0..3 {
        bytes.extend_from_slice(
            b"\x1b]133;A\x07$ \x1b]133;B\x07c\r\n\x1b]133;C\x07o\r\no\r\n\x1b]133;D;0\x07",
        );
    }
    bytes.extend_from_slice(b"\x1b]133;A\x07$ \x1b]133;B\x07");
    parsed(&bytes, 8, 4)
}

#[test]
fn up_and_down_go_to_the_prompts_beside_the_view_s_top() {
    let mut term = history();
    assert_eq!(term.grid().history_size(), 6);
    // From the bottom: the prompt at the view's top is passed, the one before it found.
    assert_eq!(jump(&term, true), Some(3));
    term.scroll_display(Scroll::Delta(3));
    assert_eq!(jump(&term, true), Some(6));
    term.scroll_display(Scroll::Delta(3));
    assert_eq!(jump(&term, true), None, "no prompt above the first");
    assert_eq!(jump(&term, false), Some(3));
    term.scroll_display(Scroll::Delta(-3));
    assert_eq!(
        jump(&term, false),
        Some(0),
        "a prompt on the screen means the bottom"
    );
}

#[test]
fn a_prompt_over_rows_starts_at_its_first_blank_rows_within_it_too() {
    let bytes = b"\x1b]133;A\x07FIRST\r\n\r\nLAST $ \x1b]133;B\x07\r\nout1\r\nout2\r\nout3\r\nout4";
    let mut term = parsed(bytes, 8, 3);
    assert_eq!(term.grid().history_size(), 4);
    assert_eq!(jump(&term, true), Some(4), "FIRST, not LAST");
    term.scroll_display(Scroll::Delta(4));
    assert_eq!(jump(&term, false), None, "LAST is the same prompt");
    let next = parsed(b"\x1b]133;A\x07$ \x1b]133;B\x07\r\n\x1b]133;A\x07$ ", 8, 2);
    assert_eq!(
        (prompt(&next.grid()[Line(0)]), prompt(&next.grid()[Line(1)])),
        (Some(1), Some(2)),
        "an empty line's next prompt is its own"
    );
}

#[test]
fn a_prompt_keeps_its_start_through_a_resize() {
    let bytes =
        b"\x1b]133;A\x07~/folder $ \x1b]133;B\x07ls\r\nout\r\nout\r\nout\r\n\x1b]133;A\x07$ ";
    let mut term = parsed(bytes, 12, 3);
    term.resize(TermSize::new(6, 3));
    let rows = term.topmost_line().0..=term.bottommost_line().0;
    let numbers: Vec<Option<u32>> = rows.map(|line| prompt(&term.grid()[Line(line)])).collect();
    let first = numbers.iter().position(|number| *number == Some(1));
    // The first prompt wraps over two rows; the view jumps to the first of them.
    assert_eq!(
        numbers.iter().filter(|number| **number == Some(1)).count(),
        2
    );
    let offset = jump(&term, true).expect("a prompt above");
    let top = term.topmost_line().0 + first.expect("prompt 1") as i32;
    assert_eq!(offset as i32, -top);
}

#[test]
fn input_carries_the_commands_ended_before_it() {
    let line = b"\x1b]133;A\x07$\x1b]133;B\x07ab\x1b]133;A\x07R\x1b]133;B\x07c";
    let term = parsed(line, 8, 2);
    let at = |column: usize| part(&term.grid()[Line(0)][Column(column)]);
    assert_eq!(
        (at(0), at(1), at(3)),
        (
            Some(Part::Prompt(1)),
            Some(Part::Input(0)),
            Some(Part::Prompt(2))
        )
    );
    assert_eq!(
        at(4),
        Some(Part::Input(0)),
        "a prompt drawn again keeps the line"
    );
    let next = parsed(
        b"\x1b]133;A\x07$\x1b]133;B\x07a\x1b]133;C\x07\x1b]133;D\x07\x1b]133;A\x07$\x1b]133;B\x07b",
        8,
        2,
    );
    assert_eq!(
        part(&next.grid()[Line(0)][Column(3)]),
        Some(Part::Input(2)),
        "C and D each end it"
    );
}

#[test]
fn a_string_s_end_keeps_its_controls_where_they_stand() {
    for start in [b'_', b'P', b'X', b'^'] {
        let mut bytes = b"ab\x1b".to_vec();
        bytes.push(start);
        bytes.extend_from_slice(b"Gi=1\x1b\r\\cd");
        for split in 0..bytes.len() {
            let (head, tail) = bytes.split_at(split);
            assert_eq!(passed(&[head, tail]), bytes, "{} at {split}", start as char);
        }
    }
}
