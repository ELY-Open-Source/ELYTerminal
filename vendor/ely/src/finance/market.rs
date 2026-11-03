use std::rc::Rc;

use gpui::{
    App, Div, ElementId, Entity, InteractiveElement, IntoElement, ParentElement, Refineable,
    RenderOnce, SharedString, StyleRefinement, Styled, Window, div,
};
use jiff::tz::TimeZone;

use super::{
    candles::{Candle, heikin_ashi, profile},
    draw::{Pen, Scene, drawing},
    labels::{Axes, labels},
    quotes::moves,
    series::{Drawn, Overlay, Study, overlay, study},
    stage::{Visible, fit},
    steer::{OnDraw, Stage, Steering},
    tools::{Drawing, Tool},
};
use crate::{
    charts::{LIMIT, Linear, Rect, measure},
    theme::ActiveTheme,
    typography::format::system_zone,
};

/// How a market chart draws its prices.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ChartKind {
    #[default]
    Candles,
    /// Open, high, low and close as bars with ticks.
    Bars,
    /// Candles smoothed by averaging, which hold a trend's color longer.
    HeikinAshi,
    Line,
    Area,
}

/// What market charts that move together share: the candles in view and the one under the pointer. Create one entity and hand it to each chart.
#[derive(Default)]
pub struct ChartSync {
    pub(crate) visible: Option<Visible>,
    pub(crate) hover: Option<i64>,
    /// The most candles the shared window has moved for: an append moves it once, a lagging chart never.
    pub(crate) seen: usize,
}

/// A market's prices over time as candles, bars or a line, on a price scale at the right. Drag or scroll sideways to move through time, hold Cmd and scroll to zoom, and double-press to return to the newest; the crosshair reads each candle.
#[derive(IntoElement)]
pub struct CandlestickChart {
    base: Div,
    id: ElementId,
    candles: Rc<Vec<Candle>>,
    kind: ChartKind,
    title: Option<SharedString>,
    volume: bool,
    profile: bool,
    overlays: Vec<Overlay>,
    studies: Vec<Study>,
    alerts: Vec<(f64, SharedString)>,
    compare: Option<(SharedString, Rc<Vec<Candle>>)>,
    sync: Option<Entity<ChartSync>>,
    red_up: bool,
    zone: Option<TimeZone>,
    drawings: Vec<Drawing>,
    tool: Option<Tool>,
    on_draw: Option<OnDraw>,
}

impl CandlestickChart {
    /// Candles oldest first, one per period.
    pub fn new(id: impl Into<ElementId>, candles: impl Into<Rc<Vec<Candle>>>) -> Self {
        let candles = candles.into();
        assert!(
            candles.windows(2).all(|pair| pair[0].time < pair[1].time),
            "candles run oldest first"
        );
        Self {
            base: div(),
            id: id.into(),
            candles,
            kind: ChartKind::Candles,
            title: None,
            volume: false,
            profile: false,
            overlays: Vec::new(),
            studies: Vec::new(),
            alerts: Vec::new(),
            compare: None,
            sync: None,
            red_up: false,
            zone: None,
            drawings: Vec::new(),
            tool: None,
            on_draw: None,
        }
    }

    /// Marks the owner keeps, drawn over the prices.
    pub fn drawings(mut self, drawings: impl IntoIterator<Item = Drawing>) -> Self {
        self.drawings = drawings.into_iter().collect();
        self
    }

    /// A tool to draw with: a drag over the prices draws with it instead of moving through time.
    pub fn tool(mut self, tool: Option<Tool>) -> Self {
        self.tool = tool;
        self
    }

    /// Gets each drawing a tool finishes, for the owner to keep.
    pub fn on_draw(mut self, handler: impl Fn(Drawing, &mut Window, &mut App) + 'static) -> Self {
        self.on_draw = Some(Rc::new(handler));
        self
    }

    pub fn kind(mut self, kind: ChartKind) -> Self {
        self.kind = kind;
        self
    }

    /// A name read at the top left, such as a symbol and its interval.
    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Volume as faint bars along the bottom.
    pub fn volume(mut self) -> Self {
        self.volume = true;
        self
    }

    /// The volume traded at each price in view, as bars from the right edge.
    pub fn profile(mut self) -> Self {
        self.profile = true;
        self
    }

    pub fn overlay(mut self, overlay: Overlay) -> Self {
        self.overlays.push(overlay);
        self
    }

    /// A study in a pane of its own under the prices.
    pub fn study(mut self, study: Study) -> Self {
        self.studies.push(study);
        self
    }

    /// A dashed level with its label at the price scale, as a price alert.
    pub fn alert(mut self, price: f64, label: impl Into<SharedString>) -> Self {
        assert!(price.is_finite(), "an alert needs a finite price");
        self.alerts.push((price, label.into()));
        self
    }

