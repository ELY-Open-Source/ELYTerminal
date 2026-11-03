//! The bundled Nerd Font, unpacked once from the release's own archive.

use std::{borrow::Cow, io::Write, sync::OnceLock};

/// Nerd Fonts' release faces, the eight weights `WEIGHTS` names, in one archive.
const ARCHIVE: &[u8] = include_bytes!("../../../assets/fonts/JetBrainsMonoNerdFontMono.tar.xz");

/// The faces the terminal registers, the eight weights and their italics.
pub const WEIGHTS: [&str; 16] = [
    "JetBrainsMonoNerdFontMono-Thin.ttf",
    "JetBrainsMonoNerdFontMono-ThinItalic.ttf",
    "JetBrainsMonoNerdFontMono-ExtraLight.ttf",
    "JetBrainsMonoNerdFontMono-ExtraLightItalic.ttf",
    "JetBrainsMonoNerdFontMono-Light.ttf",
    "JetBrainsMonoNerdFontMono-LightItalic.ttf",
    "JetBrainsMonoNerdFontMono-Regular.ttf",
    "JetBrainsMonoNerdFontMono-Italic.ttf",
    "JetBrainsMonoNerdFontMono-Medium.ttf",
    "JetBrainsMonoNerdFontMono-MediumItalic.ttf",
    "JetBrainsMonoNerdFontMono-SemiBold.ttf",
    "JetBrainsMonoNerdFontMono-SemiBoldItalic.ttf",
    "JetBrainsMonoNerdFontMono-Bold.ttf",
    "JetBrainsMonoNerdFontMono-BoldItalic.ttf",
    "JetBrainsMonoNerdFontMono-ExtraBold.ttf",
    "JetBrainsMonoNerdFontMono-ExtraBoldItalic.ttf",
];

/// The archive's fonts, read once and kept for the run.
static FONTS: OnceLock<Result<Vec<Vec<u8>>, String>> = OnceLock::new();

/// Every face of the bundled Nerd Font; unpacked at the first run, then read from disk.
pub fn fonts() -> anyhow::Result<&'static [Vec<u8>]> {
    match FONTS.get_or_init(load) {
        Ok(fonts) => Ok(fonts),
        Err(error) => anyhow::bail!("fonts: {error}"),
    }
}

/// The faces as gpui takes them, borrowed for the app's life.
pub fn registered() -> anyhow::Result<Vec<Cow<'static, [u8]>>> {
    Ok(fonts()?
        .iter()
        .map(|font| Cow::Borrowed(font.as_slice()))
        .collect())
}

/// Where the unpacked faces live, so only the first run pays for the archive.
fn cache_folder() -> Option<std::path::PathBuf> {
    Some(dirs::data_local_dir()?.join("ELYTerminal").join("fonts"))
}

/// Reads the faces from the cache, unpacking the archive there on the first run.
fn load() -> Result<Vec<Vec<u8>>, String> {
    let folder = cache_folder();
    if let Some(folder) = &folder
        && let Some(fonts) = read_cache(folder)
    {
        log::info!(
            "fonts: {} faces read from {}",
            fonts.len(),
            folder.display()
        );
        return Ok(fonts);
    }
    let started = std::time::Instant::now();
    let fonts = unpack()?;
    log::info!(
        "fonts: {} faces unpacked in {} ms",
        fonts.len(),
        started.elapsed().as_millis()
    );
    if let Some(folder) = &folder
        && let Err(error) = write_cache(folder, &fonts)
    {
        log::warn!("fonts: {} stays unwritten: {error}", folder.display());
    }
    Ok(fonts)
}

/// The cached faces, none when a face is missing or the folder holds another set.
fn read_cache(folder: &std::path::Path) -> Option<Vec<Vec<u8>>> {
    let mut fonts = Vec::with_capacity(WEIGHTS.len());
    for name in WEIGHTS {
        let face = std::fs::read(folder.join(name)).ok()?;
        if face.len() < 1_000_000 {
            return None;
        }
        fonts.push(face);
    }
    Some(fonts)
}

/// Writes each face under the name `WEIGHTS` gives it, a temp file renamed in place.
fn write_cache(folder: &std::path::Path, fonts: &[Vec<u8>]) -> std::io::Result<()> {
    std::fs::create_dir_all(folder)?;
    for (name, font) in WEIGHTS.iter().zip(fonts) {
        let path = folder.join(name);
        if std::fs::metadata(&path).is_ok_and(|meta| meta.len() == font.len() as u64) {
            continue;
        }
        let temp = path.with_extension("tmp");
        std::fs::write(&temp, font)?;
        std::fs::rename(&temp, &path)?;
    }
    Ok(())
}

