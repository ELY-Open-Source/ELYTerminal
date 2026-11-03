use gpui::SharedString;

/// A value in a document: nothing, a flag, a number, a text, or a list or a map of values in their order.
#[derive(Clone, Debug, PartialEq)]
pub enum Datum {
    Null,
    Bool(bool),
    Number(SharedString),
    Text(SharedString),
    List(Vec<Datum>),
    Map(Vec<(SharedString, Datum)>),
}

impl From<&serde_json::Value> for Datum {
    fn from(value: &serde_json::Value) -> Self {
        match value {
            serde_json::Value::Null => Datum::Null,
            serde_json::Value::Bool(on) => Datum::Bool(*on),
            serde_json::Value::Number(number) => Datum::Number(number.to_string().into()),
            serde_json::Value::String(text) => Datum::Text(text.clone().into()),
            serde_json::Value::Array(items) => Datum::List(items.iter().map(Datum::from).collect()),
            serde_json::Value::Object(map) => Datum::Map(
                map.iter()
                    .map(|(key, value)| (key.clone().into(), Datum::from(value)))
                    .collect(),
            ),
        }
    }
}

/// A line that failed to read, and why.
#[derive(Clone, Debug, PartialEq)]
pub struct Unread {
    pub line: usize,
    pub why: String,
}

impl std::fmt::Display for Unread {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Line {}: {}", self.line, self.why)
    }
}

/// JSON read into a value, keys in their written order.
pub fn read_json(text: &str) -> Result<Datum, Unread> {
    serde_json::from_str::<serde_json::Value>(text)
        .map(|value| Datum::from(&value))
        .map_err(|error| Unread {
            line: error.line(),
            why: error.to_string(),
        })
}

/// The character `digits` hex digits after an escape name.
fn hex(chars: &mut std::str::Chars, digits: usize, escape: char) -> Result<char, String> {
    let code: String = chars.by_ref().take(digits).collect();
    Some(&code)
        .filter(|code| code.len() == digits && code.chars().all(|ch| ch.is_ascii_hexdigit()))
        .and_then(|code| u32::from_str_radix(code, 16).ok())
        .and_then(char::from_u32)
        .ok_or_else(|| format!("\\{escape}{code} is no character"))
}

/// The inside of a quoted scalar with its escapes read: YAML's in double quotes, `''` in single ones. Plain text is `None`.
fn unquoted(text: &str) -> Result<Option<String>, String> {
    if let Some(inner) = text
        .strip_prefix('\'')
        .and_then(|rest| rest.strip_suffix('\''))
    {
        return Ok(Some(inner.replace("''", "'")));
    }
    let Some(inner) = text
        .strip_prefix('"')
        .and_then(|rest| rest.strip_suffix('"'))
    else {
        return Ok(None);
    };
    let mut out = String::with_capacity(inner.len());
    let mut chars = inner.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }
        let Some(escape) = chars.next() else {
            return Err("a quoted text ends in a lone \\".into());
        };
        out.push(match escape {
            '0' => '\0',
            'a' => '\x07',
            'b' => '\x08',
            't' | '\t' => '\t',
            'n' => '\n',
            'v' => '\x0b',
            'f' => '\x0c',
            'r' => '\r',
            'e' => '\x1b',
            ' ' | '"' | '/' | '\\' => escape,
            'N' => '\u{85}',
            '_' => '\u{a0}',
            'L' => '\u{2028}',
            'P' => '\u{2029}',
            'x' => hex(&mut chars, 2, escape)?,
            'u' => hex(&mut chars, 4, escape)?,
            'U' => hex(&mut chars, 8, escape)?,
            other => return Err(format!("\\{other} is no escape YAML knows")),
        });
    }
    Ok(Some(out))
}

/// A scalar's value, or why its quotes do not read.
fn scalar(text: &str, line: usize) -> Result<Datum, Unread> {
    let text = text.trim();
    if text.starts_with(['|', '>'])
        && text[1..]
            .chars()
            .all(|ch| matches!(ch, '-' | '+' | '0'..='9'))
    {
        return Err(Unread {
            line,
            why: "block text after | or > is not read".into(),
        });
    }
    let quoted = unquoted(text).map_err(|why| Unread { line, why })?;
    Ok(match (quoted, text) {
        (Some(inner), _) => Datum::Text(inner.into()),
        (None, "" | "~" | "null") => Datum::Null,
        (None, "true") => Datum::Bool(true),
        (None, "false") => Datum::Bool(false),
        (None, _) if text.parse::<f64>().is_ok() => Datum::Number(text.to_string().into()),
        (None, _) => Datum::Text(text.to_string().into()),
    })
}

