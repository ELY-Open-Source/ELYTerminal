use std::ops::Range;

use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};

use super::{BlockData, BlockKind, Media};

mod details;

impl BlockData {
    /// Blocks from Markdown, as `to_markdown` writes it: inline Markdown stays as written in each field. A nested list comes out flat, and a quote or callout holds its paragraphs as text.
    pub fn from_markdown(source: &str) -> Vec<BlockData> {
        let options = Options::ENABLE_TABLES
            | Options::ENABLE_STRIKETHROUGH
            | Options::ENABLE_TASKLISTS
            | Options::ENABLE_MATH
            | Options::ENABLE_GFM;
        let mut reader = Reader {
            source,
            ..Reader::default()
        };
        let mut events = Parser::new_ext(source, options).into_offset_iter();
        for (event, range) in events.by_ref() {
            reader.read(event, range);
        }
        reader.unclosed();
        let mut spans: Vec<Range<usize>> = events
            .reference_definitions()
            .iter()
            .map(|(_, definition)| definition.span.clone())
            .filter(|span| !reader.bodies.iter().any(|body| body.contains(&span.start)))
            .collect();
        spans.sort_by_key(|span| span.start);
        let definitions: Vec<&str> = spans.into_iter().map(|span| source[span].trim()).collect();
        if !definitions.is_empty() {
            reader.put(
                BlockKind::Paragraph,
                definitions.join(
                    "
",
                ),
            );
        }
        log::debug!(
            "block editor: {} blocks read from markdown",
            reader.out.len()
        );
        reader.out
    }
}

/// What a paragraph holds alone, which makes it a block of its own.
#[derive(Default)]
enum Sole {
    #[default]
    Nothing,
    Math(String),
    Image(String, Option<Range<usize>>),
    Mixed,
}

/// Inline text being read: its stretch of the source, and how deep in inline tags.
#[derive(Default)]
struct Leaf {
    range: Option<Range<usize>>,
    depth: usize,
    sole: Sole,
}

struct Item {
    kind: BlockKind,
    parts: Vec<String>,
    emitted: bool,
}

struct Toggle {
    open: bool,
    title: String,
    body: usize,
    depth: usize,
}

#[derive(Default)]
struct Reader<'a> {
    source: &'a str,
    out: Vec<BlockData>,
    leaf: Option<Leaf>,
    /// The outermost quote: callout or not, depth, and its paragraphs.
    quote: Option<(bool, usize, Vec<String>)>,
    lists: Vec<bool>,
    items: Vec<Item>,
    code: Option<(BlockKind, String)>,
    /// Columns, cells so far, and the row being read.
    table: Option<(usize, Vec<String>, Vec<String>)>,
    toggle: Option<Toggle>,
    /// Toggle bodies, kept as written.
    bodies: Vec<Range<usize>>,
    html: bool,
}

/// Inline lines as one field: each further line loses its indent and quote marks.
fn clean(slice: &str, quotes: usize) -> String {
    let mut lines = slice.split('\n');
    let mut out = lines.next().unwrap_or_default().to_string();
    for line in lines {
        let mut line = line.trim_start();
        for _ in 0..quotes {
            line = line.strip_prefix('>').unwrap_or(line).trim_start();
        }
        out.push('\n');
        out.push_str(line);
    }
    out
}

