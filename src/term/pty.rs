use std::{borrow::Cow, collections::HashMap, path::PathBuf, sync::Arc};

#[cfg(windows)]
use alacritty_terminal::tty;
use alacritty_terminal::{
    event::{Event, EventListener, Notify, OnResize, WindowSize},
    event_loop::{EventLoop, Msg, Notifier},
    sync::FairMutex,
    term::{Config, Term, test::TermSize},
    vte::ansi::CursorShape,
};
use futures::channel::mpsc::{UnboundedReceiver, UnboundedSender, unbounded};

use super::{
    drop::Quote,
    marks::Mark,
    mouse::MouseMode,
    notify::Ask,
    status::{Holder, Status},
    tap::Tapped,
};
use crate::settings::{CursorStyle, Settings};

/// What a session hears from its pty.
#[derive(Debug)]
pub enum PtyEvent {
    Term(Event),
    Folder(PathBuf),
    Mouse(MouseMode),
    /// An OSC 133 mark, and who held the terminal then.
    Mark(Mark, Holder),
    /// OSC 7501 status, with the group holding the terminal then.
    Status(Status, Option<i32>),
    /// OSC 9, 777 or 99, with the terminal's group then.
    Ask(Ask, Option<i32>),
    /// A title a program set, in the output's order: OSC 0 or 2, or the title stack's pop.
    Title(String),
    /// A full reset, `ESC c`.
    Reset,
    /// win32-input-mode set or reset.
    #[cfg(windows)]
    Win32(bool),
}

/// Hands the grid's events to the session; `pid` names the program in logs.
#[derive(Clone)]
pub struct Listener {
    sender: UnboundedSender<PtyEvent>,
    pid: u32,
}

impl EventListener for Listener {
    fn send_event(&self, event: Event) {
        if let Event::ChildExit(status) = &event {
            log::info!("pty {}: the program ended, {status}", self.pid);
        }
        if self.sender.unbounded_send(PtyEvent::Term(event)).is_err() {
            log::debug!("pty {}: an event after its session closed", self.pid);
        }
    }
}

/// A program, the folder it starts in, and variables over the app's own.
#[derive(Clone, Debug)]
pub struct Launch {
    /// None runs the login shell.
    pub program: Option<(String, Vec<String>)>,
    pub cwd: PathBuf,
    pub env: Vec<(String, String)>,
    /// Shell integration makes it report its folder.
    pub reports: bool,
    /// How its shell reads a path dropped on it.
    pub quote: Quote,
}

/// A running program on a pseudo-terminal and the grid it draws into.
pub struct Pty {
    pub term: Arc<FairMutex<Term<Listener>>>,
    pub pid: u32,
    pub quote: Quote,
    notifier: Notifier,
    #[cfg(unix)]
    master: std::fs::File,
}

/// What a pty reports, in order.
pub type Events = UnboundedReceiver<PtyEvent>;

#[cfg(unix)]
type Raw = super::unix::Pty;
#[cfg(windows)]
type Raw = tty::Pty;

/// The grid's options from the settings: scrollback, the cursor programs start from, kitty keys.
pub fn config(settings: &Settings) -> Config {
    let shape = match settings.cursor_style {
        CursorStyle::Block => CursorShape::Block,
        CursorStyle::Bar => CursorShape::Beam,
        CursorStyle::Underline => CursorShape::Underline,
        CursorStyle::Hollow => CursorShape::HollowBlock,
    };
    Config {
        scrolling_history: settings.scrollback as usize,
        default_cursor_style: alacritty_terminal::vte::ansi::CursorStyle {
            shape,
            blinking: settings.cursor_blink,
        },
        kitty_keyboard: true,
        ..Config::default()
    }
}

/// Starts `launch` on a pty of `columns` by `lines` cells, each `cell` pixels.
pub fn spawn(
    launch: &Launch,
    (columns, lines): (usize, usize),
    cell: (u16, u16),
    config: Config,
) -> anyhow::Result<(Pty, Events)> {
    // Windows reads the registry afresh, so a folder added since this process opened still reaches a session.
    #[cfg(windows)]
    let env = {
        let mut env = super::env::fresh()?;
        env.set("TERM", "xterm-256color".into());
        env.set("COLORTERM", "truecolor".into());
        if env.get("LANG").is_none() {
            env.set("LANG", "en_US.UTF-8".into());
        }
        for (name, value) in &launch.env {
            env.set(name, value.clone());
        }
        env.into_map()
    };
    #[cfg(not(windows))]
    let env = {
        let mut env = HashMap::from([
            ("TERM".to_string(), "xterm-256color".to_string()),
            ("COLORTERM".to_string(), "truecolor".to_string()),
        ]);
        if std::env::var_os("LANG").is_none() {
            env.insert("LANG".into(), "en_US.UTF-8".into());
        }
        env.extend(launch.env.iter().cloned());
        env
    };
    let size = fit(columns, lines, cell);
    let raw = start(launch, env, size).map_err(|error| {
        anyhow::anyhow!(
            "pty: cannot start {:?} in {}: {error}",
            launch.program,
            launch.cwd.display()
        )
    })?;
    let pid = child_id(&raw)?;
    #[cfg(unix)]
    let master = raw
        .master()
        .map_err(|error| anyhow::anyhow!("pty {pid}: no copy of the master: {error}"))?;
    log::info!(
        "pty {pid}: started {:?} in {}",
        launch.program,
        launch.cwd.display()
    );
    let (sender, events) = unbounded();
    let listener = Listener {
        sender: sender.clone(),
        pid,
    };
    let grid = TermSize::new(size.num_cols.into(), size.num_lines.into());
    let term = Arc::new(FairMutex::new(Term::new(config, &grid, listener.clone())));
    let tapped = Tapped::new(raw, sender, pid);
    #[cfg(unix)]
    let tapped = tapped.holding(master.try_clone().map_err(|error| {
        anyhow::anyhow!("pty {pid}: no copy of the master for the tap: {error}")
    })?);
    let event_loop = EventLoop::new(term.clone(), listener, tapped, true, false)?;
    let notifier = Notifier(event_loop.channel());
    event_loop.spawn();
    Ok((
        Pty {
            term,
            pid,
            quote: launch.quote,
            notifier,
            #[cfg(unix)]
            master,
        },
        events,
    ))
}

