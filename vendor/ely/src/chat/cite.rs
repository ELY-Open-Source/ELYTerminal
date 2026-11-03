use std::rc::Rc;

use gpui::{
    App, Div, ElementId, FontWeight, InteractiveElement, IntoElement, MouseButton, ParentElement,
    RenderOnce, SharedString, StatefulInteractiveElement, Styled, Window, div, prelude::*,
    transparent_black,
};

use crate::{
    forms::{Pick, Run},
    overlays::HoverCard,
    primitives::{Disclosure, FocusRing, Icon, IconName},
    theme::{ActiveTheme, IconSize, Radius, TextSize},
    typography::{Ellipsis, tabular},
};

/// Where an answer's words came from: the site, the page's title, its address and a line from it.
#[derive(Clone, Debug, PartialEq)]
pub struct Source {
    pub site: SharedString,
    pub title: SharedString,
    pub url: SharedString,
    pub snippet: Option<SharedString>,
}

/// A source at a glance: its site, title and a line from it; a press opens it.
#[derive(IntoElement)]
pub struct SourceCard {
    id: ElementId,
    number: Option<usize>,
    source: Source,
    on_open: Option<Run>,
}

impl SourceCard {
    pub fn new(id: impl Into<ElementId>, source: Source) -> Self {
        Self {
            id: id.into(),
            number: None,
            source,
            on_open: None,
        }
    }

    /// The number the answer cites it by.
    pub fn number(mut self, number: usize) -> Self {
        self.number = Some(number);
        self
    }

    pub fn on_open(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_open = Some(Rc::new(handler));
        self
    }
}

/// A source's lines: the number it is cited by, its site, its title and a line from it.
fn lines(number: Option<usize>, source: &Source, cx: &App) -> Div {
    let theme = cx.theme();
    let colors = theme.colors.clone();
    div()
        .flex()
        .flex_col()
        .gap_0p5()
        .child(
            div()
                .flex()
                .items_center()
                .gap_1p5()
                .text_size(theme.text_size(TextSize::Xs))
                .text_color(colors.fg_subtle)
                .children(number.map(|number| {
                    tabular(
                        div()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(colors.fg_muted),
                    )
                    .child(format!("{number}"))
                }))
                .child(
                    Icon::new(IconName::Globe)
                        .size(IconSize::Xs)
                        .color(colors.fg_subtle),
                )
                .child(div().min_w_0().child(Ellipsis::new(source.site.clone()))),
        )
        .child(
            div()
                .text_size(theme.text_size(TextSize::Sm))
                .font_weight(FontWeight::MEDIUM)
                .child(Ellipsis::new(source.title.clone())),
        )
        .children(source.snippet.clone().map(|snippet| {
            div()
                .text_size(theme.text_size(TextSize::Xs))
                .text_color(colors.fg_muted)
                .child(snippet)
        }))
}

impl RenderOnce for SourceCard {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let open = self.on_open;
        div()
            .id(self.id.clone())
            .p_2p5()
            .rounded(theme.radius(Radius::Md))
            .border_1()
            .border_color(colors.border)
            .bg(colors.surface)
            .when_some(open, |card, open| {
                card.tab_index(0)
                    .focus_ring(cx)
                    .cursor_pointer()
                    .hover(|card| card.bg(colors.hover))
                    .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
                    .on_click(move |_, window, cx| {
                        log::info!("source card: open");
                        open(window, cx)
                    })
            })
            .child(lines(self.number, &self.source, cx))
    }
}

/// A citation in running text: a small numbered pill; resting on it shows its source, a press opens it.
#[derive(IntoElement)]
pub struct CitationBadge {
    id: ElementId,
    number: usize,
    source: Source,
    on_open: Option<Run>,
}

impl CitationBadge {
    pub fn new(id: impl Into<ElementId>, number: usize, source: Source) -> Self {
        Self {
            id: id.into(),
            number,
            source,
            on_open: None,
        }
    }

    pub fn on_open(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_open = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for CitationBadge {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let (source, number) = (self.source, self.number);
        let prose = theme.prose_width();
        let open = self.on_open.clone();
        let pill = div()
            .id((self.id.clone(), "pill"))
            .px_1p5()
            .rounded_full()
            .border_1()
            .border_color(colors.border)
            .bg(colors.hover)
            .text_size(theme.text_size(TextSize::Xs))
            .text_color(colors.fg_muted)
            .tab_index(0)
            .focus_ring(cx)
            .cursor_pointer()
            .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
            .when_some(open, |pill, open| {
                pill.on_click(move |_, window, cx| {
                    log::info!("citation: open {number}");
                    open(window, cx)
                })
            })
            .child(tabular(div()).child(format!("{number}")));
        HoverCard::new(self.id, pill, move |_, cx| {
            div().w(prose).child(lines(Some(number), &source, cx))
        })
    }
}

/// An answer's sources, folded under how many there are with their sites in a row; open, each as a card, numbered as cited. A press opens one.
#[derive(IntoElement)]
pub struct SourceList {
    id: ElementId,
    sources: Vec<Source>,
    on_open: Option<Pick>,
}

impl SourceList {
    pub fn new(id: impl Into<ElementId>, sources: impl IntoIterator<Item = Source>) -> Self {
        Self {
            id: id.into(),
            sources: sources.into_iter().collect(),
            on_open: None,
        }
    }

    /// Gets the index of the source to open.
    pub fn on_open(mut self, handler: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_open = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for SourceList {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let open = window.use_keyed_state((self.id.clone(), "open"), cx, |_, _| false);
        let opened = *open.read(cx);
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let count = self.sources.len();
        let sites: Vec<SharedString> = self
            .sources
            .iter()
            .map(|source| source.site.clone())
            .take(3)
            .collect();
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .id((self.id.clone(), "toggle"))
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_1()
                    .rounded(theme.radius(Radius::Sm))
                    .border_1()
                    .border_color(transparent_black())
                    .tab_index(0)
                    .focus_ring(cx)
                    .cursor_pointer()
                    .text_size(theme.text_size(TextSize::Sm))
                    .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
                    .on_click(move |_, _, cx| {
                        open.update(cx, |open, cx| {
                            *open = !*open;
                            log::info!("source list: open {open}");
                            cx.notify();
                        })
                    })
                    .child(
                        div()
                            .flex_none()
                            .font_weight(FontWeight::MEDIUM)
                            .child(match count {
                                1 => "1 source".to_string(),
                                n => format!("{n} sources"),
                            }),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_color(colors.fg_subtle)
                            .child(Ellipsis::new(sites.join(" · "))),
                    )
                    .child(div().flex_none().child(
                        Disclosure::new((self.id.clone(), "chevron"), opened).size(IconSize::Sm),
                    )),
            )
            .when(opened, |list| {
                list.child(div().flex().flex_col().gap_2().children(
                    self.sources.into_iter().enumerate().map(|(ix, source)| {
                        let card =
                            SourceCard::new((self.id.clone(), format!("source-{ix}")), source)
                                .number(ix + 1);
                        match self.on_open.clone() {
                            Some(open) => card.on_open(move |window, cx| open(ix, window, cx)),
                            None => card,
                        }
                    }),
                ))
            })
    }
}
