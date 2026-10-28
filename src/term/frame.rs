use alacritty_terminal::{
    event::EventListener,
    grid::Dimensions,
    index::Point,
    term::{Term, cell::Cell, cell::Flags, color::Colors, search::Match},
    vte::ansi::CursorShape,
};
use gpui::Hsla;

use super::{
    boxes,
    ink::{DIM, Ink},
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Underline {
    #[default]
    None,
    Single,
    Double,
    Curly,
    Dotted,
    Dashed,
}

/// How a run of cells paints.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Style {
    pub fg: Hsla,
    /// None where the terminal's ground shows.
    pub bg: Option<Hsla>,
    pub bold: bool,
    pub italic: bool,
    pub underline: Underline,
    pub underline_color: Hsla,
    pub strike: bool,
}

/// One screen line: a char per cell, and style runs over its bytes.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Row {
    pub text: String,
    pub runs: Vec<(usize, Style)>,
    /// Cells painted on their own at their column: combining clusters and drawn characters.
    pub alone: Vec<(usize, String, Style)>,
}

/// The cursor where it shows: its cell, shape, and what the cell holds.
#[derive(Clone, Debug, PartialEq)]
pub struct Cursor {
    pub row: usize,
    pub column: usize,
    pub shape: CursorShape,
    pub wide: bool,
    pub text: String,
    pub style: Style,
}

/// A terminal's screen as it paints.
#[derive(Clone, Debug, Default)]
pub struct Frame {
    pub rows: Vec<Row>,
    pub cursor: Option<Cursor>,
    /// The cursor's row and column while on screen, shown or hidden.
    pub caret: Option<(usize, usize)>,
}

impl Frame {
    /// The screen's text, trailing spaces gone.
    pub fn text(&self) -> String {
        let lines: Vec<String> = self
            .rows
            .iter()
            .map(|row| {
                let mut cells: Vec<String> = row.text.chars().map(String::from).collect();
                for (column, text, _) in &row.alone {
                    cells[*column] = text.clone();
                }
                cells.concat().trim_end().to_string()
            })
            .collect();
        lines.join("\n").trim_end().to_string()
    }
}

/// The screen as it shows now, scrolled back or not; `lit` matches wear their washes.
pub fn frame<T: EventListener>(term: &Term<T>, ink: &Ink, lit: &[(Match, Hsla)]) -> Frame {
    let content = term.renderable_content();
    let offset = content.display_offset as i32;
    let mut rows = vec![Row::default(); term.screen_lines()];
    let at = content.cursor.point;
    let mut cursor = None;
    for indexed in content.display_iter {
        let (point, cell) = (indexed.point, indexed.cell);
        let row = (point.line.0 + offset) as usize;
        let mut look = style(cell, ink, content.colors);
        let covers = |contains: &dyn Fn(Point) -> bool| {
            let (line, column) = (point.line, point.column);
            contains(point)
                || (cell.flags.contains(Flags::WIDE_CHAR) && contains(Point::new(line, column + 1)))
                || (cell.flags.contains(Flags::WIDE_CHAR_SPACER)
                    && contains(Point::new(line, column - 1)))
        };
        if content
            .selection
            .as_ref()
            .is_some_and(|range| covers(&|at| range.contains(at)))
        {
            look.bg = Some(ink.selection);
            look.fg = ink.selection_text.unwrap_or(look.fg);
        } else if let Some((_, wash)) = lit.iter().find(|(hit, _)| covers(&|at| hit.contains(&at)))
        {
            look.bg = Some(*wash);
        }
        let spacer = cell
            .flags
            .intersects(Flags::WIDE_CHAR_SPACER | Flags::LEADING_WIDE_CHAR_SPACER);
        let hidden = cell.flags.contains(Flags::HIDDEN);
        let marks = cell.zerowidth().filter(|marks| !marks.is_empty());
        let alone = (!hidden && (marks.is_some() || boxes::drawn(cell.c))).then(|| {
            std::iter::once(cell.c)
                .chain(marks.into_iter().flatten().copied())
                .collect::<String>()
        });
        let shown = if spacer || hidden || alone.is_some() {
            ' '
        } else {
            cell.c
        };
        push(&mut rows[row], shown, look);
        if point == at && content.cursor.shape != CursorShape::Hidden {
            cursor = Some(Cursor {
                row,
                column: point.column.0,
                shape: content.cursor.shape,
                wide: cell.flags.contains(Flags::WIDE_CHAR),
                text: alone.clone().unwrap_or_else(|| shown.to_string()),
                style: look,
            });
        }
        if let Some(alone) = alone {
            rows[row].alone.push((point.column.0, alone, look));
        }
    }
    let row = at.line.0 + offset;
    let caret = (0..rows.len() as i32)
        .contains(&row)
        .then_some((row as usize, at.column.0));
    Frame {
        rows,
        cursor,
        caret,
    }
}

fn push(row: &mut Row, shown: char, style: Style) {
    let len = shown.len_utf8();
    row.text.push(shown);
    match row.runs.last_mut() {
        Some((bytes, last)) if *last == style => *bytes += len,
        _ => row.runs.push((len, style)),
    }
}

