use std::io::Read;

use super::{
    Error, Ids,
    command::{Command, Quiet},
    media,
    store::Image,
};

/// kitty's and Ghostty's bound on a side.
const SIDE: u32 = 10_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Format {
    Rgb,
    Rgba,
    Png,
}

/// An image whose data still arrives, as first commanded.
#[derive(Debug)]
pub(super) struct Loading {
    pub ids: Ids,
    pub quiet: Quiet,
    format: Format,
    compressed: bool,
    width: u32,
    height: u32,
    transient: bool,
    /// Read from a file or shared memory: failures tell nothing.
    local: bool,
    /// Where `a=T` shows it once whole.
    pub display: Option<super::place::Display>,
    /// What `a=f` asks of its frame.
    pub frame: Option<super::frames::Ask>,
    data: Vec<u8>,
}

impl Loading {
    /// A transmission's first command, its data read from its medium.
    pub fn start(command: &Command, ids: Ids, pid: u32) -> Result<Self, Error> {
        let format = match command.int(b'f') {
            24 => Format::Rgb,
            0 | 32 => Format::Rgba,
            100 => Format::Png,
            other => {
                log::warn!("graphics {pid}: format {other} is none of 24, 32 and 100");
                return Err(Error::Format);
            }
        };
        let compressed = match command.letter(b'o') {
            None => false,
            Some(b'z') => true,
            Some(other) => {
                log::warn!("graphics {pid}: compression {} is no zlib", other as char);
                return Err(Error::Invalid);
            }
        };
        let (width, height) = (command.int(b's'), command.int(b'v'));
        let mut loading = Self {
            ids,
            quiet: command.quiet,
            format,
            compressed,
            width,
            height,
            transient: command.int(b'N') & 1 == 1,
            local: true,
            display: None,
            frame: None,
            data: Vec::new(),
        };
        let (offset, size) = (command.int(b'O'), command.int(b'S'));
        let read = match command.letter(b't').unwrap_or(b'd') {
            b'd' => {
                loading.local = false;
                return loading.add(&command.payload, pid).map(|()| loading);
            }
            b'f' => media::file(&command.payload, false, offset, size),
            b't' => media::file(&command.payload, true, offset, size),
            b's' => media::shared(&command.payload, offset, size, loading.raw()),
            other => {
                log::warn!(
                    "graphics {pid}: medium {} is none of d, f, t and s",
                    other as char
                );
                return Err(Error::Medium);
            }
        };
        // One answer for every failure, as the spec asks.
        loading.data = read.map_err(|reason| {
            log::warn!("graphics {pid}: no image data: {reason}");
            Error::Read
        })?;
        Ok(loading)
    }

    /// The bytes raw pixels take; none for PNG or zlib.
    fn raw(&self) -> Option<usize> {
        let depth = match self.format {
            Format::Rgb => 3,
            Format::Rgba => 4,
            Format::Png => return None,
        };
        let fits = self.width <= SIDE && self.height <= SIDE && !self.compressed;
        fits.then(|| self.width as usize * self.height as usize * depth)
    }

    pub fn local(&self) -> bool {
        self.local
    }

    /// A chunk of a direct transmission.
    pub fn add(&mut self, chunk: &[u8], pid: u32) -> Result<(), Error> {
        if self.data.len() + chunk.len() > media::MAX {
            log::warn!(
                "graphics {pid}: image {} passes {} bytes",
                self.ids.id,
                media::MAX
            );
            return Err(Error::Invalid);
        }
        self.data.extend_from_slice(chunk);
        Ok(())
    }

    /// The image, its data checked against its size, in RGBA.
    pub fn finish(self, pid: u32) -> Result<Image, Error> {
        let local = self.local;
        self.decode(pid).map_err(|error| match local {
            true => Error::Read,
            false => error,
        })
    }

    fn decode(self, pid: u32) -> Result<Image, Error> {
        let Self {
            format,
            compressed,
            mut width,
            mut height,
            transient,
            mut data,
            ..
        } = self;
        if compressed {
            data = inflate(&data).map_err(|reason| {
                log::warn!("graphics {pid}: zlib: {reason}");
                Error::Decompress
            })?;
        }
        if format == Format::Png {
            let png = png(&data).map_err(|reason| {
                log::warn!("graphics {pid}: PNG: {reason}");
                Error::Invalid
            })?;
            (width, height, data) = (png.width(), png.height(), png.into_raw());
        }
        if width == 0 || height == 0 {
            return Err(Error::NoSize);
        }
        if width > SIDE || height > SIDE {
            return Err(Error::TooLarge);
        }
        let depth = if format == Format::Rgb { 3 } else { 4 };
        let expected = width as usize * height as usize * depth;
        if data.len() != expected {
            log::warn!(
                "graphics {pid}: {width}x{height} at {depth} bytes needs {expected}, not {}",
                data.len()
            );
            return Err(Error::Invalid);
        }
        if format == Format::Rgb {
            data = data
                .chunks_exact(3)
                .flat_map(|pixel| [pixel[0], pixel[1], pixel[2], 255])
                .collect();
        }
        Ok(Image::new(width, height, data, transient))
    }
}

fn inflate(data: &[u8]) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    flate2::read::ZlibDecoder::new(data)
        .take(media::MAX as u64 + 1)
        .read_to_end(&mut out)
        .map_err(|error| error.to_string())?;
    if out.len() > media::MAX {
        return Err(format!("it inflates past {} bytes", media::MAX));
    }
    Ok(out)
}

fn png(data: &[u8]) -> Result<image::RgbaImage, String> {
    let mut reader =
        image::ImageReader::with_format(std::io::Cursor::new(data), image::ImageFormat::Png);
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(SIDE);
    limits.max_image_height = Some(SIDE);
    limits.max_alloc = Some(media::MAX as u64);
    reader.limits(limits);
    let decoded = reader.decode().map_err(|error| error.to_string())?;
    Ok(decoded.to_rgba8())
}
