use std::rc::Rc;

use gpui::{
    AnyElement, App, Div, ElementId, FontWeight, Hsla, InteractiveElement, IntoElement,
    ParentElement, RenderOnce, Role, SharedString, StatefulInteractiveElement, Styled, Window, div,
    prelude::*, uniform_list,
};

use similar::DiffableStr;

use super::merging::{MARKER, Region, RegionKind, Take, conflicts, resolve};
use crate::{
    buttons::{Button, ButtonVariant},
    i18n,
    primitives::{Icon, IconName},
    theme::{ActiveTheme, ControlSize, IconSize, Palette, Radius, TextSize},
};

type OnTake = Rc<dyn Fn(usize, Take, &mut Window, &mut App)>;
type OnText = Rc<dyn Fn(String, &mut Window, &mut App)>;

/// The three buttons that settle a conflict.
fn takes(
    id: &ElementId,
    conflict: usize,
    on_take: &Option<OnTake>,
    words: [SharedString; 3],
) -> Div {
    let [ours, theirs, both] = words;
    div().flex().gap_1().children(
        [
            (Take::Ours, ours),
            (Take::Theirs, theirs),
            (Take::Both, both),
        ]
        .map(|(take, words)| {
            let on_take = on_take.clone();
            Button::new((id.clone(), format!("take-{conflict}-{take:?}")), words)
                .variant(ButtonVariant::Ghost)
                .size(ControlSize::Sm)
                .when_some(on_take, |button, on_take| {
                    button.on_click(move |_, window, cx| on_take(conflict, take, window, cx))
                })
        }),
    )
}

/// Ours, base and theirs side by side, rows in view.
#[derive(IntoElement)]
pub struct ThreeWayMerge {
    id: ElementId,
    labels: [SharedString; 3],
    regions: Rc<Vec<Region>>,
    takes: Vec<Option<Take>>,
    on_take: Option<OnTake>,
}

impl ThreeWayMerge {
    /// `labels` name ours, the base and theirs.
    pub fn new(id: impl Into<ElementId>, labels: [&str; 3], regions: Rc<Vec<Region>>) -> Self {
        Self {
            id: id.into(),
            labels: labels.map(|label| label.to_string().into()),
            regions,
            takes: Vec::new(),
            on_take: None,
        }
    }

    /// The side each conflict took so far, in order.
    pub fn takes(mut self, takes: impl IntoIterator<Item = Option<Take>>) -> Self {
        self.takes = takes.into_iter().collect();
        self
    }

    pub fn on_take(
        mut self,
        handler: impl Fn(usize, Take, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_take = Some(Rc::new(handler));
        self
    }
}

fn washes(kind: RegionKind, colors: &Palette) -> [Option<Hsla>; 3] {
    let changed = colors.success.opacity(0.1);
    match kind {
        RegionKind::Unchanged => [None, None, None],
        RegionKind::Ours => [Some(changed), None, None],
        RegionKind::Theirs => [None, None, Some(changed)],
        RegionKind::Both => [Some(changed), None, Some(changed)],
        RegionKind::Conflict => [Some(colors.warning.opacity(0.14)); 3],
    }
}

/// A row as the merge draws it.
#[derive(Clone)]
enum Line {
    /// A conflict's place, before its lines.
    Conflict(usize),
    /// A line of each side, empty past its end.
    Code(RegionKind, [Option<String>; 3]),
}

fn rows(regions: &[Region]) -> Vec<Line> {
    let mut rows = Vec::new();
    let mut conflict = 0;
    for region in regions {
        if region.kind == RegionKind::Conflict {
            rows.push(Line::Conflict(conflict));
            conflict += 1;
        }
        let count = region
            .ours
            .len()
            .max(region.base.len())
            .max(region.theirs.len());
        let line = |side: &[String], ix: usize| {
            side.get(ix)
                .map(|line| line.trim_end_matches(['\r', '\n']).to_string())
        };
        rows.extend((0..count).map(|ix| {
            Line::Code(
                region.kind,
                [
                    line(&region.ours, ix),
                    line(&region.base, ix),
                    line(&region.theirs, ix),
                ],
            )
        }));
    }
    rows
}

impl RenderOnce for ThreeWayMerge {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let height = theme.control_height(ControlSize::Sm);
        let rows = Rc::new(rows(&self.regions));
        let Self {
            id,
            labels,
            takes: taken,
            on_take,
            ..
        } = self;
        let header = div()
            .flex()
            .gap_px()
            .bg(colors.hover)
            .font_weight(FontWeight::MEDIUM)
            .text_color(colors.fg_muted)
            .children(
                labels
                    .iter()
                    .map(|label| div().flex_1().min_w_0().px_2().py_1().child(label.clone())),
            );
        let list = uniform_list((id.clone(), "rows"), rows.len(), move |range, _, cx| {
            let colors = cx.theme().colors.clone();
            range
                .map(|ix| {
                    let row = div()
                        .id((id.clone(), format!("row-{ix}")))
                        .w_full()
                        .h(height)
                        .flex()
                        .items_center();
                    match &rows[ix] {
                        Line::Conflict(at) => {
                            let said = match taken.get(*at).copied().flatten() {
                                Some(take) => i18n::text(
                                    cx,
                                    "git.merge.took",
                                    &[
                                        ("n", &(at + 1).to_string()),
                                        ("side", &i18n::text(cx, word(take), &[])),
                                    ],
                                ),
                                None => i18n::text(
                                    cx,
                                    "git.merge.conflict",
                                    &[("n", &(at + 1).to_string())],
                                ),
                            };
                            let words = [
                                "git.merge.take_ours",
                                "git.merge.take_theirs",
                                "git.merge.take_both",
                            ]
                            .map(|key| i18n::text(cx, key, &[]));
                            row.gap_2()
                                .px_2()
                                .role(Role::Heading)
                                .aria_label(said.clone())
                                .border_b_1()
                                .border_color(colors.border)
                                .font_family(cx.theme().font_family.clone())
                                .child(
                                    Icon::new(IconName::TriangleAlert)
                                        .size(IconSize::Sm)
                                        .color(colors.warning),
                                )
                                .child(div().flex_1().text_color(colors.fg_muted).child(said))
                                .child(takes(&id, *at, &on_take, words))
                                .into_any_element()
                        }
                        Line::Code(kind, sides) => {
                            let washes = washes(*kind, &colors);
                            // Each side named by its label, for assistive tech.
                            let said: Vec<String> = labels
                                .iter()
                                .zip(sides)
                                .map(|(label, line)| {
                                    format!("{label}: {}", line.as_deref().unwrap_or(""))
                                })
                                .collect();
                            row.gap_px()
                                .role(Role::Label)
                                .aria_label(SharedString::from(said.join(" · ")))
                                .children(sides.iter().zip(washes).map(|(line, wash)| {
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .h_full()
                                        .px_2()
                                        .flex()
                                        .items_center()
                                        .whitespace_nowrap()
                                        .overflow_hidden()
                                        .when_some(wash, |side, wash| side.bg(wash))
                                        .children(line.clone())
                                }))
                                .into_any_element()
                        }
                    }
                })
                .collect()
        })
        .flex_1();
        div()
            .size_full()
            .flex()
            .flex_col()
            .rounded(theme.radius(Radius::Md))
            .border_1()
            .border_color(colors.border)
            .overflow_hidden()
            .text_size(theme.text_size(TextSize::Sm))
            .child(header)
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .flex_col()
                    .font_family(theme.mono_family.clone())
                    .text_size(theme.text_size(TextSize::Xs))
                    .child(list),
            )
    }
}

