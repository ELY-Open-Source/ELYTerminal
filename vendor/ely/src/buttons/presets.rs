use gpui::ElementId;

use super::IconButton;
use crate::primitives::IconName;

/// Closes a panel, dialog or tab.
pub fn close_button(id: impl Into<ElementId>) -> IconButton {
    IconButton::new(id, IconName::X).tooltip("Close")
}

/// Returns to the previous view.
pub fn back_button(id: impl Into<ElementId>) -> IconButton {
    IconButton::new(id, IconName::ArrowLeft).tooltip("Back")
}

/// Opens more actions.
pub fn more_button(id: impl Into<ElementId>) -> IconButton {
    IconButton::new(id, IconName::Ellipsis).tooltip("More")
}
