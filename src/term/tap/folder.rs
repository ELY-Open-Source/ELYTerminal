use std::path::PathBuf;

/// The local path a `file://host/path` names; another host's is none.
pub(in crate::term) fn folder(body: &[u8]) -> Option<PathBuf> {
    folder_on(body, cfg!(windows))
}

/// The folder an OSC 7 body names; on `windows` a drive's or a share's, as Windows names it, WSL's among them.
pub(super) fn folder_on(body: &[u8], windows: bool) -> Option<PathBuf> {
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
    let Some(path) = decode(&rest[slash..]) else {
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

fn decode(bytes: &[u8]) -> Option<String> {
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
    String::from_utf8(out).ok()
}
