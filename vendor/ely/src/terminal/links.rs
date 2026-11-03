use std::{ops::Range, sync::LazyLock};

use gpui::SharedString;
use regex::Regex;

/// What a link in terminal text points at.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Target {
    Url(SharedString),
    /// A file, with the line and column a compiler or a grep names, counted from one.
    Path {
        path: SharedString,
        line: Option<u32>,
        column: Option<u32>,
    },
}

static URL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"\b(?:https?|file)://[^\s<>"'`]+"#).expect("the URL pattern compiles")
});

static PATH: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?:\.{1,2}/|/|~/)?(?:[\w.-]+/)+[\w.-]+\.\w+(?::(\d+))?(?::(\d+))?")
        .expect("the path pattern compiles")
});

/// Links in one line of text: URLs, and paths with a folder and an extension. Byte ranges, in order.
pub fn links(text: &str) -> Vec<(Range<usize>, Target)> {
    let mut found: Vec<(Range<usize>, Target)> = URL
        .find_iter(text)
        .map(|hit| {
            let end = hit.start() + trimmed(hit.as_str()).len();
            let url = &text[hit.start()..end];
            (hit.start()..end, Target::Url(url.to_string().into()))
        })
        .collect();
    for hit in PATH.captures_iter(text) {
        let whole = hit.get(0).expect("a capture has its match");
        if found
            .iter()
            .any(|(range, _)| range.contains(&whole.start()))
        {
            continue;
        }
        let number = |group: usize| {
            hit.get(group)
                .and_then(|digits| digits.as_str().parse().ok())
        };
        let (line, column) = (number(1), number(2));
        let suffix = [hit.get(1), hit.get(2)]
            .iter()
            .flatten()
            .map(|digits| digits.len() + 1)
            .sum::<usize>();
        let path = &text[whole.start()..whole.end() - suffix];
        found.push((
            whole.range(),
            Target::Path {
                path: path.to_string().into(),
                line,
                column,
            },
        ));
    }
    found.sort_by_key(|(range, _)| range.start);
    found
}

/// A URL without the punctuation that ends its sentence, keeping a closing bracket it opened.
fn trimmed(url: &str) -> &str {
    let mut end = url.len();
    while let Some(last) = url[..end].chars().last() {
        let open = match last {
            '.' | ',' | ';' | ':' | '!' | '?' => None,
            ')' => Some('('),
            ']' => Some('['),
            _ => break,
        };
        if let Some(open) = open
            && url[..end].matches(open).count() >= url[..end].matches(last).count()
        {
            break;
        }
        end -= last.len_utf8();
    }
    &url[..end]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urls_drop_the_punctuation_that_ends_their_sentence() {
        let found =
            links("see https://ely.dev/docs. or (https://a.b/c) or https://w.org/Rust_(language)");
        let urls: Vec<_> = found
            .iter()
            .map(|(_, target)| match target {
                Target::Url(url) => url.to_string(),
                other => panic!("a URL, not {other:?}"),
            })
            .collect();
        assert_eq!(
            urls,
            [
                "https://ely.dev/docs",
                "https://a.b/c",
                "https://w.org/Rust_(language)"
            ]
        );
    }

    #[test]
    fn paths_carry_the_line_and_column_a_compiler_names() {
        let text = "   --> src/main.rs:46:22";
        let found = links(text);
        assert_eq!(found.len(), 1);
        let (range, target) = &found[0];
        assert_eq!(&text[range.clone()], "src/main.rs:46:22");
        assert_eq!(
            *target,
            Target::Path {
                path: "src/main.rs".into(),
                line: Some(46),
                column: Some(22),
            }
        );
        assert!(links("a.b and v1.2").is_empty(), "a path needs a folder");
        let huge = links("src/a.rs:99999999999");
        assert_eq!(
            huge[0].1,
            Target::Path {
                path: "src/a.rs".into(),
                line: None,
                column: None
            },
            "no line past u32"
        );
    }
}
