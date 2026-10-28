use std::{collections::HashMap, ffi::OsString, io, os::unix::ffi::OsStringExt, path::PathBuf};

/// A program's arguments and variables, as the system holds them.
type Invocation = (Vec<String>, HashMap<String, OsString>);

/// The executable process `pid` runs.
#[cfg(target_os = "linux")]
pub(crate) fn path(pid: i32) -> io::Result<PathBuf> {
    std::fs::read_link(format!("/proc/{pid}/exe")).map(installed)
}

/// `path` without the ` (deleted)` an update leaves.
#[cfg(target_os = "linux")]
pub(crate) fn installed(path: PathBuf) -> PathBuf {
    use std::os::unix::ffi::OsStrExt;
    match path.as_os_str().as_bytes().strip_suffix(b" (deleted)") {
        Some(kept) => PathBuf::from(std::ffi::OsStr::from_bytes(kept)),
        None => path,
    }
}

#[cfg(target_os = "macos")]
pub(crate) fn path(pid: i32) -> io::Result<PathBuf> {
    let mut buffer = vec![0u8; libc::PROC_PIDPATHINFO_MAXSIZE as usize];
    // SAFETY: the buffer holds the size given.
    let length =
        unsafe { libc::proc_pidpath(pid, buffer.as_mut_ptr().cast(), buffer.len() as u32) };
    if length <= 0 {
        return Err(io::Error::last_os_error());
    }
    buffer.truncate(length as usize);
    Ok(PathBuf::from(OsString::from_vec(buffer)))
}

/// Whether process group `group` holds its leader alone.
#[cfg(target_os = "macos")]
pub(crate) fn lone(group: i32) -> io::Result<bool> {
    let mut pids = [0 as libc::c_int; 2];
    let room = std::mem::size_of_val(&pids) as libc::c_int;
    // SAFETY: `pids` holds `room` bytes; libproc counts pids, not bytes.
    let count = unsafe { libc::proc_listpgrppids(group, pids.as_mut_ptr().cast(), room) };
    if count < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(count == 1 && pids[0] == group)
}

#[cfg(target_os = "linux")]
pub(crate) fn lone(group: i32) -> io::Result<bool> {
    let mut members = Vec::new();
    for entry in std::fs::read_dir("/proc")? {
        let name = entry?.file_name();
        let Some(pid) = name.to_str().and_then(|name| name.parse().ok()) else {
            continue;
        };
        // SAFETY: getpgid reads a group; hidepid guards only /proc.
        match unsafe { libc::getpgid(pid) } {
            -1 => {
                let error = io::Error::last_os_error();
                // It ended while the list was read.
                if error.raw_os_error() != Some(libc::ESRCH) {
                    return Err(error);
                }
            }
            found if found == group => {
                members.push(pid);
                if members.len() > 1 {
                    return Ok(false);
                }
            }
            _ => {}
        }
    }
    Ok(members == [group])
}

/// Process `pid`'s arguments and variables.
#[cfg(target_os = "linux")]
pub(crate) fn launch(pid: i32) -> io::Result<Invocation> {
    let read = |name: &str| std::fs::read(format!("/proc/{pid}/{name}"));
    let (args, env) = (read("cmdline")?, read("environ")?);
    let args = strings(&args).map(lossy).collect();
    let env = strings(&env).filter_map(variable).collect();
    Ok((args, env))
}

/// Each NUL-ended string in `bytes`.
#[cfg(target_os = "linux")]
fn strings(bytes: &[u8]) -> impl Iterator<Item = &[u8]> {
    bytes
        .strip_suffix(b"\0")
        .unwrap_or(bytes)
        .split(|byte| *byte == 0)
}

#[cfg(target_os = "macos")]
pub(crate) fn launch(pid: i32) -> io::Result<Invocation> {
    let mut name = [libc::CTL_KERN, libc::KERN_PROCARGS2, pid];
    let mut ask = |buffer: &mut Vec<u8>| {
        let mut size = buffer.len();
        let into = if buffer.is_empty() {
            std::ptr::null_mut()
        } else {
            buffer.as_mut_ptr().cast()
        };
        // SAFETY: `into` holds `size` bytes, or null asks the size.
        let done = unsafe {
            libc::sysctl(
                name.as_mut_ptr(),
                3,
                into,
                &mut size,
                std::ptr::null_mut(),
                0,
            )
        };
        if done != 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(size)
    };
    let mut buffer = Vec::new();
    let size = ask(&mut buffer)?;
    buffer.resize(size, 0);
    let size = ask(&mut buffer)?;
    buffer.truncate(size);
    procargs(&buffer)
}

