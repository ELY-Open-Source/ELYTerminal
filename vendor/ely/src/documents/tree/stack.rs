use super::{Inline, Opened, Run, Style, html, html::InlineTag};

/// The elements open around inline text.
impl Run {
    pub(super) fn push(&mut self, opened: Opened) {
        *self.named.entry(opened.name.clone()).or_default() += 1;
        self.open.push(opened);
    }

    /// The innermost open link's address.
    pub(super) fn link(&self) -> Option<String> {
        self.open.iter().rev().find_map(|opened| opened.url.clone())
    }

    /// Opens or closes an element for an inline HTML tag.
    pub(super) fn html(&mut self, written: &str) {
        match html::inline_tag(written) {
            InlineTag::Text(shown) => self.push_text(&shown),
            InlineTag::Open { name, style, href } => self.start(&name, style, href),
            InlineTag::Close(name) => {
                self.shortcodes.split();
                self.end(&name);
            }
            InlineTag::Break => {
                self.shortcodes.split();
                self.text.text.push('\n');
            }
            InlineTag::Gone => self.shortcodes.split(),
        }
    }

    /// Opens `name`; a new `a` ends the open one.
    pub(super) fn start(&mut self, name: &str, style: Option<Style>, url: Option<String>) {
        if name == "a" {
            self.end("a");
        }
        self.shortcodes.split();
        self.push(Opened {
            name: name.to_string(),
            start: self.text.text.len(),
            style,
            url,
        });
    }

    /// Ends the innermost `name`, as HTML5 repairs nesting.
    pub(super) fn end(&mut self, name: &str) {
        if self.count(name) == 0 {
            return;
        }
        self.shortcodes.split();
        let at = self
            .open
            .iter()
            .rposition(|opened| opened.name == name)
            .expect("a counted name is open");
        let end = self.text.text.len();
        if let Some((_, from)) = &mut self.owed {
            *from = (*from).min(at);
        }
        let closed = self.open.split_off(at);
        for opened in &closed {
            close(&mut self.text, opened, end);
            *self.named.get_mut(&opened.name).expect("counted") -= 1;
        }
        for opened in closed.into_iter().skip(1) {
            if html::reopens(&opened.name) {
                self.push(Opened {
                    start: end,
                    ..opened
                });
            }
        }
    }
}

/// Records `opened` as styled up to `end`.
pub(super) fn close(text: &mut Inline, opened: &Opened, end: usize) {
    if opened.start >= end {
        return;
    }
    if let Some(style) = opened.style {
        text.styles.push((opened.start..end, style));
    }
    if let Some(url) = &opened.url {
        text.links.push((opened.start..end, url.clone()));
    }
}
