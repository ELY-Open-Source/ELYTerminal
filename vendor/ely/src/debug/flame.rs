use std::rc::Rc;

use gpui::{
    App, Bounds, ElementId, Hsla, InteractiveElement, IntoElement, MouseButton, MouseDownEvent,
    MouseMoveEvent, ParentElement, Pixels, RenderOnce, SharedString, Styled, TextAlign, Window,
    canvas, div, fill, point, size,
};

use crate::{
    theme::{ActiveTheme, ControlSize, TextSize},
    typography::format,
};

/// A frame of a profile: its function, the samples taken in it alone, and the frames it called.
#[derive(Clone, Debug, PartialEq)]
pub struct ProfileFrame {
    pub name: SharedString,
    pub own: u64,
    pub calls: Vec<ProfileFrame>,
}

impl ProfileFrame {
    /// Samples in this frame and every frame under it.
    pub fn total(&self) -> u64 {
        self.own + self.calls.iter().map(ProfileFrame::total).sum::<u64>()
    }
}

/// A frame as the graph lays it out: its path of call indices from the root, its depth, and its share of the width.
#[derive(Clone, Debug, PartialEq)]
pub struct Placed {
    pub path: Vec<usize>,
    pub name: SharedString,
    pub depth: usize,
    pub start: f32,
    pub end: f32,
    pub samples: u64,
}

/// The frames under `focus`, a path of call indices kept as far as the profile holds it, each spanning its share of the samples; `focus` fills the width and its callers stack above it, full width.
pub fn place(root: &ProfileFrame, focus: &[usize]) -> Vec<Placed> {
    let mut out = Vec::new();
    let mut frame = root;
    let mut kept = focus.len();
    for (depth, ix) in focus.iter().enumerate() {
        let Some(call) = frame.calls.get(*ix) else {
            log::error!("flame graph: focus {focus:?} leaves the profile at depth {depth}");
            kept = depth;
            break;
        };
        out.push(Placed {
            path: focus[..depth].to_vec(),
            name: frame.name.clone(),
            depth,
            start: 0.0,
            end: 1.0,
            samples: frame.total(),
        });
        frame = call;
    }
    fn under(
        frame: &ProfileFrame,
        path: Vec<usize>,
        depth: usize,
        start: f32,
        end: f32,
        out: &mut Vec<Placed>,
    ) {
        let total = frame.total().max(1) as f32;
        out.push(Placed {
            path: path.clone(),
            name: frame.name.clone(),
            depth,
            start,
            end,
            samples: frame.total(),
        });
        let mut at = start;
        for (ix, call) in frame.calls.iter().enumerate() {
            let width = (end - start) * call.total() as f32 / total;
            let mut inner = path.clone();
            inner.push(ix);
            under(call, inner, depth + 1, at, at + width, out);
            at += width;
        }
    }
    under(frame, focus[..kept].to_vec(), kept, 0.0, 1.0, &mut out);
    out
}

type OnFocus = Rc<dyn Fn(Vec<usize>, &mut Window, &mut App)>;
type OnPath = Rc<dyn Fn(Option<Vec<usize>>, &mut Window, &mut App)>;

/// A flame graph: each function as wide as its samples, callees stacked under callers, colored by name in the chart palette. Hover reads a frame; a press zooms into it and a press on a caller above zooms back out.
#[derive(IntoElement)]
pub struct Flamegraph {
    id: ElementId,
    root: Rc<ProfileFrame>,
    focus: Vec<usize>,
    hovered: Option<Vec<usize>>,
    on_focus: Option<OnFocus>,
    on_hover: Option<OnPath>,
}

impl Flamegraph {
    pub fn new(id: impl Into<ElementId>, root: impl Into<Rc<ProfileFrame>>) -> Self {
        Self {
            id: id.into(),
            root: root.into(),
            focus: Vec::new(),
            hovered: None,
            on_focus: None,
            on_hover: None,
        }
    }

    /// The frame zoomed into, as call indices from the root.
    pub fn focus(mut self, path: Vec<usize>) -> Self {
        self.focus = path;
        self
    }

    pub fn hovered(mut self, path: Option<Vec<usize>>) -> Self {
        self.hovered = path;
        self
    }

