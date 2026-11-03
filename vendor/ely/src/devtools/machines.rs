use std::rc::Rc;

use gpui::{
    App, ElementId, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window, div,
};
use jiff::Timestamp;

use crate::{
    buttons::IconButton,
    data_display::{Badge, Tone},
    forms::{Choice, Select},
    lists::{ListItem, SelectableList, row_action},
    primitives::{Icon, IconName},
    tables::{Cell, Column, DataTable, Row},
    theme::{ActiveTheme, IconSize},
    typography::format,
};

/// Where a container stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContainerState {
    Running,
    Paused,
    Restarting,
    /// Stopped, with the code it ended with.
    Exited(i32),
}

impl ContainerState {
    fn badge(self) -> (Tone, String) {
        match self {
            ContainerState::Running => (Tone::Success, "Running".into()),
            ContainerState::Paused => (Tone::Warning, "Paused".into()),
            ContainerState::Restarting => (Tone::Info, "Restarting".into()),
            ContainerState::Exited(0) => (Tone::Neutral, "Exited".into()),
            ContainerState::Exited(code) => (Tone::Danger, format!("Exited {code}")),
        }
    }
}

/// A container: its key and name, its image, where it stands, its published ports, its share of a CPU and its memory in bytes.
#[derive(Clone, Debug, PartialEq)]
pub struct Container {
    pub key: SharedString,
    pub name: SharedString,
    pub image: SharedString,
    pub state: ContainerState,
    pub ports: Vec<SharedString>,
    pub cpu: f32,
    pub memory: u64,
}

type OnKey = Rc<dyn Fn(&SharedString, &mut Window, &mut App)>;

/// A container's line under its name: its image, its ports, and its load while it runs.
fn described(container: &Container) -> String {
    let ports = match container.ports.is_empty() {
        true => String::new(),
        false => format!(" · {}", container.ports.join(", ")),
    };
    let load = match container.state == ContainerState::Running {
        true => format!(
            " · {} · {}",
            format::percent(container.cpu as f64, 0, false),
            format::file_size(container.memory, true)
        ),
        false => String::new(),
    };
    format!("{}{ports}{load}", container.image)
}

/// Containers on a host, each with its image and ports, its load while it runs, and where it stands. Its buttons, each shown when the owner handles it, start or stop it, restart it and open its logs; Enter or a double press opens it.
#[derive(IntoElement)]
pub struct ContainerList {
    id: ElementId,
    containers: Vec<Container>,
    on_open: Option<OnKey>,
    on_toggle: Option<OnKey>,
    on_restart: Option<OnKey>,
    on_logs: Option<OnKey>,
}

impl ContainerList {
    pub fn new(id: impl Into<ElementId>, containers: impl IntoIterator<Item = Container>) -> Self {
        Self {
            id: id.into(),
            containers: containers.into_iter().collect(),
            on_open: None,
            on_toggle: None,
            on_restart: None,
            on_logs: None,
        }
    }

    pub fn on_open(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_open = Some(Rc::new(handler));
        self
    }

    /// Asked to stop a running container, or to start any other.
    pub fn on_toggle(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_toggle = Some(Rc::new(handler));
        self
    }

    pub fn on_restart(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_restart = Some(Rc::new(handler));
        self
    }

    pub fn on_logs(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_logs = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for ContainerList {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id;
        let theme = cx.theme();
        let muted = theme.colors.fg_muted;
        let list = self.containers.iter().fold(
            SelectableList::new((id.clone(), "list")),
            |list, container| {
                let key = container.key.clone();
                let running = container.state == ContainerState::Running;
                let (tone, words) = container.state.badge();
                let button = |icon: IconName, tip: &'static str, handler: &Option<OnKey>| {
                    handler.clone().map(|handler| {
                        let key = key.clone();
                        row_action(
                            IconButton::new((id.clone(), format!("{tip}-{key}")), icon)
                                .tooltip(tip)
                                .on_click(move |_, window, cx| {
                                    log::info!("container list: {tip} {key}");
                                    handler(&key, window, cx);
                                }),
                        )
                    })
                };
                list.row(
                    key.clone(),
                    ListItem::new((id.clone(), format!("row-{key}")), container.name.clone())
                        .description(described(container))
                        .leading(Icon::new(IconName::Package).size(IconSize::Md).color(muted))
                        .trailing(
                            div()
                                .flex()
                                .items_center()
                                .gap_1()
                                .child(Badge::new(words).tone(tone).dot())
                                .children(button(
                                    if running {
                                        IconName::CircleStop
                                    } else {
                                        IconName::Play
                                    },
                                    if running { "Stop" } else { "Start" },
                                    &self.on_toggle,
                                ))
                                .children(button(IconName::RotateCw, "Restart", &self.on_restart))
                                .children(button(IconName::Terminal, "Logs", &self.on_logs)),
                        ),
                )
            },
        );
        match self.on_open {
            Some(on_open) => list.on_activate(move |key, window, cx| {
                log::info!("container list: open {key}");
                on_open(key, window, cx)
            }),
            None => list,
        }
    }
}