/// Unpacks the tar as it streams, keeping only the faces `WEIGHTS` names.
///
/// The archive holds 243 MB of faces; walking it a header at a time means the
/// run peaks at those eight, not at the whole tar.
fn unpack() -> Result<Vec<Vec<u8>>, String> {
    let mut picker = Picker::new();
    lzma_rs::xz_decompress(&mut std::io::Cursor::new(ARCHIVE), &mut picker)
        .map_err(|error| format!("the archive does not unpack: {error}"))?;
    picker.finish()
}

/// What a run of tar bytes means: a header filling, a body passing, padding, or its end.
enum Part {
    Header,
    Body { size: usize, keep: Option<usize> },
    Pad(usize),
    Done,
}

/// Keeps the faces as their parts of the tar stream in, dropping the rest.
struct Picker {
    header: Vec<u8>,
    part: Part,
    at: usize,
    fonts: Vec<Option<Vec<u8>>>,
}

impl Picker {
    fn new() -> Self {
        Self {
            header: Vec::with_capacity(512),
            part: Part::Header,
            at: 0,
            fonts: vec![None; WEIGHTS.len()],
        }
    }

    fn finish(self) -> Result<Vec<Vec<u8>>, String> {
        let missing: Vec<&str> = WEIGHTS
            .iter()
            .zip(&self.fonts)
            .filter(|(_, font)| font.is_none())
            .map(|(name, _)| *name)
            .collect();
        if !missing.is_empty() {
            return Err(format!("the archive lacks {}", missing.join(", ")));
        }
        Ok(self.fonts.into_iter().flatten().collect())
    }

    /// Reads the header just filled and names the part after it.
    fn header(&mut self) -> Result<(), String> {
        let header = std::mem::take(&mut self.header);
        self.header = Vec::with_capacity(512);
        if header.iter().all(|byte| *byte == 0) {
            self.part = Part::Done;
            return Ok(());
        }
        let field = |start: usize, len: usize| -> String {
            let bytes = &header[start..start + len];
            let end = bytes
                .iter()
                .position(|byte| *byte == 0)
                .unwrap_or(bytes.len());
            String::from_utf8_lossy(&bytes[..end]).into_owned()
        };
        let text = field(124, 12);
        let text = text.trim_matches(|c: char| c == '\0' || c == ' ');
        let size = usize::from_str_radix(text, 8)
            .map_err(|_| format!("a tar size {text:?} is not octal"))?;
        let name = field(0, 100);
        let prefix = field(345, 155);
        let name = if prefix.is_empty() {
            name
        } else {
            format!("{prefix}/{name}")
        };
        let keep = WEIGHTS.iter().position(|want| *want == name);
        if let Some(index) = keep {
            self.fonts[index] = Some(Vec::with_capacity(size));
        }
        self.at = 0;
        self.part = Part::Body { size, keep };
        Ok(())
    }
}

impl Write for Picker {
    fn write(&mut self, mut bytes: &[u8]) -> std::io::Result<usize> {
        let taken = bytes.len();
        while !bytes.is_empty() {
            match self.part {
                Part::Done => break,
                Part::Header => {
                    let room = 512 - self.header.len();
                    let (head, rest) = bytes.split_at(room.min(bytes.len()));
                    self.header.extend_from_slice(head);
                    bytes = rest;
                    if self.header.len() == 512 {
                        self.header().map_err(invalid)?;
                    }
                }
                Part::Body { size, keep } => {
                    let (body, rest) = bytes.split_at((size - self.at).min(bytes.len()));
                    if let Some(font) = keep.and_then(|index| self.fonts[index].as_mut()) {
                        font.extend_from_slice(body);
                    }
                    self.at += body.len();
                    bytes = rest;
                    if self.at == size {
                        // The padding rounds the body up to the next 512 bytes.
                        self.part = Part::Pad(size.next_multiple_of(512) - size);
                        self.at = 0;
                    }
                }
                Part::Pad(left) => {
                    let (_, rest) = bytes.split_at(left.min(bytes.len()));
                    self.at += left.min(bytes.len());
                    bytes = rest;
                    if self.at >= left {
                        self.part = Part::Header;
                        self.at = 0;
                    }
                }
            }
        }
        Ok(taken)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn invalid(error: String) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, error)
}

#[cfg(test)]
mod tests;
