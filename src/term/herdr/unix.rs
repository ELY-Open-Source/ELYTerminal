use std::{
    collections::HashMap,
    ffi::OsString,
    io,
    os::unix::net::UnixStream,
    path::{Path, PathBuf},
};

use anyhow::Context as _;

use super::{WAIT, follows};
use crate::term::{
    Pty,
    unix::{launch, path},
};

pub(super) type Pid = i32;

/// The foreground's group and its leader's executable: an exec keeps the group and swaps the program.
pub(super) type Program = (Pid, Option<PathBuf>);

/// herdr's executable.
const HERDR: &str = "herdr";

/// The process group leading the session's foreground.
pub(super) fn leader(pty: &Pty) -> io::Result<Pid> {
    pty.foreground()?
        .ok_or_else(|| io::Error::other("no process group holds the terminal"))
}

pub(super) fn running(group: Pid) -> io::Result<Program> {
    Ok((group, path(group).ok()))
}

/// The socket of the herdr server whose terminal client leads process group `group`, if it is one.
pub(super) fn client((group, exe): Program) -> anyhow::Result<Option<PathBuf>> {
    let Some(exe) = exe else {
        log::debug!("herdr: process {group}, another user's or gone, does not read");
        return Ok(None);
    };
    if !exe
        .file_name()
        .is_some_and(|name| name.eq_ignore_ascii_case(HERDR))
    {
        return Ok(None);
    }
    let (args, env) =
        launch(group).with_context(|| format!("herdr: process {group} does not read"))?;
    let Some(socket) = follows(&args, &env)? else {
        return Ok(None);
    };
    if socket.is_absolute() {
        return Ok(Some(socket));
    }
    // herdr reads a relative socket from its client's folder.
    let folder =
        cwd(group).with_context(|| format!("herdr: process {group}'s folder does not read"))?;
    Ok(Some(folder.join(socket)))
}

/// herdr's folder without `XDG_CONFIG_HOME`.
pub(super) fn home(env: &HashMap<String, OsString>) -> anyhow::Result<PathBuf> {
    let home = env.get("HOME").context("herdr: its client has no HOME")?;
    Ok(PathBuf::from(home).join(".config/herdr"))
}

/// herdr's socket, which must answer within `WAIT`.
pub(super) fn connect(socket: &Path) -> io::Result<UnixStream> {
    let stream = UnixStream::connect(socket)?;
    stream.set_read_timeout(Some(WAIT))?;
    stream.set_write_timeout(Some(WAIT))?;
    Ok(stream)
}

#[cfg(target_os = "linux")]
pub(super) fn cwd(pid: Pid) -> io::Result<PathBuf> {
    std::fs::read_link(format!("/proc/{pid}/cwd"))
}

#[cfg(target_os = "macos")]
pub(super) fn cwd(pid: Pid) -> io::Result<PathBuf> {
    use std::os::unix::ffi::OsStringExt;
    // SAFETY: the struct is plain C data, valid zeroed.
    let mut info: libc::proc_vnodepathinfo = unsafe { std::mem::zeroed() };
    let size = std::mem::size_of::<libc::proc_vnodepathinfo>() as libc::c_int;
    // SAFETY: `info` holds the size given.
    let read = unsafe {
        libc::proc_pidinfo(
            pid,
            libc::PROC_PIDVNODEPATHINFO,
            0,
            (&raw mut info).cast(),
            size,
        )
    };
    if read <= 0 {
        return Err(io::Error::last_os_error());
    }
    if read != size {
        return Err(io::Error::other(format!(
            "proc_pidinfo read {read} of {size} bytes"
        )));
    }
    let path = info
        .pvi_cdir
        .vip_path
        .iter()
        .flatten()
        .map(|byte| *byte as u8);
    Ok(PathBuf::from(OsString::from_vec(
        path.take_while(|byte| *byte != 0).collect(),
    )))
}