fn style(cell: &Cell, ink: &Ink, set: &Colors) -> Style {
    let flags = cell.flags;
    let (mut fg, mut bg) = (ink.resolve(cell.fg, set), ink.resolve(cell.bg, set));
    if flags.contains(Flags::INVERSE) {
        std::mem::swap(&mut fg, &mut bg);
    }
    if flags.contains(Flags::DIM) {
        fg = fg.opacity(DIM);
    }
    let hidden = flags.contains(Flags::HIDDEN);
    if hidden {
        fg = bg;
    }
    let underline = if hidden {
        Underline::None
    } else if flags.contains(Flags::DOUBLE_UNDERLINE) {
        Underline::Double
    } else if flags.contains(Flags::UNDERCURL) {
        Underline::Curly
    } else if flags.contains(Flags::DOTTED_UNDERLINE) {
        Underline::Dotted
    } else if flags.contains(Flags::DASHED_UNDERLINE) {
        Underline::Dashed
    } else if flags.contains(Flags::UNDERLINE) {
        Underline::Single
    } else {
        Underline::None
    };
    Style {
        fg,
        bg: (bg != ink.bg).then_some(bg),
        bold: flags.contains(Flags::BOLD),
        italic: flags.contains(Flags::ITALIC),
        underline,
        underline_color: cell
            .underline_color()
            .map_or(fg, |color| ink.resolve(color, set)),
        strike: flags.contains(Flags::STRIKEOUT) && !hidden,
    }
}

#[cfg(test)]
mod tests {
    use alacritty_terminal::{
        event::VoidListener,
        term::{Config, test::TermSize},
        vte::ansi::Processor,
    };

    use super::*;
    use crate::themes;

    fn shown(bytes: &[u8]) -> (Frame, Ink) {
        let mut term = Term::new(Config::default(), &TermSize::new(12, 3), VoidListener);
        let mut parser: Processor = Processor::new();
        parser.advance(&mut term, bytes);
        let ink = Ink::new(themes::find("dracula").expect("dracula"), term.colors());
        (frame(&term, &ink, &[]), ink)
    }

    #[test]
    fn cells_join_into_runs_of_one_style() {
        let (frame, ink) = shown(b"\x1b[31mred\x1b[0m ok\r\n\x1b[7minv\x1b[0m");
        assert_eq!(frame.text(), "red ok\ninv");
        let (bytes, red) = frame.rows[0].runs[0];
        assert_eq!((bytes, red.fg, red.bg), (3, ink.ansi[1], None));
        let (_, inverse) = frame.rows[1].runs[0];
        assert_eq!((inverse.fg, inverse.bg), (ink.bg, Some(ink.fg)));
    }

    #[test]
    fn a_wide_char_keeps_the_columns_after_it() {
        let (frame, _) = shown("中a".as_bytes());
        assert_eq!(
            frame.rows[0].text.chars().take(3).collect::<String>(),
            "中 a"
        );
    }

    #[test]
    fn an_edit_that_splits_a_wide_char_clears_it_whole_in_the_erase_ground() {
        let (frame, _) = shown("A单中BC\x1b[1;3H\x1b[2P".as_bytes());
        assert_eq!(frame.text(), "A  BC");
        let (frame, _) = shown("A单BC\x1b[1;3H\x1b[4hX".as_bytes());
        assert_eq!(frame.text(), "A X BC");
        let (frame, ink) = shown("A\x1b[44m单\x1b[0mBC\x1b[1;2H\x1b[42m\x1b[X".as_bytes());
        assert_eq!(frame.text(), "A  BC");
        let (bytes, ground) = frame.rows[0].runs[1];
        assert_eq!((bytes, ground.bg), (2, Some(ink.ansi[2])));
    }

    #[test]
    fn combining_marks_stay_on_their_cell() {
        let (frame, _) = shown("e\u{301}x".as_bytes());
        assert_eq!(frame.text(), "e\u{301}x");
        assert_eq!(&frame.rows[0].text[..2], " x");
        assert_eq!(frame.rows[0].alone[0].1, "e\u{301}");
    }

    #[test]
    fn underline_styles_and_colors_read_from_sgr() {
        let (frame, ink) = shown(
            b"\x1b[4ma\x1b[4:2mb\x1b[4:3mc\x1b[4:4md\x1b[4:5me\x1b[58;2;1;2;3m\x1b[4mf\x1b[0;9mg",
        );
        let kinds: Vec<_> = frame.rows[0]
            .runs
            .iter()
            .map(|(_, s)| s.underline)
            .collect();
        assert_eq!(
            &kinds[..6],
            [
                Underline::Single,
                Underline::Double,
                Underline::Curly,
                Underline::Dotted,
                Underline::Dashed,
                Underline::Single
            ]
        );
        let (_, colored) = frame.rows[0].runs[5];
        assert_ne!(colored.underline_color, ink.fg);
        assert!(frame.rows[0].runs[6].1.strike);
    }

