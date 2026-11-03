/// `text` with each known `:shortcode:` as its emoji.
fn emoji(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find(':') {
        out.push_str(&rest[..at]);
        let after = &rest[at + 1..];
        let len = after
            .find(|ch: char| !shortcode_char(ch))
            .unwrap_or(after.len());
        let found = after[len..]
            .starts_with(':')
            .then(|| emojis::get_by_shortcode(&after[..len]))
            .flatten();
        match found {
            Some(found) => {
                out.push_str(found.as_str());
                rest = &after[len + 1..];
            }
            None => {
                out.push(':');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

fn shortcode_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '_' | '+' | '-')
}

/// Where the first `:name:` starts.
fn candidate(text: &str) -> Option<usize> {
    let mut from = 0;
    while let Some(at) = text[from..].find(':').map(|at| from + at) {
        let after = &text[at + 1..];
        let len = after
            .find(|ch: char| !shortcode_char(ch))
            .unwrap_or(after.len());
        if len > 0 && after[len..].starts_with(':') {
            return Some(at);
        }
        from = at + 1 + len;
    }
    None
}

/// One DOM text node's shortcodes, as GitHub reads them.
#[derive(Default)]
pub(super) struct Shortcodes {
    /// Settled by the node's first `:name:`.
    reads: Option<bool>,
    /// Text before it held more than white space.
    inked: bool,
    last: Option<char>,
}

impl Shortcodes {
    /// A new DOM text node starts.
    pub(super) fn split(&mut self) {
        *self = Self::default();
    }

    /// Blank or spaced text before the first reads them all.
    pub(super) fn read(&mut self, text: &str) -> String {
        if self.reads.is_none() {
            let at = candidate(text);
            let lead = &text[..at.unwrap_or(text.len())];
            self.inked |= !lead.chars().all(|ch| ch.is_ascii_whitespace());
            self.last = lead.chars().next_back().or(self.last);
            self.reads = at.map(|_| !self.inked || self.last == Some(' '));
        }
        match self.reads {
            Some(true) => emoji(text),
            _ => text.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_shortcodes_become_emoji() {
        assert_eq!(emoji("hi :smile: :+1:"), "hi 😄 👍");
        assert_eq!(emoji(":nope: at 10:30 and ::"), ":nope: at 10:30 and ::");
        assert_eq!(emoji("a:smile:b :x:smile:"), "a😄b ❌smile:");
        assert_eq!(emoji("é :smile: 中"), "é 😄 中");
    }

    #[test]
    fn long_runs_scan_in_linear_time() {
        let colons = ":zq".repeat(1 << 16);
        let started = web_time::Instant::now();
        assert_eq!(emoji(&colons).len(), colons.len());
        assert_eq!(Shortcodes::default().read(&colons).len(), colons.len());
        let spent = started.elapsed();
        assert!(spent.as_secs() < 2, "192 KiB took {spent:?}");
    }
}
