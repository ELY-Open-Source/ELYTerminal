use std::{
    collections::HashMap,
    ffi::{OsStr, OsString},
    io::{self, BufRead, BufReader, Write},
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::Context as _;
use gpui::{Context, Task};
use serde::Deserialize;

use super::{
    status::{HERDR, Record, State},
    view::{TerminalEvent, TerminalView},
};

#[cfg(unix)]
mod unix;
#[cfg(unix)]
use unix::{Program, client, connect, home, leader, running};
#[cfg(windows)]
mod windows;
#[cfg(windows)]
use windows::{Program, client, connect, home, leader, running};

/// How often the session's foreground program is checked.
const CHECK: Duration = Duration::from_millis(500);

/// How long herdr may take to answer.
const WAIT: Duration = Duration::from_secs(1);

/// The socket request for herdr's session: its focus, its panes' folders and its agents.
const SNAPSHOT: &[u8] = b"{\"id\":\"elyterminal\",\"method\":\"session.snapshot\",\"params\":{}}\n";

#[cfg(windows)]
use super::procs::Invocation;

/// Follows the pane herdr's server has focused while herdr's client runs in the session.
pub fn follow(cx: &mut Context<TerminalView>) -> Task<()> {
    cx.spawn(async move |view, cx| {
        let mut seen: Option<Program> = None;
        let (mut socket, mut shown, mut retry, mut failed) = (None, None, false, String::new());
        let mut states: Vec<(String, State)> = Vec::new();
        loop {
            cx.background_executor().timer(CHECK).await;
            let Ok((pid, lead, reports)) = view.update(cx, |view, _| {
                (view.pty.pid, leader(&view.pty), view.reports)
            }) else {
                break;
            };
            let program = match lead {
                Ok(lead) => {
                    cx.background_executor()
                        .spawn(async move { running(lead) })
                        .await
                }
                Err(error) => Err(error),
            };
            let program = match program {
                Ok(program) => Some(program),
                Err(error) => {
                    if seen.is_some() {
                        log::debug!("terminal {pid}: its foreground does not read: {error}");
                    }
                    None
                }
            };
            if program != seen {
                seen = program.clone();
                shown = None;
                retry = true;
                failed.clear();
                if socket.take().is_some() {
                    log::info!(
                        "terminal {pid}: herdr left the foreground, its agents' records with it"
                    );
                    states.clear();
                    let left = view.update(cx, |view, cx| {
                        view.herdr = false;
                        view.agents(&[], cx);
                    });
                    if left.is_err() {
                        break;
                    }
                }
            }
            if retry {
                let found = match program {
                    Some(program) => {
                        cx.background_executor()
                            .spawn(async move { client(program) })
                            .await
                    }
                    None => Ok(None),
                };
                retry = found.is_err();
                socket = found.unwrap_or_else(|error| {
                    failure(&mut failed, pid, &error);
                    None
                });
                if let Some(socket) = &socket {
                    log::info!(
                        "terminal {pid}: herdr runs here, its server at {}",
                        socket.display()
                    );
                }
                let runs = socket.is_some();
                if view.update(cx, |view, _| view.herdr = runs).is_err() {
                    break;
                }
            }
            let Some(path) = socket.clone() else {
                continue;
            };
            let (folder, agents) = match cx
                .background_executor()
                .spawn(async move { snapshot(&path) })
                .await
            {
                Ok(shows) => shows,
                Err(error) => {
                    failure(&mut failed, pid, &error);
                    continue;
                }
            };
            failed.clear();
            let report = folder.filter(|folder| shown.as_ref() != Some(folder));
            // The shell reports only once herdr has left; its folder then stands.
            let Ok(still) = view.update(cx, |view, cx| {
                let still = view.reports == reports;
                if still {
                    view.agents(&agents, cx);
                    if let Some(folder) = report.clone() {
                        cx.emit(TerminalEvent::Folder(folder));
                    }
                }
                still
            }) else {
                break;
            };
            if !still {
                continue;
            }
            let now: Vec<(String, State)> = agents
                .into_iter()
                .map(|(id, record)| (id, record.state))
                .collect();
            if now != states {
                log::info!("terminal {pid}: herdr's agents are {now:?}");
                states = now;
            }
            if let Some(folder) = report {
                log::info!(
                    "terminal {pid}: herdr's focused pane is in {}",
                    folder.display()
                );
                shown = Some(folder);
            }
        }
    })
}

/// Logs `error` once while it lasts; a server not up yet is no error.
fn failure(failed: &mut String, pid: u32, error: &anyhow::Error) {
    let text = format!("{error:#}");
    if text != *failed {
        let level = if starting(error) {
            log::Level::Info
        } else {
            log::Level::Error
        };
        log::log!(level, "terminal {pid}: {text}; the panel keeps its folder");
        *failed = text;
    }
}

/// herdr's client runs before its server listens: no socket yet, or none answering.
fn starting(error: &anyhow::Error) -> bool {
    error.downcast_ref::<io::Error>().is_some_and(|error| {
        matches!(
            error.kind(),
            io::ErrorKind::NotFound | io::ErrorKind::ConnectionRefused
        )
    })
}

/// The socket a herdr process with `args` and `env` shows, if it is herdr's terminal client.
pub(super) fn follows(
    args: &[String],
    env: &HashMap<String, OsString>,
) -> anyhow::Result<Option<PathBuf>> {
    if !attaches(args) {
        return Ok(None);
    }
    let remote = |arg: &String| {
        ["--remote", "--machine"]
            .iter()
            .any(|flag| arg == flag || arg.starts_with(&format!("{flag}=")))
    };
    anyhow::ensure!(
        !args.iter().any(remote),
        "herdr: its client shows another machine's server, whose folders cannot show here"
    );
    socket(args, env).map(Some)
}

/// Whether `args` start herdr's terminal client, not one of its commands.
fn attaches(args: &[String]) -> bool {
    let mut rest = args.iter().skip(1);
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--session" | "--remote" | "--machine" | "--remote-keybindings" => {
                rest.next();
            }
            "--version" | "-V" | "--help" | "-h" | "--default-config" | "--skill" => return false,
            "client" => return true,
            "session" => return rest.next().is_some_and(|next| next == "attach"),
            flag if flag.starts_with('-') => {}
            _ => return false,
        }
    }
    true
}

