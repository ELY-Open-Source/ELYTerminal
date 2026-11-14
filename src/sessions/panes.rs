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

/// How panes share the terminal column: one, or two halves split along an axis.
#[derive(Clone, Debug, PartialEq)]
pub enum Layout {
    Pane(EntityId),
    Split(Axis, Box<Layout>, Box<Layout>),
}

impl Layout {
    /// Splits pane `at` along `axis`, `new` after it; false when no pane is `at`.
    pub fn split(&mut self, at: EntityId, axis: Axis, new: EntityId) -> bool {
        match self {
            Layout::Pane(id) if *id == at => {
                *self = Layout::Split(
                    axis,
                    Box::new(Layout::Pane(at)),
                    Box::new(Layout::Pane(new)),
                );
                true
            }
            Layout::Pane(_) => false,
            Layout::Split(_, first, second) => {
                first.split(at, axis, new) || second.split(at, axis, new)
            }
        }
    }

    /// Takes pane `gone` out; its half's other half fills the split. False when it is the last or absent.
    pub fn remove(&mut self, gone: EntityId) -> bool {
        let Layout::Split(_, first, second) = self else {
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
            Layout::Split(_, first, second) => {
                let mut ids = first.order();
                ids.extend(second.order());
                ids
            }
        }
    }
}

#[cfg(test)]
mod tests;
