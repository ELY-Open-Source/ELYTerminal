//! What a command answers: its ids and its words.

/// A failure, as its reply names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Error {
    Both,
    Invalid,
    Read,
    Decompress,
    Format,
    Medium,
    NoSize,
    TooLarge,
    Space,
    NoId,
    NoImage,
    VirtualParent,
    NoParentImage,
    NoParentPlacement,
    SelfParent,
    Cycle,
    TooDeep,
    FrameTooLarge,
    NoBase,
    FrameSpace,
    NoSource,
    NoDestination,
    DestinationOut,
    SourceOut,
    Overlap,
}

impl Error {
    /// Ghostty's words; kitty's for a file that does not read.
    pub fn message(self) -> &'static str {
        match self {
            Self::Both => "EINVAL: image ID and number are mutually exclusive",
            Self::Invalid => "EINVAL: invalid data",
            Self::Read => "EBADF:Failed to read image file",
            Self::Decompress => "EINVAL: decompression failed",
            Self::Format => "EINVAL: unsupported format",
            Self::Medium => "EINVAL: unsupported medium",
            Self::NoSize => "EINVAL: dimensions required",
            Self::TooLarge => "EINVAL: dimensions too large",
            Self::Space => "ENOMEM: out of memory",
            Self::NoId => "EINVAL: image ID or number required",
            Self::NoImage => "ENOENT: image not found",
            Self::VirtualParent => "EINVAL: virtual placement cannot refer to a parent",
            Self::NoParentImage => "ENOPARENT: parent image not found",
            Self::NoParentPlacement => "ENOPARENT: parent placement not found",
            Self::SelfParent => "EINVAL: placement cannot be its own parent",
            Self::Cycle => "ECYCLE: parent chain creates a cycle",
            Self::TooDeep => "ETOODEEP: parent chain too deep",
            Self::FrameTooLarge => "EINVAL: frame dimensions exceed image",
            Self::NoBase => "EINVAL: base frame not found",
            Self::FrameSpace => "ENOSPC: animation frame storage full",
            Self::NoSource => "ENOENT: source frame not found",
            Self::NoDestination => "ENOENT: destination frame not found",
            Self::DestinationOut => "EINVAL: destination rectangle out of bounds",
            Self::SourceOut => "EINVAL: source rectangle out of bounds",
            Self::Overlap => "EINVAL: source and destination rectangles overlap",
        }
    }
}

/// What a command names its image by, echoed back.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct Ids {
    pub id: u32,
    pub number: u32,
    pub placement: u32,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) struct Reply {
    pub ids: Ids,
    pub message: &'static str,
    /// An animation frame's number, `r`; 0 for none.
    frame: u32,
}

impl Reply {
    pub fn ok(ids: Ids) -> Self {
        Self {
            ids,
            message: "OK",
            frame: 0,
        }
    }

    /// The reply naming frame `frame`, as Ghostty's does.
    pub fn frame(self, frame: u32) -> Self {
        Self { frame, ..self }
    }

    pub fn failed(ids: Ids, error: Error) -> Self {
        Self {
            ids,
            message: error.message(),
            frame: 0,
        }
    }

    /// The reply's escape; none without an id or a number.
    pub fn text(&self) -> Option<String> {
        let Ids {
            id,
            number,
            placement,
        } = self.ids;
        if id == 0 && number == 0 {
            return None;
        }
        let pairs = [
            ("i", id),
            ("I", number),
            ("p", placement),
            ("r", self.frame),
        ];
        let keys: Vec<String> = pairs
            .into_iter()
            .filter(|(_, value)| *value > 0)
            .map(|(key, value)| format!("{key}={value}"))
            .collect();
        Some(format!("\x1b_G{};{}\x1b\\", keys.join(","), self.message))
    }
}
