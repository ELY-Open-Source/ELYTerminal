use std::{ops::Range, path::Path, rc::Rc, sync::Arc};

use gpui::{
    AnyElement, App, ElementId, Entity, FontStyle, FontWeight, HighlightStyle, InteractiveText,
    IntoElement, ParentElement, Pixels, RenderOnce, SharedString, StrikethroughStyle, Styled,
    StyledText, TextLayout, UnderlineStyle, Window, div, prelude::*, relative,
};

use super::{
    contents::anchor,
    diagram::Diagram,
    picture::Picture,
    tree::{Inline, Node, Style, parse},
};
use crate::{
    editor::{code_colors, code_colors_with},
    navigation::{Anchor, Sections},
    primitives::FocusRing as _,
    theme::{ActiveTheme, Palette, Radius, TextSize},
    typography::{Latex, failed},
};

mod blocks;
mod math_text;
mod point;
mod table;

pub(crate) use point::{OnPoint, Place, Pointed, point_lines};
pub(crate) use table::{CellAt, OpenCell};

pub(crate) type OnLink = Rc<dyn Fn(&str, &mut Window, &mut App)>;
/// Reveals the heading whose source starts at a byte.
pub(crate) type OnHeading = Rc<dyn Fn(usize, &mut Window, &mut App)>;
type DrawCode = Rc<dyn Fn(ElementId, Option<SharedString>, SharedString) -> AnyElement>;

/// Markdown drawn as a document; links go to the host.
#[derive(IntoElement)]
pub struct MarkdownRenderer {
    id: ElementId,
    source: SharedString,
    on_link: Option<OnLink>,
    size: TextSize,
    sections: Option<Entity<Sections>>,
    code: Option<DrawCode>,
    base: Option<Arc<Path>>,
}

impl MarkdownRenderer {
    pub fn new(id: impl Into<ElementId>, source: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            source: source.into(),
            on_link: None,
            size: TextSize::Base,
            sections: None,
            code: None,
            base: None,
        }
    }

    /// The folder relative picture addresses start from.
    pub fn base(mut self, folder: impl Into<Arc<Path>>) -> Self {
        self.base = Some(folder.into());
        self
    }

    /// The body's size; headings keep theirs.
    pub fn text_size(mut self, size: TextSize) -> Self {
        self.size = size;
        self
    }

    /// Marks headings for `sections`; `outline` lists them.
    pub fn sections(mut self, sections: &Entity<Sections>) -> Self {
        self.sections = Some(sections.clone());
        self
    }

    /// Code blocks drawn by the owner: id, language, code.
    pub fn code(
        mut self,
        draw: impl Fn(ElementId, Option<SharedString>, SharedString) -> AnyElement + 'static,
    ) -> Self {
        self.code = Some(Rc::new(draw));
        self
    }

    /// Takes link presses instead of the browser.
    pub fn on_link(mut self, handler: impl Fn(&str, &mut Window, &mut App) + 'static) -> Self {
        self.on_link = Some(Rc::new(handler));
        self
    }
}

/// What an owner hands its drawing: pictures' folder and presses.
pub(crate) struct Hooks<'a> {
    pub base: &'a Option<Arc<Path>>,
    pub on_link: &'a Option<OnLink>,
    pub on_heading: &'a Option<OnHeading>,
    pub on_point: &'a Option<OnPoint>,
    /// The cell typed in, drawn in its place.
    pub cell: Option<&'a OpenCell>,
}

/// Headings and strong text share one weight.
pub(crate) const HEAVY: FontWeight = FontWeight::BOLD;

/// Typora's heading steps over the body.
pub(crate) fn heading_step(level: u8) -> f32 {
    match level {
        1 => 2.25,
        2 => 1.75,
        3 => 1.5,
        4 => 1.25,
        _ => 1.0,
    }
}

/// How one inline style draws.
fn highlight(style: &Style, colors: &Palette, rule: Pixels) -> HighlightStyle {
    match style {
        Style::Strong => HighlightStyle {
            font_weight: Some(HEAVY),
            ..HighlightStyle::default()
        },
        Style::Emphasis => HighlightStyle {
            font_style: Some(FontStyle::Italic),
            ..HighlightStyle::default()
        },
        Style::Strike => HighlightStyle {
            strikethrough: Some(StrikethroughStyle {
                thickness: rule,
                color: Some(colors.fg_muted),
            }),
            ..HighlightStyle::default()
        },
        Style::Code | Style::Math => HighlightStyle {
            background_color: Some(colors.sunken),
            color: Some(if *style == Style::Math {
                colors.syntax.function
            } else {
                colors.fg
            }),
            ..HighlightStyle::default()
        },
        Style::Link => HighlightStyle {
            color: Some(colors.link),
            underline: Some(UnderlineStyle {
                thickness: rule,
                color: Some(colors.link.opacity(0.4)),
                wavy: false,
            }),
            ..HighlightStyle::default()
        },
        Style::Footnote => HighlightStyle {
            color: Some(colors.fg_subtle),
            ..HighlightStyle::default()
        },
        Style::Underline => HighlightStyle {
            underline: Some(UnderlineStyle {
                thickness: rule,
                color: None,
                wavy: false,
            }),
            ..HighlightStyle::default()
        },
        Style::Mark => HighlightStyle {
            background_color: Some(colors.warning_subtle),
            ..HighlightStyle::default()
        },
    }
}

