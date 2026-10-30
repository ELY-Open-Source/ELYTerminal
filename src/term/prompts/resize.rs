use alacritty_terminal::{
    grid::{Dimensions, GridCell},
    index::{Column, Line},
    term::{Term, TermMode, cell::Cell},
};

use super::{Part, part, redraw_prompt};

pub(crate) fn prepare_resize<T>(term: &mut Term<T>, columns: usize) -> bool {
    let cursor = &term.grid().cursor;
    if columns == term.columns() || term.mode().contains(TermMode::ALT_SCREEN) {
        return false;
    }
    let Some(prompt) = redraw_prompt(&cursor.template) else {
        return false;
    };
    let mut first = None;
    for line in (term.topmost_line().0..=cursor.point.line.0).rev() {
        let mut current = false;
        let mut previous = false;
        for cell in &term.grid()[Line(line)] {
            if let Some(Part::Prompt(number)) = part(cell) {
                current |= number == prompt;
                previous |= number != prompt;
            }
        }
        if current {
            first = Some(line);
        } else if previous {
            break;
        }
    }
    let Some(first) = first else {
        return false;
    };
    let row = &term.grid()[Line(first)];
    if row
        .into_iter()
        .take_while(|cell| part(cell) != Some(Part::Prompt(prompt)))
        .any(|cell| !cell.is_empty())
    {
        return false;
    }
    let last = term.bottommost_line().0;
    let grid = term.grid_mut();
    // Keep physical rows until the line editor redraws.
    for line in first..=last {
        for cell in &mut grid[Line(line)] {
            let hyperlink = cell.hyperlink();
            *cell = Cell::default();
            cell.set_hyperlink(hyperlink);
        }
    }
    grid.cursor.point.column = Column(grid.cursor.point.column.0.min(columns - 1));
    grid.cursor.input_needs_wrap = false;
    true
}

#[cfg(test)]
mod tests {
    use alacritty_terminal::{
        event::VoidListener,
        index::Point,
        term::{Config, test::TermSize},
        vte::ansi::Processor,
    };

    use super::*;
    use crate::term::prompts::{Prompts, start};

    fn parsed(bytes: &[u8], columns: usize, lines: usize) -> Term<VoidListener> {
        let mut term = Term::new(
            Config::default(),
            &TermSize::new(columns, lines),
            VoidListener,
        );
        let mut prompts = Prompts::default();
        let mut parser: Processor = Processor::new();
        parser.advance(&mut term, prompts.pass(bytes));
        term
    }

