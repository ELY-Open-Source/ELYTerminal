use std::ops::{Range, RangeInclusive};

use alacritty_terminal::{
    grid::Dimensions,
    index::{Boundary, Column, Line, Point},
    term::{
        Term,
        cell::{Cell, Flags},
    },
};
use ely_gpui_component::terminal::{Target, links};
use gpui::SharedString;

use super::prompts;

/// Auto-detected URLs need a complete token within both budgets.
const URL_CELLS: usize = 4096;
const URL_BYTES: usize = 16 * 1024;

/// The OSC 8 target at `point`, else a URL in its wrapped line; a prompt's own link is no target.
pub fn link_at<T>(term: &Term<T>, point: Point) -> Option<String> {
    let point = character_at(term, point);
    if let Some(link) = term.grid()[point]
        .hyperlink()
        .filter(|link| link.uri() != prompts::LINK)
    {
        return Some(link.uri().to_string());
    }
    url_at(term, point).map(|(url, _, _)| url.to_string())
}

fn character_at<T>(term: &Term<T>, mut point: Point) -> Point {
    if term.grid()[point].flags.contains(Flags::WIDE_CHAR_SPACER) {
        point.column -= 1;
    }
    point
}

pub(super) fn hit_at<T>(term: &Term<T>, point: Point) -> Option<RangeInclusive<Point>> {
    let grid = term.grid();
    let top = Line(-(grid.display_offset() as i32));
    let first_visible = Point::new(top, Column(0));
    let last_visible = Point::new(top + term.screen_lines() - 1, term.last_column());
    if !(first_visible..=last_visible).contains(&point) {
        return None;
    }
    let point = character_at(term, point);
    if let Some(link) = grid[point]
        .hyperlink()
        .filter(|link| link.uri() != prompts::LINK)
    {
        let (mut first, mut last) = (point, point);
        while first > first_visible {
            let previous = first.sub(term, Boundary::Grid, 1);
            if grid[previous].hyperlink().as_ref() != Some(&link) {
                break;
            }
            first = previous;
        }
        while last < last_visible {
            let next = last.add(term, Boundary::Grid, 1);
            if grid[next].hyperlink().as_ref() != Some(&link) {
                break;
            }
            last = next;
        }
        return Some(first..=last);
    }
    let (_, bytes, mut cell) = url_at(term, point)?;
    let (mut byte, mut first) = (0, None);
    for _ in 0..URL_CELLS {
        let content = &grid[cell];
        if !spacer(content) {
            let end = byte
                + content.c.len_utf8()
                + content
                    .zerowidth()
                    .into_iter()
                    .flatten()
                    .map(|ch| ch.len_utf8())
                    .sum::<usize>();
            if end > bytes.start {
                first.get_or_insert(cell);
            }
            if end >= bytes.end {
                let last = if content.flags.contains(Flags::WIDE_CHAR) {
                    cell.add(term, Boundary::Grid, 1)
                } else {
                    cell
                };
                return Some(first?..=last);
            }
            byte = end;
        }
        cell = cell.add(term, Boundary::Grid, 1);
    }
    None
}

fn spacer(cell: &Cell) -> bool {
    cell.flags
        .intersects(Flags::WIDE_CHAR_SPACER | Flags::LEADING_WIDE_CHAR_SPACER)
}

fn token<T>(term: &Term<T>, point: Point) -> Option<RangeInclusive<Point>> {
    let grid = term.grid();
    let separator = |point: Point| {
        let cell = &grid[point];
        !spacer(cell) && (cell.c.is_whitespace() || matches!(cell.c, '<' | '>' | '"' | '\'' | '`'))
    };
    if separator(point) {
        return None;
    }
    let (mut first, mut last, mut count) = (point, point, 1);
    loop {
        let previous = first.sub(term, Boundary::Grid, 1);
        if previous == first
            || (first.column == Column(0) && !grid[previous].flags.contains(Flags::WRAPLINE))
            || separator(previous)
        {
            break;
        }
        if count == URL_CELLS {
            return None;
        }
        first = previous;
        count += 1;
    }
    loop {
        let next = last.add(term, Boundary::Grid, 1);
        if next == last
            || (last.column == term.last_column() && !grid[last].flags.contains(Flags::WRAPLINE))
            || separator(next)
        {
            break;
        }
        if count == URL_CELLS {
            return None;
        }
        last = next;
        count += 1;
    }
    Some(first..=last)
}

fn url_at<T>(term: &Term<T>, point: Point) -> Option<(SharedString, Range<usize>, Point)> {
    let grid = term.grid();
    let token = token(term, point)?;
    let mut cell = *token.start();
    let (mut text, mut at) = (String::new(), 0);
    loop {
        if cell == point {
            at = text.len();
        }
        if !spacer(&grid[cell]) {
            for ch in
                std::iter::once(&grid[cell].c).chain(grid[cell].zerowidth().into_iter().flatten())
            {
                if text.len() + ch.len_utf8() > URL_BYTES {
                    return None;
                }
                text.push(*ch);
            }
        }
        if cell == *token.end() {
            break;
        }
        cell = cell.add(term, Boundary::Grid, 1);
    }
    links(&text)
        .into_iter()
        .find_map(|(bytes, target)| match target {
            Target::Url(url) if bytes.contains(&at) => Some((url, bytes, *token.start())),
            _ => None,
        })
}