/// Highlights laid over a text's own.
type Over = Vec<(Range<usize>, HighlightStyle)>;

/// `inline` as text; hidden ranges lose highlight, `over` adds some.
fn drawn_text(
    inline: &Inline,
    (hidden, over): (&[Range<usize>], Over),
    links: Option<ElementId>,
    on_link: &Option<OnLink>,
    cx: &App,
) -> (AnyElement, TextLayout) {
    let theme = cx.theme();
    let colors = &theme.colors;
    let rule = theme.underline_thickness();
    let styles: Vec<_> = (inline.styles.iter())
        .filter(|(range, _)| !hidden.contains(range))
        .map(|(range, style)| (range.clone(), highlight(style, colors, rule)))
        .chain(over)
        .collect();
    let styled = StyledText::new(inline.text.clone()).with_highlights(crate::editor::stack(styles));
    let layout = styled.layout().clone();
    let drawn = match links {
        None => styled.into_any_element(),
        Some(id) => {
            let (ranges, urls): (Vec<_>, Vec<_>) = inline.links.iter().cloned().unzip();
            let on_link = on_link.clone();
            InteractiveText::new(id, styled)
                .on_click(ranges, move |ix, window, cx| {
                    open_link(&on_link, &urls[ix], window, cx)
                })
                .into_any_element()
        }
    };
    (drawn, layout)
}

/// Draws nodes under one owner's ids and hooks.
struct Draw<'a> {
    id: &'a ElementId,
    hooks: Hooks<'a>,
    next: std::cell::Cell<usize>,
    sections: &'a Option<Entity<Sections>>,
    headings: std::cell::Cell<usize>,
    code: &'a Option<DrawCode>,
    /// The body's size; headings step up from it.
    body: TextSize,
    /// Text size against the theme's.
    scale: f32,
    /// Lists open around the node drawn.
    lists: std::cell::Cell<usize>,
    /// Drawing a table's cells.
    table: std::cell::Cell<bool>,
    /// Tables drawn so far.
    tables: std::cell::Cell<usize>,
}

