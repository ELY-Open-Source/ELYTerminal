//! `a=p`, and `a=T` once whole: a placement, virtual or relative.

use alacritty_terminal::index::{Column, Point};

use super::{
    Error, Graphics, Ids, Reply,
    place::{Display, Kind, Placement},
};

impl Graphics {
    /// Places it; the cursor passes grid ones unless `C=1`.
    pub(super) fn display(&mut self, display: Display, ids: Ids, cursor: Point) -> Option<Reply> {
        let pid = self.pid;
        if ids.id == 0 && ids.number == 0 {
            return Some(Reply::failed(ids, Error::NoId));
        }
        // Checked before the image, as kitty checks it.
        if display.virtual_ && display.parent > 0 {
            return Some(Reply::failed(ids, Error::VirtualParent));
        }
        let mut shared = self.lock();
        let (cell, (columns, lines)) = (shared.cell, shared.size);
        let screen = shared.screen();
        let id = match ids.id {
            0 => screen.store.newest(ids.number),
            id => Some(id),
        };
        let Some((id, image)) = id.and_then(|id| Some((id, screen.store.get(id)?))) else {
            return Some(Reply::failed(ids, Error::NoImage));
        };
        let placed = Ids { id, ..ids };
        let kind = if display.virtual_ {
            Kind::Virtual
        } else if display.parent > 0 {
            // A replaced placement keeps its serial: a cycle shows.
            let mine = |kept: &&Placement| {
                ids.placement > 0 && (kept.image, kept.id) == (id, ids.placement)
            };
            let child = screen.placements.iter().find(mine).map(|kept| kept.serial);
            match screen.parent(display.parent, display.parent_placement, child) {
                Ok(parent) => Kind::Relative {
                    parent,
                    shift: display.shift,
                },
                Err(error) => return Some(Reply::failed(placed, error)),
            }
        } else {
            Kind::Grid
        };
        let placement = Placement {
            image: id,
            id: ids.placement,
            kind,
            line: cursor.line.0,
            column: cursor.column.0,
            display,
            serial: 0,
        };
        let (wide, tall) = placement.cells(image, cell);
        log::debug!(
            "graphics {pid}: image {id} placed {kind:?} at {}, {} over {wide} by {tall} cells",
            cursor.line.0,
            cursor.column.0
        );
        screen.place(placement);
        drop(shared);
        // Only a grid placement moves the cursor, per kitty's spec.
        if kind == Kind::Grid && !display.stay {
            // As Ghostty moves it: down the rows, past the columns.
            let target = cursor.column.0 + wide as usize;
            let wraps = target >= columns;
            let down = (tall as usize).saturating_sub(1) + usize::from(wraps);
            let room = lines.saturating_sub(1 + cursor.line.0.max(0) as usize) + lines;
            self.moved = Some((down.min(room), Column(if wraps { 0 } else { target })));
        }
        Some(Reply::ok(placed))
    }
}