impl Reader<'_> {
    fn read(&mut self, event: Event, range: Range<usize>) {
        if self.toggle.is_some() {
            return self.in_toggle(event, range);
        }
        if self.html {
            self.html = !matches!(event, Event::End(TagEnd::HtmlBlock));
            return;
        }
        if let Some((_, code)) = &mut self.code {
            match event {
                Event::Text(text) => code.push_str(&text),
                Event::End(TagEnd::CodeBlock) => self.end_code(),
                _ => {}
            }
            return;
        }
        match event {
            Event::Start(Tag::HtmlBlock) => self.html_block(range),
            Event::Start(Tag::Paragraph | Tag::Heading { .. } | Tag::TableCell) => {
                self.flush_leaf();
                self.leaf = Some(Leaf::default());
            }
            Event::End(TagEnd::Paragraph) => self.end_paragraph(),
            Event::End(TagEnd::Heading(level)) => {
                let text = self.leaf_text();
                self.put(BlockKind::Heading(level as u8), text);
            }
            Event::Start(Tag::BlockQuote(kind)) => match &mut self.quote {
                Some((_, depth, _)) => *depth += 1,
                None => {
                    self.flush_leaf();
                    self.quote = Some((kind.is_some(), 1, Vec::new()));
                }
            },
            Event::End(TagEnd::BlockQuote(_)) => self.end_quote(),
            Event::Start(Tag::CodeBlock(kind)) => {
                self.flush_leaf();
                let fence = match kind {
                    CodeBlockKind::Fenced(language) => language.trim().to_string(),
                    CodeBlockKind::Indented => String::new(),
                };
                let kind = match fence.as_str() {
                    "mermaid" => BlockKind::Diagram,
                    "math" => BlockKind::Math,
                    _ => BlockKind::Code,
                };
                self.code = Some((kind, String::new()));
            }
            Event::Start(Tag::List(start)) => {
                self.flush_leaf();
                self.emit_item();
                self.lists.push(start.is_some());
            }
            Event::End(TagEnd::List(_)) => {
                self.lists.pop();
            }
            Event::Start(Tag::Item) => {
                let numbered = self.lists.last().copied().unwrap_or_default();
                let kind = match numbered {
                    true => BlockKind::Numbered,
                    false => BlockKind::Bullet,
                };
                self.items.push(Item {
                    kind,
                    parts: Vec::new(),
                    emitted: false,
                });
                self.leaf = Some(Leaf::default());
            }
            Event::TaskListMarker(done) => {
                if let Some(item) = self.items.last_mut() {
                    item.kind = BlockKind::Todo(done);
                }
            }
            Event::End(TagEnd::Item) => {
                self.flush_leaf();
                self.emit_item();
                self.items.pop();
            }
            Event::Rule => self.put(BlockKind::Divider, String::new()),
            Event::Start(Tag::Table(aligns)) => {
                self.table = Some((aligns.len(), Vec::new(), Vec::new()));
            }
            Event::End(TagEnd::TableCell) => {
                let text = self.leaf_text().replace("\\|", "|");
                if let Some((_, _, row)) = &mut self.table {
                    row.push(text);
                }
            }
            Event::End(TagEnd::TableHead | TagEnd::TableRow) => {
                if let Some((columns, cells, row)) = &mut self.table {
                    row.resize(*columns, String::new());
                    cells.append(row);
                }
            }
            Event::End(TagEnd::Table) => self.end_table(),
            event => self.inline(event, range),
        }
    }

    fn inline(&mut self, event: Event, range: Range<usize>) {
        let Some(leaf) = &mut self.leaf else {
            return;
        };
        let (start, end) = leaf.range.as_ref().map_or((range.start, range.end), |had| {
            (had.start.min(range.start), had.end.max(range.end))
        });
        leaf.range = Some(start..end);
        let top = leaf.depth == 0;
        if let (false, Sole::Image(_, alt)) =
            (top || matches!(event, Event::End(_)), &mut leaf.sole)
        {
            let had = alt.get_or_insert(range.clone());
            *had = had.start.min(range.start)..had.end.max(range.end);
        }
        match event {
            Event::Start(Tag::Image { dest_url, .. }) if top => {
                leaf.sole = match leaf.sole {
                    Sole::Nothing => Sole::Image(dest_url.to_string(), None),
                    _ => Sole::Mixed,
                };
                leaf.depth += 1;
            }
            Event::Start(_) => {
                if top {
                    leaf.sole = Sole::Mixed;
                }
                leaf.depth += 1;
            }
            Event::End(_) => leaf.depth = leaf.depth.saturating_sub(1),
            Event::DisplayMath(math) if top => {
                leaf.sole = match leaf.sole {
                    Sole::Nothing => Sole::Math(math.to_string()),
                    _ => Sole::Mixed,
                };
            }
            _ if top => leaf.sole = Sole::Mixed,
            _ => {}
        }
    }

    fn quotes(&self) -> usize {
        self.quote.as_ref().map_or(0, |(_, depth, _)| *depth)
    }

    fn leaf_text(&mut self) -> String {
        let (source, quotes) = (self.source, self.quotes());
        self.leaf
            .take()
            .and_then(|leaf| leaf.range)
            .map(|range| clean(&source[range], quotes))
            .unwrap_or_default()
    }

    /// Text an item read before a block inside it began.
    fn flush_leaf(&mut self) {
        if self.leaf.as_ref().is_some_and(|leaf| leaf.range.is_some()) {
            let text = self.leaf_text();
            self.put(BlockKind::Paragraph, text);
        }
        self.leaf = None;
    }

    /// Text where it belongs: in the open quote, the open item, or a block of its own.
    fn put(&mut self, kind: BlockKind, text: String) {
        if let Some((_, _, parts)) = &mut self.quote {
            parts.push(text);
            return;
        }
        if kind == BlockKind::Paragraph
            && let Some(item) = self.items.last_mut()
            && !item.emitted
        {
            item.parts.push(text);
            return;
        }
        self.emit_item();
        let data = match kind {
            BlockKind::Divider => BlockData::new(kind, Vec::<String>::new()),
            kind => BlockData::new(kind, [text]),
        };
        self.out.push(data);
    }

    fn emit_item(&mut self) {
        let quoted = self.quote.is_some();
        let Some(item) = self.items.last_mut().filter(|item| !item.emitted) else {
            return;
        };
        item.emitted = true;
        if quoted {
            return;
        }
        let data = BlockData::new(item.kind.clone(), [item.parts.join("\n\n")]);
        self.out.push(data);
    }

    fn end_paragraph(&mut self) {
        let alone = self.quote.is_none() && self.items.is_empty();
        let sole = self
            .leaf
            .as_mut()
            .map(|leaf| std::mem::take(&mut leaf.sole));
        match sole {
            Some(Sole::Math(math)) if alone => {
                self.leaf = None;
                self.put(BlockKind::Math, math.trim().to_string());
            }
            Some(Sole::Image(source, alt)) if alone => {
                self.leaf = None;
                let source_text = self.source;
                let caption = alt.map_or(String::new(), |alt| source_text[alt].to_string());
                self.put(BlockKind::Image(Media::new(source)), caption);
            }
            _ => {
                let text = self.leaf_text();
                self.put(BlockKind::Paragraph, text);
            }
        }
    }

    fn end_quote(&mut self) {
        let Some((callout, depth, parts)) = &mut self.quote else {
            return;
        };
        *depth -= 1;
        if *depth > 0 {
            return;
        }
        let kind = match callout {
            true => BlockKind::Callout,
            false => BlockKind::Quote,
        };
        let text = parts.join("\n\n");
        self.quote = None;
        self.put(kind, text);
    }

    fn end_code(&mut self) {
        let Some((kind, mut code)) = self.code.take() else {
            return;
        };
        if code.ends_with('\n') {
            code.pop();
        }
        self.put(kind, code);
    }

    fn end_table(&mut self) {
        let Some((columns, mut cells, _)) = self.table.take() else {
            return;
        };
        if cells.len() < columns * 2 {
            cells.resize(columns * 2, String::new());
        }
        let kind = BlockKind::Table {
            columns,
            merged: Vec::new(),
        };
        self.emit_item();
        self.out.push(BlockData::new(kind, cells));
    }
}
