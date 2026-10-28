use std::{
    collections::HashMap,
    ffi::{OsStr, OsString},
    fs::{File, OpenOptions},
    io,
    os::windows::{ffi::OsStrExt, fs::OpenOptionsExt},
    path::{Path, PathBuf},
};

use anyhow::Context as _;
use windows_sys::Win32::{
    Foundation::ERROR_PIPE_BUSY, Storage::FileSystem::SECURITY_IDENTIFICATION,
    System::Pipes::WaitNamedPipeW,
};

use super::{Invocation, WAIT, follows};
use crate::term::{
    Pty,
    procs::{launch, processes},
};

pub(super) type Pid = u32;

/// The herdr processes at or under the session's shell, nearest first.
pub(super) type Program = Vec<Pid>;

/// herdr's executable.
const HERDR: &str = "herdr.exe";

/// The session's shell: Windows has no foreground group, so herdr is sought under it.
pub(super) fn leader(pty: &Pty) -> io::Result<Pid> {
    Ok(pty.pid)
}

pub(super) fn running(shell: Pid) -> io::Result<Program> {
    Ok(herdrs(&processes()?, shell))
}

/// The herdr processes at or under `root`, breadth first; what a herdr starts, its server among them, is its own.
pub(super) fn herdrs(processes: &[(Pid, Pid, String)], root: Pid) -> Vec<Pid> {
    let herdr = |pid: Pid| {
        processes
            .iter()
            .any(|(id, _, name)| *id == pid && name.eq_ignore_ascii_case(HERDR))
    };
    let (mut under, mut found, mut at) = (vec![root], Vec::new(), 0);
    while let Some(&pid) = under.get(at) {
        at += 1;
        if herdr(pid) {
            found.push(pid);
            continue;
        }
        for (child, ..) in processes.iter().filter(|(_, of, _)| *of == pid) {
            // A parent id outlives its process, so a stale one may lead back up.
            if !under.contains(child) {
                under.push(*child);
            }
        }
    }
    found
}

/// The socket of the herdr server whose terminal client is among `herdrs`; a server or a command is passed by.
pub(super) fn client(herdrs: Program) -> anyhow::Result<Option<PathBuf>> {
    pick(herdrs.into_iter().map(|pid| (pid, launch(pid))))
}

/// The first terminal client's socket in `reads`; with none, a failed read is the error, so it is tried again.
pub(super) fn pick(
    reads: impl IntoIterator<Item = (Pid, io::Result<Invocation>)>,
) -> anyhow::Result<Option<PathBuf>> {
    let mut unread = None;
    for (pid, read) in reads {
        match read.with_context(|| format!("herdr: process {pid} does not read")) {
            // herdr names its pipe after the socket's path as written, relative or not.
            Ok((args, env)) => {
                if let Some(socket) = follows(&args, &env)? {
                    return Ok(Some(socket));
                }
            }
            Err(error) => {
                unread.get_or_insert(error);
            }
        }
    }
    unread.map_or(Ok(None), Err)
}

/// herdr's folder without `XDG_CONFIG_HOME`, as herdr finds it on Windows.
pub(super) fn home(env: &HashMap<String, OsString>) -> anyhow::Result<PathBuf> {
    if let Some(data) = env.get("APPDATA") {
        return Ok(PathBuf::from(data).join("herdr"));
    }
    if let Some(profile) = env.get("USERPROFILE") {
        return Ok(PathBuf::from(profile).join(r"AppData\Roaming\herdr"));
    }
    let home = env
        .get("HOME")
        .context("herdr: its client has no APPDATA, USERPROFILE or HOME")?;
    Ok(PathBuf::from(home).join(".config/herdr"))
}

/// The pipe herdr names after its socket's path; a busy one is waited for, as herdr's client waits.
pub(super) fn connect(socket: &Path) -> io::Result<File> {
    let pipe = format!(r"\\.\pipe\{}", socket.to_string_lossy());
    // Identification only: the pipe's server may not act as this user.
    let open = || {
        OpenOptions::new()
            .read(true)
            .write(true)
            .security_qos_flags(SECURITY_IDENTIFICATION)
            .open(&pipe)
    };
    match open() {
        Err(error) if error.raw_os_error() == Some(ERROR_PIPE_BUSY as i32) => {
            let name: Vec<u16> = OsStr::new(&pipe).encode_wide().chain([0]).collect();
            // SAFETY: `name` ends with a NUL.
            if unsafe { WaitNamedPipeW(name.as_ptr(), WAIT.as_millis() as u32) } == 0 {
                return Err(io::Error::last_os_error());
            }
            open()
        }
        opened => opened,
    }
}