/// A pod's phase, as Kubernetes names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PodPhase {
    Pending,
    Running,
    Succeeded,
    Failed,
    Unknown,
}

impl PodPhase {
    fn cell(self) -> Cell {
        let (words, tone) = match self {
            PodPhase::Pending => ("Pending", Tone::Warning),
            PodPhase::Running => ("Running", Tone::Success),
            PodPhase::Succeeded => ("Succeeded", Tone::Neutral),
            PodPhase::Failed => ("Failed", Tone::Danger),
            PodPhase::Unknown => ("Unknown", Tone::Neutral),
        };
        Cell::Tag(words.into(), tone)
    }
}

/// A pod: its key and name, its namespace, its phase, its containers ready of all, its restarts, when it started, and its node.
#[derive(Clone, Debug, PartialEq)]
pub struct Pod {
    pub key: SharedString,
    pub name: SharedString,
    pub namespace: SharedString,
    pub phase: PodPhase,
    pub ready: (u32, u32),
    pub restarts: u32,
    pub started: Timestamp,
    pub node: SharedString,
}

type OnKeys = Rc<dyn Fn(&[SharedString], &mut Window, &mut App)>;

/// Pods in a table, sortable by any column: name, namespace, phase, containers ready, restarts, age and node; a namespace narrows them.
#[derive(IntoElement)]
pub struct PodList {
    id: ElementId,
    pods: Vec<Pod>,
    now: Timestamp,
    selected: Vec<SharedString>,
    on_select: Option<OnKeys>,
}

impl PodList {
    /// `now` ages each pod.
    pub fn new(
        id: impl Into<ElementId>,
        pods: impl IntoIterator<Item = Pod>,
        now: Timestamp,
    ) -> Self {
        Self {
            id: id.into(),
            pods: pods.into_iter().collect(),
            now,
            selected: Vec::new(),
            on_select: None,
        }
    }

    pub fn selected(mut self, keys: impl IntoIterator<Item = impl Into<SharedString>>) -> Self {
        self.selected = keys.into_iter().map(Into::into).collect();
        self
    }

