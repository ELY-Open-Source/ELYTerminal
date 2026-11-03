use std::rc::Rc;

use gpui::{
    AnyElement, App, Bounds, ElementId, Hsla, InteractiveElement, IntoElement, ParentElement,
    PathBuilder, Pixels, RenderOnce, Role, SharedString, StatefulInteractiveElement, Styled,
    Window, canvas, div, point, prelude::*, size, uniform_list,
};

use super::graph::{GraphRow, Half};
use crate::{
    data_display::{Avatar, Tag, Tone},
    primitives::FocusRing,
    theme::{ActiveTheme, AvatarSize, ControlSize, IconSize, Radius, TextSize},
    typography::Ellipsis,
};

/// A commit as lists show it; first parent first.
#[derive(Clone, Debug, PartialEq)]
pub struct Commit {
    pub id: SharedString,
    pub parents: Vec<SharedString>,
    pub subject: SharedString,
    pub author: SharedString,
    pub when: SharedString,
    pub refs: Vec<SharedString>,
}

type OnPick = Rc<dyn Fn(&SharedString, &mut Window, &mut App)>;

/// One commit in a row: author, subject, refs, id, when.
#[derive(IntoElement)]
pub struct CommitItem {
    id: ElementId,
    commit: Commit,
    selected: bool,
    on_pick: Option<OnPick>,
}

