use std::time::Instant;

use ely_gpui_component::{lists::List, motion::Reorder};
use gpui::{AnyElement, Context, IntoElement, ParentElement, Styled, div};

use crate::sessions;

use super::Workspace;

/// The session shown once session `from` moves to place `to`.
pub fn after_move(active: usize, from: usize, to: usize) -> usize {
    if active == from {
        to
    } else if from < active && active <= to {
        active - 1
    } else if to <= active && active < from {
        active + 1
    } else {
        active
    }
}

impl Workspace {
    /// The sessions' rows; only a drag needs Reorder, and List keeps its own gap.
    pub(super) fn session_list(
        &self,
        drag: bool,
        now: Instant,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let workspace = cx.entity();
        let home = dirs::home_dir();
        let rows: Vec<(String, AnyElement)> = self
            .sessions
            .iter()
            .enumerate()
            .map(|(index, session)| {
                let active = index == self.active;
                let row =
                    sessions::row(&workspace, index, session, active, home.as_deref(), now, cx);
                (session.id.to_string(), row.into_any_element())
            })
            .collect();
        if drag {
            rows.into_iter()
                .fold(
                    Reorder::new("session-rows").on_reorder(move |from, to, _, cx| {
                        workspace.update(cx, |this, cx| this.reorder(from, to, cx))
                    }),
                    |list, (key, row)| list.row(key, div().pb_0p5().child(row)),
                )
                .into_any_element()
        } else {
            List::new()
                .children(rows.into_iter().map(|(_, row)| row))
                .into_any_element()
        }
    }

    /// Moves session `from` to place `to`, both inside the list.
    pub(super) fn move_session(&mut self, from: usize, to: usize) {
        let moved = self.sessions.remove(from);
        self.sessions.insert(to, moved);
        self.active = after_move(self.active, from, to);
    }

    /// Moves session `from` to place `to`, as a drag asks.
    pub fn reorder(&mut self, from: usize, to: usize, cx: &mut Context<Self>) {
        if from == to || from >= self.sessions.len() || to >= self.sessions.len() {
            return;
        }
        log::info!("app: session {} moves to {}", from + 1, to + 1);
        self.move_session(from, to);
        cx.notify();
    }
}

#[cfg(test)]
mod tests {
    use super::after_move;

    #[test]
    fn the_shown_session_follows_its_row() {
        assert_eq!(after_move(2, 2, 0), 0, "the shown row moved");
        assert_eq!(after_move(2, 0, 3), 1, "a row below it went up");
        assert_eq!(after_move(2, 3, 0), 3, "a row above it went down");
        assert_eq!(after_move(3, 1, 0), 3, "a move below it leaves it");
        assert_eq!(after_move(0, 2, 3), 0, "a move below the shown one");
    }
}
