//! A session's panes: each its own terminal, laid out in splits.

use std::path::PathBuf;

use gpui::{Axis, Entity, EntityId, Subscription};

use crate::term::TerminalView;

/// A terminal inside a session, with what its program reports.
pub struct Pane {
    pub terminal: Entity<TerminalView>,
    /// The program's title; empty while it names none.
    pub title: String,
    /// The folder it reports, else the one it started in.
    pub folder: PathBuf,
    pub program: Program,
    /// Its shell has integration, or it reported a folder anyway.
    pub reports: bool,
    /// The host and folder a shell over ssh reports, while it lasts.
    pub there: Option<(String, PathBuf)>,
    pub _events: [Subscription; 2],
}

/// What a pane runs, so a split, duplicate or session here runs it again.
#[derive(Clone, Debug, Default)]
pub struct Program {
    /// Its path; empty for the login shell.
    pub shell: String,
    /// The arguments its profile or `-e` gave it.
    pub args: Vec<String>,
    /// It runs as `-e` wrote it, without integration.
    pub written: bool,
}

impl Pane {
    pub fn id(&self) -> EntityId {
        self.terminal.entity_id()
    }
}

/// How panes share the terminal column: one, or two parts split along an axis, the first's share given.
#[derive(Clone, Debug, PartialEq)]
pub enum Layout {
    Pane(EntityId),
    Split(Axis, f32, Box<Layout>, Box<Layout>),
}

/// Where a pane moves.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Toward {
    Left,
    Right,
    Up,
    Down,
}

/// A pane's box in the column, from 0 to 1 each way: left, top, right, bottom.
type Place = [f32; 4];

impl Layout {
    /// Splits pane `at` along `axis`, `new` after it; false when no pane is `at`.
    pub fn split(&mut self, at: EntityId, axis: Axis, new: EntityId) -> bool {
        match self {
            Layout::Pane(id) if *id == at => {
                *self = Layout::Split(
                    axis,
                    0.5,
                    Box::new(Layout::Pane(at)),
                    Box::new(Layout::Pane(new)),
                );
                true
            }
            Layout::Pane(_) => false,
            Layout::Split(_, _, first, second) => {
                first.split(at, axis, new) || second.split(at, axis, new)
            }
        }
    }

    /// Takes pane `gone` out; its half's other half fills the split. False when it is the last or absent.
    pub fn remove(&mut self, gone: EntityId) -> bool {
        let Layout::Split(_, _, first, second) = self else {
            return false;
        };
        let kept = match (first.as_ref(), second.as_ref()) {
            (Layout::Pane(id), _) if *id == gone => second.as_ref().clone(),
            (_, Layout::Pane(id)) if *id == gone => first.as_ref().clone(),
            _ => return first.remove(gone) || second.remove(gone),
        };
        *self = kept;
        true
    }

    /// Its panes, left to right and top to bottom.
    pub fn order(&self) -> Vec<EntityId> {
        match self {
            Layout::Pane(id) => vec![*id],
            Layout::Split(_, _, first, second) => {
                let mut ids = first.order();
                ids.extend(second.order());
                ids
            }
        }
    }

    /// The split whose parts start with panes `key`, given the first's `share`; false when none is.
    pub fn resize(&mut self, key: (EntityId, EntityId), share: f32) -> bool {
        let Layout::Split(_, own, first, second) = self else {
            return false;
        };
        if (first.order()[0], second.order()[0]) == key {
            *own = share;
            return true;
        }
        first.resize(key, share) || second.resize(key, share)
    }

    /// Panes `a` and `b` trade places.
    pub fn swap(&mut self, a: EntityId, b: EntityId) {
        match self {
            Layout::Pane(id) if *id == a => *id = b,
            Layout::Pane(id) if *id == b => *id = a,
            Layout::Pane(_) => {}
            Layout::Split(_, _, first, second) => {
                first.swap(a, b);
                second.swap(a, b);
            }
        }
    }

    /// Each pane's place in `inside`.
    fn places(&self, inside: Place) -> Vec<(EntityId, Place)> {
        let [left, top, right, bottom] = inside;
        match self {
            Layout::Pane(id) => vec![(*id, inside)],
            Layout::Split(axis, share, first, second) => {
                let (one, two) = match axis {
                    Axis::Horizontal => {
                        let cut = left + (right - left) * share;
                        ([left, top, cut, bottom], [cut, top, right, bottom])
                    }
                    Axis::Vertical => {
                        let cut = top + (bottom - top) * share;
                        ([left, top, right, cut], [left, cut, right, bottom])
                    }
                };
                let mut places = first.places(one);
                places.extend(second.places(two));
                places
            }
        }
    }

    /// The pane beside `of` toward `toward`: of those across that edge and alongside it, the nearest, then the most alongside.
    pub fn neighbor(&self, of: EntityId, toward: Toward) -> Option<EntityId> {
        let places = self.places([0.0, 0.0, 1.0, 1.0]);
        let &(_, [left, top, right, bottom]) = places.iter().find(|(id, _)| *id == of)?;
        let beside = |[l, t, r, b]: Place| -> Option<(f32, f32)> {
            let (gap, along) = match toward {
                Toward::Left => (left - r, bottom.min(b) - top.max(t)),
                Toward::Right => (l - right, bottom.min(b) - top.max(t)),
                Toward::Up => (top - b, right.min(r) - left.max(l)),
                Toward::Down => (t - bottom, right.min(r) - left.max(l)),
            };
            (gap >= -1e-4 && along > 1e-4).then_some((gap, along))
        };
        places
            .iter()
            .filter(|(id, _)| *id != of)
            .filter_map(|(id, place)| Some((*id, beside(*place)?)))
            .min_by(|(_, (gap, along)), (_, (other_gap, other_along))| {
                gap.total_cmp(other_gap).then(other_along.total_cmp(along))
            })
            .map(|(id, _)| id)
    }
}

#[cfg(test)]
mod tests;