    pub fn on_select(
        mut self,
        handler: impl Fn(&[SharedString], &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_select = Some(Rc::new(handler));
        self
    }
}

/// Seconds from `started` to `now`; a start ahead of now, as clocks between machines drift, reads as now.
pub(crate) fn running_for(started: Timestamp, now: Timestamp) -> u64 {
    (now.as_second() - started.as_second()).max(0) as u64
}

/// The namespaces among `pods`, in the order they first come.
fn namespaces(pods: &[Pod]) -> Vec<SharedString> {
    let mut seen: Vec<SharedString> = Vec::new();
    for pod in pods {
        if !seen.contains(&pod.namespace) {
            seen.push(pod.namespace.clone());
        }
    }
    seen
}

/// The pods in `namespace`; the empty namespace, which Kubernetes never names, stands for all.
fn in_namespace<'a>(pods: &'a [Pod], namespace: &'a str) -> impl Iterator<Item = &'a Pod> {
    pods.iter()
        .filter(move |pod| namespace.is_empty() || pod.namespace == namespace)
}

impl RenderOnce for PodList {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id;
        let space = window.use_keyed_state((id.clone(), "namespace"), cx, |_, _| {
            SharedString::default()
        });
        let chosen = space.read(cx).clone();
        let spaces = namespaces(&self.pods);
        let rows: Vec<Row> = in_namespace(&self.pods, &chosen)
            .map(|pod| {
                Row::new(
                    pod.key.clone(),
                    [
                        Cell::Text(pod.name.clone()),
                        Cell::Text(pod.namespace.clone()),
                        pod.phase.cell(),
                        Cell::Text(format!("{}/{}", pod.ready.0, pod.ready.1).into()),
                        Cell::Number(pod.restarts as f64),
                        Cell::Duration(running_for(pod.started, self.now)),
                        Cell::Text(pod.node.clone()),
                    ],
                )
            })
            .collect();
        let theme = cx.theme();
        let mut table = DataTable::new(
            (id.clone(), "table"),
            [
                Column::new("name", "Name"),
                Column::new("namespace", "Namespace"),
                Column::new("phase", "Status"),
                Column::new("ready", "Ready"),
                Column::new("restarts", "Restarts").end(),
                Column::new("age", "Age"),
                Column::new("node", "Node"),
            ],
        )
        .rows(rows)
        .selected(self.selected);
        if let Some(on_select) = self.on_select {
            table = table.on_select(move |keys, window, cx| on_select(keys, window, cx));
        }
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div().max_w(gpui::rems(theme.label_width().0 * 2.0)).child(
                    Select::new(
                        (id, "spaces"),
                        [Choice::new("", "All namespaces")].into_iter().chain(
                            spaces
                                .iter()
                                .map(|space| Choice::new(space.clone(), space.clone())),
                        ),
                    )
                    .selected(chosen)
                    .on_change(move |value, _, cx| {
                        space.update(cx, |space, cx| {
                            *space = value.clone();
                            cx.notify();
                        })
                    }),
                ),
            )
            .child(table)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Container, ContainerState, Pod, PodPhase, described, in_namespace, namespaces, running_for,
    };
    use crate::data_display::Tone;

    #[test]
    fn a_clean_exit_rests_quiet_and_a_stopped_container_shows_no_load() {
        assert_eq!(
            ContainerState::Exited(0).badge(),
            (Tone::Neutral, "Exited".to_string())
        );
        assert_eq!(
            ContainerState::Exited(1).badge(),
            (Tone::Danger, "Exited 1".to_string())
        );
        let mut web = Container {
            key: "web".into(),
            name: "web".into(),
            image: "img".into(),
            state: ContainerState::Running,
            ports: vec!["80:80".into()],
            cpu: 0.25,
            memory: 1 << 20,
        };
        assert_eq!(described(&web), "img · 80:80 · 25% · 1.0 MiB");
        web.state = ContainerState::Exited(0);
        assert_eq!(described(&web), "img · 80:80");
    }

    #[test]
    fn a_pod_runs_from_its_start_and_a_clock_ahead_reads_as_now() {
        let now: jiff::Timestamp = "2026-09-27T12:00:00Z".parse().expect("a time");
        let started: jiff::Timestamp = "2026-09-27T10:35:00Z".parse().expect("a time");
        assert_eq!(running_for(started, now), 5_100);
        assert_eq!(running_for(now, started), 0);
    }

    #[test]
    fn namespaces_come_once_in_order_and_narrow_the_pods() {
        let now: jiff::Timestamp = "2026-09-27T12:00:00Z".parse().expect("a time");
        let pod = |key: &str, namespace: &str| Pod {
            key: key.to_string().into(),
            name: key.to_string().into(),
            namespace: namespace.to_string().into(),
            phase: PodPhase::Running,
            ready: (1, 1),
            restarts: 0,
            started: now,
            node: "n1".into(),
        };
        let pods = [pod("a", "web"), pod("b", "jobs"), pod("c", "web")];
        assert_eq!(namespaces(&pods), ["web", "jobs"]);
        let keys = |namespace| {
            in_namespace(&pods, namespace)
                .map(|pod| pod.key.to_string())
                .collect::<Vec<_>>()
        };
        assert_eq!(keys("web"), ["a", "c"]);
        assert_eq!(keys(""), ["a", "b", "c"]);
    }
}
