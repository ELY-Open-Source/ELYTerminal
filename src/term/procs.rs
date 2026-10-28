use std::{
    collections::HashMap,
    ffi::{OsStr, OsString},
    io,
    os::windows::{
        ffi::{OsStrExt, OsStringExt},
        io::{AsRawHandle, FromRawHandle, OwnedHandle},
    },
};

use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};
use windows_sys::Win32::{
    Foundation::{ERROR_NO_MORE_FILES, INVALID_HANDLE_VALUE},
    System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW,
        TH32CS_SNAPPROCESS,
    },
};

/// A program's arguments, and its variables as the system holds them.
pub(crate) type Invocation = (Vec<String>, HashMap<String, OsString>);

/// Every process: its id, its parent's, and its executable's name.
pub(crate) fn processes() -> io::Result<Vec<(u32, u32, String)>> {
    // SAFETY: a plain call; its handle is owned below.
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if snapshot == INVALID_HANDLE_VALUE {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: the handle is new and ours.
    let snapshot = unsafe { OwnedHandle::from_raw_handle(snapshot) };
    let mut entry = PROCESSENTRY32W {
        dwSize: size_of::<PROCESSENTRY32W>() as u32,
        ..Default::default()
    };
    let mut processes = Vec::new();
    // SAFETY: `entry` holds the size it names.
    let mut read = unsafe { Process32FirstW(snapshot.as_raw_handle(), &mut entry) };
    while read != 0 {
        let name = entry.szExeFile.split(|unit| *unit == 0).next();
        processes.push((
            entry.th32ProcessID,
            entry.th32ParentProcessID,
            String::from_utf16_lossy(name.unwrap_or_default()),
        ));
        // SAFETY: as above.
        read = unsafe { Process32NextW(snapshot.as_raw_handle(), &mut entry) };
    }
    let error = io::Error::last_os_error();
    // The list ends with ERROR_NO_MORE_FILES.
    if error.raw_os_error() == Some(ERROR_NO_MORE_FILES as i32) {
        Ok(processes)
    } else {
        Err(error)
    }
}

/// The arguments and variables of process `pid`; names fold to upper case, as Windows matches them.
pub(crate) fn launch(pid: u32) -> io::Result<Invocation> {
    let id = sysinfo::Pid::from_u32(pid);
    let mut system = System::new();
    let read = ProcessRefreshKind::nothing()
        .with_cmd(UpdateKind::Always)
        .with_environ(UpdateKind::Always);
    system.refresh_processes_specifics(ProcessesToUpdate::Some(&[id]), true, read);
    let process = system
        .process(id)
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "the process is gone"))?;
    // sysinfo leaves a list empty when it does not read; every process has both.
    if process.cmd().is_empty() || process.environ().is_empty() {
        return Err(io::Error::other(
            "its command line or variables do not read",
        ));
    }
    let args = process
        .cmd()
        .iter()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect();
    let env = process
        .environ()
        .iter()
        .filter_map(|pair| variable(pair))
        .collect();
    Ok((args, env))
}

/// A `NAME=value` variable, its name in upper case; its value keeps its units.
fn variable(pair: &OsStr) -> Option<(String, OsString)> {
    let pair: Vec<u16> = pair.encode_wide().collect();
    let at = pair.iter().position(|unit| *unit == u16::from(b'='))?;
    Some((
        String::from_utf16_lossy(&pair[..at]).to_ascii_uppercase(),
        OsString::from_wide(&pair[at + 1..]),
    ))
}

/// When process `pid` started, in 100 ns ticks; none if hidden.
pub(crate) fn started(pid: u32) -> Option<u64> {
    use windows_sys::Win32::System::Threading::{
        GetProcessTimes, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
    };
    // SAFETY: a plain call; a handle it gives is owned below.
    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    if handle.is_null() {
        return None;
    }
    // SAFETY: the handle is new and ours.
    let handle = unsafe { OwnedHandle::from_raw_handle(handle) };
    // SAFETY: plain C data, valid zeroed.
    let mut times: [windows_sys::Win32::Foundation::FILETIME; 4] = unsafe { std::mem::zeroed() };
    let [created, exited, kernel, user] = &mut times;
    // SAFETY: each pointer is to a FILETIME of ours.
    let read = unsafe { GetProcessTimes(handle.as_raw_handle(), created, exited, kernel, user) };
    (read != 0)
        .then(|| u64::from(times[0].dwHighDateTime) << 32 | u64::from(times[0].dwLowDateTime))
}