    fn text(term: &Term<VoidListener>) -> String {
        (term.topmost_line().0..=term.bottommost_line().0)
            .map(|line| {
                term.grid()[Line(line)]
                    .into_iter()
                    .map(|cell| cell.c)
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn a_redrawn_prompt_keeps_its_physical_rows_during_shrink() {
        let mut term = parsed(
            b"OUTPUT123456\r\n\x1b]133;A;redraw=1\x07PROMPT123456\r\n> \x1b]133;B\x07",
            12,
            8,
        );
        let before = term.grid().cursor.point;
        assert!(prepare_resize(&mut term, 6));
        assert_eq!(term.grid().cursor.point, before);
        assert_eq!(start(&term, before.line.0), Some(1));
        term.resize(TermSize::new(6, 8));
        assert_eq!(term.grid().cursor.point.line, before.line);
        assert!(text(&term).replace('\n', "").contains("OUTPUT123456"));
        assert!(!text(&term).contains("PROMPT"));
    }

    #[test]
    fn an_editors_wrapped_input_is_cleared_before_growing() {
        let mut term = parsed(
            b"\x1b]133;A;redraw=1\x07PROMPT\r\n> \x1b]133;B\x07abcdefghijklmnop\x1b[1A\x1b[4G",
            8,
            8,
        );
        let before = term.grid().cursor.point;
        assert!(text(&term).replace('\n', "").contains("abcdefghijklmnop"));
        assert!(prepare_resize(&mut term, 12));
        term.resize(TermSize::new(12, 8));
        assert_eq!(term.grid().cursor.point, before);
        assert!(text(&term).trim().is_empty());
    }

    #[test]
    fn consecutive_resizes_can_find_the_cleared_prompt() {
        let mut term = parsed(
            b"\x1b]133;A;redraw=1\x07PROMPT\r\n> \x1b]133;B\x07abcdefghijk",
            8,
            8,
        );
        assert!(prepare_resize(&mut term, 6));
        term.resize(TermSize::new(6, 8));
        assert!(prepare_resize(&mut term, 4));
        term.resize(TermSize::new(4, 8));
        assert!(prepare_resize(&mut term, 12));
        term.resize(TermSize::new(12, 8));
        assert_eq!(start(&term, term.grid().cursor.point.line.0), Some(0));
        assert!(text(&term).trim().is_empty());
    }

    #[test]
    fn a_cursor_past_the_new_width_keeps_its_physical_line() {
        let mut term = parsed(b"\x1b]133;A;redraw=1\x07> \x1b]133;B\x07abcdefghij", 12, 8);
        assert!(term.grid().cursor.input_needs_wrap);
        assert!(prepare_resize(&mut term, 6));
        term.resize(TermSize::new(6, 8));
        assert_eq!(term.grid().cursor.point, Point::new(Line(0), Column(5)));
        assert!(!term.grid().cursor.input_needs_wrap);
    }

    #[test]
    fn an_empty_prompt_does_not_clear_a_previous_command() {
        let mut term = parsed(
            b"\x1b]133;A;redraw=1\x07$ \x1b]133;B\x07echo OUTPUT\x1b]133;C\x07\r\nOUTPUT\r\n\x1b]133;D;0\x07\x1b]133;A;redraw=1\x07\x1b]133;B\x07",
            16,
            8,
        );
        let before = text(&term);
        assert!(!prepare_resize(&mut term, 8));
        assert_eq!(text(&term), before);
    }

    #[test]
    fn output_before_a_prompt_on_the_same_row_is_preserved() {
        let mut term = parsed(b"OUTPUT\x1b]133;A;redraw=1\x07$ \x1b]133;B\x07abc", 16, 8);
        let before = text(&term);
        assert!(!prepare_resize(&mut term, 8));
        assert_eq!(text(&term), before);
    }

    #[test]
    fn a_prompt_mid_redraw_can_be_resized_before_its_input_mark() {
        let mut term = parsed(b"OUTPUT\r\n\x1b]133;A;redraw=1\x07PROMPT123456", 12, 8);
        assert_eq!(part(&term.grid().cursor.template), Some(Part::Prompt(1)));
        assert!(prepare_resize(&mut term, 6));
        term.resize(TermSize::new(6, 8));
        assert_eq!(term.grid().cursor.point.line, Line(1));
        assert!(text(&term).contains("OUTPUT"));
        assert!(!text(&term).contains("PROMPT"));
    }

    #[test]
    fn a_prompt_above_the_viewport_is_cleared_with_its_input() {
        let mut term = parsed(
            b"OUTPUT\r\n\x1b]133;A;redraw=1\x07PROMPT12\r\n> \x1b]133;B\x07abcdefghijklmnopqrstuvwxyz",
            8,
            3,
        );
        assert!(start(&term, term.grid().cursor.point.line.0).unwrap() < 0);
        assert!(prepare_resize(&mut term, 4));
        term.resize(TermSize::new(4, 3));
        assert!(!text(&term).contains("PROMPT"));
        assert!(!text(&term).contains("abcdefgh"));
        assert!(text(&term).replace('\n', "").contains("OUTPUT"));
    }

    #[test]
    fn changing_height_and_width_keeps_the_editors_row_distance() {
        let mut term = parsed(
            b"OUTPUT\r\n\x1b]133;A;redraw=1\x07PROMPT123456\r\n> \x1b]133;B\x07abcdefghijklmnop",
            12,
            8,
        );
        let before = term.grid().cursor.point.line.0 - start(&term, 7).unwrap();
        assert!(prepare_resize(&mut term, 6));
        term.resize(TermSize::new(6, 3));
        let after = term.grid().cursor.point.line.0
            - start(&term, term.grid().cursor.point.line.0).unwrap();
        assert_eq!(after, before);
        assert!(text(&term).replace('\n', "").contains("OUTPUT"));
    }

    #[test]
    fn only_an_active_prompt_that_promises_redraw_is_cleared() {
        for bytes in [
            &b"PROMPT\r\n> abc"[..],
            &b"\x1b]133;A\x07PROMPT\r\n> \x1b]133;B\x07abc"[..],
            &b"\x1b]133;A;redraw=1\x07PROMPT\r\n> \x1b]133;B\x07abc\x1b]133;C\x07"[..],
            &b"\x1b]133;A;redraw=1\x07PROMPT\r\n> \x1b]133;B\x07abc\x1b[?1049h"[..],
        ] {
            let mut term = parsed(bytes, 12, 8);
            let before = text(&term);
            assert!(!prepare_resize(&mut term, 6), "{bytes:?}");
            assert_eq!(text(&term), before);
        }
    }
}
