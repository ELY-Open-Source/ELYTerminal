//! Types as gpui's clipboard holds them: text, an image, files.

use std::{
    path::{Path, PathBuf},
    rc::Rc,
};

use gpui::{ClipboardEntry, ClipboardItem, ClipboardString, ExternalPaths, Image, ImageFormat};

use super::Fail;
use crate::term::tap::folder::file_uri;

pub(in crate::term) const TEXT: &str = "text/plain";
pub(in crate::term) const FILES: &str = "text/uri-list";

fn is_text(mime: &str) -> bool {
    mime == TEXT || mime.starts_with("text/plain;")
}

/// A write as gpui keeps it: first text, image, files.
pub fn item(parts: &[(String, Rc<Vec<u8>>)]) -> Result<ClipboardItem, Fail> {
    let mut entries: Vec<ClipboardEntry> = Vec::new();
    let held =
        |entries: &[ClipboardEntry], kind: fn(&ClipboardEntry) -> bool| entries.iter().any(kind);
    for (mime, data) in parts {
        if is_text(mime) && !held(&entries, |it| matches!(it, ClipboardEntry::String(_))) {
            let Ok(text) = String::from_utf8(data.to_vec()) else {
                log::warn!("clipboard: {mime} is not UTF-8");
                return Err(Fail::Invalid);
            };
            entries.push(ClipboardEntry::String(ClipboardString::new(text)));
        } else if let Some(format) = ImageFormat::from_mime_type(mime)
            && !held(&entries, |it| matches!(it, ClipboardEntry::Image(_)))
        {
            entries.push(ClipboardEntry::Image(Image::from_bytes(
                format,
                data.to_vec(),
            )));
        } else if mime == FILES
            && !held(&entries, |it| {
                matches!(it, ClipboardEntry::ExternalPaths(_))
            })
        {
            let paths = paths(data);
            if !paths.is_empty() {
                entries.push(ClipboardEntry::ExternalPaths(ExternalPaths(paths.into())));
            }
        } else {
            log::debug!("clipboard: {mime} passed by; text, an image and files are kept");
        }
    }
    if entries.is_empty() {
        log::warn!("clipboard: a write holds no text, image or file");
        return Err(Fail::Invalid);
    }
    Ok(ClipboardItem { entries })
}

/// Whether the read-back clipboard holds the written text, files, image.
pub fn holds(read: Option<&ClipboardItem>, written: &ClipboardItem) -> bool {
    let Some(read) = read else {
        return false;
    };
    written.entries.iter().all(|entry| match entry {
        // A system may convert it; the same format must match.
        ClipboardEntry::Image(image) => read.entries.iter().any(|kept| match kept {
            ClipboardEntry::Image(kept) => kept.format != image.format || kept.bytes == image.bytes,
            _ => false,
        }),
        entry => read.entries.contains(entry),
    })
}

/// Which entry `mime` reads: text, an image or files.
pub(super) fn reads(mime: &str) -> Option<&'static str> {
    if is_text(mime) {
        Some("text")
    } else if ImageFormat::from_mime_type(mime).is_some() {
        Some("image")
    } else {
        (mime == FILES).then_some("files")
    }
}

/// The types `item` offers.
pub fn types(item: &ClipboardItem) -> Vec<String> {
    let mut types: Vec<String> = Vec::new();
    for entry in &item.entries {
        let mime = match entry {
            ClipboardEntry::String(_) => TEXT,
            ClipboardEntry::Image(image) => image.format.mime_type(),
            ClipboardEntry::ExternalPaths(_) => FILES,
        };
        if !types.iter().any(|kept| kept == mime) {
            types.push(mime.to_string());
        }
    }
    types
}

/// `item`'s data for `mime`, if it holds that type.
pub fn data(item: &ClipboardItem, mime: &str) -> Option<Vec<u8>> {
    item.entries.iter().find_map(|entry| match entry {
        ClipboardEntry::String(text) if is_text(mime) => Some(text.text().clone().into_bytes()),
        ClipboardEntry::Image(image) if image.format.mime_type() == mime => {
            Some(image.bytes.clone())
        }
        ClipboardEntry::ExternalPaths(paths) if mime == FILES => {
            Some(uris(paths.paths()).into_bytes())
        }
        _ => None,
    })
}

/// A `text/uri-list`'s local files; comments and other hosts' pass by.
fn paths(data: &[u8]) -> Vec<PathBuf> {
    data.split(|&byte| byte == b'\n')
        .map(|line| line.strip_suffix(b"\r").unwrap_or(line))
        .filter(|line| !line.is_empty() && !line.starts_with(b"#"))
        .filter_map(file_uri)
        .collect()
}

/// Files as a `text/uri-list`, each line ended by CR LF.
pub(super) fn uris(paths: &[PathBuf]) -> String {
    paths
        .iter()
        .filter_map(|path| uri(path))
        .fold(String::new(), |list, uri| list + &uri + "\r\n")
}

/// A local path as `file://` names it.
fn uri(path: &Path) -> Option<String> {
    #[cfg(unix)]
    let bytes = std::os::unix::ffi::OsStrExt::as_bytes(path.as_os_str()).to_vec();
    #[cfg(windows)]
    let bytes = {
        let Some(text) = path.to_str() else {
            log::warn!("clipboard: {} is not Unicode", path.display());
            return None;
        };
        let text = text.replace('\\', "/");
        // A drive's path gains a slash; a share's keeps two.
        let text = if text.starts_with("//") {
            text
        } else {
            format!("/{text}")
        };
        text.into_bytes()
    };
    let mut uri = String::from("file://");
    for byte in bytes {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b'/' | b':' => {
                uri.push(char::from(byte))
            }
            byte => uri.push_str(&format!("%{byte:02X}")),
        }
    }
    Some(uri)
}
