use std::time::Duration;

use ely_gpui_component::shell::ShellColumn;
use gpui::{Context, Pixels, Task, px};

use super::Workspace;
use crate::settings::{COLUMN, SettingsStore};

/// How long a dragged border rests before its width is saved.
const SETTLE: Duration = Duration::from_millis(500);

/// The right column's least width where Ely draws the window's buttons: room for them, the gear and the toggle.
const PANEL_LEAST: f32 = 240.0;

/// A side column, whose border toward the terminal drags its width.
#[derive(Clone, Copy, Debug)]
pub(super) enum Side {
    Sessions,
    Panel,
}

/// Widths a drag shows that the settings do not hold yet.
#[derive(Default)]
pub(super) struct Dragged {
    sessions: Option<f32>,
    panel: Option<f32>,
    saving: Option<Task<()>>,
}

impl Workspace {
    /// Column `side`, as wide as it was dragged or saved.
    pub(super) fn side_column(&self, side: Side, cx: &Context<Self>) -> ShellColumn {
        let settings = SettingsStore::get(cx);
        let (id, dragged, saved) = match side {
            Side::Sessions => ("sessions", self.dragged.sessions, settings.sessions_width),
            Side::Panel => ("panel", self.dragged.panel, settings.panel_width),
        };
        let least = match side {
            Side::Panel if !cfg!(target_os = "macos") => PANEL_LEAST,
            _ => *COLUMN.start(),
        };
        let workspace = cx.entity();
        ShellColumn::new(id).resizable(
            px(dragged.unwrap_or(saved).max(least)),
            px(least),
            px(*COLUMN.end()),
            move |width, _, cx| workspace.update(cx, |this, cx| this.drag(side, width, cx)),
        )
    }

    /// Shows `width` at once and saves it once the drag rests.
    fn drag(&mut self, side: Side, width: Pixels, cx: &mut Context<Self>) {
        let width = f32::from(width).round();
        match side {
            Side::Sessions => self.dragged.sessions = Some(width),
            Side::Panel => self.dragged.panel = Some(width),
        }
        self.dragged.saving = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(SETTLE).await;
            if let Err(error) = this.update(cx, |this, cx| this.save(cx)) {
                log::error!("app: the columns' widths were not saved: {error:#}");
            }
        }));
        cx.notify();
    }

    fn save(&mut self, cx: &mut Context<Self>) {
        let (sessions, panel) = (self.dragged.sessions.take(), self.dragged.panel.take());
        let saved = SettingsStore::update(cx, |settings| {
            settings.sessions_width = sessions.unwrap_or(settings.sessions_width);
            settings.panel_width = panel.unwrap_or(settings.panel_width);
        });
        match saved {
            Ok(()) => {
                let settings = SettingsStore::get(cx);
                log::info!(
                    "app: the sessions column is {} px wide, the right column {} px",
                    settings.sessions_width,
                    settings.panel_width
                );
            }
            // The columns go back to the saved widths.
            Err(error) => log::error!("app: the columns' widths are not kept: {error:#}"),
        }
        cx.notify();
    }
}
