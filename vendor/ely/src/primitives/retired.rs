//! Images a view stopped drawing, freed from the window's atlas.

use std::sync::Arc;

use gpui::{RenderImage, Window};

/// Images to free; GPUI keeps each painted image until told.
#[derive(Default, Clone)]
pub struct Retired(Vec<Arc<RenderImage>>);

impl Retired {
    /// An image no longer drawn; freed at the next paint.
    pub fn push(&mut self, image: Arc<RenderImage>) {
        self.0.push(image);
    }

    /// Takes over another owner's waiting images.
    pub fn append(&mut self, mut other: Retired) {
        self.0.append(&mut other.0);
    }

    /// Frees images no one else holds; call while painting.
    pub fn release(&mut self, window: &mut Window) {
        // A shared image may still be drawn; it waits.
        let (free, held): (Vec<_>, Vec<_>) = self
            .0
            .drain(..)
            .partition(|image| Arc::strong_count(image) == 1);
        self.0 = held;
        for image in free {
            if let Err(error) = window.drop_image(image) {
                log::error!("images: a retired image stayed: {error}");
            }
        }
    }

    /// Frees every image: one copy per window that drew it.
    pub fn release_all(&mut self, window: &mut Window) {
        for image in self.0.drain(..) {
            if let Err(error) = window.drop_image(image) {
                log::error!("images: a retired image stayed: {error}");
            }
        }
    }

    /// Whether nothing waits to be freed.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}
