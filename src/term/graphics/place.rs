//! Placements: where images show, and how scrolls move them.

use std::ops::Range;

use alacritty_terminal::index::{Column, Line, Point};

use super::{
    command::Command,
    store::{Image, Store},
};

/// How a placement shows its image, from `a=p` or `a=T`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct Display {
    /// The source rectangle, `x`, `y`, `w`, `h`.
    pub source: [u32; 4],
    /// `X` and `Y`, inside the first cell.
    pub offset: (u32, u32),
    /// `c` and `r`, 0 where unasked.
    pub cells: (u32, u32),
    pub z: i32,
    pub stay: bool,
    pub virtual_: bool,
    pub parent: u32,
}

impl Display {
    pub fn of(command: &Command) -> Self {
        let int = |key| command.int(key);
        Self {
            source: [int(b'x'), int(b'y'), int(b'w'), int(b'h')],
            offset: (int(b'X'), int(b'Y')),
            cells: (int(b'c'), int(b'r')),
            z: int(b'z') as i32,
            stay: int(b'C') == 1,
            virtual_: int(b'U') == 1,
            parent: int(b'P'),
        }
    }
}

/// One image shown on the grid, from its top-left cell.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Placement {
    pub image: u32,
    pub id: u32,
    /// The grid line of its top, negative in the history.
    pub line: i32,
    pub column: usize,
    pub display: Display,
    /// Rows a scroll region cut from its top and bottom.
    pub cut: (u32, u32),
    /// Its order of placing, so equal ones draw alike.
    pub serial: u64,
}

impl Placement {
    /// The part of the image shown, clamped as Ghostty clamps.
    pub fn source(&self, image: &Image) -> [u32; 4] {
        let [x, y, w, h] = self.display.source;
        let (x, y) = (x.min(image.width), y.min(image.height));
        let w = if w > 0 { w } else { image.width }.min(image.width - x);
        let h = if h > 0 { h } else { image.height }.min(image.height - y);
        [x, y, w, h]
    }

    /// The offset inside the first cell, kept inside it.
    pub fn offset(&self, cell: (u32, u32)) -> (u32, u32) {
        let (x, y) = self.display.offset;
        (
            x.min(cell.0.saturating_sub(1)),
            y.min(cell.1.saturating_sub(1)),
        )
    }

    /// The shown size in pixels, Ghostty's: cells given stretch it.
    pub fn size(&self, image: &Image, cell: (u32, u32)) -> (u32, u32) {
        let [_, _, w, h] = self.source(image);
        let (ox, oy) = self.offset(cell);
        let scale =
            |length: u64, by: u32, of: u32| (length * u64::from(by)).checked_div(u64::from(of));
        let clamp = |length: u64| u32::try_from(length).unwrap_or(u32::MAX);
        match self.display.cells {
            (0, 0) => (w, h),
            (c, 0) => {
                let width = clamp(u64::from(cell.0) * u64::from(c)).saturating_sub(ox);
                (width, clamp(scale(width.into(), h, w).unwrap_or(0)))
            }
            (0, r) => {
                let height = clamp(u64::from(cell.1) * u64::from(r)).saturating_sub(oy);
                (clamp(scale(height.into(), w, h).unwrap_or(0)), height)
            }
            (c, r) => (
                clamp(u64::from(cell.0) * u64::from(c)).saturating_sub(ox),
                clamp(u64::from(cell.1) * u64::from(r)).saturating_sub(oy),
            ),
        }
    }

    /// The columns and rows it covers.
    pub fn cells(&self, image: &Image, cell: (u32, u32)) -> (u32, u32) {
        if self.display.cells.0 > 0 && self.display.cells.1 > 0 {
            return self.display.cells;
        }
        let (w, h) = self.size(image, cell);
        let (ox, oy) = self.offset(cell);
        let over = |length: u32, side: u32| length.saturating_add(side.max(1) - 1) / side.max(1);
        (
            over(w.saturating_add(ox), cell.0),
            over(h.saturating_add(oy), cell.1),
        )
    }

    /// The first and last lines it shows, cuts aside.
    fn rows(&self, image: &Image, cell: (u32, u32)) -> (i32, i32) {
        let rows = self.cells(image, cell).1 as i32;
        let top = self.line + self.cut.0 as i32;
        (top, self.line + rows - 1 - self.cut.1 as i32)
    }

    /// Whether it covers the cell at `line` and `column`.
    fn covers(&self, image: &Image, cell: (u32, u32), line: i32, column: usize) -> bool {
        let (top, bottom) = self.rows(image, cell);
        let columns = self.cells(image, cell).0 as usize;
        (top..=bottom).contains(&line) && (self.column..self.column + columns).contains(&column)
    }

    fn gone(&self, image: &Image, cell: (u32, u32)) -> bool {
        self.cut.0 + self.cut.1 >= self.cells(image, cell).1
    }
}

/// What a delete picks, as `d` names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Pick {
    Shown,
    Image {
        id: u32,
        placement: u32,
    },
    Cell {
        line: i32,
        column: usize,
        z: Option<i32>,
    },
    Column(usize),
    Row(i32),
    Z(i32),
    Range(u32, u32),
}

/// One screen's images and placements.
#[derive(Debug)]
pub(super) struct Screen {
    pub store: Store,
    pub placements: Vec<Placement>,
    serial: u64,
}

