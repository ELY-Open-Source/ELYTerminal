//! What a terminal runs that closing it would end, so a close asks first.

use std::{ffi::OsStr, io, path::Path};

use super::{pty::Pty, status::State, view::TerminalView};

/// Shells, which wait at a prompt when they lead.
const SHELLS: [&str; 12] = [
    "sh",
    "bash",
    "zsh",
    "fish",
    "nu",
    "dash",
    "ksh",
    "csh",
    "tcsh",
    "pwsh",
    "powershell",
    "cmd",
];

/// Whether `name`, an executable's path or name, is a shell.
fn shell(name: &OsStr) -> bool {
    let name = Path::new(name)
        .file_stem()
        .map(|stem| stem.to_string_lossy().to_lowercase());
    name.is_some_and(|name| SHELLS.contains(&name.as_str()))
}

/// The program leading the terminal, by name; none while a shell leads, at its prompt.
#[cfg(unix)]
pub(super) fn running(pty: &Pty) -> io::Result<Option<String>> {
    let Some(group) = pty.foreground()? else {
        return Ok(None);
    };
    // Another user's program, under sudo among them, hides its executable.
    let Ok(path) = super::unix::path(group) else {
        return Ok(Some(format!("process {group}")));
    };
    let name = path.file_name().unwrap_or(path.as_os_str());
    // A shell at its prompt holds its group alone; `sh -c` beside its job does not.
    let prompt = group == pty.pid as i32 && shell(name) && super::unix::lone(group)?;
    Ok((!prompt).then(|| name.to_string_lossy().into_owned()))
}

/// The program the terminal runs, or the first its shell started: Windows has no foreground group.
#[cfg(windows)]
pub(super) fn running(pty: &Pty) -> io::Result<Option<String>> {
    let processes = super::procs::processes()?;
    let root = processes
        .iter()
        .find(|(pid, ..)| *pid == pty.pid)
        .ok_or_else(|| io::Error::other(format!("process {} is not listed", pty.pid)))?;
    if !shell(OsStr::new(&root.2)) {
        return Ok(Some(root.2.clone()));
    }
    Ok(processes
        .into_iter()
        .find(|(_, parent, _)| *parent == pty.pid)
        .map(|(_, _, name)| name))
}

impl TerminalView {
    /// What closing it would end: a program past its shell, or one that reports work or waits.
    pub fn busy(&self) -> Option<String> {
        match running(&self.pty) {
            Ok(Some(name)) => return Some(name),
            Ok(None) => {}
            Err(error) => {
                log::warn!(
                    "terminal {}: what it runs does not read: {error}",
                    self.pty.pid
                );
                return Some(format!("process {}", self.pty.pid));
            }
        }
        let (id, record) = self.records.shown()?;
        matches!(record.state, State::Working | State::Blocked)
            .then(|| self.records.app(id).unwrap_or(&self.title).to_string())
    }
}

#[cfg(test)]
mod tests;