/// Each character's offset and whether it stands outside a quoted scalar. A quote opens only at the line's start or after whitespace; `\` escapes inside double quotes and `''` is one quote inside single ones.
fn outside(line: &str) -> Vec<(usize, char, bool)> {
    let mut marks = Vec::with_capacity(line.len());
    let mut chars = line.char_indices().peekable();
    let (mut quote, mut last) = (None, ' ');
    while let Some((at, ch)) = chars.next() {
        let escaped = match quote {
            Some('"') => ch == '\\',
            Some('\'') => ch == '\'' && chars.peek().is_some_and(|(_, next)| *next == '\''),
            _ => false,
        };
        if escaped {
            marks.push((at, ch, false));
            if let Some((next_at, next)) = chars.next() {
                marks.push((next_at, next, false));
                last = next;
            }
            continue;
        }
        let free = match quote {
            Some(open) if ch == open => {
                quote = None;
                false
            }
            Some(_) => false,
            None if matches!(ch, '"' | '\'') && last.is_whitespace() => {
                quote = Some(ch);
                false
            }
            None => true,
        };
        marks.push((at, ch, free));
        last = ch;
    }
    marks
}

/// Where a line's comment starts: a `#` at its start or after whitespace, outside a quoted scalar.
fn comment(line: &str) -> Option<usize> {
    let marks = outside(line);
    (0..marks.len())
        .find(|&ix| {
            let (_, ch, free) = marks[ix];
            free && ch == '#' && (ix == 0 || marks[ix - 1].1.is_whitespace())
        })
        .map(|ix| marks[ix].0)
}

/// A map entry's key and value, split at the first colon outside quotes that ends the line or comes before a space.
fn entry_parts(words: &str) -> Option<(&str, &str)> {
    let marks = outside(words);
    (0..marks.len())
        .find(|&ix| {
            let (_, ch, free) = marks[ix];
            free && ch == ':' && marks.get(ix + 1).is_none_or(|(_, next, _)| *next == ' ')
        })
        .map(|ix| {
            let at = marks[ix].0;
            (&words[..at], &words[at + 1..])
        })
}

/// Whether a line starts a list item.
fn dashed(words: &str) -> bool {
    let words = words.trim_start();
    words.starts_with("- ") || words == "-"
}

/// Lines of a YAML block: each one's number, indent and words, without comments and blank lines.
fn lines(text: &str) -> Vec<(usize, usize, &str)> {
    text.lines()
        .enumerate()
        .filter_map(|(ix, line)| {
            let words = &line[..comment(line).unwrap_or(line.len())];
            let indent = words.len() - words.trim_start().len();
            (!words.trim().is_empty() && words.trim() != "---")
                .then(|| (ix + 1, indent, words.trim_end()))
        })
        .collect()
}

/// The block that starts at `at` and holds the lines at `indent`.
fn block(lines: &[(usize, usize, &str)], at: &mut usize, indent: usize) -> Result<Datum, Unread> {
    if dashed(lines[*at].2) {
        let mut items = Vec::new();
        while *at < lines.len() && lines[*at].1 == indent && dashed(lines[*at].2) {
            let (number, _, words) = lines[*at];
            let rest = words.trim_start()[1..].trim_start();
            *at += 1;
            items.push(if rest.is_empty() {
                nested(lines, at, indent)?
            } else if entry_parts(rest).is_some() {
                let inner = indent + words.trim_start().len() - rest.len();
                entries(lines, at, inner, Some((number, rest)))?
            } else {
                scalar(rest, number)?
            });
        }
        Ok(Datum::List(items))
    } else {
        entries(lines, at, indent, None)
    }
}

/// What lies under a key or a dash with nothing after it: a deeper block, or nothing.
fn nested(lines: &[(usize, usize, &str)], at: &mut usize, indent: usize) -> Result<Datum, Unread> {
    match lines.get(*at) {
        Some((_, deeper, _)) if *deeper > indent => block(lines, at, *deeper),
        _ => Ok(Datum::Null),
    }
}

