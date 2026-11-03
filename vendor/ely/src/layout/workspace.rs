use anyhow::Context as _;
use gpui::{App, Entity};
use serde::{Deserialize, Serialize};

use super::{Dock, DockLayout, PaneGroup, PaneLayout};

/// Bump when a field changes meaning; never reuse a removed name.
pub const WORKSPACE_VERSION: u32 = 1;

/// Saved window layout: docks and editor panes, as JSON.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Workspace {
    pub version: u32,
    pub dock: DockLayout,
    pub panes: PaneLayout,
}

impl Workspace {
    pub fn capture(dock: &Dock, panes: &PaneGroup) -> Self {
        Self {
            version: WORKSPACE_VERSION,
            dock: dock.layout(),
            panes: panes.layout(),
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("workspace layout always serializes")
    }

    /// Unknown fields are ignored; a newer version is refused.
    pub fn from_json(json: &str) -> anyhow::Result<Self> {
        let workspace: Self = serde_json::from_str(json).context("workspace layout json")?;
        anyhow::ensure!(
            workspace.version <= WORKSPACE_VERSION,
            "workspace version {} is newer than this build ({WORKSPACE_VERSION})",
            workspace.version
        );
        Ok(workspace)
    }

    /// Applies all of it or none of it.
    pub fn apply(
        &self,
        dock: &Entity<Dock>,
        panes: &Entity<PaneGroup>,
        cx: &mut App,
    ) -> anyhow::Result<()> {
        self.panes.validate()?;
        dock.update(cx, |dock, cx| dock.restore(&self.dock, cx))?;
        panes.update(cx, |group, cx| group.restore(self.panes.clone(), cx))
    }
}

#[cfg(test)]
mod tests {
    use gpui::Axis;

    use super::*;
    use crate::layout::{DockSide, Spot};

    fn sample() -> Workspace {
        Workspace {
            version: WORKSPACE_VERSION,
            dock: DockLayout {
                spots: vec![
                    ("files".into(), Spot::Docked(DockSide::Left)),
                    ("outline".into(), Spot::Floating { x: 40.0, y: 60.0 }),
                ],
                active: [Some("files".into()), None, None, None],
                sizes: [15.0, 17.5, 12.0, 13.75],
            },
            panes: PaneLayout::Split {
                axis: Axis::Horizontal,
                sizes: vec![0.5, 0.5],
                children: vec![PaneLayout::Pane(1), PaneLayout::Pane(2)],
            },
        }
    }

    #[test]
    fn round_trips_through_json() {
        let saved = sample();
        assert_eq!(Workspace::from_json(&saved.to_json()).unwrap(), saved);
    }

    #[test]
    fn ignores_unknown_fields_but_refuses_newer_versions() {
        let mut value = serde_json::to_value(sample()).unwrap();
        value["added_later"] = serde_json::json!(true);
        assert!(Workspace::from_json(&value.to_string()).is_ok());
        value["version"] = serde_json::json!(WORKSPACE_VERSION + 1);
        assert!(Workspace::from_json(&value.to_string()).is_err());
    }

    #[test]
    fn invalid_pane_trees_are_refused() {
        let bad = PaneLayout::Split {
            axis: Axis::Vertical,
            sizes: vec![1.0],
            children: vec![PaneLayout::Pane(1), PaneLayout::Pane(1)],
        };
        assert!(bad.validate().is_err());
    }
}
