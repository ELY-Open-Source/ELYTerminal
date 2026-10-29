use std::{
    collections::HashMap,
    sync::atomic::{AtomicU64, Ordering},
};

use super::{Error, frames::Animation};

/// kitty's quota and Ghostty's default, per screen.
pub(super) const LIMIT: usize = 320 * 1000 * 1000;

/// An image as it was transmitted, its pixels RGBA.
#[derive(Clone, Debug)]
pub(super) struct Image {
    pub id: u32,
    pub number: u32,
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
    pub transient: bool,
    /// When it was stored: newest answers, oldest goes first.
    pub generation: u64,
    /// What shows, a texture's key; a new frame changes it.
    pub version: u64,
    /// Its frames past the first, once animation names it.
    pub animation: Option<Animation>,
}

impl Image {
    pub(super) fn new(width: u32, height: u32, pixels: Vec<u8>, transient: bool) -> Self {
        Self {
            id: 0,
            number: 0,
            width,
            height,
            pixels,
            transient,
            generation: 0,
            version: 0,
            animation: None,
        }
    }

    /// Its bytes, the frames' too.
    pub fn size(&self) -> usize {
        self.pixels.len() + self.animation.as_ref().map_or(0, Animation::bytes)
    }

    /// The pixels shown now: its current frame's.
    pub fn shown(&self) -> &[u8] {
        let current = self
            .animation
            .as_ref()
            .map_or(0, |animation| animation.current);
        self.frame(current + 1).expect("the current frame is there")
    }

    /// Frame `number`, 1 its own pixels; none past the last.
    pub fn frame(&self, number: usize) -> Option<&[u8]> {
        match number {
            0 => None,
            1 => Some(&self.pixels),
            _ => Some(&self.animation.as_ref()?.frames.get(number - 2)?.pixels),
        }
    }

    pub fn frame_mut(&mut self, number: usize) -> Option<&mut Vec<u8>> {
        match number {
            0 => None,
            1 => Some(&mut self.pixels),
            _ => Some(&mut self.animation.as_mut()?.frames.get_mut(number - 2)?.pixels),
        }
    }

    /// A new version: what shows changed.
    pub fn changed(&mut self) {
        self.version = fresh();
    }
}

/// One count for both screens, so textures never collide.
fn fresh() -> u64 {
    static COUNT: AtomicU64 = AtomicU64::new(1);
    COUNT.fetch_add(1, Ordering::Relaxed)
}

/// One session's transmitted images, within its quota.
#[derive(Debug)]
pub(super) struct Store {
    images: HashMap<u32, Image>,
    bytes: usize,
    limit: usize,
    /// The next id an image that names none takes.
    next: u32,
}

impl Store {
    pub(super) fn new(limit: usize) -> Self {
        Self {
            images: HashMap::new(),
            bytes: 0,
            limit,
            next: 1,
        }
    }

    pub fn get(&self, id: u32) -> Option<&Image> {
        self.images.get(&id)
    }

    pub fn get_mut(&mut self, id: u32) -> Option<&mut Image> {
        self.images.get_mut(&id)
    }

    /// Free id: a number's lowest, as kitty picks, else next.
    pub(super) fn free(&mut self, numbered: bool) -> u32 {
        let mut id = if numbered { 1 } else { self.next };
        while id == 0 || self.images.contains_key(&id) {
            id = id.wrapping_add(1);
        }
        if !numbered {
            self.next = id.wrapping_add(1);
        }
        id
    }

    pub(super) fn images(&self) -> impl Iterator<Item = &Image> {
        self.images.values()
    }

    pub(super) fn images_mut(&mut self) -> impl Iterator<Item = &mut Image> {
        self.images.values_mut()
    }

    /// The newest image of `number`.
    pub(super) fn newest(&self, number: u32) -> Option<u32> {
        self.images
            .values()
            .filter(|image| image.number == number)
            .max_by_key(|image| image.generation)
            .map(|image| image.id)
    }

    /// Stores `image`, evicting unplaced, then oldest, past the quota.
    pub(super) fn add(
        &mut self,
        mut image: Image,
        pid: u32,
        placed: &dyn Fn(u32) -> bool,
    ) -> Result<(), Error> {
        let size = image.size();
        self.remove(image.id);
        self.reserve(image.id, size, pid, placed)?;
        image.generation = fresh();
        image.version = image.generation;
        log::debug!(
            "graphics {pid}: image {} stored, {}x{}, {} bytes of {}",
            image.id,
            image.width,
            image.height,
            self.bytes,
            self.limit
        );
        self.images.insert(image.id, image);
        Ok(())
    }

    /// Room for `size` more bytes, evicting images but `keep`.
    pub(super) fn reserve(
        &mut self,
        keep: u32,
        size: usize,
        pid: u32,
        placed: &dyn Fn(u32) -> bool,
    ) -> Result<(), Error> {
        if size > self.limit {
            log::warn!("graphics {pid}: image {keep}'s {size} bytes pass the quota");
            return Err(Error::Space);
        }
        while self.bytes + size > self.limit {
            let others = self.images.values().filter(|image| image.id != keep);
            let oldest = others
                .min_by_key(|image| (placed(image.id), !image.transient, image.generation))
                .map(|image| image.id)
                .ok_or(Error::Space)?;
            log::info!("graphics {pid}: image {oldest} evicted for {size} bytes");
            self.remove(oldest);
        }
        self.bytes += size;
        Ok(())
    }

    /// Gives back `size` bytes a frame held.
    pub(super) fn release(&mut self, size: usize) {
        self.bytes -= size;
    }

    /// Drops an image and its data; whether one was there.
    pub(super) fn remove(&mut self, id: u32) -> bool {
        let Some(image) = self.images.remove(&id) else {
            return false;
        };
        self.bytes -= image.size();
        true
    }

    /// The ids from `first` to `last`.
    pub(super) fn within(&self, first: u32, last: u32) -> Vec<u32> {
        let mut ids: Vec<u32> = self
            .images
            .keys()
            .copied()
            .filter(|id| (first..=last).contains(id))
            .collect();
        ids.sort_unstable();
        ids
    }
}
