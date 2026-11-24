#[cfg(unix)]
use std::os::unix::ffi::OsStringExt;
use std::path::PathBuf;

use crate::term::PtyEvent;

/// The event a folder report makes, a remote folder's with its host; `last` keeps this host's.
pub(super) fn event(body: &[u8], last: &mut Option<PathBuf>, pid: u32) -> Option<PtyEvent> {
    if let Some(folder) = reported(body) {
        if last.as_ref() == Some(&folder) {
            log::debug!("pty {pid}: folder {} again", folder.display());
        } else {
            log::info!("pty {pid}: folder {}", folder.display());
            *last = Some(folder.clone());
        }
        return Some(PtyEvent::Folder(folder, None));
    }
    let (host, folder) = there(body)?;
    log::debug!("pty {pid}: folder {} on {host}", folder.display());
    *last = None;
    Some(PtyEvent::Folder(folder, Some(host)))
}

/// The host and POSIX path of an OSC 7 that names another host.
fn there(body: &[u8]) -> Option<(String, PathBuf)> {
    let rest = body.strip_prefix(b"7;file://")?;
    let slash = rest.iter().position(|&byte| byte == b'/')?;
    let host = std::str::from_utf8(&rest[..slash]).ok()?;
    let local = gethostname::gethostname();
    let here = host.is_empty()
        || host.eq_ignore_ascii_case("localhost")
        || local
            .to_str()
            .is_some_and(|name| host.eq_ignore_ascii_case(name));
    if here {
        return None;
    }
    let Ok(path) = String::from_utf8(decode(&rest[slash..])) else {
        log::warn!("pty: host {host}'s folder is not UTF-8");
        return None;
    };
    Some((host.to_string(), PathBuf::from(path)))
}

/// The folder OSC 7's URI or ConEmu's OSC 9;9 names.
fn reported(body: &[u8]) -> Option<PathBuf> {
    reported_on(body, cfg!(windows))
}

/// As `reported`, on Windows when `windows`.
fn reported_on(body: &[u8], windows: bool) -> Option<PathBuf> {
    if let Some(uri) = body.strip_prefix(b"7;") {
        return folder_on(uri, windows);
    }
    let raw = body.strip_prefix(b"9;9;")?;
    // ConEmu's path stands as Windows names it, encoding nothing.
    let Ok(text) = std::str::from_utf8(raw) else {
        log::warn!("pty: OSC 9;9's path is not UTF-8");
        return None;
    };
    let text = text
        .strip_prefix('"')
        .and_then(|text| text.strip_suffix('"'))
        .unwrap_or(text);
    let bytes = text.as_bytes();
    let drive = bytes.len() > 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':';
    if windows && (drive || text.starts_with(r"\\")) {
        return Some(PathBuf::from(text));
    }
    log::debug!("pty: OSC 9;9 path {text} names no Windows folder");
    None
}

/// A `file://` URI's local path; Unix names keep their bytes.
pub(crate) fn file_uri(body: &[u8]) -> Option<PathBuf> {
    path_on(body, cfg!(windows), true)
}

/// The folder an OSC 7 body names; on `windows` a drive's or a share's, as Windows names it, WSL's among them.
pub(super) fn folder_on(body: &[u8], windows: bool) -> Option<PathBuf> {
    path_on(body, windows, false)
}

/// The path a body names; `raw` keeps non-UTF-8 Unix names.
fn path_on(
    body: &[u8],
    windows: bool,
    #[cfg_attr(windows, allow(unused_variables))] raw: bool,
) -> Option<PathBuf> {
    let Some(rest) = body.strip_prefix(b"file://") else {
        log::warn!("pty: {} is no file:// URI", String::from_utf8_lossy(body));
        return None;
    };
    let slash = rest.iter().position(|&byte| byte == b'/')?;
    let host = String::from_utf8_lossy(&rest[..slash]);
    let local = gethostname::gethostname();
    if !(host.is_empty()
        || host.eq_ignore_ascii_case("localhost")
        || local
            .to_str()
            .is_some_and(|name| host.eq_ignore_ascii_case(name)))
    {
        log::debug!("pty: a file URI from host {host}, not this one");
        return None;
    }
    let bytes = decode(&rest[slash..]);
    #[cfg(unix)]
    if raw && !windows {
        return Some(PathBuf::from(std::ffi::OsString::from_vec(bytes)));
    }
    let Ok(path) = String::from_utf8(bytes) else {
        log::warn!("pty: a file URI's path is not UTF-8 after decoding");
        return None;
    };
    if !windows {
        return Some(PathBuf::from(path));
    }
    let drive = path.as_bytes().get(1..3).is_some_and(|two| two[1] == b':')
        && path.as_bytes()[1].is_ascii_alphabetic();
    if path.starts_with("//") {
        Some(PathBuf::from(path.replace('/', "\\")))
    } else if drive {
        Some(PathBuf::from(&path[1..]))
    } else {
        log::debug!("pty: path {path} names no Windows file");
        None
    }
}

fn decode(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        let hex = bytes
            .get(at + 1..at + 3)
            .and_then(|two| std::str::from_utf8(two).ok())
            .and_then(|two| u8::from_str_radix(two, 16).ok());
        match (bytes[at], hex) {
            (b'%', Some(value)) => {
                out.push(value);
                at += 3;
            }
            (byte, _) => {
                out.push(byte);
                at += 1;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests;