/// The socket herdr 0.9's client finds: a named session, `HERDR_SOCKET_PATH`, `HERDR_SESSION`, the default.
fn socket(args: &[String], env: &HashMap<String, OsString>) -> anyhow::Result<PathBuf> {
    let dir = match env.get("XDG_CONFIG_HOME") {
        Some(config) => PathBuf::from(config).join("herdr"),
        None => home(env)?,
    };
    let named = |name: &OsStr| {
        if name == "default" {
            dir.join("herdr.sock")
        } else {
            dir.join("sessions").join(name).join("herdr.sock")
        }
    };
    if let Some(name) = session(args) {
        return Ok(named(OsStr::new(&name)));
    }
    if let Some(path) = env.get("HERDR_SOCKET_PATH") {
        return Ok(PathBuf::from(path));
    }
    Ok(env
        .get("HERDR_SESSION")
        .map_or_else(|| dir.join("herdr.sock"), |name| named(name)))
}

/// The session `session attach NAME`, `--session NAME` or `--session=NAME` names, as herdr reads it.
fn session(args: &[String]) -> Option<String> {
    if let [_, noun, verb, name] = args
        && noun == "session"
        && verb == "attach"
    {
        return Some(name.clone());
    }
    let mut named = None;
    let mut rest = args.iter().skip(1);
    while let Some(arg) = rest.next() {
        if arg == "--" {
            break;
        }
        if arg == "--session" {
            named = rest.next().cloned();
        } else if let Some(name) = arg.strip_prefix("--session=") {
            named = Some(name.to_owned());
        }
    }
    named
}

/// What herdr's server shows: the folder of the pane it has focused, none while it has none or knows none, and its agents.
fn snapshot(socket: &Path) -> anyhow::Result<Shows> {
    let ask = || -> io::Result<String> {
        let mut stream = connect(socket)?;
        stream.write_all(SNAPSHOT)?;
        let mut reply = String::new();
        BufReader::new(stream).read_line(&mut reply)?;
        Ok(reply)
    };
    let reply = ask().with_context(|| format!("herdr: asking {}", socket.display()))?;
    shows(&reply).with_context(|| format!("herdr: {}", socket.display()))
}

/// The focused pane's folder, and herdr's agents as records.
type Shows = (Option<PathBuf>, Vec<(String, Record)>);

#[derive(Deserialize)]
struct Reply {
    result: Option<Snapshot>,
    error: Option<Failure>,
}

#[derive(Deserialize)]
struct Snapshot {
    snapshot: Session,
}

#[derive(Deserialize)]
struct Session {
    focused_pane_id: Option<String>,
    panes: Vec<Pane>,
    /// A herdr before agents lists none.
    #[serde(default)]
    agents: Vec<Agent>,
}

#[derive(Deserialize)]
struct Pane {
    pane_id: String,
    cwd: Option<PathBuf>,
}

#[derive(Deserialize)]
struct Agent {
    pane_id: String,
    agent_status: AgentStatus,
    agent: Option<String>,
    display_agent: Option<String>,
    name: Option<String>,
    title: Option<String>,
    terminal_title_stripped: Option<String>,
}

/// An agent's state, as herdr names it.
#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum AgentStatus {
    Idle,
    Working,
    Blocked,
    Done,
    #[serde(other)]
    Unknown,
}

#[derive(Deserialize)]
struct Failure {
    code: String,
    message: String,
}

/// What herdr's `session.snapshot` reply shows.
pub(super) fn shows(reply: &str) -> anyhow::Result<Shows> {
    let reply: Reply = serde_json::from_str(reply).context("its reply does not parse")?;
    if let Some(Failure { code, message }) = reply.error {
        anyhow::bail!("{code}: {message}");
    }
    let session = reply.result.context("its reply holds no result")?.snapshot;
    let agents = records(session.agents);
    let Some(id) = session.focused_pane_id else {
        return Ok((None, agents));
    };
    let pane = session
        .panes
        .into_iter()
        .find(|pane| pane.pane_id == id)
        .with_context(|| format!("its focused pane {id} is not among its panes"))?;
    if let Some(cwd) = &pane.cwd {
        anyhow::ensure!(
            cwd.is_absolute(),
            "pane {id}'s folder {} is relative",
            cwd.display()
        );
    }
    Ok((pane.cwd, agents))
}

/// Each agent whose state herdr knows, as the record of its pane: herdr names no kind and no progress.
fn records(agents: Vec<Agent>) -> Vec<(String, Record)> {
    let record = |agent: Agent| {
        let state = match agent.agent_status {
            AgentStatus::Idle => State::Idle,
            AgentStatus::Working => State::Working,
            AgentStatus::Blocked => State::Blocked,
            AgentStatus::Done => State::Done,
            AgentStatus::Unknown => return None,
        };
        let record = Record {
            state,
            kind: None,
            progress: None,
            app: agent.agent,
            title: agent.name.or(agent.display_agent),
            msg: agent.terminal_title_stripped.or(agent.title),
        };
        Some((format!("{HERDR}{}", agent.pane_id), record))
    };
    agents.into_iter().filter_map(record).collect()
}

#[cfg(test)]
mod tests;