fn word(take: Take) -> &'static str {
    match take {
        Take::Ours => "git.merge.ours",
        Take::Theirs => "git.merge.theirs",
        Take::Both => "git.merge.both",
    }
}

/// Marked conflicts, each accepting current, incoming, or both.
#[derive(IntoElement)]
pub struct ConflictResolver {
    id: ElementId,
    text: SharedString,
    on_resolve: Option<OnText>,
}

impl ConflictResolver {
    pub fn new(id: impl Into<ElementId>, text: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
            on_resolve: None,
        }
    }

    /// Gets the whole text once a conflict settles.
    pub fn on_resolve(mut self, handler: impl Fn(String, &mut Window, &mut App) + 'static) -> Self {
        self.on_resolve = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for ConflictResolver {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let found = conflicts(&self.text, MARKER);
        let text = self.text.to_string();
        let on_take: Option<OnTake> = self.on_resolve.map(|on_resolve| {
            Rc::new(
                move |ix: usize, take: Take, window: &mut Window, cx: &mut App| {
                    if let Some(settled) = resolve(&text, ix, take, MARKER) {
                        on_resolve(settled, window, cx)
                    }
                },
            ) as OnTake
        });
        let (current, incoming) = (colors.success.opacity(0.1), colors.info.opacity(0.1));
        let mut rows: Vec<AnyElement> = Vec::new();
        for (ix, line) in self.text.tokenize_lines().into_iter().enumerate() {
            let line = line.trim_end_matches(['\r', '\n']);
            if let Some(at) = found.iter().position(|conflict| conflict.start == ix) {
                rows.push(
                    takes(
                        &self.id,
                        at,
                        &on_take,
                        [
                            "git.merge.accept_current",
                            "git.merge.accept_incoming",
                            "git.merge.accept_both",
                        ]
                        .map(|key| i18n::text(cx, key, &[])),
                    )
                    .py_0p5()
                    .font_family(theme.font_family.clone())
                    .into_any_element(),
                );
            }
            let wash = found.iter().find_map(|conflict| {
                if ix == conflict.start {
                    Some(current.opacity(2.0))
                } else if ix > conflict.start && ix < conflict.middle {
                    Some(current)
                } else if ix > conflict.middle && ix < conflict.end {
                    Some(incoming)
                } else if ix == conflict.end {
                    Some(incoming.opacity(2.0))
                } else {
                    None
                }
            });
            let marker = found
                .iter()
                .any(|conflict| [conflict.start, conflict.middle, conflict.end].contains(&ix));
            rows.push(
                div()
                    .px_2()
                    .whitespace_nowrap()
                    .when_some(wash, |row, wash| row.bg(wash))
                    .when(marker, |row| row.text_color(colors.fg_muted))
                    .child(line.to_string())
                    .into_any_element(),
            );
        }
        div()
            .flex()
            .flex_col()
            .rounded(theme.radius(Radius::Md))
            .border_1()
            .border_color(colors.border)
            .py_1()
            .font_family(theme.mono_family.clone())
            .text_size(theme.text_size(TextSize::Xs))
            .text_color(colors.fg)
            .children(rows)
    }
}