    pub fn on_focus(
        mut self,
        handler: impl Fn(Vec<usize>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_focus = Some(Rc::new(handler));
        self
    }

    pub fn on_hover(
        mut self,
        handler: impl Fn(Option<Vec<usize>>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_hover = Some(Rc::new(handler));
        self
    }
}

/// A name cut to `room` characters, ending in an ellipsis; nothing when fewer than two fit.
fn fitted(name: &str, room: usize) -> String {
    let count = name.chars().count();
    match room {
        room if room >= count => name.to_string(),
        room if room < 2 => String::new(),
        room => name.chars().take(room - 1).chain(['…']).collect(),
    }
}

/// A steady color for a function's name.
fn tint(name: &str, palette: &[Hsla; 8]) -> Hsla {
    let hash = name.bytes().fold(0u32, |hash, byte| {
        hash.wrapping_mul(31).wrapping_add(byte as u32)
    });
    palette[hash as usize % palette.len()]
}

impl RenderOnce for Flamegraph {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let row = theme
            .control_height(ControlSize::Sm)
            .to_pixels(window.rem_size());
        let placed = Rc::new(place(&self.root, &self.focus));
        let depth = placed.iter().map(|frame| frame.depth).max().unwrap_or(0) + 1;
        let total = self.root.total();
        let reading = self
            .hovered
            .as_ref()
            .and_then(|path| placed.iter().find(|frame| &frame.path == path))
            .map(|frame| {
                format!(
                    "{} · {} · {:.1} % of all",
                    frame.name,
                    format::plural(frame.samples, "sample", "samples"),
                    frame.samples as f32 * 100.0 / total.max(1) as f32
                )
            });
        let hit = move |placed: &[Placed],
                        bounds: Bounds<Pixels>,
                        at: gpui::Point<Pixels>|
              -> Option<Vec<usize>> {
            let x = (at.x - bounds.origin.x) / bounds.size.width;
            let level = ((at.y - bounds.origin.y) / row).floor() as usize;
            placed
                .iter()
                .find(|frame| frame.depth == level && frame.start <= x && x < frame.end)
                .map(|frame| frame.path.clone())
        };
        let (paint_placed, press_placed, move_placed) =
            (placed.clone(), placed.clone(), placed.clone());
        let (hovered, palette) = (self.hovered.clone(), colors.chart);
        let (on_focus, on_hover) = (self.on_focus, self.on_hover);
        let (fg, hover_ring, text_size) = (colors.fg, colors.focus, theme.text_size(TextSize::Xs));
        let font = theme.mono_family.clone();
        let gap = theme.chart().hairline;
        let bounds_cell = Rc::new(std::cell::Cell::new(Bounds::default()));
        let (measure, pressed_at, moved_at) =
            (bounds_cell.clone(), bounds_cell.clone(), bounds_cell);
        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .h(row)
                    .text_size(theme.text_size(TextSize::Sm))
                    .text_color(colors.fg_muted)
                    .child(reading.unwrap_or_else(|| {
                        format!(
                            "{} in all · press a frame to zoom in",
                            format::plural(total, "sample", "samples")
                        )
                    })),
            )
            .child(
                div()
                    .id(self.id.clone())
                    .relative()
                    .w_full()
                    .h(row * depth as f32)
                    .cursor_pointer()
                    .on_mouse_down(
                        MouseButton::Left,
                        move |event: &MouseDownEvent, window, cx| {
                            if let (Some(on_focus), Some(path)) = (
                                &on_focus,
                                hit(&press_placed, pressed_at.get(), event.position),
                            ) {
                                log::info!("flamegraph: focus {path:?}");
                                on_focus(path, window, cx)
                            }
                        },
                    )
                    .on_mouse_move(move |event: &MouseMoveEvent, window, cx| {
                        if let Some(on_hover) = &on_hover {
                            on_hover(
                                hit(&move_placed, moved_at.get(), event.position),
                                window,
                                cx,
                            )
                        }
                    })
                    .child(
                        canvas(
                            move |bounds, _, _| measure.set(bounds),
                            move |bounds, _, window, cx| {
                                let pixels = text_size.to_pixels(window.rem_size());
                                let face = gpui::font(font.clone());
                                let resolved = window.text_system().resolve_font(&face);
                                let advance =
                                    match window.text_system().advance(resolved, pixels, 'm') {
                                        Ok(advance) => advance.width,
                                        Err(error) => {
                                            log::error!(
                                                "flame graph: no advance for its font: {error:#}"
                                            );
                                            pixels * 0.6
                                        }
                                    };
                                for frame in paint_placed.iter() {
                                    let x0 = bounds.origin.x + bounds.size.width * frame.start;
                                    let width = bounds.size.width * (frame.end - frame.start);
                                    let origin =
                                        point(x0, bounds.origin.y + row * frame.depth as f32);
                                    let cell = Bounds::new(
                                        origin,
                                        size(width - gap.min(width), row - gap),
                                    );
                                    let color = tint(&frame.name, &palette);
                                    let lit = hovered.as_ref() == Some(&frame.path);
                                    window.paint_quad(fill(
                                        cell,
                                        if lit { color } else { color.opacity(0.55) },
                                    ));
                                    if lit {
                                        window.paint_quad(gpui::outline(
                                            cell,
                                            hover_ring,
                                            gpui::BorderStyle::Solid,
                                        ));
                                    }
                                    let label = fitted(
                                        &frame.name,
                                        ((width - pixels * 0.8) / advance).max(0.0) as usize,
                                    );
                                    if !label.is_empty() {
                                        let run = gpui::TextRun {
                                            len: label.len(),
                                            font: face.clone(),
                                            color: fg,
                                            background_color: None,
                                            underline: None,
                                            strikethrough: None,
                                        };
                                        let painted = window
                                            .text_system()
                                            .shape_line(label.into(), pixels, &[run], None)
                                            .paint(
                                                origin
                                                    + point(
                                                        pixels * 0.4,
                                                        (row - pixels * 1.2) / 2.0,
                                                    ),
                                                pixels * 1.2,
                                                TextAlign::Left,
                                                None,
                                                window,
                                                cx,
                                            );
                                        if let Err(error) = painted {
                                            log::error!(
                                                "flame graph: a label failed to paint: {error:#}"
                                            );
                                        }
                                    }
                                }
                            },
                        )
                        .absolute()
                        .top_0()
                        .left_0()
                        .size_full(),
                    ),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(name: &str, own: u64, calls: Vec<ProfileFrame>) -> ProfileFrame {
        ProfileFrame {
            name: name.to_string().into(),
            own,
            calls,
        }
    }

    fn profile() -> ProfileFrame {
        frame(
            "main",
            10,
            vec![
                frame("render", 30, vec![frame("layout", 20, vec![])]),
                frame("io", 40, vec![]),
            ],
        )
    }

    #[test]
    fn names_fit_their_frames() {
        assert_eq!(fitted("render", 10), "render");
        assert_eq!(fitted("render", 4), "ren…");
        assert_eq!(fitted("render", 1), "");
    }

    #[test]
    fn frames_span_their_share() {
        let placed = place(&profile(), &[]);
        let spans: Vec<(&str, usize, f32, f32)> = placed
            .iter()
            .map(|frame| (frame.name.as_ref(), frame.depth, frame.start, frame.end))
            .collect();
        assert_eq!(
            spans,
            [
                ("main", 0, 0.0, 1.0),
                ("render", 1, 0.0, 0.5),
                ("layout", 2, 0.0, 0.2),
                ("io", 1, 0.5, 0.9)
            ]
        );
    }

    #[test]
    fn a_focused_frame_fills_the_width_under_its_callers() {
        let placed = place(&profile(), &[0]);
        let spans: Vec<(&str, usize, f32, f32)> = placed
            .iter()
            .map(|frame| (frame.name.as_ref(), frame.depth, frame.start, frame.end))
            .collect();
        assert_eq!(
            spans,
            [
                ("main", 0, 0.0, 1.0),
                ("render", 1, 0.0, 1.0),
                ("layout", 2, 0.0, 0.4)
            ]
        );
        assert_eq!(placed[1].path, [0], "the focused frame keeps its path");
    }
}
