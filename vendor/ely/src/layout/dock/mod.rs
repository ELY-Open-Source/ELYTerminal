mod view;

use std::rc::Rc;

use gpui::{AnyElement, App, Context, Rems, SharedString, Window};
use serde::{Deserialize, Serialize};

use crate::{primitives::IconName, theme::ActiveTheme};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DockSide {
    Left,
    Right,
    Top,
    Bottom,
}

impl DockSide {
    const ALL: [DockSide; 4] = [
        DockSide::Left,
        DockSide::Right,
        DockSide::Top,
        DockSide::Bottom,
    ];

    fn index(self) -> usize {
        self as usize
    }
}

/// Where a panel lives: a dock, or floating at a window position.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Spot {
    Docked(DockSide),
    Floating { x: f32, y: f32 },
}

#[derive(Clone, Debug)]
pub struct DockPanel {
    pub id: SharedString,
    pub title: SharedString,
    pub icon: IconName,
}

/// Saved dock arrangement, per side in `DockSide` order. Sizes are in rems.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DockLayout {
    pub spots: Vec<(String, Spot)>,
    pub active: [Option<String>; 4],
    pub sizes: [f32; 4],
}

type Center = Rc<dyn Fn(&mut Window, &mut App) -> AnyElement>;
type Content = Rc<dyn Fn(&SharedString, &mut Window, &mut App) -> AnyElement>;

/// Side docks around a center; panels move between docks and float.
pub struct Dock {
    panels: Vec<DockPanel>,
    spots: Vec<Spot>,
    active: [Option<usize>; 4],
    sizes: [Rems; 4],
    center: Center,
    content: Content,
    dragging: bool,
}

impl Dock {
    pub fn new(
        center: impl Fn(&mut Window, &mut App) -> AnyElement + 'static,
        content: impl Fn(&SharedString, &mut Window, &mut App) -> AnyElement + 'static,
        cx: &App,
    ) -> Self {
        let theme = cx.theme();
        let (side, strip) = (theme.sidebar_width(false), theme.pane_min());
        Self {
            panels: Vec::new(),
            spots: Vec::new(),
            active: [None; 4],
            sizes: [side, side, strip, strip],
            center: Rc::new(center),
            content: Rc::new(content),
            dragging: false,
        }
    }

    /// Panics on a repeated id.
    pub fn add(&mut self, panel: DockPanel, side: DockSide) {
        assert!(
            self.find(&panel.id).is_none(),
            "dock panel {} added twice",
            panel.id
        );
        self.panels.push(panel);
        self.spots.push(Spot::Docked(side));
        let slot = &mut self.active[side.index()];
        if slot.is_none() {
            *slot = Some(self.panels.len() - 1);
        }
    }

    fn find(&self, id: &str) -> Option<usize> {
        self.panels.iter().position(|panel| panel.id.as_ref() == id)
    }

    fn on_side(&self, side: DockSide) -> Vec<usize> {
        (0..self.panels.len())
            .filter(|ix| self.spots[*ix] == Spot::Docked(side))
            .collect()
    }

    pub fn activate(&mut self, id: &str, cx: &mut Context<Self>) {
        let ix = self
            .find(id)
            .unwrap_or_else(|| panic!("no dock panel {id}"));
        if let Spot::Docked(side) = self.spots[ix] {
            self.active[side.index()] = Some(ix);
            cx.notify();
        }
    }

    pub fn move_panel(&mut self, id: &str, spot: Spot, cx: &mut Context<Self>) {
        let ix = self
            .find(id)
            .unwrap_or_else(|| panic!("no dock panel {id}"));
        let from = self.spots[ix];
        self.spots[ix] = spot;
        if let Spot::Docked(side) = from
            && self.active[side.index()] == Some(ix)
        {
            self.active[side.index()] = self.on_side(side).first().copied();
        }
        if let Spot::Docked(side) = spot {
            self.active[side.index()] = Some(ix);
        }
        log::info!("dock: {id} {from:?} -> {spot:?}");
        cx.notify();
    }

    pub fn layout(&self) -> DockLayout {
        let name = |ix: Option<usize>| ix.map(|ix| self.panels[ix].id.to_string());
        DockLayout {
            spots: self
                .panels
                .iter()
                .zip(&self.spots)
                .map(|(panel, spot)| (panel.id.to_string(), *spot))
                .collect(),
            active: DockSide::ALL.map(|side| name(self.active[side.index()])),
            sizes: self.sizes.map(|size| size.0),
        }
    }

    /// All or nothing. A side the layout leaves unset shows its first panel.
    pub fn restore(&mut self, layout: &DockLayout, cx: &mut Context<Self>) -> anyhow::Result<()> {
        let mut named: Vec<&str> = layout.spots.iter().map(|(id, _)| id.as_str()).collect();
        named.sort_unstable();
        named.dedup();
        anyhow::ensure!(
            named.len() == layout.spots.len(),
            "layout places a panel twice"
        );
        anyhow::ensure!(
            layout
                .sizes
                .iter()
                .all(|size| size.is_finite() && *size > 0.0),
            "dock sizes {:?} must be finite and positive",
            layout.sizes
        );
        anyhow::ensure!(
            layout.spots.iter().all(|(_, spot)| match spot {
                Spot::Docked(_) => true,
                Spot::Floating { x, y } => x.is_finite() && y.is_finite(),
            }),
            "floating positions must be finite"
        );
        let unknown: Vec<&str> = layout
            .spots
            .iter()
            .map(|(id, _)| id.as_str())
            .chain(layout.active.iter().flatten().map(String::as_str))
            .filter(|id| self.find(id).is_none())
            .collect();
        anyhow::ensure!(
            unknown.is_empty(),
            "layout names unknown panels: {unknown:?}"
        );
        let mut spots = self.spots.clone();
        for (id, spot) in &layout.spots {
            spots[self.find(id).expect("checked above")] = *spot;
        }
        let mut active = [None; 4];
        for side in DockSide::ALL {
            let chosen = layout.active[side.index()]
                .as_deref()
                .and_then(|id| self.find(id));
            if let Some(ix) = chosen {
                anyhow::ensure!(
                    spots[ix] == Spot::Docked(side),
                    "active panel {} is not docked {side:?}",
                    self.panels[ix].id
                );
            }
            active[side.index()] =
                chosen.or_else(|| (0..spots.len()).find(|ix| spots[*ix] == Spot::Docked(side)));
        }
        self.spots = spots;
        self.active = active;
        self.sizes = layout.sizes.map(gpui::rems);
        log::info!("dock: restored {} panel spots", layout.spots.len());
        cx.notify();
        Ok(())
    }
}
