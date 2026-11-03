use std::{ops::Range, rc::Rc};

use gpui::{
    AnyElement, App, ElementId, FontWeight, HighlightStyle, InteractiveElement, IntoElement,
    MouseButton, ParentElement, RenderOnce, SharedString, StatefulInteractiveElement, Styled,
    StyledText, Window, div, prelude::*,
};

use super::{symbols::SymbolKind, syntax};
use crate::i18n;
use crate::{
    buttons::{ButtonVariant, IconButton},
    navigation::{fuzzy, marked},
    primitives::{Icon, IconName},
    theme::{ActiveTheme, ControlSize, Elevation, IconSize, Radius, TextSize},
    typography::Ellipsis,
};

/// A completion: what it inserts, its kind, a detail such as its type, and its documentation.
#[derive(Clone, Debug, PartialEq)]
pub struct CompletionItem {
    pub label: SharedString,
    pub kind: SymbolKind,
    pub detail: Option<SharedString>,
    pub docs: Option<SharedString>,
}

impl CompletionItem {
    pub fn new(label: impl Into<SharedString>, kind: SymbolKind) -> Self {
        Self {
            label: label.into(),
            kind,
            detail: None,
            docs: None,
        }
    }

    pub fn detail(mut self, detail: impl Into<SharedString>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    pub fn docs(mut self, docs: impl Into<SharedString>) -> Self {
        self.docs = Some(docs.into());
        self
    }
}

/// The items that fit what was typed, best first, with the letters that matched.
pub fn completions(
    items: &[CompletionItem],
    typed: &str,
) -> Vec<(CompletionItem, Vec<Range<usize>>)> {
    let mut fits: Vec<(i32, CompletionItem, Vec<Range<usize>>)> = items
        .iter()
        .filter_map(|item| {
            if typed.is_empty() {
                return Some((0, item.clone(), Vec::new()));
            }
            fuzzy(typed, &item.label).map(|fit| (fit.score, item.clone(), fit.hits))
        })
        .collect();
    fits.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.label.len().cmp(&b.1.label.len())));
    fits.into_iter()
        .map(|(_, item, hits)| (item, hits))
        .collect()
}

type OnPick = Rc<dyn Fn(&CompletionItem, &mut Window, &mut App)>;

/// Completions at the cursor: each with its kind's mark, the typed letters bold and its detail; the chosen one's documentation beside.
#[derive(IntoElement)]
pub struct CompletionMenu {
    id: ElementId,
    items: Vec<CompletionItem>,
    typed: SharedString,
    selected: usize,
    on_pick: Option<OnPick>,
}

impl CompletionMenu {
    pub fn new(id: impl Into<ElementId>, items: impl IntoIterator<Item = CompletionItem>) -> Self {
        Self {
            id: id.into(),
            items: items.into_iter().collect(),
            typed: SharedString::default(),
            selected: 0,
            on_pick: None,
        }
    }

    /// What was typed so far, which filters and marks the items.
    pub fn typed(mut self, typed: impl Into<SharedString>) -> Self {
        self.typed = typed.into();
        self
    }

    /// The chosen item among those that fit.
    pub fn selected(mut self, selected: usize) -> Self {
        self.selected = selected;
        self
    }

