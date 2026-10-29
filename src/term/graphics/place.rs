//! Placements: where images show, and how scrolls move them.

use std::ops::Range;

use alacritty_terminal::index::{Column, Line, Point};

use super::{
    command::Command,
    store::{Image, Store},
};

/// How a placement shows its image, from `a=p` or `a=T`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) struct Display {
    /// The source rectangle, `x`, `y`, `w`, `h`; cuts leave fractions.
    pub source: [f32; 4],
    /// `X` and `Y`, inside the first cell.
    pub offset: (u32, u32),
    /// `c` and `r`, 0 where unasked.
    pub cells: (u32, u32),
    pub z: i32,
    pub stay: bool,
    pub virtual_: bool,
    /// `P` and `Q`, the parent's image and placement.
    pub parent: u32,
    pub parent_placement: u32,
    /// `H` and `V`, cells from the parent's top-left.
    pub shift: (i32, i32),
}

impl Display {
    pub fn of(command: &Command) -> Self {
        let int = |key| command.int(key);
        Self {
            source: [b'x', b'y', b'w', b'h'].map(|key| int(key) as f32),
            offset: (int(b'X'), int(b'Y')),
            cells: (int(b'c'), int(b'r')),
            z: int(b'z') as i32,
            stay: int(b'C') == 1,
            virtual_: int(b'U') == 1,
            parent: int(b'P'),
            parent_placement: int(b'Q'),
            shift: (int(b'H') as i32, int(b'V') as i32),
        }
    }
}

/// Where a placement stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Kind {
    /// At its line and column.
    Grid,
    /// Where `U+10EEEE` cells name it.
    Virtual,
    /// At its parent's top-left, shifted `H` and `V` cells.
    Relative { parent: u64, shift: (i32, i32) },
}

/// One image shown, from its top-left cell.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Placement {
    pub image: u32,
    pub id: u32,
    pub kind: Kind,
    /// A grid placement's line, negative in the history.
    pub line: i32,
    pub column: usize,
    pub display: Display,
    /// Its order of placing, and the name its children use.
    pub serial: u64,
}

