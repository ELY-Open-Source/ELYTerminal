use std::{rc::Rc, time::Duration};

use gpui::{
    AnyElement, App, ElementId, FontWeight, Hsla, IntoElement, ParentElement, RenderOnce,
    SharedString, Styled, Window, div, prelude::*,
};

use crate::{
    data_display::Meter,
    forms::Switch,
    lists::{List, ListItem},
    motion::Pulse,
    theme::{ActiveTheme, IconSize, Palette, Radius, TextSize},
    typography::{
        Ellipsis,
        format::{DurationStyle, duration, file_size, plural},
    },
};

/// A dot in `color`, ringing while `live`.
fn dot(id: ElementId, color: Hsla, live: bool, cx: &App) -> AnyElement {
    let mark = div()
        .flex_none()
        .size(cx.theme().status_dot())
        .rounded_full()
        .bg(color);
    if live {
        Pulse::new(id).child(mark).into_any_element()
    } else {
        mark.into_any_element()
    }
}

/// Where a sandbox stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SandboxState {
    Starting,
    Ready,
    Busy,
    Stopped,
    Failed,
}

impl SandboxState {
    fn word(self) -> &'static str {
        match self {
            Self::Starting => "Starting",
            Self::Ready => "Ready",
            Self::Busy => "Busy",
            Self::Stopped => "Stopped",
            Self::Failed => "Failed",
        }
    }

    fn color(self, colors: &Palette) -> Hsla {
        match self {
            Self::Starting => colors.warning,
            Self::Busy => colors.info,
            Self::Ready => colors.success,
            Self::Stopped => colors.fg_subtle,
            Self::Failed => colors.danger,
        }
    }
}

/// Where an agent's sandbox stands: its name, a dot and a word, what it uses of what it has, and how long it has run.
#[derive(IntoElement)]
pub struct SandboxStatus {
    id: ElementId,
    name: SharedString,
    state: SandboxState,
    cpu: Option<f32>,
    memory: Option<(u64, u64)>,
    uptime: Option<Duration>,
}

impl SandboxStatus {
    pub fn new(
        id: impl Into<ElementId>,
        name: impl Into<SharedString>,
        state: SandboxState,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            state,
            cpu: None,
            memory: None,
            uptime: None,
        }
    }

    /// The share of its processors in use, 0 to 1.
    pub fn cpu(mut self, share: f32) -> Self {
        assert!((0.0..=1.0).contains(&share), "cpu {share} of 1");
        self.cpu = Some(share);
        self
    }

    /// Memory in use and its limit, in bytes.
    pub fn memory(mut self, used: u64, limit: u64) -> Self {
        assert!(limit > 0, "memory {used} of no limit");
        if used > limit {
            log::error!("environment: memory {used} past its limit {limit}; the meter fills");
        }
        self.memory = Some((used, limit));
        self
    }

    pub fn uptime(mut self, uptime: Duration) -> Self {
        self.uptime = Some(uptime);
        self
    }
}

impl RenderOnce for SandboxStatus {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let live = matches!(self.state, SandboxState::Starting | SandboxState::Busy);
        let color = self.state.color(&colors);
        div()
            .flex()
            .flex_col()
            .gap_3()
            .p_4()
            .rounded(theme.radius(Radius::Lg))
            .border_1()
            .border_color(colors.border)
            .bg(colors.surface)
            .text_size(theme.text_size(TextSize::Sm))
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap_x_2()
                    .gap_y_1()
                    .child(
                        div()
                            .flex_1()
                            .min_w(theme.label_width())
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(dot((self.id.clone(), "state").into(), color, live, cx))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(Ellipsis::new(self.name)),
                            ),
                    )
                    .child(
                        div()
                            .flex_none()
                            .flex()
                            .items_center()
                            .gap_2()
                            .text_color(colors.fg_muted)
                            .child(self.state.word())
                            .children(self.uptime.map(|up| {
                                div().text_color(colors.fg_subtle).child(format!(
                                    "up {}",
                                    duration(up.as_secs(), DurationStyle::Compact)
                                ))
                            })),
                    ),
            )
            .children(
                self.cpu
                    .map(|share| Meter::new((self.id.clone(), "cpu"), "Processors", share)),
            )
            .children(self.memory.map(|(used, limit)| {
                Meter::new(
                    (self.id.clone(), "memory"),
                    "Memory",
                    used.min(limit) as f32 / limit as f32,
                )
                .detail(format!(
                    "{} of {}",
                    file_size(used, true),
                    file_size(limit, true)
                ))
            }))
    }
}

/// Where a connected server stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ServerState {
    Connected,
    Connecting,
    Failed,
    Off,
}

/// A server or connector the agent can reach: its key and name, where it runs or why it failed, where it stands and how many tools it brings.
#[derive(Clone, Debug, PartialEq)]
pub struct McpServer {
    pub key: SharedString,
    pub name: SharedString,
    pub detail: SharedString,
    pub state: ServerState,
    pub tools: usize,
}

type OnToggle = Rc<dyn Fn(&SharedString, bool, &mut Window, &mut App)>;

/// Servers and connectors the agent can reach: each with a dot for where it stands, what it runs or why it failed, how many tools it brings, and a switch.
#[derive(IntoElement)]
pub struct McpServerList {
    id: ElementId,
    servers: Vec<McpServer>,
    on_toggle: Option<OnToggle>,
}

impl McpServerList {
    pub fn new(id: impl Into<ElementId>, servers: impl IntoIterator<Item = McpServer>) -> Self {
        Self {
            id: id.into(),
            servers: servers.into_iter().collect(),
            on_toggle: None,
        }
    }

    /// Gets a server's key and whether it is now on.
    pub fn on_toggle(
        mut self,
        handler: impl Fn(&SharedString, bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_toggle = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for McpServerList {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let colors = cx.theme().colors.clone();
        List::new()
            .divided()
            .children(self.servers.into_iter().map(|server| {
                let (color, live) = match server.state {
                    ServerState::Connected => (colors.success, false),
                    ServerState::Connecting => (colors.warning, true),
                    ServerState::Failed => (colors.danger, false),
                    ServerState::Off => (colors.fg_subtle, false),
                };
                let toggle = self.on_toggle.clone().map(|toggle| {
                    let key = server.key.clone();
                    Switch::new(
                        (self.id.clone(), format!("switch-{}", server.key)),
                        server.state != ServerState::Off,
                    )
                    .on_change(move |on, window, cx| {
                        log::info!("mcp server {key}: {}", if on { "on" } else { "off" });
                        toggle(&key, on, window, cx)
                    })
                });
                ListItem::new(
                    (self.id.clone(), format!("server-{}", server.key)),
                    server.name,
                )
                .description(server.detail)
                .leading(
                    div()
                        .size(cx.theme().icon_size(IconSize::Sm))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(dot(
                            (self.id.clone(), format!("dot-{}", server.key)).into(),
                            color,
                            live,
                            cx,
                        )),
                )
                .trailing(
                    div()
                        .flex()
                        .items_center()
                        .gap_3()
                        .text_size(cx.theme().text_size(TextSize::Xs))
                        .text_color(colors.fg_subtle)
                        .when(server.state == ServerState::Connected, |end| {
                            end.child(plural(server.tools as u64, "tool", "tools"))
                        })
                        .children(toggle),
                )
            }))
    }
}