    /// Another symbol's closes over the same periods, drawn from where this one starts in view so their moves compare.
    pub fn compare(
        mut self,
        name: impl Into<SharedString>,
        candles: impl Into<Rc<Vec<Candle>>>,
    ) -> Self {
        let (name, candles) = (name.into(), candles.into());
        if candles.len() != self.candles.len() {
            log::error!(
                "market chart: {name} has {} candles for {} periods; not compared",
                candles.len(),
                self.candles.len()
            );
            self.compare = None;
            return self;
        }
        self.compare = Some((name, candles));
        self
    }

    /// Moves, zooms and points together with every chart given the same sync.
    pub fn sync(mut self, sync: &Entity<ChartSync>) -> Self {
        self.sync = Some(sync.clone());
        self
    }

    /// Rising prices in red and falling in green, as markets in China and Japan read them.
    pub fn red_up(mut self) -> Self {
        self.red_up = true;
        self
    }

    /// The time zone its times read in; the system's unless set.
    pub fn zone(mut self, zone: TimeZone) -> Self {
        self.zone = Some(zone);
        self
    }
}

impl Styled for CandlestickChart {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for CandlestickChart {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let zone = self
            .zone
            .clone()
            .unwrap_or_else(|| system_zone("CandlestickChart"));
        let sync = self.sync.clone();
        let stage: Entity<Stage> =
            window.use_keyed_state((self.id.clone(), "stage"), cx, |_, cx| Stage {
                _synced: sync
                    .as_ref()
                    .map(|sync| cx.observe(sync, |_, _, cx| cx.notify())),
                ..Stage::default()
            });
        let theme = cx.theme();
        let (sizes, rem) = (theme.chart(), window.rem_size());
        let pixels = |length: gpui::Rems| f32::from(length.to_pixels(rem));
        let bounds = stage.read(cx).bounds;
        let (gutter, foot, inset) = (
            pixels(sizes.gutter) * 1.25,
            pixels(sizes.foot),
            pixels(sizes.inset),
        );
        let frame = Rect {
            x: inset,
            y: inset,
            w: (f32::from(bounds.size.width) - inset - gutter).max(0.0),
            h: (f32::from(bounds.size.height) - inset - foot).max(0.0),
        };
        let study_tall = pixels(sizes.height) * 0.3;
        let main = Rect {
            h: (frame.h - study_tall * self.studies.len() as f32).max(0.0),
            ..frame
        };
        let total = self.candles.len();
        let own = stage.read(cx).seen;
        let (held, shared) = match &self.sync {
            Some(sync) => {
                let sync = sync.read(cx);
                ((sync.visible, sync.hover), sync.seen)
            }
            None => ((stage.read(cx).visible, stage.read(cx).hover), own),
        };
        let fresh = total.saturating_sub(own).min(total.saturating_sub(shared));
        let visible = match held.0 {
            Some(visible)
                if fresh > 0
                    && own > 0
                    && visible.start + visible.count >= (total - fresh) as f64 - 0.5 =>
            {
                visible.pan(fresh as f64, total)
            }
            Some(visible) => visible.fitted(total),
            None => Visible::latest(total, f64::from(main.w / pixels(sizes.candle)).max(1.0)),
        };
        if own != total {
            log::debug!("market chart: {own} candles to {total}");
            let shrunk = total < own;
            let kept = held.0.filter(|_| fresh > 0 || shrunk).map(|_| visible);
            stage.update(cx, |stage, _| stage.seen = total);
            match &self.sync {
                Some(sync) => sync.update(cx, |sync, _| {
                    sync.seen = if shrunk { total } else { sync.seen.max(total) };
                    sync.visible = kept.or(sync.visible);
                }),
                None => stage.update(cx, |stage, _| stage.visible = kept.or(stage.visible)),
            }
        }
        let range = visible.range(total);
        let shown = Rc::new(if self.kind == ChartKind::HeikinAshi {
            heikin_ashi(&self.candles)
        } else {
            (*self.candles).clone()
        });
        let drawn: Rc<Vec<Drawn>> = Rc::new(
            self.overlays
                .iter()
                .enumerate()
                .map(|(ix, kind)| overlay(*kind, &self.candles, ix + 1))
                .collect(),
        );
        let studied: Rc<Vec<Drawn>> = Rc::new(
            self.studies
                .iter()
                .map(|kind| study(*kind, &self.candles, 1))
                .collect(),
        );
        let compare = self.compare.as_ref().and_then(|(_, other)| {
            let first = range.clone().next()?;
            let ratio = self.candles[first].close / other[first].close;
            let closes: Vec<f64> = other.iter().map(|candle| candle.close * ratio).collect();
            if !closes
                .iter()
                .all(|close| close.is_finite() && close.abs() <= LIMIT)
            {
                log::error!(
                    "market chart: the comparison scaled by {ratio} leaves what a chart draws"
                );
                return None;
            }
            Some(closes.into_iter().map(Some).collect::<Vec<_>>())
        });
        let in_view = &shown[range.clone()];
        let mut reach = in_view.iter().fold(None::<(f64, f64)>, |reach, candle| {
            let (low, high) = reach.unwrap_or((candle.low, candle.high));
            Some((low.min(candle.low), high.max(candle.high)))
        });
        for extra in drawn
            .iter()
            .filter_map(|drawn| drawn.reach(range.clone()))
            .chain(compare.iter().filter_map(|compare| {
                compare[range.clone()]
                    .iter()
                    .flatten()
                    .fold(None::<(f64, f64)>, |reach, value| {
                        let (low, high) = reach.unwrap_or((*value, *value));
                        Some((low.min(*value), high.max(*value)))
                    })
            }))
        {
            let (low, high) = reach.unwrap_or(extra);
            reach = Some((low.min(extra.0), high.max(extra.1)));
        }
        let ((low, high), ticks) = fit(
            reach.map_or(0.0, |reach| reach.0),
            reach.map_or(1.0, |reach| reach.1),
        );
        let mut panes = vec![(
            main,
            Linear::new((low, high), (main.y + main.h, main.y)),
            ticks,
        )];
        for (ix, drawn) in studied.iter().enumerate() {
            let pane = Rect {
                x: frame.x,
                y: main.y + main.h + study_tall * ix as f32,
                w: frame.w,
                h: study_tall,
            };
            let inner = Rect {
                y: pane.y + inset,
                h: (pane.h - inset * 2.0).max(0.0),
                ..pane
            };
            let (domain, ticks) = match drawn.reach(range.clone()) {
                Some(range) if drawn.range.is_some() => (range, drawn.guides.clone()),
                Some((low, high)) => fit(low, high),
                None => ((0.0, 1.0), Vec::new()),
            };
            panes.push((
                pane,
                Linear::new(domain, (inner.y + inner.h, inner.y)),
                ticks,
            ));
        }
        let (rise, fall) = moves(self.red_up, cx);
        let pointer = stage.read(cx).pointer;
        let hover = held.1.filter(|ix| *ix >= 0 && (*ix as usize) < total);
        let row = pointer.and_then(|(_, y)| {
            panes
                .iter()
                .position(|(pane, _, _)| y >= pane.y && y <= pane.y + pane.h)
                .map(|pane| (pane, y))
        });
        let last = self
            .candles
            .last()
            .map(|candle| (candle.close, candle.rose()));
        let volume = self.volume.then(|| {
            in_view
                .iter()
                .map(|candle| candle.volume)
                .fold(0.0, f64::max)
        });
        let profiled = if self.profile {
            profile(in_view, (low, high), 24)
        } else {
            Vec::new()
        };
        let sketch = stage.read(cx).sketch;
        let marks: Vec<Drawing> = self
            .drawings
            .iter()
            .copied()
            .chain(sketch.map(|(tool, from, to)| Drawing::of(tool, from, to)))
            .collect();
        let price = panes[0].1;
        let axes = Axes {
            panes: panes.clone(),
            visible,
            range: range.clone(),
            hover,
            row,
            zone: zone.clone(),
            title: self.title.clone(),
            compare: compare
                .as_ref()
                .and(self.compare.as_ref())
                .map(|(name, _)| name.clone()),
            drawings: marks.clone(),
        };
        let words = labels(
            &axes,
            (&shown, &self.candles),
            (&drawn, &studied),
            (&self.alerts, last),
            (rise, fall),
            window,
            cx,
        );
        let theme = cx.theme();
        let scene = Scene {
            panes,
            visible,
            range,
            candles: shown,
            kind: self.kind,
            volume,
            profile: profiled,
            overlays: drawn,
            studies: studied,
            compare,
            last,
            alerts: self.alerts.iter().map(|(price, _)| *price).collect(),
            cross: hover.map(|ix| (ix, row)),
            drawings: marks,
            pen: Pen {
                palette: theme.colors.clone(),
                rise,
                fall,
                stroke: sizes.stroke.to_pixels(rem),
                hairline: sizes.hairline,
            },
        };
        let height = sizes.height + sizes.height * 0.3 * self.studies.len() as f32;
        let mut root = div()
            .debug_selector(|| "chart-root".into())
            .relative()
            .h(height);
        root.style().refine(self.base.style());
        let steering = Steering {
            stage: stage.clone(),
            sync: self.sync.clone(),
            visible,
            total,
            main,
            price,
            tool: self.tool,
            on_draw: self.on_draw.clone(),
        };
        steering
            .attach(root.id((self.id.clone(), "market")))
            .child(drawing(scene))
            .children(words)
            .child(measure(stage, |stage| &mut stage.bounds))
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use jiff::Timestamp;

    use super::{Candle, CandlestickChart};

    fn days(count: i64) -> Rc<Vec<Candle>> {
        let candle = |ix: i64| {
            let time = Timestamp::from_second(ix * 86_400).expect("a day");
            Candle::new(time, (10.0, 11.0, 9.0, 10.0), 1.0)
        };
        Rc::new((0..count).map(candle).collect())
    }

    #[test]
    fn a_comparison_that_lags_clears_the_one_before() {
        let chart = CandlestickChart::new("chart", days(2))
            .compare("one", days(2))
            .compare("two", days(1));
        assert!(chart.compare.is_none());
    }
}
