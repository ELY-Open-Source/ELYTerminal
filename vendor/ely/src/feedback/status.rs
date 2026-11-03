use std::{f32::consts::TAU, rc::Rc, time::Duration};

use gpui::{
    Animation, AnimationExt, App, ElementId, IntoElement, ParentElement, RenderOnce, Styled,
    Window, div, prelude::*, radians,
};

use super::messages::rise;
use crate::{
    buttons::{Button, ButtonVariant},
    forms::Run,
    primitives::{Icon, IconName},
    shell::{Connectivity, connectivity_dot},
    theme::{ActiveTheme, IconSize, TextSize},
    typography::format::plural,
};

/// One turn of a working sync icon.
const TURN: Duration = Duration::from_millis(1400);

/// Where a save stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SaveState {
    Saving,
    Saved,
    Failed,
}

/// A quiet line for autosave: saving, saved, or failed with Retry.
#[derive(IntoElement)]
pub struct SavingIndicator {
    id: ElementId,
    state: SaveState,
    on_retry: Option<Run>,
}

impl SavingIndicator {
    pub fn new(id: impl Into<ElementId>, state: SaveState) -> Self {
        Self {
            id: id.into(),
            state,
            on_retry: None,
        }
    }

    pub fn on_retry(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_retry = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for SavingIndicator {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = &theme.colors;
        let (icon, text, tone) = match self.state {
            SaveState::Saving => (IconName::CloudUpload, "Saving…", colors.fg_muted),
            SaveState::Saved => (IconName::CloudCheck, "Saved", colors.fg_muted),
            SaveState::Failed => (IconName::CloudAlert, "Couldn't save", colors.danger),
        };
        let retry = (self.id.clone(), "retry");
        let line = div()
            .flex()
            .items_center()
            .gap_1p5()
            .text_size(theme.text_size(TextSize::Sm))
            .text_color(tone)
            .child(Icon::new(icon).size(IconSize::Sm).color(tone))
            .child(text)
            .when_some(
                self.on_retry.filter(|_| self.state == SaveState::Failed),
                |line, run| {
                    line.child(
                        Button::new(retry, "Retry")
                            .variant(ButtonVariant::Link)
                            .on_click(move |_, window, cx| run(window, cx)),
                    )
                },
            );
        rise(self.id, self.state, line, window, cx)
    }
}

/// Where a sync stands; `Syncing` counts what is left.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SyncState {
    Synced,
    Syncing(usize),
    Paused,
    Failed,
}

/// A line for background sync. Its arrows turn while it works.
#[derive(IntoElement)]
pub struct SyncStatus {
    id: ElementId,
    state: SyncState,
}

impl SyncStatus {
    pub fn new(id: impl Into<ElementId>, state: SyncState) -> Self {
        Self {
            id: id.into(),
            state,
        }
    }
}

impl RenderOnce for SyncStatus {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = &theme.colors;
        let still = theme.reduced_motion;
        let (icon, text, tone) = match self.state {
            SyncState::Synced => (
                IconName::CloudCheck,
                "Up to date".to_string(),
                colors.fg_muted,
            ),
            SyncState::Syncing(left) => (
                IconName::RefreshCw,
                format!("Syncing {}", plural(left as u64, "item", "items")),
                colors.fg_muted,
            ),
            SyncState::Paused => (
                IconName::CirclePause,
                "Sync paused".to_string(),
                colors.fg_muted,
            ),
            SyncState::Failed => (
                IconName::CloudAlert,
                "Sync failed".to_string(),
                colors.danger,
            ),
        };
        let icon = Icon::new(icon).size(IconSize::Sm).color(tone);
        let icon = if matches!(self.state, SyncState::Syncing(_)) && !still {
            icon.with_animation(
                (self.id.clone(), "turn"),
                Animation::new(TURN).repeat(),
                |icon, t| icon.rotate(radians(TAU * t)),
            )
            .into_any_element()
        } else {
            icon.into_any_element()
        };
        let line = div()
            .flex()
            .items_center()
            .gap_1p5()
            .text_size(theme.text_size(TextSize::Sm))
            .text_color(tone)
            .child(icon)
            .child(text);
        let phase = std::mem::discriminant(&self.state);
        rise(self.id, phase, line, window, cx)
    }
}

/// A dot and a word for a live connection: connected, reconnecting or offline. `shell::OfflineIndicator` is the pill that shows while offline or reconnecting, and briefly once back.
#[derive(IntoElement)]
pub struct ConnectionStatus {
    id: ElementId,
    state: Connectivity,
}

impl ConnectionStatus {
    pub fn new(id: impl Into<ElementId>, state: Connectivity) -> Self {
        Self {
            id: id.into(),
            state,
        }
    }
}

impl RenderOnce for ConnectionStatus {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let text = match self.state {
            Connectivity::Online => "Connected",
            Connectivity::Reconnecting => "Reconnecting…",
            Connectivity::Offline => "Offline",
        };
        let line = div()
            .flex()
            .items_center()
            .gap_2()
            .text_size(theme.text_size(TextSize::Sm))
            .text_color(theme.colors.fg_muted)
            .child(connectivity_dot((self.id.clone(), "dot"), self.state, cx))
            .child(text);
        rise(self.id, self.state, line, window, cx)
    }
}