impl CommitItem {
    pub fn new(id: impl Into<ElementId>, commit: Commit) -> Self {
        Self {
            id: id.into(),
            commit,
            selected: false,
            on_pick: None,
        }
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn on_pick(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_pick = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for CommitItem {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let commit = self.commit;
        let short: String = commit.id.chars().take(7).collect();
        let (pick, picked) = (self.on_pick, commit.id.clone());
        // Refs are tags drawn only; the name carries them.
        let spoken: SharedString = match commit.refs.is_empty() {
            true => commit.subject.clone(),
            false => format!("{} ({})", commit.subject, commit.refs.join(", ")).into(),
        };
        div()
            .id(self.id.clone())
            .role(Role::ListItem)
            .aria_label(spoken)
            .aria_selected(self.selected)
            .tab_index(0)
            .border_1()
            .border_color(gpui::transparent_black())
            .focus_ring(cx)
            .flex_1()
            .min_w_0()
            .h(theme.control_height(ControlSize::Lg))
            .flex()
            .items_center()
            .gap_2()
            .px_2()
            .rounded(theme.radius(Radius::Sm))
            .text_size(theme.text_size(TextSize::Sm))
            .when(self.selected, |row| row.bg(colors.active))
            .when(!self.selected, |row| row.hover(|row| row.bg(colors.hover)))
            .when_some(pick, |row, pick| {
                row.cursor_pointer()
                    .on_click(move |_, window, cx| pick(&picked, window, cx))
            })
            .child(
                Avatar::new((self.id.clone(), "author"), commit.author.clone())
                    .size(AvatarSize::Xs),
            )
            .children(commit.refs.iter().enumerate().map(|(ix, name)| {
                let tone = if name.starts_with('v') {
                    Tone::Warning
                } else if name.contains('/') {
                    Tone::Neutral
                } else {
                    Tone::Info
                };
                Tag::new((self.id.clone(), format!("ref-{ix}")), name.clone()).tone(tone)
            }))
            .child(
                div()
                    .flex_1()
                    .min_w_24()
                    .text_color(colors.fg)
                    .child(Ellipsis::new(commit.subject)),
            )
            // Narrow, the details give way before the subject.
            .child(
                div()
                    .flex_shrink(1.0)
                    .min_w_0()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .flex()
                    .gap_2()
                    .child(div().text_color(colors.fg_muted).child(commit.author))
                    .child(
                        div()
                            .font_family(theme.mono_family.clone())
                            .text_color(colors.fg_subtle)
                            .child(short),
                    )
                    .child(div().text_color(colors.fg_subtle).child(commit.when)),
            )
    }
}

type OnEnd = Rc<dyn Fn(&mut App)>;

/// Commits newest first; draws rows in view, fills its box.
#[derive(IntoElement)]
pub struct CommitList {
    id: ElementId,
    commits: Rc<Vec<Commit>>,
    graph: Option<Rc<Vec<GraphRow>>>,
    selected: Option<SharedString>,
    on_pick: Option<OnPick>,
    on_end: Option<OnEnd>,
}

impl CommitList {
    pub fn new(id: impl Into<ElementId>, commits: Rc<Vec<Commit>>) -> Self {
        Self {
            id: id.into(),
            commits,
            graph: None,
            selected: None,
            on_pick: None,
            on_end: None,
        }
    }

    pub fn selected(mut self, id: impl Into<SharedString>) -> Self {
        self.selected = Some(id.into());
        self
    }

    pub fn on_pick(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_pick = Some(Rc::new(handler));
        self
    }

    /// Lanes beside the rows, from `lanes`.
    pub fn graph(mut self, rows: Rc<Vec<GraphRow>>) -> Self {
        self.graph = Some(rows);
        self
    }

    /// Called when rows near the end draw.
    pub fn on_end(mut self, handler: impl Fn(&mut App) + 'static) -> Self {
        self.on_end = Some(Rc::new(handler));
        self
    }
}

/// Rows from the end that ask for more.
const NEAR_END: usize = 20;

/// Paints one row's lanes, `span` wide so rows line up.
fn lanes_cell(
    row: GraphRow,
    span: usize,
    lane: Pixels,
    height: Pixels,
    stroke: Pixels,
    palette: [Hsla; 8],
) -> AnyElement {
    let width = lane * span as f32;
    canvas(
        |_, _, _| {},
        move |bounds: Bounds<Pixels>, _, window, _| {
            let x = |at: usize| bounds.origin.x + lane * (at as f32 + 0.5);
            let (top, middle, bottom) = (
                bounds.origin.y,
                bounds.origin.y + height / 2.0,
                bounds.origin.y + height,
            );
            for piece in &row.strokes {
                let (from, to) = match piece.half {
                    Half::Top => (point(x(piece.from), top), point(x(piece.to), middle)),
                    Half::Bottom => (point(x(piece.from), middle), point(x(piece.to), bottom)),
                    Half::Through => (point(x(piece.from), top), point(x(piece.to), bottom)),
                };
                let color = palette[piece.from.max(piece.to) % palette.len()];
                let mut path = PathBuilder::stroke(stroke);
                path.move_to(from);
                path.line_to(to);
                match path.build() {
                    Ok(path) => window.paint_path(path, color),
                    Err(error) => log::error!("commit graph: a stroke failed to build: {error:#}"),
                }
            }
            let dot = lane * 0.45;
            let center = point(x(row.lane), middle);
            window.paint_quad(
                gpui::fill(
                    Bounds::new(center - point(dot / 2.0, dot / 2.0), size(dot, dot)),
                    palette[row.lane % palette.len()],
                )
                .corner_radii(dot / 2.0),
            );
        },
    )
    .flex_none()
    .w(width)
    .h(height)
    .into_any_element()
}

impl RenderOnce for CommitList {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        if let Some(graph) = &self.graph
            && graph.len() != self.commits.len()
        {
            log::error!(
                "commit list: {} graph rows for {} commits; rows past them draw bare",
                graph.len(),
                self.commits.len()
            );
        }
        let span = self
            .graph
            .iter()
            .flat_map(|rows| rows.iter())
            .map(|row| row.width)
            .max()
            .unwrap_or(0);
        let Self {
            id,
            commits,
            graph,
            selected,
            on_pick,
            on_end,
        } = self;
        let count = commits.len();
        uniform_list((id.clone(), "rows"), count, move |range, window, cx| {
            if let Some(on_end) = on_end.clone().filter(|_| range.end + NEAR_END >= count) {
                cx.defer(move |cx| on_end(cx));
            }
            let theme = cx.theme();
            let rem = window.rem_size();
            let height = theme.control_height(ControlSize::Lg).to_pixels(rem);
            let lane = theme.icon_size(IconSize::Md).to_pixels(rem);
            let stroke = theme.chart().hairline * 1.5;
            let palette = theme.colors.chart;
            range
                .map(|ix| {
                    let commit = commits[ix].clone();
                    let picked = selected.as_ref() == Some(&commit.id);
                    let item =
                        CommitItem::new((id.clone(), commit.id.clone()), commit).selected(picked);
                    let item = match &on_pick {
                        Some(pick) => {
                            let pick = pick.clone();
                            item.on_pick(move |id, window, cx| pick(id, window, cx))
                        }
                        None => item,
                    };
                    let lanes = graph.as_ref().and_then(|rows| rows.get(ix)).cloned();
                    div()
                        .w_full()
                        .flex()
                        .items_center()
                        .children(
                            lanes.map(|row| lanes_cell(row, span, lane, height, stroke, palette)),
                        )
                        .child(item)
                })
                .collect()
        })
        .size_full()
    }
}
