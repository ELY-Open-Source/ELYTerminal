//! The door other programs ask a running ELYTerminal through: a loopback port and a token only the user reads.

use std::{
    fs,
    io::{BufRead, BufReader, Read, Write},
    net::{Ipv4Addr, TcpListener, TcpStream},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::Duration,
};

use anyhow::{Context as _, bail, ensure};
use futures::{
    StreamExt,
    channel::mpsc::{UnboundedSender, unbounded},
};
use gpui::{App, Global};
use serde::{Deserialize, Serialize};

use crate::{settings::SettingsStore, workspace::Workspace};

/// The file beside the settings that names the door's port and token.
const FILE: &str = "control";

pub const USAGE: &str = "usage: elyterminal ctl sessions
       elyterminal ctl read SESSION [LINES]
       elyterminal ctl send SESSION TEXT
       elyterminal ctl key SESSION KEY...
sessions lists each session: its number, * for the one shown, its title and its folder.
read prints the last LINES lines SESSION holds, its screen without a count.
send types TEXT and Enter. key presses each KEY, such as enter, up, ctrl-c or shift-tab.
ELYTerminal must be running with Settings › Terminal › Command line control on.";

/// How long a caller may take to ask, and the app to answer.
const PATIENCE: Duration = Duration::from_secs(5);
const ANSWERING: Duration = Duration::from_secs(10);
/// The longest request or answer, in bytes.
const LONGEST: u64 = 1 << 20;

#[derive(Serialize, Deserialize)]
struct Request {
    token: String,
    words: Vec<String>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Reply {
    Ok(String),
    Error(String),
}

/// A request and where its answer goes.
struct Ask {
    words: Vec<String>,
    answer: mpsc::Sender<Reply>,
}

/// The open door; dropping it shuts it.
struct Door {
    open: Arc<AtomicBool>,
    port: u16,
    file: PathBuf,
}

impl Drop for Door {
    fn drop(&mut self) {
        self.open.store(false, Ordering::SeqCst);
        if let Err(error) = fs::remove_file(&self.file) {
            log::error!("control: {} stays: {error}", self.file.display());
        }
        // A last caller wakes the listener, which then ends.
        if let Err(error) = TcpStream::connect((Ipv4Addr::LOCALHOST, self.port)) {
            log::debug!("control: the listener had gone: {error}");
        }
        log::info!("control: the door on port {} shut", self.port);
    }
}

/// The door while the settings want it.
struct Control(Option<Door>);

impl Global for Control {}

/// Opens the door while the settings ask for it, and shuts it when they stop.
pub fn start(cx: &mut App) {
    cx.set_global(Control(None));
    sync(cx);
    cx.observe_global::<SettingsStore>(sync).detach();
    // The file names a port nothing answers once the app is gone.
    cx.on_app_quit(|cx| {
        cx.set_global(Control(None));
        async {}
    })
    .detach();
}

fn sync(cx: &mut App) {
    let wanted = SettingsStore::get(cx).control;
    if wanted == cx.global::<Control>().0.is_some() {
        return;
    }
    let door = if wanted {
        match open(cx) {
            Ok(door) => Some(door),
            Err(error) => return log::error!("control: the door did not open: {error:#}"),
        }
    } else {
        None
    };
    cx.set_global(Control(door));
}

/// The folder the settings and the door's file live in.
fn folder() -> anyhow::Result<PathBuf> {
    let config = dirs::config_dir().context("control: the platform names no config folder")?;
    Ok(config.join("ELYTerminal"))
}

fn open(cx: &mut App) -> anyhow::Result<Door> {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).context("no loopback port")?;
    let port = listener.local_addr()?.port();
    let mut token = [0; 32];
    getrandom::fill(&mut token).map_err(|error| anyhow::anyhow!("no random token: {error}"))?;
    let token: String = token.iter().map(|byte| format!("{byte:02x}")).collect();
    let file = folder()?.join(FILE);
    write_private(&file, &format!("{port} {token}\n"))?;
    let open = Arc::new(AtomicBool::new(true));
    let (asks, mut asked) = unbounded::<Ask>();
    let serving = open.clone();
    thread::Builder::new()
        .name("control door".into())
        .spawn(move || serve(listener, &token, &serving, &asks))
        .context("no thread for the door")?;
    cx.spawn(async move |cx| {
        while let Some(ask) = asked.next().await {
            let reply = match cx.update(|cx| answer(&ask.words, cx)) {
                Ok(text) => Reply::Ok(text),
                Err(error) => Reply::Error(format!("{error:#}")),
            };
            if ask.answer.send(reply).is_err() {
                log::warn!("control: a caller left before its answer");
            }
        }
    })
    .detach();
    log::info!(
        "control: the door is open on port {port}, named in {}",
        file.display()
    );
    Ok(Door { open, port, file })
}

/// Writes `text` where only the user reads it, whole or not at all.
fn write_private(file: &Path, text: &str) -> anyhow::Result<()> {
    let fresh = file.with_extension("new");
    match fs::remove_file(&fresh) {
        Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
            return Err(error).with_context(|| format!("{} is in the way", fresh.display()));
        }
        _ => {}
    }
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
    let mut written = options
        .open(&fresh)
        .with_context(|| format!("cannot create {}", fresh.display()))?;
    written.write_all(text.as_bytes())?;
    fs::rename(&fresh, file).with_context(|| format!("cannot name {}", file.display()))
}

