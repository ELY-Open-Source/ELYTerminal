use markdown::decode_named;

use super::Style;

/// One HTML tag as written: its lowercase name and attributes.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Tag {
    pub name: String,
    pub closing: bool,
    pub attributes: Vec<(String, String)>,
}

impl Tag {
    pub fn attribute(&self, name: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }
}

/// How Typora shows an inline tag; others show text.
pub(crate) fn inline_style(name: &str) -> Option<Style> {
    match name {
        "u" => Some(Style::Underline),
        "b" | "strong" => Some(Style::Strong),
        "i" | "em" => Some(Style::Emphasis),
        "s" | "del" | "strike" => Some(Style::Strike),
        "code" | "kbd" => Some(Style::Code),
        "mark" => Some(Style::Mark),
        "a" => Some(Style::Link),
        _ => None,
    }
}

/// HTML5's formatting elements reopen past a crossed close.
pub(crate) fn reopens(name: &str) -> bool {
    matches!(
        name,
        "a" | "b" | "code" | "em" | "i" | "s" | "strike" | "strong" | "u"
    )
}

/// GFM's tag filter: these tags show as text.
pub(crate) fn filtered(name: &str) -> bool {
    matches!(
        name,
        "title"
            | "textarea"
            | "style"
            | "xmp"
            | "iframe"
            | "noembed"
            | "noframes"
            | "script"
            | "plaintext"
    )
}

/// Elements a browser lays as blocks: a line each side.
pub(crate) fn breaks(name: &str) -> bool {
    matches!(
        name,
        "address"
            | "article"
            | "aside"
            | "blockquote"
            | "center"
            | "details"
            | "div"
            | "dl"
            | "dd"
            | "dt"
            | "figcaption"
            | "figure"
            | "footer"
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "header"
            | "hr"
            | "li"
            | "main"
            | "nav"
            | "ol"
            | "p"
            | "pre"
            | "section"
            | "summary"
            | "table"
            | "tr"
            | "ul"
    )
}

/// What one inline HTML tag does, as GitHub shows it.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum InlineTag {
    /// Opens `name`; an anchor styles only with an address.
    Open {
        name: String,
        style: Option<Style>,
        href: Option<String>,
    },
    Close(String),
    Break,
    /// A filtered tag, shown as its text.
    Text(String),
    /// Gone; the text around it stays.
    Gone,
}

/// What the inline HTML `written` does.
pub(super) fn inline_tag(written: &str) -> InlineTag {
    let Some(tag) = tag(written) else {
        return InlineTag::Gone;
    };
    if filtered(&tag.name) {
        return InlineTag::Text(text(written));
    }
    if tag.name == "br" {
        return InlineTag::Break;
    }
    let Some(style) = inline_style(&tag.name) else {
        return InlineTag::Gone;
    };
    if tag.closing {
        return InlineTag::Close(tag.name);
    }
    let href = match style {
        Style::Link => tag.attribute("href").map(str::to_string),
        _ => None,
    };
    let style = (style != Style::Link || href.is_some()).then_some(style);
    InlineTag::Open {
        name: tag.name,
        style,
        href,
    }
}

/// Start tags that end an open `p`, as HTML5 lists them.
pub(crate) fn closes_p(name: &str) -> bool {
    matches!(
        name,
        "address"
            | "article"
            | "aside"
            | "blockquote"
            | "center"
            | "details"
            | "dialog"
            | "dir"
            | "div"
            | "dl"
            | "dd"
            | "dt"
            | "fieldset"
            | "figcaption"
            | "figure"
            | "footer"
            | "form"
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "header"
            | "hgroup"
            | "hr"
            | "li"
            | "listing"
            | "main"
            | "menu"
            | "nav"
            | "ol"
            | "p"
            | "pre"
            | "search"
            | "section"
            | "summary"
            | "table"
            | "ul"
    )
}

/// Bytes through the `>` ending the tag at `html`'s start.
pub(crate) fn tag_end(html: &str) -> Option<usize> {
    let (mut quote, mut last) = (None, '<');
    for (at, ch) in html.char_indices().skip(1) {
        match quote {
            Some(open) if ch == open => quote = None,
            Some(_) => {}
            None if ch == '>' => return Some(at + 1),
            None if matches!(ch, '"' | '\'') && last == '=' => quote = Some(ch),
            None => {}
        }
        if !spacing(ch) {
            last = ch;
        }
    }
    None
}

