use gpui::Rems;

use super::{Theme, tokens::px_to_rems};

/// Media tools' measures.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MediaSizes {
    /// A crop box's handle, square.
    pub crop_handle: Rems,
    /// A mark's line on a picture.
    pub mark_stroke: Rems,
    /// An arrow's head, from its tip.
    pub arrow_head: Rems,
    /// A numbered pin, round.
    pub pin: Rems,
    /// A strip of video frames, tall.
    pub strip: Rems,
    /// A trim's handle, wide.
    pub trim_handle: Rems,
    /// A waveform's bar, wide.
    pub wave_bar: Rems,
    /// The gap between a waveform's bars.
    pub wave_gap: Rems,
    /// A waveform or a spectrum, tall.
    pub waveform: Rems,
    /// A song's cover, square.
    pub artwork: Rems,
}

impl Theme {
    pub fn media(&self) -> MediaSizes {
        MediaSizes {
            crop_handle: px_to_rems(10.0),
            mark_stroke: px_to_rems(2.0),
            arrow_head: px_to_rems(12.0),
            pin: px_to_rems(20.0),
            strip: px_to_rems(48.0),
            trim_handle: px_to_rems(8.0),
            wave_bar: px_to_rems(3.0),
            wave_gap: px_to_rems(2.0),
            waveform: px_to_rems(48.0),
            artwork: px_to_rems(64.0),
        }
    }
}
