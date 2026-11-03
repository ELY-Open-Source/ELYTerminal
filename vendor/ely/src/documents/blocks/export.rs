use std::ops::Range;

use pulldown_cmark::{Event, Parser};

use super::{BlockData, BlockKind, Media};

impl BlockData {
    /// The document as Markdown, GitHub's flavor: a callout is a `[!NOTE]` quote, a toggle a `<details>`, a diagram a `mermaid` fence, math a `math` fence. Fields already hold Markdown and pass through as written, so a blank line in prose starts a new block; a heading, caption, toggle title or table cell keeps one line, and a caption's lone bracket comes back escaped. HTML in a field is read as HTML. What Markdown has no form for takes its nearest: a table's merges, media's width and alignment, columns as paragraphs, a synced block as its text, video and embeds as links.
    pub fn to_markdown(blocks: &[BlockData]) -> String {
        let mut out = String::new();
        let mut previous: Option<&BlockKind> = None;
        let mut number = 0;
        for block in blocks {
            number = match block.kind {
                BlockKind::Numbered => number + 1,
                _ => 0,
            };
            let Some(text) = write(block, number) else {
                continue;
            };
            if let Some(previous) = previous {
                out.push_str(if same_list(previous, &block.kind) {
                    "\n"
                } else {
                    "\n\n"
                });
            }
            out.push_str(&text);
            previous = Some(&block.kind);
        }
        out.push('\n');
        out
    }
}

/// Whether two kinds follow on in one Markdown list.
fn same_list(previous: &BlockKind, next: &BlockKind) -> bool {
    let bulleted = |kind: &BlockKind| matches!(kind, BlockKind::Bullet | BlockKind::Todo(_));
    (bulleted(previous) && bulleted(next))
        || (*previous == BlockKind::Numbered && *next == BlockKind::Numbered)
}

/// One block as Markdown; nothing for an empty paragraph, which Markdown cannot hold.
fn write(block: &BlockData, number: usize) -> Option<String> {
    let texts = &block.texts;
    let text = || texts[0].as_str();
    let written = match &block.kind {
        BlockKind::Paragraph | BlockKind::Synced(_) if text().trim().is_empty() => return None,
        BlockKind::Paragraph | BlockKind::Synced(_) => text().to_string(),
        BlockKind::Heading(level) => {
            let level = usize::from((*level).clamp(1, 6));
            format!("{} {}", "#".repeat(level), text().replace('\n', " "))
        }
        BlockKind::Bullet => item("- ", text()),
        BlockKind::Numbered => item(&format!("{number}. "), text()),
        BlockKind::Todo(done) => item(if *done { "- [x] " } else { "- [ ] " }, text()),
        BlockKind::Quote => quoted(text()),
        BlockKind::Callout => format!("> [!NOTE]\n{}", quoted(text())),
        BlockKind::Toggle(open) => {
            let open = if *open { " open" } else { "" };
            let title = texts[0].replace('\n', " ");
            let head = format!("<details{open}>\n<summary>{title}</summary>");
            match texts[1].trim() {
                "" => format!("{head}\n\n</details>"),
                body => format!("{head}\n\n{body}\n\n</details>"),
            }
        }
        BlockKind::Code => fenced(text(), ""),
        BlockKind::Diagram => fenced(text(), "mermaid"),
        BlockKind::Math => fenced(text(), "math"),
        BlockKind::Divider => "---".to_string(),
        BlockKind::Table { columns, .. } => table(texts, *columns),
        BlockKind::Image(media) => {
            let caption = brackets(&text().replace('\n', " "));
            format!("![{caption}]({})", address(media))
        }
        BlockKind::Video(media) | BlockKind::Embed(media) => {
            let words = if text().is_empty() {
                &media.source
            } else {
                text()
            };
            format!("[{words}]({})", address(media))
        }
        BlockKind::Columns(_) => {
            let filled: Vec<&str> = texts
                .iter()
                .map(|text| text.trim())
                .filter(|text| !text.is_empty())
                .collect();
            if filled.is_empty() {
                return None;
            }
            filled.join("\n\n")
        }
    };
    Some(written)
}

/// A list item: the marker, then each further line under its text.
fn item(marker: &str, text: &str) -> String {
    let indent = " ".repeat(marker.len());
    let mut lines = text.lines();
    let mut out = format!("{marker}{}", lines.next().unwrap_or_default());
    for line in lines {
        out.push('\n');
        out.push_str(&indent);
        out.push_str(line);
    }
    out
}

fn quoted(text: &str) -> String {
    let lines: Vec<String> = text
        .split('\n')
        .map(|line| match line {
            "" => ">".to_string(),
            line => format!("> {line}"),
        })
        .collect();
    lines.join("\n")
}

/// Code between fences longer than any run of backticks inside it.
fn fenced(text: &str, language: &str) -> String {
    let longest = text
        .split(|ch| ch != '`')
        .map(str::len)
        .max()
        .unwrap_or_default();
    let fence = "`".repeat(longest.max(2) + 1);
    format!("{fence}{language}\n{text}\n{fence}")
}

fn table(texts: &[String], columns: usize) -> String {
    let row = |cells: &[String]| {
        let cells: Vec<String> = cells
            .iter()
            .map(|cell| pipes(cell).replace('\n', " "))
            .collect();
        format!("| {} |", cells.join(" | "))
    };
    let mut rows = texts.chunks(columns);
    let head = rows.next().expect("a table holds its header row");
    let mut out = vec![row(head), format!("|{}", " --- |".repeat(columns))];
    out.extend(rows.map(row));
    out.join("\n")
}

/// A link's address, in angle brackets when it holds a space or a bracket.
fn address(media: &Media) -> String {
    match media.source.contains([' ', '(', ')', '<', '>']) {
        true => format!("<{}>", media.source.replace('<', "\\<").replace('>', "\\>")),
        false => media.source.to_string(),
    }
}

/// A caption that cannot end its image early: a bracket left open or shut alone outside code, or a last backslash, is escaped.
fn brackets(text: &str) -> String {
    let code: Vec<Range<usize>> = Parser::new(text)
        .into_offset_iter()
        .filter_map(|(event, range)| matches!(event, Event::Code(_)).then_some(range))
        .collect();
    let (mut open, mut lone, mut slashes) = (Vec::new(), Vec::new(), 0);
    for (at, ch) in text.char_indices() {
        let free = slashes % 2 == 0 && !code.iter().any(|span| span.contains(&at));
        match ch {
            '[' if free => open.push(at),
            ']' if free && open.pop().is_none() => lone.push(at),
            _ => {}
        }
        slashes = if ch == '\\' { slashes + 1 } else { 0 };
    }
    lone.extend(open);
    let mut out = String::with_capacity(text.len() + lone.len() + 1);
    for (at, ch) in text.char_indices() {
        if lone.contains(&at) {
            out.push('\\');
        }
        out.push(ch);
    }
    if slashes % 2 == 1 {
        out.push('\\');
    }
    out
}

/// A cell's pipes escaped where they are not already, so none splits its row.
fn pipes(cell: &str) -> String {
    let (mut out, mut slashes) = (String::with_capacity(cell.len()), 0);
    for ch in cell.chars() {
        if ch == '|' && slashes % 2 == 0 {
            out.push('\\');
        }
        slashes = if ch == '\\' { slashes + 1 } else { 0 };
        out.push(ch);
    }
    out
}