/// `html` as one tag; comments and the rest are none.
pub(crate) fn tag(html: &str) -> Option<Tag> {
    let inner = html.trim().strip_prefix('<')?.strip_suffix('>')?;
    let (closing, inner) = match inner.strip_prefix('/') {
        Some(rest) => (true, rest),
        None => (false, inner),
    };
    if !inner.starts_with(|ch: char| ch.is_ascii_alphabetic()) {
        return None;
    }
    let named = inner
        .find(|ch: char| spacing(ch) || ch == '/')
        .unwrap_or(inner.len());
    Some(Tag {
        name: inner[..named].to_ascii_lowercase(),
        closing,
        attributes: attributes(&inner[named..]),
    })
}

/// `name="value"`, `name='value'`, `name=value` and bare names.
fn attributes(mut rest: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    loop {
        rest = rest.trim_start_matches(spacing);
        // A slash between attributes closes the tag itself.
        if let Some(after) = rest.strip_prefix('/') {
            rest = after;
            continue;
        }
        let named = rest
            .find(|ch: char| spacing(ch) || ch == '=' || ch == '/')
            .unwrap_or(rest.len());
        if named == 0 {
            return out;
        }
        let name = rest[..named].to_ascii_lowercase();
        rest = rest[named..].trim_start_matches(spacing);
        let Some(after) = rest.strip_prefix('=') else {
            out.push((name, String::new()));
            continue;
        };
        let after = after.trim_start_matches(spacing);
        let (value, left) = match after.chars().next() {
            Some(quote @ ('"' | '\'')) => match after[1..].find(quote) {
                Some(end) => (&after[1..1 + end], &after[2 + end..]),
                None => (&after[1..], ""),
            },
            _ => {
                let end = after.find(spacing).unwrap_or(after.len());
                (&after[..end], &after[end..])
            }
        };
        out.push((name, decoded(value, true)));
        rest = left;
    }
}

/// HTML's whitespace, which is ASCII alone.
pub(crate) fn spacing(ch: char) -> bool {
    matches!(ch, ' ' | '\t' | '\n' | '\r' | '\x0C')
}

/// Text between tags, references decoded as HTML does.
pub(crate) fn text(written: &str) -> String {
    decoded(written, false)
}

