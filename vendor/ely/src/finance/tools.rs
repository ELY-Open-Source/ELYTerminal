use std::rc::Rc;

use gpui::{
    App, ElementId, IntoElement, PathBuilder, Pixels, Point, RenderOnce, SharedString, Window, fill,
};

use super::{
    draw::Scene,
    series::{Overlay, Study},
};
use crate::{
    buttons::{ButtonVariant, ToggleGroup, ToggleItem},
    charts::{Rect, at, finish, place, ring, tint},
    menus::{DropdownMenu, Menu, MenuItem},
    primitives::IconName,
    theme::ControlSize,
};

/// A tool that draws on a market chart.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tool {
    Trend,
    Level,
    Box,
    Fib,
}

/// A mark drawn on a market chart, pinned to candles and prices so it moves with them: each point is a candle, which may fall between two, and a price.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Drawing {
    /// A straight line between two points.
    Trend((f64, f64), (f64, f64)),
    /// A level across the chart.
    Level(f64),
    /// A shaded rectangle between two corners.
    Box((f64, f64), (f64, f64)),
    /// Retracement levels between a swing's two ends.
    Fib((f64, f64), (f64, f64)),
}

/// The shares of a swing that retracement levels mark.
pub const FIB: [f64; 6] = [0.0, 0.236, 0.382, 0.5, 0.618, 1.0];

impl Drawing {
    /// What a tool draws from where a drag began to where it is.
    pub(crate) fn of(tool: Tool, from: (f64, f64), to: (f64, f64)) -> Drawing {
        match tool {
            Tool::Trend => Drawing::Trend(from, to),
            Tool::Level => Drawing::Level(to.1),
            Tool::Box => Drawing::Box(from, to),
            Tool::Fib => Drawing::Fib(from, to),
        }
    }

    /// Each retracement level's price, from the swing's end back toward its start.
    pub(crate) fn levels((_, start): (f64, f64), (_, end): (f64, f64)) -> [(f64, f64); 6] {
        FIB.map(|share| (share, end - (end - start) * share))
    }
}

/// Each tool with its key, words and icon; the pointer draws nothing.
const TOOLS: [(Option<Tool>, &str, &str, IconName); 5] = [
    (None, "pointer", "Point", IconName::MousePointer2),
    (
        Some(Tool::Trend),
        "trend",
        "Trend line",
        IconName::TrendingUp,
    ),
    (Some(Tool::Level), "level", "Level", IconName::Minus),
    (Some(Tool::Box), "box", "Box", IconName::Square),
    (
        Some(Tool::Fib),
        "fib",
        "Fibonacci retracement",
        IconName::AlignJustify,
    ),
];

type OnTool = Rc<dyn Fn(Option<Tool>, &mut Window, &mut App)>;

/// The tools that draw on a market chart, and the pointer that draws nothing, one chosen at a time.
#[derive(IntoElement)]
pub struct DrawingToolbar {
    id: ElementId,
    tool: Option<Tool>,
    on_change: Option<OnTool>,
}

impl DrawingToolbar {
    pub fn new(id: impl Into<ElementId>, tool: Option<Tool>) -> Self {
        Self {
            id: id.into(),
            tool,
            on_change: None,
        }
    }

