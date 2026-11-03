use alacritty_terminal::{
    grid::Dimensions,
    index::{Column, Direction, Point},
    term::{
        Term,
        search::{Match, RegexIter, RegexSearch},
    },
};
use gpui::Context;

use super::view::Terminal;
use crate::editor::FindOptions;

/// A find's matches over the scrollback and the screen, and the one stepped to.
#[derive(Default)]
pub(crate) struct Search {
    all: Vec<Match>,
    current: Option<usize>,
}

impl Search {
    pub(crate) fn visible(&self) -> (&[Match], Option<usize>) {
        (&self.all, self.current)
    }
}

/// `query` under `options` as the grid's search reads it; word edges are ASCII, which its lazy DFA needs.
fn pattern(query: &str, options: FindOptions) -> String {
    let body = if options.regex {
        query.to_string()
    } else {
        regex::escape(query)
    };
    let body = if options.word {
        format!(r"(?-u:\b)(?:{body})(?-u:\b)")
    } else {
        body
    };
    let case = if options.case { "(?-i)" } else { "(?i)" };
    format!("{case}{body}")
}

/// Every match of `pattern` from the oldest line to the newest.
fn search_all<T>(term: &Term<T>, pattern: &str) -> Result<Vec<Match>, String> {
    let mut regex = RegexSearch::new(pattern).map_err(|error| error.to_string())?;
    let start = Point::new(term.topmost_line(), Column(0));
    let end = Point::new(term.bottommost_line(), term.last_column());
    Ok(RegexIter::new(start, end, Direction::Right, term, &mut regex).collect())
}

impl Terminal {
    /// Finds `query` in the scrollback and the screen and steps to the newest match; a bad pattern is an error to show.
    pub fn find(
        &mut self,
        query: &str,
        options: FindOptions,
        cx: &mut Context<Self>,
    ) -> Result<usize, String> {
        let all = if query.is_empty() {
            Vec::new()
        } else {
            search_all(&self.term.lock(), &pattern(query, options))?
        };
        log::debug!("terminal: {} matches of {query:?}", all.len());
        self.search = Search {
            current: all.len().checked_sub(1),
            all,
        };
        self.reveal(cx);
        Ok(self.search.all.len())
    }

    /// Steps to the next match, or the one before, round the ends.
    pub fn step(&mut self, forward: bool, cx: &mut Context<Self>) {
        let count = self.search.all.len();
        let Some(at) = self.search.current else {
            return;
        };
        self.search.current = Some(if forward {
            (at + 1) % count
        } else {
            (at + count - 1) % count
        });
        self.reveal(cx);
    }

    /// How many matches there are, and which one is current.
    pub fn matches(&self) -> (usize, Option<usize>) {
        (self.search.all.len(), self.search.current)
    }

    fn reveal(&mut self, cx: &mut Context<Self>) {
        if let Some(at) = self.search.current {
            let start = *self.search.all[at].start();
            self.term.lock().scroll_to_point(start);
        }
        cx.notify();
    }
}

#[cfg(test)]
mod tests {
    use alacritty_terminal::term::test::mock_term;

    use super::*;

    #[test]
    fn find_takes_case_words_and_patterns() {
        let term = mock_term("one two One\r\nthree phone one");
        let count = |query: &str, options: FindOptions| {
            search_all(&term, &pattern(query, options)).map(|all| all.len())
        };
        assert_eq!(count("one", FindOptions::default()), Ok(4));
        let case = FindOptions {
            case: true,
            ..FindOptions::default()
        };
        assert_eq!(count("One", case), Ok(1));
        let word = FindOptions {
            word: true,
            ..FindOptions::default()
        };
        assert_eq!(count("one", word), Ok(3), "phone holds no whole word");
        let regex = FindOptions {
            regex: true,
            ..FindOptions::default()
        };
        assert_eq!(count("t[wh]", regex), Ok(2));
        assert!(count("(", regex).is_err());
    }
}