impl Draw<'_> {
    fn key(&self, what: &str) -> ElementId {
        let at = self.next.get();
        self.next.set(at + 1);
        (self.id.clone(), format!("{what}-{at}")).into()
    }

    fn inline(&self, inline: &Inline, cx: &App) -> AnyElement {
        let links = (!inline.links.is_empty()).then(|| self.key("links"));
        if !inline.styles.iter().any(|(_, style)| *style == Style::Math) {
            let (drawn, layout) =
                drawn_text(inline, (&[], Vec::new()), links, self.hooks.on_link, cx);
            let at = move |position| {
                layout
                    .index_for_position(position)
                    .unwrap_or_else(|near| near)
            };
            return self.point_text(drawn, at, inline);
        }
        let on_link = self.hooks.on_link.clone();
        let (drawn, at) =
            math_text::MathText::new(self.key("math"), inline.clone(), links, on_link);
        self.point_text(drawn.into_any_element(), at, inline)
    }

    fn nodes(&self, nodes: &[Node], cx: &App) -> Vec<AnyElement> {
        nodes.iter().map(|node| self.node(node, cx)).collect()
    }

    fn node(&self, node: &Node, cx: &App) -> AnyElement {
        let table = self.table.get();
        self.table.set(table || matches!(node, Node::Table { .. }));
        let drawn = self.drawn(node, cx);
        self.table.set(table);
        drawn
    }

    fn drawn(&self, node: &Node, cx: &App) -> AnyElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        match node {
            Node::Paragraph(inline) => div()
                .text_color(colors.fg)
                .child(self.inline(inline, cx))
                .into_any_element(),
            Node::Heading(level, inline) => {
                // Every level bold.
                let size = theme.text_size(self.body) * heading_step(*level) * self.scale;
                let ordinal = self.headings.get();
                self.headings.set(ordinal + 1);
                let heading = div()
                    .debug_selector(move || format!("markdown-heading-{ordinal}"))
                    .pt_2()
                    .text_size(size)
                    .line_height(relative(1.3))
                    .font_weight(HEAVY)
                    .text_color(if *level == 6 {
                        colors.fg_muted
                    } else {
                        colors.fg
                    })
                    .child(self.inline(inline, cx));
                match self.sections {
                    Some(sections) => Anchor::new(sections, anchor(ordinal))
                        .child(heading)
                        .into_any_element(),
                    None => heading.into_any_element(),
                }
            }
            Node::Quote(inner) => div()
                .flex()
                .flex_col()
                .gap_2()
                .pl_4()
                .border_l_2()
                .border_color(colors.border_strong)
                .text_color(colors.fg_muted)
                .children(self.nodes(inner, cx))
                .into_any_element(),
            Node::Code {
                language,
                text,
                from,
            } => {
                if language.as_deref() == Some("mermaid") {
                    let diagram = Diagram::new(self.key("diagram"), text.clone());
                    return self.point_code(diagram.into_any_element(), None, text, *from);
                }
                if let Some(draw) = self.code {
                    return draw(
                        self.key("code"),
                        language.clone().map(Into::into),
                        text.clone().into(),
                    );
                }
                // A named fence reads as its language; prose stays plain.
                let styles = match language.as_deref() {
                    Some(name) => crate::forms::syntax_fenced(name)
                        .map_or_else(Vec::new, |syntax| code_colors_with(text, syntax, cx)),
                    None => code_colors(text, cx),
                };
                let code = StyledText::new(text.clone()).with_highlights(styles);
                let layout = code.layout().clone();
                let block = div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .p_3()
                    .rounded(theme.radius(Radius::Md))
                    .bg(colors.sunken)
                    .font_family(theme.mono_family.clone())
                    .text_size(theme.text_size(TextSize::Sm) * self.scale)
                    .text_color(colors.syntax.variable)
                    .children(language.clone().map(|name| {
                        div()
                            .text_size(theme.text_size(TextSize::Xs) * self.scale)
                            .text_color(colors.fg_subtle)
                            .child(name)
                    }))
                    .child(code)
                    .into_any_element();
                self.point_code(block, Some(layout), text, *from)
            }
            Node::List { start, items } => self.list(start, items, cx),
            Node::Table { aligns, head, rows } => self.table(aligns, head, rows, cx),
            Node::Rule => div()
                .debug_selector(|| "markdown-rule".into())
                .my_2()
                .border_t_2()
                .border_color(colors.border_strong)
                .into_any_element(),
            Node::Math(tex) => div()
                .flex()
                .justify_center()
                .py_2()
                .child(match Latex::parse(self.key("math"), tex) {
                    Ok(math) => math.display().into_any_element(),
                    Err(error) => failed(&error, tex, cx),
                })
                .into_any_element(),
            Node::Image { alt, url, link, at } => {
                let picture = div().id(self.key("image")).child(Picture {
                    id: self.key("picture"),
                    alt: alt.clone(),
                    url: url.clone(),
                    base: self.hooks.base.clone(),
                });
                let drawn = match link.clone() {
                    None => picture.into_any_element(),
                    Some(address) => {
                        let on_link = self.hooks.on_link.clone();
                        picture
                            .tab_index(0)
                            .focus_ring(cx)
                            .cursor_pointer()
                            .on_click(move |_, window, cx| {
                                open_link(&on_link, &address, window, cx)
                            })
                            .into_any_element()
                    }
                };
                self.point_image(drawn, *at, url, link)
            }
            Node::Footnote { label, body } => div()
                .flex()
                .gap_2()
                .text_size(theme.text_size(TextSize::Sm))
                .text_color(colors.fg_muted)
                .child(div().flex_none().child(format!("[{label}]")))
                .child(
                    div()
                        .flex_1()
                        .flex()
                        .flex_col()
                        .children(self.nodes(body, cx)),
                )
                .into_any_element(),
            Node::Alert { kind, body } => self.alert(*kind, body, cx),
            Node::Toc(headings) => self.toc(headings, cx),
            Node::FrontMatter { text, from } => self.front_matter(text, *from, cx),
        }
    }
}

/// Opens `url` through the owner, else the system.
fn open_link(on_link: &Option<OnLink>, url: &str, window: &mut Window, cx: &mut App) {
    // The address is the document's: never logged.
    log::info!("markdown: a link opens");
    match on_link {
        Some(on_link) => on_link(url, window, cx),
        None => cx.open_url(url),
    }
}

/// Parsed `nodes` drawn under `id`, with the owner's hooks.
pub(crate) fn draw_nodes(
    id: &ElementId,
    nodes: &[Node],
    hooks: Hooks,
    scale: f32,
    cx: &App,
) -> Vec<AnyElement> {
    let draw = Draw {
        id,
        hooks,
        next: std::cell::Cell::new(0),
        sections: &None,
        headings: std::cell::Cell::new(0),
        code: &None,
        body: TextSize::Base,
        scale,
        lists: std::cell::Cell::new(0),
        table: std::cell::Cell::new(false),
        tables: std::cell::Cell::new(0),
    };
    draw.nodes(nodes, cx)
}

impl RenderOnce for MarkdownRenderer {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let nodes = parse(&self.source);
        let draw = Draw {
            id: &self.id,
            hooks: Hooks {
                base: &self.base,
                on_link: &self.on_link,
                on_heading: &None,
                on_point: &None,
                cell: None,
            },
            next: std::cell::Cell::new(0),
            sections: &self.sections,
            headings: std::cell::Cell::new(0),
            code: &self.code,
            body: self.size,
            scale: 1.0,
            lists: std::cell::Cell::new(0),
            table: std::cell::Cell::new(false),
            tables: std::cell::Cell::new(0),
        };
        div()
            .flex()
            .flex_col()
            .gap_3()
            .text_size(theme.text_size(self.size))
            .line_height(theme.text_size(self.size) * 1.6)
            .children(draw.nodes(&nodes, cx))
    }
}

#[cfg(test)]
mod tests;