    pub fn on_change(
        mut self,
        handler: impl Fn(Option<Tool>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for DrawingToolbar {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let chosen = TOOLS
            .iter()
            .find(|(tool, ..)| *tool == self.tool)
            .map(|(_, key, ..)| *key)
            .expect("every tool is listed");
        let group = TOOLS.iter().fold(
            ToggleGroup::new(self.id).size(ControlSize::Sm),
            |group, (_, key, words, icon)| {
                group.item(ToggleItem::new(*key).icon(*icon).tooltip(*words))
            },
        );
        let group = group.selected([chosen]);
        match self.on_change {
            Some(on_change) => group.on_change(move |keys, window, cx| {
                let tool = keys
                    .first()
                    .and_then(|key| TOOLS.iter().find(|(_, listed, ..)| *listed == key.as_ref()))
                    .and_then(|(tool, ..)| *tool);
                log::info!("drawing toolbar: {tool:?}");
                on_change(tool, window, cx)
            }),
            None => group,
        }
    }
}

/// The overlays and studies an indicator menu offers.
const OVERLAYS: [(Overlay, &str); 5] = [
    (Overlay::Sma(20), "Moving average 20"),
    (Overlay::Ema(50), "Exponential average 50"),
    (Overlay::Bollinger(20, 2.0), "Bollinger bands"),
    (Overlay::Vwap, "VWAP"),
    (Overlay::Sma(200), "Moving average 200"),
];

const STUDIES: [(Study, &str); 4] = [
    (Study::Macd, "MACD"),
    (Study::Rsi, "RSI"),
    (Study::Kdj, "KDJ"),
    (Study::Obv, "On-balance volume"),
];

type OnIndicators = Rc<dyn Fn(Vec<Overlay>, Vec<Study>, &mut Window, &mut App)>;

/// A menu of indicators, each checked when shown: overlays over the prices, and studies in panes of their own.
#[derive(IntoElement)]
pub struct IndicatorSelector {
    id: ElementId,
    overlays: Vec<Overlay>,
    studies: Vec<Study>,
    on_change: Option<OnIndicators>,
}

impl IndicatorSelector {
    pub fn new(id: impl Into<ElementId>, overlays: Vec<Overlay>, studies: Vec<Study>) -> Self {
        Self {
            id: id.into(),
            overlays,
            studies,
            on_change: None,
        }
    }

    /// Gets the overlays and studies after one is checked or cleared.
    pub fn on_change(
        mut self,
        handler: impl Fn(Vec<Overlay>, Vec<Study>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for IndicatorSelector {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let (overlays, studies) = (Rc::new(self.overlays), Rc::new(self.studies));
        let on_change = self.on_change;
        let flip = |overlay: Option<Overlay>, study: Option<Study>| {
            let (overlays, studies, on_change) =
                (overlays.clone(), studies.clone(), on_change.clone());
            move |window: &mut Window, cx: &mut App| {
                let (mut overlays, mut studies) = ((*overlays).clone(), (*studies).clone());
                if let Some(overlay) = overlay {
                    match overlays.iter().position(|shown| *shown == overlay) {
                        Some(ix) => drop(overlays.remove(ix)),
                        None => overlays.push(overlay),
                    }
                }
                if let Some(study) = study {
                    match studies.iter().position(|shown| *shown == study) {
                        Some(ix) => drop(studies.remove(ix)),
                        None => studies.push(study),
                    }
                }
                log::info!("indicators: {overlays:?} and {studies:?}");
                if let Some(on_change) = &on_change {
                    on_change(overlays, studies, window, cx);
                }
            }
        };
        let over = OVERLAYS.iter().map(|(overlay, words)| {
            MenuItem::check(*words, overlays.contains(overlay)).on_click(flip(Some(*overlay), None))
        });
        let under = STUDIES.iter().map(|(study, words)| {
            MenuItem::check(*words, studies.contains(study)).on_click(flip(None, Some(*study)))
        });
        let menu = Menu::new()
            .group("Over the prices", over)
            .group("In a pane below", under);
        let shown = overlays.len() + studies.len();
        let label: SharedString = if shown == 0 {
            "Indicators".into()
        } else {
            format!("Indicators · {shown}").into()
        };
        DropdownMenu::new(self.id, label, menu)
            .icon(IconName::Activity)
            .variant(ButtonVariant::Ghost)
    }
}

/// Marks drawn over the prices: trend lines with their ends, levels, boxes, and retracement levels with the middle band shaded.
pub(super) fn drawings(scene: &Scene, origin: Point<Pixels>, window: &mut Window) {
    let (main, scale, pen) = (scene.panes[0].0, &scene.panes[0].1, &scene.pen);
    let spot = |(ix, price): (f64, f64)| (scene.visible.x(ix, (main.x, main.w)), scale.at(price));
    let (ink, accent) = (pen.palette.fg_muted, tint(&pen.palette, 0));
    for drawing in &scene.drawings {
        match *drawing {
            Drawing::Trend(from, to) => {
                let (from, to) = (spot(from), spot(to));
                let mut line = PathBuilder::stroke(pen.stroke * 0.75);
                line.move_to(at(origin, from));
                line.line_to(at(origin, to));
                finish(line, ink, window);
                for end in [from, to] {
                    ring(
                        at(origin, end),
                        pen.stroke * 1.5,
                        (pen.palette.bg, ink),
                        pen.hairline,
                        window,
                    );
                }
            }
            Drawing::Level(price) => {
                let y = scale.at(price);
                window.paint_quad(fill(
                    place(
                        origin,
                        Rect {
                            x: main.x,
                            y,
                            w: main.w,
                            h: f32::from(pen.hairline),
                        },
                    ),
                    ink,
                ));
            }
            Drawing::Box(from, to) => {
                let ((x0, y0), (x1, y1)) = (spot(from), spot(to));
                let rect = Rect {
                    x: x0.min(x1),
                    y: y0.min(y1),
                    w: (x1 - x0).abs(),
                    h: (y1 - y0).abs(),
                };
                window.paint_quad(
                    fill(place(origin, rect), accent.opacity(0.1))
                        .border_widths(pen.hairline)
                        .border_color(accent),
                );
            }
            Drawing::Fib(from, to) => {
                let (x0, x1) = (spot(from).0.min(spot(to).0), spot(from).0.max(spot(to).0));
                let levels = Drawing::levels(from, to);
                let (upper, lower) = (scale.at(levels[2].1), scale.at(levels[4].1));
                let band = Rect {
                    x: x0,
                    y: upper.min(lower),
                    w: x1 - x0,
                    h: (upper - lower).abs(),
                };
                window.paint_quad(fill(place(origin, band), accent.opacity(0.08)));
                for (share, price) in levels {
                    let color = if share == FIB[0] || share == FIB[5] {
                        ink
                    } else {
                        pen.palette.fg_subtle
                    };
                    window.paint_quad(fill(
                        place(
                            origin,
                            Rect {
                                x: x0,
                                y: scale.at(price),
                                w: x1 - x0,
                                h: f32::from(pen.hairline),
                            },
                        ),
                        color,
                    ));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tools_make_their_drawings_and_levels_run_back_from_the_end() {
        assert_eq!(
            Drawing::of(Tool::Level, (1.0, 10.0), (4.0, 12.0)),
            Drawing::Level(12.0)
        );
        assert_eq!(
            Drawing::of(Tool::Trend, (1.0, 10.0), (4.0, 12.0)),
            Drawing::Trend((1.0, 10.0), (4.0, 12.0))
        );
        let levels = Drawing::levels((0.0, 100.0), (5.0, 200.0));
        assert_eq!(levels[0], (0.0, 200.0), "the swing's end");
        assert_eq!(levels[3], (0.5, 150.0), "halfway back");
        assert_eq!(levels[5], (1.0, 100.0), "all the way back");
    }
}