#[cfg(unix)]
fn start(launch: &Launch, env: HashMap<String, String>, size: WindowSize) -> std::io::Result<Raw> {
    super::unix::spawn(launch.program.as_ref(), &launch.cwd, &env, size)
}

#[cfg(windows)]
fn start(launch: &Launch, env: HashMap<String, String>, size: WindowSize) -> std::io::Result<Raw> {
    let options = tty::Options {
        shell: launch.program.clone().map(|(program, args)| {
            // alacritty puts the program into the command line as it is, and Windows reads the first word of it, so a path with a space needs its quotes.
            let program = if program.contains(' ') {
                format!("\"{program}\"")
            } else {
                program
            };
            tty::Shell::new(program, args)
        }),
        working_directory: Some(launch.cwd.clone()),
        drain_on_exit: true,
        env,
        escape_args: true,
    };
    tty::new(&options, size, 0)
}

impl Pty {
    /// Sends bytes to the program, as typing does.
    pub fn write(&self, bytes: impl Into<Cow<'static, [u8]>>) {
        self.notifier.notify(bytes);
    }

    /// Fits the grid and tells the program its new size; both get the same counts.
    pub fn resize(&mut self, (columns, lines): (usize, usize), cell: (u16, u16)) {
        let size = fit(columns, lines, cell);
        self.notifier.on_resize(size);
        self.term
            .lock()
            .resize(TermSize::new(size.num_cols.into(), size.num_lines.into()));
    }
}

#[cfg(unix)]
impl Pty {
    /// The foreground process group; none once none holds the terminal.
    pub fn foreground(&self) -> std::io::Result<Option<i32>> {
        super::unix::foreground(&self.master)
    }
}

#[cfg(all(test, unix))]
impl Pty {
    /// Whether the program reads raw keys, canonical mode off.
    pub(crate) fn reads_keys(&self) -> bool {
        use rustix_openpty::rustix::termios::{LocalModes, tcgetattr};
        let modes = tcgetattr(&self.master).expect("the pty's modes");
        !modes.local_modes.contains(LocalModes::ICANON)
    }
}

impl Drop for Pty {
    fn drop(&mut self) {
        if self.notifier.0.send(Msg::Shutdown).is_err() {
            log::debug!("pty {}: the event loop had already stopped", self.pid);
        } else {
            log::info!("pty {}: closing", self.pid);
        }
    }
}

#[cfg(unix)]
fn child_id(pty: &Raw) -> anyhow::Result<u32> {
    Ok(pty.id())
}

#[cfg(windows)]
fn child_id(pty: &Raw) -> anyhow::Result<u32> {
    pty.child_watcher()
        .pid()
        .map(u32::from)
        .ok_or_else(|| anyhow::anyhow!("pty: Windows gave the program no id"))
}

/// A pty's size, at least 2 by 1; counts past what the kernel holds are cut and logged.
pub fn fit(columns: usize, lines: usize, cell: (u16, u16)) -> WindowSize {
    let cells = |count: usize, side: u16| {
        let most = (i16::MAX as u16).min(u16::MAX / side.max(1));
        let kept = u16::try_from(count).unwrap_or(u16::MAX).min(most);
        if usize::from(kept) < count {
            log::error!("pty: {count} cells of {side} px pass what a pty holds; {kept} kept");
        }
        kept
    };
    WindowSize {
        num_lines: cells(lines.max(1), cell.1),
        num_cols: cells(columns.max(2), cell.0),
        cell_width: cell.0,
        cell_height: cell.1,
    }
}

#[cfg(all(test, unix))]
mod tests;

#[cfg(test)]
mod scrollback {
    use alacritty_terminal::{event::VoidListener, grid::Dimensions, vte::ansi::Processor};

    use super::*;

    #[test]
    fn the_setting_caps_history_and_a_change_trims_it() {
        let settings = Settings {
            scrollback: 5,
            ..Settings::default()
        };
        let mut term = Term::new(config(&settings), &TermSize::new(10, 3), VoidListener);
        let mut parser: Processor = Processor::new();
        let lines: String = (1..=20).map(|line| format!("{line}\r\n")).collect();
        parser.advance(&mut term, lines.as_bytes());
        assert_eq!(term.grid().history_size(), 5);
        term.set_options(config(&Settings {
            scrollback: 2,
            ..settings
        }));
        assert_eq!(term.grid().history_size(), 2);
    }
}