/// A map's entries at `indent`, the first one given when it sat after a dash.
fn entries(
    lines: &[(usize, usize, &str)],
    at: &mut usize,
    indent: usize,
    first: Option<(usize, &str)>,
) -> Result<Datum, Unread> {
    let mut map = Vec::new();
    let mut entry = |number: usize, words: &str, at: &mut usize| -> Result<(), Unread> {
        let Some((key, value)) = entry_parts(words) else {
            return Err(Unread {
                line: number,
                why: format!("{words:?} is no key and value"),
            });
        };
        let key = match unquoted(key.trim()).map_err(|why| Unread { line: number, why })? {
            Some(inner) => SharedString::from(inner),
            None => SharedString::from(key.trim().to_string()),
        };
        let value = match lines.get(*at) {
            _ if !value.trim().is_empty() => scalar(value, number)?,
            Some((_, same, next)) if *same == indent && dashed(next) => block(lines, at, indent)?,
            _ => nested(lines, at, indent)?,
        };
        map.push((key, value));
        Ok(())
    };
    if let Some((number, words)) = first {
        entry(number, words, at)?;
    }
    while *at < lines.len() && lines[*at].1 == indent {
        let (number, _, words) = lines[*at];
        *at += 1;
        entry(number, words.trim_start(), at)?;
    }
    if let Some((number, deeper, _)) = lines.get(*at)
        && *deeper > indent
    {
        return Err(Unread {
            line: *number,
            why: "this line is indented further than its block".into(),
        });
    }
    Ok(Datum::Map(map))
}

/// YAML's block style read into a value: maps, lists and scalars, with comments dropped. Flow style reads as text.
pub fn read_yaml(text: &str) -> Result<Datum, Unread> {
    let lines = lines(text);
    if lines.is_empty() {
        return Ok(Datum::Null);
    }
    let mut at = 0;
    let datum = block(&lines, &mut at, lines[0].1)?;
    match lines.get(at) {
        Some((number, _, _)) => Err(Unread {
            line: *number,
            why: "this line sits outside the document's first block".into(),
        }),
        None => Ok(datum),
    }
}

#[cfg(test)]
mod tests {
    use super::{Datum, read_json, read_yaml};

    fn text(words: &str) -> Datum {
        Datum::Text(words.to_string().into())
    }