#[cfg(test)]
mod tests {
    use alacritty_terminal::{
        event::VoidListener,
        index::{Column, Line},
        term::{Config, test::TermSize},
        vte::ansi::Processor,
    };

    use super::*;

    fn link(bytes: &str, line: i32, column: usize) -> Option<String> {
        let mut term = Term::new(Config::default(), &TermSize::new(12, 4), VoidListener);
        let mut parser: Processor = Processor::new();
        parser.advance(&mut term, bytes.as_bytes());
        link_at(&term, Point::new(Line(line), Column(column)))
    }

    #[test]
    fn an_osc_8_link_names_its_target() {
        let text = "a \x1b]8;;https://ely.dev/x\x1b\\ely\x1b]8;;\x1b\\ b";
        assert_eq!(link(text, 0, 3).as_deref(), Some("https://ely.dev/x"));
        assert_eq!(link(text, 0, 0), None);
    }

    #[test]
    fn a_prompt_s_own_link_is_no_target() {
        let text = "\x1b]8;;\x7f\x1b\\$ https://ely.dev\x1b]8;;\x1b\\";
        assert_eq!(link(text, 0, 0), None);
        assert_eq!(link(text, 0, 4).as_deref(), Some("https://ely.dev"));
    }

    #[test]
    fn a_url_ends_before_its_sentence_does() {
        assert_eq!(link("(a.io/x).", 0, 2), None, "no scheme, no link");
        let text = "(http://a.b)";
        assert_eq!(link(text, 0, 4).as_deref(), Some("http://a.b"));
        assert_eq!(link(text, 0, 11), None);
    }

    #[test]
    fn a_url_keeps_its_combining_marks() {
        let text = "https://e.test/cafe\u{301}/x";
        assert_eq!(link(text, 1, 1).as_deref(), Some(text));
    }

    #[test]
    fn a_url_runs_across_wrapped_rows_and_wide_cells() {
        let text = "\u{4e2d} https://ely.dev/abc";
        assert_eq!(link(text, 1, 3).as_deref(), Some("https://ely.dev/abc"));
        assert_eq!(link(text, 0, 3).as_deref(), Some("https://ely.dev/abc"));
        assert_eq!(link(text, 0, 1), None);
        let wide = "file://a/\u{4e2d} x";
        assert_eq!(
            link(wide, 0, 10).as_deref(),
            Some("file://a/\u{4e2d}"),
            "a wide cell's spacer is its char"
        );
    }

    fn hit(text: &str, line: i32, column: usize) -> RangeInclusive<Point> {
        let mut term = Term::new(Config::default(), &TermSize::new(12, 4), VoidListener);
        <Processor>::new().advance(&mut term, text.as_bytes());
        hit_at(&term, at(line, column)).expect("a link")
    }

    fn at(line: i32, column: usize) -> Point {
        Point::new(Line(line), Column(column))
    }

    #[test]
    fn hover_covers_the_wrapped_url_without_its_punctuation() {
        assert_eq!(hit("中 (http://a.b).", 0, 8), at(0, 4)..=at(1, 1));
        assert_eq!(hit("file://a/中 x", 0, 10), at(0, 0)..=at(0, 10));
        assert_eq!(
            hit("https://e.test/cafe\u{301}/x", 1, 1),
            at(0, 0)..=at(1, 8)
        );
    }

    #[test]
    fn osc_8_hover_covers_its_label_and_keeps_separate_links_apart() {
        let text = "a \x1b]8;;https://a.b\x07abcdefghijklmn\x1b]8;;\x07 z";
        assert_eq!(hit(text, 1, 1), at(0, 2)..=at(1, 3));
        let text = "\x1b]8;;https://a.b\x07one\x1b]8;;\x07 \x1b]8;;https://a.b\x07two\x1b]8;;\x07";
        assert_eq!(hit(text, 0, 1), at(0, 0)..=at(0, 2));
        assert_eq!(hit(text, 0, 5), at(0, 4)..=at(0, 6));
    }

    #[test]
    fn a_long_osc_8_link_is_searched_only_inside_the_viewport() {
        use alacritty_terminal::grid::Scroll;

        let mut term = Term::new(Config::default(), &TermSize::new(12, 4), VoidListener);
        let text = format!(
            "\x1b]8;;https://a.b\x07{}\x1b]8;;\x07",
            "x".repeat(12 * 2048)
        );
        <Processor>::new().advance(&mut term, text.as_bytes());
        assert!(term.grid().history_size() > 1000);
        for scroll in [Scroll::Bottom, Scroll::Delta(100), Scroll::Top] {
            term.scroll_display(scroll);
            let top = -(term.grid().display_offset() as i32);
            let hover = hit_at(&term, at(top + 1, 5)).expect("a visible link");
            assert_eq!(hover, at(top, 0)..=at(top + 3, 11));
            let outside = if top == 0 {
                Point::new(term.grid().topmost_line(), Column(0))
            } else {
                Point::new(term.grid().bottommost_line(), Column(0))
            };
            assert!(hit_at(&term, outside).is_none());
            assert_eq!(link_at(&term, outside).as_deref(), Some("https://a.b"));
        }
    }

