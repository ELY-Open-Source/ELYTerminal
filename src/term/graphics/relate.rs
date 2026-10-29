//! Relative placements: a parent, its chain, and where it roots.

use super::{
    Error,
    place::{Kind, Placement, Screen},
};

/// The links a chain holds at most, kitty's and Ghostty's.
const CHAIN: usize = 8;

impl Screen {
    /// The parent `P` and `Q` name, checked as Ghostty checks.
    pub fn parent(&self, image: u32, placement: u32, child: Option<u64>) -> Result<u64, Error> {
        if self.store.get(image).is_none() {
            return Err(Error::NoParentImage);
        }
        // Without `Q`, the image's oldest placement, as kitty picks.
        let named =
            |kept: &&Placement| kept.image == image && (placement == 0 || kept.id == placement);
        let found = self
            .placements
            .iter()
            .filter(named)
            .min_by_key(|kept| kept.serial);
        let parent = found.ok_or(Error::NoParentPlacement)?;
        if Some(parent.serial) == child {
            return Err(Error::SelfParent);
        }
        let (mut at, mut depth) = (parent, 1);
        loop {
            if Some(at.serial) == child {
                return Err(Error::Cycle);
            }
            let Kind::Relative { parent: up, .. } = at.kind else {
                return Ok(parent.serial);
            };
            if depth >= CHAIN {
                return Err(Error::TooDeep);
            }
            depth += 1;
            at = self.serial(up).ok_or(Error::NoParentPlacement)?;
        }
    }

    /// `kept`'s root, and the columns and lines its chain shifts.
    pub fn root<'a>(&'a self, kept: &'a Placement) -> Option<(&'a Placement, i32, i32)> {
        let (mut at, mut columns, mut lines, mut depth) = (kept, 0i32, 0i32, 0);
        while let Kind::Relative { parent, shift } = at.kind {
            depth += 1;
            if depth > CHAIN {
                return None;
            }
            columns = columns.saturating_add(shift.0);
            lines = lines.saturating_add(shift.1);
            at = self.serial(parent)?;
        }
        Some((at, columns, lines))
    }

    fn serial(&self, serial: u64) -> Option<&Placement> {
        self.placements.iter().find(|kept| kept.serial == serial)
    }
}

#[cfg(test)]
mod tests;