/// `KERN_PROCARGS2`: argc, path, NUL padding, arguments, then variables.
#[cfg(target_os = "macos")]
pub(crate) fn procargs(buffer: &[u8]) -> io::Result<Invocation> {
    let invalid =
        |what: &str| io::Error::new(io::ErrorKind::InvalidData, format!("KERN_PROCARGS2 {what}"));
    let (count, rest) = buffer
        .split_first_chunk::<4>()
        .ok_or_else(|| invalid("holds no argument count"))?;
    let count = usize::try_from(i32::from_ne_bytes(*count))
        .map_err(|_| invalid("holds a negative argument count"))?;
    let mut parts = rest
        .split(|byte| *byte == 0)
        .skip(1)
        .skip_while(|part| part.is_empty());
    let args = parts.by_ref().take(count).map(lossy).collect();
    let env = parts
        .take_while(|part| !part.is_empty())
        .filter_map(variable)
        .collect();
    Ok((args, env))
}

fn lossy(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// A `NAME=value` variable; its value keeps its bytes.
fn variable(pair: &[u8]) -> Option<(String, OsString)> {
    let at = pair.iter().position(|byte| *byte == b'=')?;
    Some((
        lossy(&pair[..at]),
        OsString::from_vec(pair[at + 1..].to_vec()),
    ))
}

/// Process `pid`'s arguments alone, which another user's may hide.
#[cfg(target_os = "linux")]
pub(crate) fn args(pid: i32) -> io::Result<Vec<String>> {
    let read = std::fs::read(format!("/proc/{pid}/cmdline"))?;
    Ok(strings(&read).map(lossy).collect())
}

#[cfg(target_os = "macos")]
pub(crate) fn args(pid: i32) -> io::Result<Vec<String>> {
    launch(pid).map(|(args, _)| args)
}

/// The name process `pid` runs, where its executable does not read.
#[cfg(target_os = "linux")]
pub(crate) fn name(pid: i32) -> io::Result<PathBuf> {
    let name = std::fs::read_to_string(format!("/proc/{pid}/comm"))?;
    Ok(PathBuf::from(name.trim_end()))
}

#[cfg(target_os = "macos")]
pub(crate) fn name(pid: i32) -> io::Result<PathBuf> {
    let mut buffer = vec![0u8; 256];
    // SAFETY: the buffer holds the size given.
    let length = unsafe { libc::proc_name(pid, buffer.as_mut_ptr().cast(), buffer.len() as u32) };
    if length <= 0 {
        return Err(io::Error::last_os_error());
    }
    buffer.truncate(length as usize);
    Ok(PathBuf::from(OsString::from_vec(buffer)))
}

/// The foreground group of a pty under `leader`, another than its own.
#[cfg(target_os = "linux")]
pub(crate) fn inner(leader: i32) -> io::Result<Option<i32>> {
    // Fields after the name: state, parent, group, session, tty, foreground.
    let stat = |pid: i32| -> io::Result<Vec<i64>> {
        let stat = std::fs::read(format!("/proc/{pid}/stat"))?;
        let end = stat.iter().rposition(|&byte| byte == b')').unwrap_or(0);
        Ok(String::from_utf8_lossy(&stat[end + 1..])
            .split_whitespace()
            .skip(1)
            .take(5)
            .filter_map(|field| field.parse().ok())
            .collect())
    };
    let tty = stat(leader)?.get(3).copied();
    let mut under = vec![leader];
    let mut at = 0;
    let all: Vec<(i32, Vec<i64>)> = std::fs::read_dir("/proc")?
        .filter_map(|entry| entry.ok()?.file_name().to_str()?.parse().ok())
        .filter_map(|pid: i32| Some((pid, stat(pid).ok()?)))
        .collect();
    while let Some(&pid) = under.get(at) {
        at += 1;
        for (child, fields) in &all {
            if fields.first() == Some(&i64::from(pid)) && !under.contains(child) {
                under.push(*child);
                if fields.get(3).copied() != tty && fields.get(4).is_some_and(|&group| group > 0) {
                    return Ok(fields.get(4).and_then(|&group| i32::try_from(group).ok()));
                }
            }
        }
    }
    Ok(None)
}

#[cfg(target_os = "macos")]
pub(crate) fn inner(leader: i32) -> io::Result<Option<i32>> {
    // ps reads another user's process; proc_pidinfo does not.
    let out = std::process::Command::new("/bin/ps")
        .args(["-A", "-o", "pid=,ppid=,tpgid=,tt="])
        .output()?;
    if !out.status.success() {
        return Err(io::Error::other(format!("ps failed: {}", out.status)));
    }
    let rows: Vec<(i32, i32, i32, String)> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let mut number = || fields.next()?.parse().ok();
            let (pid, ppid, tpgid) = (number()?, number()?, number()?);
            Some((pid, ppid, tpgid, fields.next()?.to_string()))
        })
        .collect();
    let tty = rows
        .iter()
        .find(|row| row.0 == leader)
        .map(|row| row.3.clone())
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "the relay is gone"))?;
    let (mut under, mut at) = (vec![leader], 0);
    while let Some(&pid) = under.get(at) {
        at += 1;
        for (child, _, tpgid, tt) in rows.iter().filter(|row| row.1 == pid) {
            if !under.contains(child) {
                under.push(*child);
                if *tt != tty && *tt != "??" && *tpgid > 0 {
                    return Ok(Some(*tpgid));
                }
            }
        }
    }
    Ok(None)
}
