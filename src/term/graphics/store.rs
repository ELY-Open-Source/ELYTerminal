use std::collections::HashMap;

use super::Error;

/// kitty's quota and Ghostty's default, per screen.
pub(super) const LIMIT: usize = 320 * 1000 * 1000;

/// An image as it was transmitted, its pixels RGBA.
#[derive(Debug)]
pub(super) struct Image {
    pub id: u32,
    pub number: u32,
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
    pub transient: bool,
    /// When it was stored: newest answers, oldest goes first.
    generation: u64,
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
        }
    }
}

/// One session's transmitted images, within its quota.
#[derive(Debug)]
pub(super) struct Store {
    images: HashMap<u32, Image>,
    bytes: usize,
    limit: usize,
    generation: u64,
    /// The next id an image that names none takes.
    next: u32,
}

impl Store {
    pub(super) fn new(limit: usize) -> Self {
        Self {
            images: HashMap::new(),
            bytes: 0,
            limit,
            generation: 0,
            next: 1,
        }
    }

    #[cfg(test)]
    pub fn get(&self, id: u32) -> Option<&Image> {
        self.images.get(&id)
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

    /// The newest image of `number`.
    pub(super) fn newest(&self, number: u32) -> Option<u32> {
        self.images
            .values()
            .filter(|image| image.number == number)
            .max_by_key(|image| image.generation)
            .map(|image| image.id)
    }

    /// Stores `image`, evicting the oldest past the quota.
    pub(super) fn add(&mut self, mut image: Image, pid: u32) -> Result<(), Error> {
        let size = image.pixels.len();
        if size > self.limit {
            log::warn!(
                "graphics {pid}: image {} of {size} bytes passes the quota",
                image.id
            );
            return Err(Error::Space);
        }
        self.remove(image.id);
        while self.bytes + size > self.limit {
            let oldest = self
                .images
                .values()
                .min_by_key(|image| (!image.transient, image.generation))
                .map(|image| image.id)
                .expect("bytes over zero mean an image");
            log::info!("graphics {pid}: image {oldest} evicted for {size} bytes");
            self.remove(oldest);
        }
        self.generation += 1;
        image.generation = self.generation;
        self.bytes += size;
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

    /// Drops an image and its data; whether one was there.
    pub(super) fn remove(&mut self, id: u32) -> bool {
        let Some(image) = self.images.remove(&id) else {
            return false;
        };
        self.bytes -= image.pixels.len();
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