/// References decoded as HTML does, in a value or text.
fn decoded(text: &str, attribute: bool) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find('&') {
        out.push_str(&rest[..at]);
        rest = &rest[at..];
        match reference(rest, attribute) {
            Some((value, len)) => {
                out.push_str(&value);
                rest = &rest[len..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// Longest reference name HTML knows, in bytes.
const LONGEST: usize = 32;

/// A reference at `text`'s start, as HTML reads it.
pub(super) fn reference(text: &str, attribute: bool) -> Option<(String, usize)> {
    let body = text.strip_prefix('&')?;
    if let Some(number) = body.strip_prefix('#') {
        let (digits, radix) = match number.strip_prefix(['x', 'X']) {
            Some(hex) => (hex, 16),
            None => (number, 10),
        };
        let len = digits
            .find(|ch: char| !ch.is_digit(radix))
            .unwrap_or(digits.len());
        if len == 0 {
            return None;
        }
        let code = digits[..len].chars().try_fold(0u32, |code, ch| {
            code.checked_mul(radix)?.checked_add(ch.to_digit(radix)?)
        });
        let ended = usize::from(digits[len..].starts_with(';'));
        let used = 2 + (number.len() - digits.len()) + len + ended;
        return Some((numeric(code).to_string(), used));
    }
    let len = body
        .find(|ch: char| !ch.is_ascii_alphanumeric())
        .unwrap_or(body.len());
    let name = &body[..len];
    if (1..=LONGEST).contains(&len)
        && body[len..].starts_with(';')
        && let Some(value) = decode_named(name, true)
    {
        return Some((value, len + 2));
    }
    // Text takes a legacy prefix; a value, the whole name.
    let shortest = match attribute {
        true if body[len..].starts_with(['=', ';']) => return None,
        true => len,
        false => 1,
    };
    let name = (shortest..=len.min(LONGEST))
        .rev()
        .map(|end| &name[..end])
        .find(|name| legacy(name))?;
    let value = decode_named(name, true).expect("a legacy name");
    Some((value, name.len() + 1))
}

/// A name HTML decodes without its semicolon.
fn legacy(name: &str) -> bool {
    if matches!(name, "AMP" | "LT" | "GT" | "QUOT" | "COPY" | "REG") {
        return true;
    }
    decode_named(name, false).is_some_and(|value| {
        let mut chars = value.chars();
        let one = chars.next().filter(|_| chars.next().is_none());
        one.is_some_and(|ch| matches!(ch, '&' | '<' | '>' | '"' | '\u{A0}'..='\u{FF}'))
    })
}

/// The character a numeric reference names, as HTML maps it.
fn numeric(code: Option<u32>) -> char {
    // Windows-1252 for the C1 block, as browsers do.
    const C1: [u32; 32] = [
        0x20AC, 0x81, 0x201A, 0x0192, 0x201E, 0x2026, 0x2020, 0x2021, 0x02C6, 0x2030, 0x0160,
        0x2039, 0x0152, 0x8D, 0x017D, 0x8F, 0x90, 0x2018, 0x2019, 0x201C, 0x201D, 0x2022, 0x2013,
        0x2014, 0x02DC, 0x2122, 0x0161, 0x203A, 0x0153, 0x9D, 0x017E, 0x0178,
    ];
    match code {
        Some(code @ 0x80..=0x9F) => char::from_u32(C1[(code - 0x80) as usize]),
        Some(0) | None => None,
        Some(code) => char::from_u32(code),
    }
    .unwrap_or(char::REPLACEMENT_CHARACTER)
}

/// `n` in superscript digits.
pub(crate) fn superscript(n: usize) -> String {
    const DIGITS: [char; 10] = ['⁰', '¹', '²', '³', '⁴', '⁵', '⁶', '⁷', '⁸', '⁹'];
    n.to_string()
        .bytes()
        .map(|digit| DIGITS[usize::from(digit - b'0')])
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tag_reads_its_name_and_attributes() {
        let link = tag("<A HREF=\"https://ely.dev\" title='x' hidden>").expect("a tag");
        assert_eq!((link.name.as_str(), link.closing), ("a", false));
        assert_eq!(link.attribute("href"), Some("https://ely.dev"));
        assert_eq!(link.attribute("title"), Some("x"));
        assert_eq!(link.attribute("hidden"), Some(""));
        assert_eq!(
            tag("</u>").map(|t| (t.name, t.closing)),
            Some(("u".into(), true))
        );
        assert_eq!(tag("<br/>").map(|t| t.name), Some("br".into()));
        assert_eq!(tag("<u-box>").map(|t| t.name), Some("u-box".into()));
        assert_eq!(tag("<!-- note -->"), None);
        assert_eq!(tag("<1>"), None);
    }

    #[test]
    fn attribute_values_keep_slashes_and_decode_references() {
        let bare = tag("<a href=/path/>").expect("a tag");
        assert_eq!(bare.attribute("href"), Some("/path/"));
        let closed = tag("<img src=\"a.png\" />").expect("a tag");
        assert_eq!(closed.attribute("src"), Some("a.png"));
        let query = tag("<a href=\"https://x.test/?q=1&amp;p=2&#38;r=&#x33;&nope;\">");
        let query = query.expect("a tag");
        assert_eq!(
            query.attribute("href"),
            Some("https://x.test/?q=1&p=2&r=3&nope;")
        );
    }

    #[test]
    fn values_decode_as_a_browser_decodes_them() {
        let href = |html: &str| {
            tag(html)
                .expect("a tag")
                .attribute("href")
                .map(str::to_owned)
        };
        let cases = [
            ("<a href=\"?q=1&#38page=2\">", "?q=1&page=2"),
            ("<a href=\"/&#128;&#x9F;&#129;\">", "/€Ÿ\u{81}"),
            (
                "<a href=\"&#0;&#x110000;&#xD800;\">",
                "\u{FFFD}\u{FFFD}\u{FFFD}",
            ),
            (
                "<a href=\"&amp page&ampx&amp=&notin;&AMP\">",
                "& page&ampx&amp=∉&",
            ),
            ("<a href=/a\u{A0}b>", "/a\u{A0}b"),
        ];
        for (html, want) in cases {
            assert_eq!(href(html).as_deref(), Some(want), "{html}");
        }
    }

    #[test]
    fn long_runs_scan_in_linear_time() {
        let amps = "&".repeat(1 << 18);
        let started = web_time::Instant::now();
        assert_eq!(decoded(&amps, true).len(), amps.len());
        assert_eq!(text(&amps).len(), amps.len());
        let spent = started.elapsed();
        assert!(spent.as_secs() < 2, "256 KiB took {spent:?}");
    }

    #[test]
    fn text_takes_the_longest_legacy_name() {
        assert_eq!(
            text("&notit; &amp &ampx &lt3 &nope;"),
            "¬it; & &x <3 &nope;"
        );
        assert_eq!(decoded("&notit;&ampx", true), "&notit;&ampx");
    }

    #[test]
    fn a_tag_ends_outside_its_quotes() {
        assert_eq!(tag_end("<a href=\"x>y\">z"), Some(14));
        assert_eq!(tag_end("<b it's>r"), Some(8));
        assert_eq!(tag_end("<b"), None);
    }

    #[test]
    fn numbers_go_up() {
        assert_eq!(superscript(1), "¹");
        assert_eq!(superscript(120), "¹²⁰");
    }
}