impl Placement {
    /// The part of the image shown, clamped as Ghostty clamps.
    pub fn source(&self, image: &Image) -> [f32; 4] {
        let [x, y, w, h] = self.display.source;
        let (width, height) = (image.width as f32, image.height as f32);
        let (x, y) = (x.min(width), y.min(height));
        let w = if w > 0. { w } else { width }.min(width - x);
        let h = if h > 0. { h } else { height }.min(height - y);
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
        let pixels = |length: f32| length.round() as u32;
        let span =
            |side: u32, cells: u32, offset: u32| side.saturating_mul(cells).saturating_sub(offset);
        match self.display.cells {
            (0, 0) => (pixels(w), pixels(h)),
            (c, 0) => {
                let width = span(cell.0, c, ox);
                (width, pixels(width as f32 * h / w))
            }
            (0, r) => {
                let height = span(cell.1, r, oy);
                (pixels(height as f32 * w / h), height)
            }
            (c, r) => (span(cell.0, c, ox), span(cell.1, r, oy)),
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

    /// The first and last lines it shows.
    fn rows(&self, image: &Image, cell: (u32, u32)) -> (i32, i32) {
        let rows = self.cells(image, cell).1 as i32;
        (self.line, self.line + rows - 1)
    }

    /// Cuts `rows` off its top into its source, as Ghostty.
    fn clip_top(&mut self, image: &Image, cell: (u32, u32), rows: u32) -> bool {
        let [x, y, w, h] = self.source(image);
        let shown = self.size(image, cell).1 as f32;
        let gone = cell
            .1
            .saturating_mul(rows)
            .saturating_sub(self.offset(cell).1) as f32;
        // A row showing part of a pixel keeps that part.
        if shown <= 0. || gone >= shown {
            return false;
        }
        let crop = h * gone / shown;
        self.display.source = [x, y + crop, w, h - crop];
        self.display.offset.1 = 0;
        if self.display.cells.1 > 0 {
            self.display.cells.1 -= rows;
        }
        self.line += rows as i32;
        true
    }

    /// Cuts `rows` off its bottom into its source, as Ghostty.
    fn clip_bottom(&mut self, image: &Image, cell: (u32, u32), rows: u32) -> bool {
        let [x, y, w, h] = self.source(image);
        let (shown, tall) = (self.size(image, cell).1 as f32, self.cells(image, cell).1);
        let room = cell
            .1
            .saturating_mul(tall.saturating_sub(rows))
            .saturating_sub(self.offset(cell).1) as f32;
        if shown <= 0. || room <= 0. {
            return false;
        }
        self.display.source = [x, y, w, h * room.min(shown) / shown];
        if self.display.cells.1 > 0 {
            self.display.cells.1 -= rows;
        }
        true
    }

    /// Whether it covers the cell at `line` and `column`.
    fn covers(&self, image: &Image, cell: (u32, u32), line: i32, column: usize) -> bool {
        let (top, bottom) = self.rows(image, cell);
        let columns = self.cells(image, cell).0 as usize;
        (top..=bottom).contains(&line) && (self.column..self.column + columns).contains(&column)
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

    /// Places it; a replaced one's serial and children stay.
    pub fn place(&mut self, mut placement: Placement) {
        let key = (placement.image, placement.id);
        let same = |kept: &Placement| placement.id > 0 && (kept.image, kept.id) == key;
        match self.placements.iter().position(same) {
            Some(at) => {
                placement.serial = self.placements[at].serial;
                self.placements[at] = placement;
            }
            None => {
                self.serial += 1;
                placement.serial = self.serial;
                self.placements.push(placement);
            }
        }
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

    /// Drops placements whose image or parent went; their images.
    pub(super) fn orphans(&mut self) -> Vec<u32> {
        let mut gone = Vec::new();
        loop {
            let serials: Vec<u64> = self.placements.iter().map(|kept| kept.serial).collect();
            let (store, before) = (&self.store, self.placements.len());
            self.placements.retain(|kept| {
                let parent = match kept.kind {
                    Kind::Relative { parent, .. } => serials.contains(&parent),
                    Kind::Grid | Kind::Virtual => true,
                };
                let kept_ = parent && store.get(kept.image).is_some();
                if !kept_ {
                    gone.push(kept.image);
                }
                kept_
            });
            if self.placements.len() == before {
                return gone;
            }
        }
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
            if kept.kind != Kind::Grid {
                return true;
            }
            let (top, bottom) = kept.rows(image, cell);
            // A region from the top moves alacritty's history up too.
            let history = lines > 0 && start == 0 && top < 0;
            let whole = lines > 0 && start == 0 && end == screen as i32;
            if !(history || whole || (top >= start && bottom < end)) {
                return true;
            }
            kept.line -= lines;
            if !(history || whole) {
                let (top, _) = kept.rows(image, cell);
                if top < start && !kept.clip_top(image, cell, (start - top) as u32) {
                    return false;
                }
                let (_, bottom) = kept.rows(image, cell);
                if bottom >= end && !kept.clip_bottom(image, cell, (bottom - end + 1) as u32) {
                    return false;
                }
            }
            kept.rows(image, cell).1 >= oldest.0
        });
        self.orphans();
    }

    /// The screen's `lines` cleared: what shows there goes.
    pub fn screen_cleared(&mut self, lines: usize, cell: (u32, u32)) {
        let store = &self.store;
        self.placements.retain(|kept| {
            store.get(kept.image).is_some_and(|image| {
                let (top, bottom) = kept.rows(image, cell);
                kept.kind != Kind::Grid || bottom < 0 || top >= lines as i32
            })
        });
        self.orphans();
    }

    /// The history cleared: rows above the screen are cut away.
    pub fn history_cleared(&mut self, cell: (u32, u32)) {
        let store = &self.store;
        self.placements.retain_mut(|kept| {
            let Some(image) = store.get(kept.image) else {
                return false;
            };
            let (top, _) = kept.rows(image, cell);
            kept.kind != Kind::Grid || top >= 0 || kept.clip_top(image, cell, (-top) as u32)
        });
        self.orphans();
    }

    /// Grid placements moved to their anchors' new cells, or dropped.
    pub fn resized(&mut self, moved: &[Option<Point>]) {
        let mut moved = moved.iter();
        self.placements.retain_mut(|kept| {
            if kept.kind != Kind::Grid {
                return true;
            }
            let Some(point) = moved.next().copied().flatten() else {
                return false;
            };
            kept.line = point.line.0;
            kept.column = point.column.0;
            true
        });
        self.orphans();
    }

    /// Each grid placement's top-left cell.
    pub fn anchors(&self) -> Vec<Point> {
        let grid = self
            .placements
            .iter()
            .filter(|kept| kept.kind == Kind::Grid);
        grid.map(|kept| Point::new(Line(kept.line), Column(kept.column)))
            .collect()
    }

    /// Deletes what `pick` names; `free` drops images left unplaced.
    pub fn delete(&mut self, pick: Pick, free: bool, lines: usize, cell: (u32, u32)) {
        let mut touched: Vec<u32> = match pick {
            Pick::Image { id, placement: 0 } => vec![id],
            Pick::Range(first, last) => self.store.within(first, last),
            _ => Vec::new(),
        };
        // Places pick grid placements; relatives go with parents, as Ghostty's.
        let placed = |kept: &Placement, image: &Image| {
            let grid = kept.kind == Kind::Grid;
            match pick {
                Pick::Shown => {
                    let (top, bottom) = kept.rows(image, cell);
                    grid && bottom >= 0 && top < lines as i32
                }
                Pick::Image { id, placement } => {
                    kept.image == id && (placement == 0 || kept.id == placement)
                }
                Pick::Cell { line, column, z } => {
                    let covers = kept.covers(image, cell, line, column);
                    grid && covers && z.is_none_or(|z| kept.display.z == z)
                }
                Pick::Column(column) => {
                    let columns = kept.cells(image, cell).0 as usize;
                    grid && (kept.column..kept.column + columns).contains(&column)
                }
                Pick::Row(line) => {
                    let (top, bottom) = kept.rows(image, cell);
                    grid && (top..=bottom).contains(&line)
                }
                Pick::Z(z) => kept.kind != Kind::Virtual && kept.display.z == z,
                Pick::Range(first, last) => (first..=last).contains(&kept.image),
            }
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
        touched.extend(self.orphans());
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
pub(super) mod tests;