    pub fn on_pick(
        mut self,
        handler: impl Fn(&CompletionItem, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_pick = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for CompletionMenu {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let fits = completions(&self.items, &self.typed);
        let chosen = self.selected.min(fits.len().saturating_sub(1));
        let docs = fits
            .get(chosen)
            .and_then(|(item, _)| item.docs.clone().map(|docs| (item.detail.clone(), docs)));
        let rows: Vec<AnyElement> = fits
            .iter()
            .enumerate()
            .map(|(ix, (item, hits))| {
                let pick = self.on_pick.clone();
                let picked = item.clone();
                div()
                    .id((self.id.clone(), format!("item-{ix}")))
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_2()
                    .py_0p5()
                    .rounded(theme.radius(Radius::Sm))
                    .when(ix == chosen, |row| row.bg(colors.active))
                    .cursor_pointer()
                    .hover(|row| row.bg(colors.hover))
                    .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
                    .when_some(pick, |row, pick| {
                        row.on_click(move |_, window, cx| {
                            log::info!("completion: {}", picked.label);
                            pick(&picked, window, cx)
                        })
                    })
                    .child(
                        Icon::new(item.kind.icon())
                            .size(IconSize::Sm)
                            .color(item.kind.color(&colors)),
                    )
                    .child(
                        div()
                            .flex_none()
                            .font_family(theme.mono_family.clone())
                            .child(marked(item.label.clone(), hits.clone(), cx)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_right()
                            .text_color(colors.fg_subtle)
                            .children(item.detail.clone().map(Ellipsis::new)),
                    )
                    .into_any_element()
            })
            .collect();
        let empty = rows.is_empty();
        div()
            .flex()
            .items_start()
            .gap_1()
            .text_size(theme.text_size(TextSize::Sm))
            .child(
                div()
                    .w(theme.label_width() * 2.75)
                    .p_1()
                    .rounded(theme.radius(Radius::Md))
                    .border_1()
                    .border_color(colors.border)
                    .bg(colors.overlay)
                    .shadow(theme.elevation(Elevation::Floating))
                    .when(empty, |list| {
                        list.child(
                            div()
                                .px_2()
                                .py_1()
                                .text_color(colors.fg_subtle)
                                .child(i18n::text(cx, "completion.empty", &[])),
                        )
                    })
                    .children(rows),
            )
            .children(docs.map(|(detail, docs)| {
                div()
                    .w(theme.label_width() * 2.5)
                    .p_3()
                    .flex()
                    .flex_col()
                    .gap_1p5()
                    .rounded(theme.radius(Radius::Md))
                    .border_1()
                    .border_color(colors.border)
                    .bg(colors.overlay)
                    .shadow(theme.elevation(Elevation::Floating))
                    .children(detail.map(|detail| {
                        div()
                            .font_family(theme.mono_family.clone())
                            .text_color(colors.fg)
                            .child(detail)
                    }))
                    .child(div().text_color(colors.fg_muted).child(docs))
            }))
    }
}

/// A callable's shape: its label and where each parameter sits in it.
#[derive(Clone, Debug, PartialEq)]
pub struct Signature {
    pub label: SharedString,
    pub params: Vec<Range<usize>>,
    pub docs: Option<SharedString>,
}

type OnStep = Rc<dyn Fn(usize, &mut Window, &mut App)>;

/// The signature of the call being typed, the parameter at the cursor bold and underlined; arrows walk its overloads.
#[derive(IntoElement)]
pub struct SignatureHelp {
    id: ElementId,
    signatures: Vec<Signature>,
    active: usize,
    param: usize,
    on_step: Option<OnStep>,
}

impl SignatureHelp {
    pub fn new(id: impl Into<ElementId>, signatures: impl IntoIterator<Item = Signature>) -> Self {
        let signatures: Vec<Signature> = signatures.into_iter().collect();
        assert!(
            !signatures.is_empty(),
            "signature help shows at least one signature"
        );
        Self {
            id: id.into(),
            signatures,
            active: 0,
            param: 0,
            on_step: None,
        }
    }

    /// The overload shown, and the parameter the cursor is in.
    pub fn at(mut self, overload: usize, param: usize) -> Self {
        self.active = overload;
        self.param = param;
        self
    }

    /// Gets the overload an arrow steps to.
    pub fn on_step(mut self, handler: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_step = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for SignatureHelp {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let count = self.signatures.len();
        let active = self.active.min(count - 1);
        let signature = &self.signatures[active];
        let mut styles = syntax::colors(&signature.label, cx);
        if let Some(range) = signature.params.get(self.param) {
            styles.push((
                range.clone(),
                HighlightStyle {
                    font_weight: Some(FontWeight::BOLD),
                    underline: Some(gpui::UnderlineStyle {
                        thickness: theme.underline_thickness(),
                        color: Some(colors.fg),
                        wavy: false,
                    }),
                    ..HighlightStyle::default()
                },
            ));
        }
        let styles = super::layout::stack(styles);
        let step = |by: usize, icon: IconName, key: &'static str| {
            let on_step = self.on_step.clone();
            let next = (active + by) % count;
            IconButton::new((self.id.clone(), key), icon)
                .variant(ButtonVariant::Ghost)
                .size(ControlSize::Sm)
                .disabled(count < 2)
                .when_some(on_step, |button, on_step| {
                    button.on_click(move |_, window, cx| on_step(next, window, cx))
                })
        };
        div()
            .max_w(theme.label_width() * 5.0)
            .flex()
            .flex_col()
            .gap_1()
            .px_2()
            .py_1p5()
            .rounded(theme.radius(Radius::Md))
            .border_1()
            .border_color(colors.border)
            .bg(colors.overlay)
            .shadow(theme.elevation(Elevation::Floating))
            .text_size(theme.text_size(TextSize::Sm))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .when(count > 1, |row| {
                        row.child(step(count - 1, IconName::ChevronUp, "previous"))
                            .child(
                                div()
                                    .text_color(colors.fg_subtle)
                                    .child(format!("{}/{count}", active + 1)),
                            )
                            .child(step(1, IconName::ChevronDown, "next"))
                    })
                    .child(
                        div()
                            .font_family(theme.mono_family.clone())
                            .text_color(colors.syntax.variable)
                            .child(
                                StyledText::new(signature.label.clone()).with_highlights(styles),
                            ),
                    ),
            )
            .children(
                signature
                    .docs
                    .clone()
                    .map(|docs| div().text_color(colors.fg_muted).child(docs)),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completions_fit_what_was_typed_best_first() {
        let items = [
            CompletionItem::new("render", SymbolKind::Method),
            CompletionItem::new("request_layout", SymbolKind::Method),
            CompletionItem::new("width", SymbolKind::Field),
        ];
        let found = completions(&items, "re");
        let labels: Vec<&str> = found.iter().map(|(item, _)| &item.label[..]).collect();
        assert_eq!(labels, ["render", "request_layout"]);
        assert!(!found[0].1.is_empty(), "the typed letters are marked");
        assert_eq!(completions(&items, "").len(), 3);
    }
}