    #[test]
    fn the_cursor_reads_its_cell() {
        let (frame, _) = shown("ab中\x1b[2D".as_bytes());
        let cursor = frame.cursor.expect("a cursor");
        assert_eq!((cursor.row, cursor.column, cursor.wide), (0, 2, true));
        assert_eq!(cursor.text, "中");
        let (hidden, _) = shown(b"x\x1b[?25l");
        assert_eq!(hidden.cursor, None);
    }

    #[test]
    fn drawn_characters_leave_the_font_with_their_marks() {
        let (frame, ink) = shown("a\u{2500}\u{E0B0}\u{2500}\u{301}\x1b[2D".as_bytes());
        assert_eq!(&frame.rows[0].text[..4], "a   ");
        let alone: Vec<_> = frame.rows[0]
            .alone
            .iter()
            .map(|(at, text, _)| (*at, text.as_str()))
            .collect();
        assert_eq!(
            alone,
            [(1, "\u{2500}"), (2, "\u{E0B0}"), (3, "\u{2500}\u{301}")]
        );
        assert_eq!(frame.rows[0].alone[0].2.fg, ink.fg);
        assert_eq!(frame.text(), "a\u{2500}\u{E0B0}\u{2500}\u{301}");
        assert_eq!(frame.cursor.expect("a cursor").text, "\u{E0B0}");
        let (gone, _) = shown(b"ab\x1b[?25l");
        assert_eq!(
            (gone.cursor, gone.caret),
            (None, Some((0, 2))),
            "a hidden cursor keeps its cell"
        );
        let (hidden, _) = shown(b"\x1b[8m\xe2\x94\x80");
        assert!(hidden.rows[0].alone.is_empty(), "hidden lines draw nothing");
    }

    #[test]
    fn selected_cells_wear_the_selection_colors() {
        use alacritty_terminal::{
            index::{Column, Line, Side},
            selection::{Selection, SelectionType},
        };
        let mut term = Term::new(Config::default(), &TermSize::new(12, 3), VoidListener);
        let mut parser: Processor = Processor::new();
        parser.advance(&mut term, "ab\u{4e2d}d".as_bytes());
        let start = Point::new(Line(0), Column(1));
        let mut selection = Selection::new(SelectionType::Simple, start, Side::Left);
        selection.update(Point::new(Line(0), Column(3)), Side::Left);
        term.selection = Some(selection);
        let ink = Ink::new(themes::find("dracula").expect("dracula"), term.colors());
        let frame = frame(&term, &ink, &[]);
        let grounds: Vec<_> = frame.rows[0]
            .runs
            .iter()
            .map(|(bytes, style)| (*bytes, style.bg))
            .collect();
        assert_eq!(
            &grounds[..3],
            [(1, None), (5, Some(ink.selection)), (8, None)],
            "b and both halves of the wide cell, whose spacer is picked"
        );
    }

    #[test]
    fn found_matches_wear_their_washes_under_the_selection() {
        use alacritty_terminal::index::{Column, Line, Side};
        use alacritty_terminal::selection::{Selection, SelectionType};
        let mut term = Term::new(Config::default(), &TermSize::new(12, 3), VoidListener);
        let mut parser: Processor = Processor::new();
        parser.advance(&mut term, "ab\u{4e2d}d ab".as_bytes());
        let at = |column| Point::new(Line(0), Column(column));
        let ink = Ink::new(themes::find("dracula").expect("dracula"), term.colors());
        let (found, current) = (ink.ansi[3].opacity(0.25), ink.ansi[3].opacity(0.55));
        let lit = [(at(1)..=at(2), found), (at(6)..=at(7), current)];
        let grounds = |frame: &Frame| -> Vec<_> {
            frame.rows[0]
                .runs
                .iter()
                .map(|(bytes, style)| (*bytes, style.bg))
                .collect()
        };
        let shown = frame(&term, &ink, &lit);
        assert_eq!(
            &grounds(&shown)[..5],
            [
                (1, None),
                (5, Some(found)),
                (2, None),
                (2, Some(current)),
                (4, None)
            ],
            "b and both halves of the wide cell"
        );
        term.selection = Some(Selection::new(SelectionType::Simple, at(6), Side::Left));
        term.selection
            .as_mut()
            .expect("a selection")
            .update(at(6), Side::Right);
        let shown = frame(&term, &ink, &lit);
        assert_eq!(
            grounds(&shown)[3],
            (1, Some(ink.selection)),
            "the selection wins"
        );
    }

    #[test]
    fn hidden_cells_show_nothing_but_their_ground() {
        let (frame, _) = shown("\x1b[8;4;9;41mSe\u{301}\x1b[0m\x1b[2D".as_bytes());
        assert_eq!(frame.rows[0].text.trim_end(), "");
        assert!(frame.rows[0].alone.is_empty());
        let (_, style) = frame.rows[0].runs[0];
        assert_eq!((style.underline, style.strike), (Underline::None, false));
        assert!(style.bg.is_some(), "the ground stays");
        assert_eq!(frame.cursor.expect("a cursor").text, " ");
    }
}