    fn parsed(text: &str) -> Term<VoidListener> {
        let mut term = Term::new(Config::default(), &TermSize::new(12, 4), VoidListener);
        <Processor>::new().advance(&mut term, text.as_bytes());
        term
    }

    #[test]
    fn a_long_plain_logical_line_stops_at_the_cell_budget() {
        use alacritty_terminal::grid::Scroll;

        let mut term = parsed(&"x".repeat(12 * 2048));
        for scroll in [Scroll::Bottom, Scroll::Delta(100), Scroll::Top] {
            term.scroll_display(scroll);
            let point = at(-(term.grid().display_offset() as i32) + 1, 5);
            assert!(token(&term, point).is_none());
            assert!(hit_at(&term, point).is_none());
            assert!(link_at(&term, point).is_none());
        }
    }

    #[test]
    fn an_oversized_url_is_not_opened_or_underlined_as_a_truncated_url() {
        let url = format!("https://a.b/{}", "x".repeat(URL_CELLS));
        let term = parsed(&url);
        for point in [Point::new(term.grid().topmost_line(), Column(3)), at(2, 5)] {
            assert!(link_at(&term, point).is_none());
        }
        assert!(hit_at(&term, at(2, 5)).is_none());
        let url = format!("https://a.b/{}", "x".repeat(URL_CELLS - 12));
        let term = parsed(&url);
        assert_eq!(link_at(&term, at(2, 5)).as_deref(), Some(url.as_str()));
    }

    #[test]
    fn a_short_wrapped_url_is_found_inside_a_long_logical_line() {
        let url = "https://a.b/cafe\u{301}/中";
        let term = parsed(&format!("{} ({url}).", "x".repeat(URL_CELLS * 2)));
        let point = at(2, 0);
        assert_eq!(link_at(&term, point).as_deref(), Some(url));
        assert!(hit_at(&term, point).is_some());
    }

    #[test]
    fn hard_newlines_do_not_join_url_fragments() {
        let term = parsed("https://a.b/\r\npath");
        assert!(link_at(&term, at(1, 1)).is_none());
        assert_eq!(link_at(&term, at(0, 4)).as_deref(), Some("https://a.b/"));
    }

    #[test]
    fn combining_marks_cannot_exceed_the_text_budget() {
        let term = parsed(&format!("https://a.b/e{}", "\u{301}".repeat(URL_BYTES)));
        assert!(token(&term, at(0, 5)).is_some());
        assert!(url_at(&term, at(0, 5)).is_none());
        assert!(link_at(&term, at(0, 5)).is_none());
        assert!(hit_at(&term, at(0, 5)).is_none());
    }

    #[test]
    fn a_short_url_in_compact_json_is_not_rejected_with_a_long_field() {
        let term = parsed(&format!(
            r#"{{"padding":"{}","url":"https://a.b"}}"#,
            "中".repeat(5000)
        ));
        let point = term
            .renderable_content()
            .display_iter
            .find(|cell| cell.cell.c == 'h')
            .expect("the URL is visible")
            .point;
        assert_eq!(link_at(&term, point).as_deref(), Some("https://a.b"));
        assert_eq!(
            hit_at(&term, point),
            Some(point..=point.add(&term, Boundary::Grid, 10))
        );
    }

    #[test]
    fn url_delimiters_isolate_long_neighbors_on_both_sides() {
        use alacritty_terminal::grid::Scroll;

        for delimiter in ['"', '\'', '<', '>', '`'] {
            for trailing in [false, true] {
                let padding = "x".repeat(5000);
                let text = if trailing {
                    format!("{delimiter}https://a.b{delimiter}{padding}")
                } else {
                    format!("{padding}{delimiter}https://a.b{delimiter}")
                };
                let mut term = parsed(&text);
                if trailing {
                    term.scroll_display(Scroll::Top);
                }
                let point = term
                    .renderable_content()
                    .display_iter
                    .find(|cell| cell.cell.c == 'h')
                    .expect("the URL is visible")
                    .point;
                assert_eq!(link_at(&term, point).as_deref(), Some("https://a.b"));
                assert_eq!(
                    hit_at(&term, point),
                    Some(point..=point.add(&term, Boundary::Grid, 10))
                );
                for edge in [
                    point.sub(&term, Boundary::Grid, 1),
                    point.add(&term, Boundary::Grid, 11),
                ] {
                    assert!(link_at(&term, edge).is_none());
                    assert!(hit_at(&term, edge).is_none());
                }
            }
        }
    }
}
