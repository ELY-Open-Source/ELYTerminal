mod annotate;
mod audio;
mod camera;
mod controls;
mod crop;
mod cropper;
mod listen;
mod marks;
mod model;
mod player;
mod record;
mod scrubber;
mod strip;
mod subtitles;
mod thumbnail;
mod turn;
mod upload;
mod viewer;

#[cfg(all(test, feature = "test-support"))]
mod tests;

use std::time::Duration;

pub use annotate::ImageAnnotator;
pub use audio::{AudioSpectrum, AudioWaveform};
pub(crate) use camera::name_chip;
pub use camera::{CameraPreview, Device, DeviceKind, DeviceSelector};
pub use controls::{MediaControls, PlaybackSpeedControl, Repeat, VolumeControl};
pub use crop::Crop;
pub use cropper::ImageCropper;
pub use listen::{AudioPlayer, Playlist, Track};
pub use marks::{Mark, Tool};
pub use model::{ModelViewer, Orbit};
pub use player::{SPEEDS, VideoPlayer};
pub(crate) use record::device_toggle;
pub use record::{MicLevelMeter, Recording, ScreenRecorderControls};
pub use scrubber::Scrubber;
pub use strip::VideoThumbnailStrip;
pub use subtitles::{Cue, SubtitleEditor};
pub use thumbnail::ImageThumbnail;
pub use upload::ImageUpload;
pub use viewer::ImageViewer;

/// `at` within `length`: a live position past the end logs and sits at the end.
fn played(at: Duration, length: Duration, owner: &str) -> Duration {
    if at > length {
        log::error!("{owner}: {at:?} past {length:?}; at the end");
    }
    at.min(length)
}