    #[test]
    fn json_keeps_its_keys_in_order_and_says_where_it_breaks() {
        let datum = read_json(r#"{"b": 1, "a": [true, null, "x"]}"#).expect("json");
        let Datum::Map(entries) = datum else {
            panic!("a map")
        };
        assert_eq!(entries[0].0, "b");
        assert_eq!(
            entries[1].1,
            Datum::List(vec![Datum::Bool(true), Datum::Null, text("x")])
        );
        assert_eq!(
            read_json("{\n  \"a\": 1,\n}")
                .expect_err("a trailing comma")
                .line,
            3
        );
    }

    #[test]
    fn yaml_reads_nested_maps_lists_and_maps_in_lists() {
        let datum = read_yaml(
            "apiVersion: apps/v1 # the api\nkind: Deployment\nspec:\n  replicas: 3\n  ports:\n    - 80\n    - name: https\n      port: 443\n",
        )
        .expect("yaml");
        let Datum::Map(entries) = datum else {
            panic!("a map")
        };
        assert_eq!(entries[0], ("apiVersion".into(), text("apps/v1")));
        let Datum::Map(spec) = &entries[2].1 else {
            panic!("spec is a map")
        };
        assert_eq!(spec[0], ("replicas".into(), Datum::Number("3".into())));
        let Datum::List(ports) = &spec[1].1 else {
            panic!("ports is a list")
        };
        assert_eq!(ports[0], Datum::Number("80".into()));
        assert_eq!(
            ports[1],
            Datum::Map(vec![
                ("name".into(), text("https")),
                ("port".into(), Datum::Number("443".into()))
            ])
        );
    }

    #[test]
    fn yaml_reads_a_list_at_its_key_indent() {
        let steps = |run: &str| Datum::Map(vec![("run".into(), text(run))]);
        assert_eq!(
            read_yaml("steps:\n- run: build\n- run: test\nname: ci\n").expect("yaml"),
            Datum::Map(vec![
                (
                    "steps".into(),
                    Datum::List(vec![steps("build"), steps("test")])
                ),
                ("name".into(), text("ci"))
            ])
        );
        assert_eq!(
            read_yaml("ports:\n- 80\n- 443\n").expect("yaml"),
            Datum::Map(vec![(
                "ports".into(),
                Datum::List(vec![
                    Datum::Number("80".into()),
                    Datum::Number("443".into())
                ])
            )])
        );
    }

    #[test]
    fn a_hash_inside_quotes_is_no_comment() {
        assert_eq!(
            read_yaml(concat!(
                "title: \"Issue #12\" # a note\n",
                "message: 'Fix #42'\n",
                "note: it's #1\n",
                "url: http://x/#frag\n",
                "v: a#b\n",
                "a: \"x \\\" # y\" # c\n",
                "b: 'it''s #1' # c\n",
            ))
            .expect("yaml"),
            Datum::Map(vec![
                ("title".into(), text("Issue #12")),
                ("message".into(), text("Fix #42")),
                ("note".into(), text("it's")),
                ("url".into(), text("http://x/#frag")),
                ("v".into(), text("a#b")),
                ("a".into(), text("x \" # y")),
                ("b".into(), text("it's #1")),
            ])
        );
    }

    #[test]
    fn every_escape_reads_and_a_lone_backslash_fails() {
        assert_eq!(
            read_yaml(concat!(
                r#"a: "c:\\dir\/\n\r\0""#,
                "\n",
                r#"b: "\a\b\e\f\v\N\_\L\P\ \x41\u00e9\U0001F600""#,
                "\n",
                "c: \"x\\\ty\"\n",
            ))
            .expect("yaml"),
            Datum::Map(vec![
                ("a".into(), text("c:\\dir/\n\r\0")),
                (
                    "b".into(),
                    text("\x07\x08\x1b\x0c\x0b\u{85}\u{a0}\u{2028}\u{2029} Aé😀")
                ),
                ("c".into(), text("x\ty")),
            ])
        );
        let lone = read_yaml(r#"a: "abc\""#).expect_err("a lone backslash");
        assert_eq!(
            (lone.line, lone.why.as_str()),
            (1, "a quoted text ends in a lone \\")
        );
        let short = read_yaml(r#"a: "\u12""#).expect_err("a short code");
        assert_eq!(short.why, "\\u12 is no character");
    }

    #[test]
    fn a_quoted_key_or_item_keeps_its_colon() {
        assert_eq!(
            read_yaml("\"note: important\": true\nlist:\n  - \"a: b\"\n  - c: d\n").expect("yaml"),
            Datum::Map(vec![
                ("note: important".into(), Datum::Bool(true)),
                (
                    "list".into(),
                    Datum::List(vec![
                        text("a: b"),
                        Datum::Map(vec![("c".into(), text("d"))])
                    ])
                ),
            ])
        );
    }

    #[test]
    fn quoted_text_reads_its_escapes_and_keys_lose_their_quotes() {
        assert_eq!(
            read_yaml("a: \"say \\\"hi\\\"\"\nb: 'it''s'\nc: \"tab\\tx\"\n'q': 2\n\"k\": 1\n")
                .expect("yaml"),
            Datum::Map(vec![
                ("a".into(), text("say \"hi\"")),
                ("b".into(), text("it's")),
                ("c".into(), text("tab\tx")),
                ("q".into(), Datum::Number("2".into())),
                ("k".into(), Datum::Number("1".into())),
            ])
        );
        let unread = read_yaml("a: 1\nb: \"\\q\"\n").expect_err("an unknown escape");
        assert_eq!(
            (unread.line, unread.why.as_str()),
            (2, "\\q is no escape YAML knows")
        );
    }

    #[test]
    fn yaml_says_which_line_it_could_not_read() {
        assert_eq!(read_yaml("a: 1\njust words\n").expect_err("no key").line, 2);
        let block = read_yaml("a: 1\nrun: |-\n  make\n").expect_err("block text");
        assert_eq!(
            (block.line, block.why.as_str()),
            (2, "block text after | or > is not read")
        );
        assert_eq!(
            read_yaml("a: 1\n    b: 2\n")
                .expect_err("stray indent")
                .line,
            2
        );
    }
}