/// Answers each caller in turn until the door shuts.
fn serve(listener: TcpListener, token: &str, open: &AtomicBool, asks: &UnboundedSender<Ask>) {
    for stream in listener.incoming() {
        if !open.load(Ordering::SeqCst) {
            break;
        }
        let served = stream
            .context("a caller did not connect")
            .and_then(|stream| serve_one(&stream, token, asks));
        if let Err(error) = served {
            log::warn!("control: a request failed: {error:#}");
        }
    }
}

fn serve_one(stream: &TcpStream, token: &str, asks: &UnboundedSender<Ask>) -> anyhow::Result<()> {
    stream.set_read_timeout(Some(PATIENCE))?;
    stream.set_write_timeout(Some(PATIENCE))?;
    let request: Request = read_line(stream)?;
    let reply = if same(request.token.as_bytes(), token.as_bytes()) {
        let verb = request.words.first().map_or("nothing", String::as_str);
        log::info!("control: asked to {verb}, {} words", request.words.len());
        let (answer, answered) = mpsc::channel();
        let ask = Ask {
            words: request.words,
            answer,
        };
        asks.unbounded_send(ask).context("the app is quitting")?;
        answered
            .recv_timeout(ANSWERING)
            .context("the app did not answer in time")?
    } else {
        log::warn!("control: a caller without the token was refused");
        Reply::Error("the token is not this door's".into())
    };
    write_line(stream, &reply)
}

/// Whether two tokens match, in time that tells nothing of where they differ.
fn same(given: &[u8], held: &[u8]) -> bool {
    given.len() == held.len()
        && given
            .iter()
            .zip(held)
            .fold(0, |differ, (a, b)| differ | (a ^ b))
            == 0
}

fn read_line<T: serde::de::DeserializeOwned>(stream: &TcpStream) -> anyhow::Result<T> {
    let mut line = String::new();
    BufReader::new(stream.take(LONGEST))
        .read_line(&mut line)
        .context("the line did not arrive")?;
    ensure!(
        line.ends_with('\n'),
        "the line is cut or over {LONGEST} bytes"
    );
    serde_json::from_str(&line).context("the line is not what the door speaks")
}

fn write_line(mut stream: &TcpStream, value: &impl Serialize) -> anyhow::Result<()> {
    let mut line = serde_json::to_string(value)?;
    line.push('\n');
    stream
        .write_all(line.as_bytes())
        .context("the line did not leave")
}

/// What the main window answers to `words`.
fn answer(words: &[String], cx: &mut App) -> anyhow::Result<String> {
    let window = cx
        .windows()
        .into_iter()
        .find_map(|window| window.downcast::<Workspace>())
        .context("no terminal window is open")?;
    window.update(cx, |workspace, _, cx| workspace.asked(words, cx))?
}

/// `elyterminal ctl …` asks the running copy and prints its answer; the exit code, none for any other command line.
pub fn command_line() -> Option<i32> {
    let mut args = std::env::args().skip(1);
    if args.next().as_deref() != Some("ctl") {
        return None;
    }
    let words: Vec<String> = args.collect();
    #[cfg(windows)]
    console();
    Some(match ask(&words) {
        Ok(text) => {
            print!("{text}");
            0
        }
        Err(error) => {
            eprintln!("elyterminal: {error:#}");
            1
        }
    })
}

/// A release build is a window program: with no output to write to, it borrows the console it was started from.
#[cfg(windows)]
fn console() {
    use windows_sys::Win32::{
        Foundation::INVALID_HANDLE_VALUE,
        System::Console::{ATTACH_PARENT_PROCESS, AttachConsole, GetStdHandle, STD_OUTPUT_HANDLE},
    };
    let out = unsafe { GetStdHandle(STD_OUTPUT_HANDLE) };
    // A pipe or a file already takes the output; a console would take it away.
    if out.is_null() || out == INVALID_HANDLE_VALUE {
        unsafe { AttachConsole(ATTACH_PARENT_PROCESS) };
    }
}

fn ask(words: &[String]) -> anyhow::Result<String> {
    ensure!(!words.is_empty(), "no command\n{USAGE}");
    let file = folder()?.join(FILE);
    let named = match fs::read_to_string(&file) {
        Ok(named) => named,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => bail!(
            "no door at {}: ELYTerminal is not running, or Settings › Terminal › Command line control is off",
            file.display()
        ),
        Err(error) => return Err(error).with_context(|| format!("cannot read {}", file.display())),
    };
    let (port, token) = named
        .trim_end()
        .split_once(' ')
        .with_context(|| format!("{} names no port and token", file.display()))?;
    let port: u16 = port
        .parse()
        .with_context(|| format!("{} names no port", file.display()))?;
    let stream = TcpStream::connect((Ipv4Addr::LOCALHOST, port))
        .with_context(|| format!("nothing listens on port {port}; is ELYTerminal running?"))?;
    stream.set_read_timeout(Some(ANSWERING + PATIENCE))?;
    let request = Request {
        token: token.to_string(),
        words: words.to_vec(),
    };
    write_line(&stream, &request)?;
    match read_line(&stream)? {
        Reply::Ok(text) => Ok(text),
        Reply::Error(why) => bail!("{why}"),
    }
}

#[cfg(test)]
mod tests;