impl Screen {
    pub fn new(limit: usize) -> Self {
        Self {
            store: Store::new(limit),
            placements: Vec::new(),
            serial: 0,
        }
    }

    /// Places it, replacing a placement of its id.
    pub fn place(&mut self, mut placement: Placement) {
        if placement.id > 0 {
            let (image, id) = (placement.image, placement.id);
            self.placements
                .retain(|kept| (kept.image, kept.id) != (image, id));
        }
        self.serial += 1;
        placement.serial = self.serial;
        self.placements.push(placement);
    }

    /// Stores `image`; evicted images take their placements along.
    pub fn store(&mut self, image: Image, pid: u32) -> Result<(), super::Error> {
        let placed: Vec<u32> = self.placements.iter().map(|kept| kept.image).collect();
        self.store.add(image, pid, &|id| placed.contains(&id))?;
        self.orphans();
        Ok(())
    }

    /// Drops an image and its placements; whether one was there.
    pub fn remove(&mut self, id: u32) -> bool {
        let was = self.store.remove(id);
        self.orphans();
        was
    }

    fn orphans(&mut self) {
        let store = &self.store;
        self.placements
            .retain(|kept| store.get(kept.image).is_some());
    }

    /// Lines in `region` moved up `lines`, as Ghostty moves them.
    pub fn scrolled(
        &mut self,
        region: &Range<Line>,
        lines: i32,
        oldest: Line,
        cell: (u32, u32),
        screen: usize,
    ) {
        let (start, end) = (region.start.0, region.end.0);
        let store = &self.store;
        self.placements.retain_mut(|kept| {
            let Some(image) = store.get(kept.image) else {
                return false;
            };
            let (top, bottom) = kept.rows(image, cell);
            // A region from the top moves alacritty's history up too.
            let history = lines > 0 && start == 0 && top < 0;
            let whole = lines > 0 && start == 0 && end == screen as i32;
            if !(history || whole || (top >= start && bottom < end)) {
                return true;
            }
            kept.line -= lines;
            let (top, bottom) = kept.rows(image, cell);
            if !(history || whole) && top < start {
                kept.cut.0 += (start - top) as u32;
            }
            if !(history || whole) && bottom >= end {
                kept.cut.1 += (bottom - end + 1) as u32;
            }
            !kept.gone(image, cell) && kept.rows(image, cell).1 >= oldest.0
        });
    }

    /// The screen's `lines` cleared: what shows there goes.
    pub fn screen_cleared(&mut self, lines: usize, cell: (u32, u32)) {
        let store = &self.store;
        self.placements.retain(|kept| {
            store.get(kept.image).is_some_and(|image| {
                let (top, bottom) = kept.rows(image, cell);
                bottom < 0 || top >= lines as i32
            })
        });
    }

    /// The history cleared: rows above the screen are cut away.
    pub fn history_cleared(&mut self, cell: (u32, u32)) {
        let store = &self.store;
        self.placements.retain_mut(|kept| {
            let Some(image) = store.get(kept.image) else {
                return false;
            };
            let (top, _) = kept.rows(image, cell);
            if top < 0 {
                kept.cut.0 += (-top) as u32;
            }
            !kept.gone(image, cell)
        });
    }

    /// Each placement's cell moved as `moved` says; one gone drops.
    pub fn resized(&mut self, moved: &dyn Fn(Point) -> Option<Point>) {
        self.placements.retain_mut(|kept| {
            let Some(point) = moved(Point::new(Line(kept.line), Column(kept.column))) else {
                return false;
            };
            (kept.line, kept.column) = (point.line.0, point.column.0);
            true
        });
    }

    /// Deletes what `pick` names; `free` drops images left unplaced.
    pub fn delete(&mut self, pick: Pick, free: bool, lines: usize, cell: (u32, u32)) {
        let mut touched: Vec<u32> = match pick {
            Pick::Image { id, placement: 0 } => vec![id],
            Pick::Range(first, last) => self.store.within(first, last),
            _ => Vec::new(),
        };
        let placed = |kept: &Placement, image: &Image| match pick {
            Pick::Shown => {
                let (top, bottom) = kept.rows(image, cell);
                bottom >= 0 && top < lines as i32
            }
            Pick::Image { id, placement } => {
                kept.image == id && (placement == 0 || kept.id == placement)
            }
            Pick::Cell { line, column, z } => {
                kept.covers(image, cell, line, column) && z.is_none_or(|z| kept.display.z == z)
            }
            Pick::Column(column) => {
                let columns = kept.cells(image, cell).0 as usize;
                (kept.column..kept.column + columns).contains(&column)
            }
            Pick::Row(line) => {
                let (top, bottom) = kept.rows(image, cell);
                (top..=bottom).contains(&line)
            }
            Pick::Z(z) => kept.display.z == z,
            Pick::Range(first, last) => (first..=last).contains(&kept.image),
        };
        let store = &self.store;
        self.placements.retain(|kept| {
            let Some(image) = store.get(kept.image) else {
                return false;
            };
            let gone = placed(kept, image);
            if gone {
                touched.push(kept.image);
            }
            !gone
        });
        if !free {
            return;
        }
        for id in touched {
            if !self.placements.iter().any(|kept| kept.image == id) && self.store.remove(id) {
                log::debug!("graphics: image {id} deleted");
            }
        }
    }
}

#[cfg(test)]
mod tests;
