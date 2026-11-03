use std::collections::HashMap;

use gpui::{
    App, AssetSource, Global, Hsla, IntoElement, Radians, RenderOnce, SharedString, Transformation,
    Window, prelude::*, size, svg,
};

use crate::theme::{ActiveTheme, IconSize};

macro_rules! icons {
    ($($variant:ident => $file:literal,)*) => {
        /// Lucide icons bundled with Ely.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum IconName {
            $($variant,)*
        }

        impl IconName {
            pub const ALL: &[IconName] = &[$(IconName::$variant,)*];

            pub fn path(self) -> &'static str {
                match self {
                    $(IconName::$variant => concat!("icons/", $file, ".svg"),)*
                }
            }

            /// Lucide name, such as `chevron-down`.
            pub fn name(self) -> &'static str {
                match self {
                    $(IconName::$variant => $file,)*
                }
            }
        }
    };
}

icons! {
    AArrowDown => "a-arrow-down",
    AArrowUp => "a-arrow-up",
    Accessibility => "accessibility",
    Activity => "activity",
    AlarmClock => "alarm-clock",
    AlignCenter => "align-center",
    AlignCenterHorizontal => "align-center-horizontal",
    AlignCenterVertical => "align-center-vertical",
    AlignEndHorizontal => "align-end-horizontal",
    AlignEndVertical => "align-end-vertical",
    AlignHorizontalDistributeCenter => "align-horizontal-distribute-center",
    AlignJustify => "align-justify",
    AlignLeft => "align-left",
    AlignRight => "align-right",
    AlignStartHorizontal => "align-start-horizontal",
    AlignStartVertical => "align-start-vertical",
    AlignVerticalDistributeCenter => "align-vertical-distribute-center",
    AppWindow => "app-window",
    Archive => "archive",
    ArrowDownLeft => "arrow-down-left",
    ArrowDownUp => "arrow-down-up",
    ArrowDownToDot => "arrow-down-to-dot",
    ArrowDownToLine => "arrow-down-to-line",
    ArrowDownWideNarrow => "arrow-down-wide-narrow",
    ArrowDown => "arrow-down",
    ArrowLeft => "arrow-left",
    ArrowRight => "arrow-right",
    ArrowUpFromDot => "arrow-up-from-dot",
    ArrowUpNarrowWide => "arrow-up-narrow-wide",
    ArrowUpRight => "arrow-up-right",
    ArrowUp => "arrow-up",
    AtSign => "at-sign",
    AudioLines => "audio-lines",
    Ban => "ban",
    Battery => "battery",
    Bell => "bell",
    Bold => "bold",
    BookOpen => "book-open",
    Bookmark => "bookmark",
    Bot => "bot",
    Box => "box",
    Braces => "braces",
    Brush => "brush",
    Bug => "bug",
    CalendarDays => "calendar-days",
    CalendarRange => "calendar-range",
    Calendar => "calendar",
    Camera => "camera",
    Captions => "captions",
    CaptionsOff => "captions-off",
    CaseSensitive => "case-sensitive",
    ChartArea => "chart-area",
    ChartBar => "chart-bar",
    ChartCandlestick => "chart-candlestick",
    ChartColumn => "chart-column",
    ChartLine => "chart-line",
    ChartPie => "chart-pie",
    CheckCheck => "check-check",
    ChartGantt => "chart-gantt",
    Check => "check",
    ChevronDown => "chevron-down",
    ChevronLeft => "chevron-left",
    ChevronRight => "chevron-right",
    ChevronUp => "chevron-up",
    ChevronsLeft => "chevrons-left",
    ChevronsLeftRight => "chevrons-left-right",
    ChevronsRight => "chevrons-right",
    ChevronsUpDown => "chevrons-up-down",
    CircleAlert => "circle-alert",
    CircleCheck => "circle-check",
    CircleDashed => "circle-dashed",
    CircleDot => "circle-dot",
    CircleDotDashed => "circle-dot-dashed",
    CircleHelp => "circle-help",
    CirclePause => "circle-pause",
    CircleStop => "circle-stop",
    CircleX => "circle-x",
    Circle => "circle",
    ClipboardCheck => "clipboard-check",
    Clipboard => "clipboard",
    Clock => "clock",
    CloudAlert => "cloud-alert",
    CloudCheck => "cloud-check",
    CloudOff => "cloud-off",
    Cloud => "cloud",
    CloudUpload => "cloud-upload",
    Code => "code",
    Columns2 => "columns-2",
    Columns3 => "columns-3",
    Command => "command",
    Compass => "compass",
    Contrast => "contrast",
    Copy => "copy",
    CornerDownRight => "corner-down-right",
    Cpu => "cpu",
    CreditCard => "credit-card",
    Crop => "crop",
    Crosshair => "crosshair",
    Database => "database",
    Delete => "delete",
    Dices => "dices",
    DollarSign => "dollar-sign",
    Dot => "dot",
    Download => "download",
    EllipsisVertical => "ellipsis-vertical",
    Ellipsis => "ellipsis",
    Eraser => "eraser",
    ExternalLink => "external-link",
    EyeOff => "eye-off",
    Eye => "eye",
    FileArchive => "file-archive",
    FileAudio => "file-audio",
    FileCode => "file-code",
    FileCog => "file-cog",
    FileImage => "file-image",
    FileJson => "file-json",
    FilePlus => "file-plus",
    FileSpreadsheet => "file-spreadsheet",
    FileTerminal => "file-terminal",
    FileText => "file-text",
    File => "file",
    FileType => "file-type",
    FileVideoCamera => "file-video-camera",
    Filter => "filter",
    Fingerprint => "fingerprint",
    Flag => "flag",
    FolderOpen => "folder-open",
    FolderPlus => "folder-plus",
    Folder => "folder",
    Forward => "forward",
    Frame => "frame",
    Frown => "frown",
    Fullscreen => "fullscreen",
    Gauge => "gauge",
    GitBranch => "git-branch",
    GitCommitHorizontal => "git-commit-horizontal",
    GitMerge => "git-merge",
    GitPullRequest => "git-pull-request",
    GitPullRequestClosed => "git-pull-request-closed",
    GitPullRequestDraft => "git-pull-request-draft",
    Globe => "globe",
    GripHorizontal => "grip-horizontal",
    GripVertical => "grip-vertical",
    Hand => "hand",
    HardDrive => "hard-drive",
    Hash => "hash",
    Heading => "heading",
    Heading1 => "heading-1",
    Heading2 => "heading-2",
    Heading3 => "heading-3",
    Headphones => "headphones",
    Heart => "heart",
    Hexagon => "hexagon",
    History => "history",
    Hourglass => "hourglass",
    House => "house",
    Image => "image",
    ImageOff => "image-off",
    Inbox => "inbox",
    Info => "info",
    Italic => "italic",
    Kanban => "kanban",
    Key => "key",
    Keyboard => "keyboard",
    Languages => "languages",
    Laugh => "laugh",
    Layers => "layers",
    LayoutDashboard => "layout-dashboard",
    LayoutGrid => "layout-grid",
    Library => "library",
    Lightbulb => "lightbulb",
    Link => "link",
    ListChecks => "list-checks",
    ListOrdered => "list-ordered",
    ListTodo => "list-todo",
    List => "list",
    LoaderCircle => "loader-circle",
    LockOpen => "lock-open",
    Lock => "lock",
    LogIn => "log-in",
    LogOut => "log-out",
    Mail => "mail",
    MapPin => "map-pin",
    Map => "map",
    Maximize2 => "maximize-2",
    Meh => "meh",
    MemoryStick => "memory-stick",
    Menu => "menu",
    Merge => "merge",
    MessageSquare => "message-square",
    MessageSquareDiff => "message-square-diff",
    MessageSquareWarning => "message-square-warning",
    MicOff => "mic-off",
    Mic => "mic",
    Minimize2 => "minimize-2",
    Minus => "minus",
    Monitor => "monitor",
    Moon => "moon",
    MousePointer2 => "mouse-pointer-2",
    Move => "move",
    MoveUpRight => "move-up-right",
    Music => "music",
    Network => "network",
    NotebookPen => "notebook-pen",
    OctagonAlert => "octagon-alert",
    Package => "package",
    Palette => "palette",
    PanelBottom => "panel-bottom",
    PanelLeft => "panel-left",
    PanelRight => "panel-right",
    Paperclip => "paperclip",
    Pause => "pause",
    PenLine => "pen-line",
    PenTool => "pen-tool",
    Pencil => "pencil",
    PhoneOff => "phone-off",
    Phone => "phone",
    PictureInPicture2 => "picture-in-picture-2",
    Pipette => "pipette",
    PinOff => "pin-off",
    Pin => "pin",
    Play => "play",
    Plus => "plus",
    Power => "power",
    Printer => "printer",
    Puzzle => "puzzle",
    Quote => "quote",
    Redo2 => "redo-2",
    RedoDot => "redo-dot",
    RefreshCw => "refresh-cw",
    Regex => "regex",
    Repeat => "repeat",
    Repeat1 => "repeat-1",
    Reply => "reply",
    ReplyAll => "reply-all",
    Rocket => "rocket",
    RotateCcw => "rotate-ccw",
    RotateCw => "rotate-cw",
    Rows2 => "rows-2",
    Save => "save",
    Scissors => "scissors",
    ScreenShare => "screen-share",
    ScreenShareOff => "screen-share-off",
    Search => "search",
    SearchX => "search-x",
    Send => "send",
    Server => "server",
    Settings => "settings",
    Shapes => "shapes",
    Share2 => "share-2",
    Shield => "shield",
    Shuffle => "shuffle",
    SkipBack => "skip-back",
    SkipForward => "skip-forward",
    Slash => "slash",
    SlidersHorizontal => "sliders-horizontal",
    Smartphone => "smartphone",
    Smile => "smile",
    SmilePlus => "smile-plus",
    Sparkles => "sparkles",
    Spline => "spline",
    SquareCheck => "square-check",
    Square => "square",
    SquareFunction => "square-function",
    SquarePen => "square-pen",
    SquareRoundCorner => "square-round-corner",
    Star => "star",
    StepForward => "step-forward",
    StickyNote => "sticky-note",
    Strikethrough => "strikethrough",
    Sun => "sun",
    Table => "table",
    Tag => "tag",
    Target => "target",
    Terminal => "terminal",
    ThumbsDown => "thumbs-down",
    ThumbsUp => "thumbs-up",
    Timer => "timer",
    Trash2 => "trash-2",
    TrendingDown => "trending-down",
    TrendingUp => "trending-up",
    TriangleAlert => "triangle-alert",
    Type => "type",
    Underline => "underline",
    Undo2 => "undo-2",
    Upload => "upload",
    User => "user",
    UserPlus => "user-plus",
    Users => "users",
    Variable => "variable",
    VideoOff => "video-off",
    Video => "video",
    Volume => "volume",
    Volume1 => "volume-1",
    Volume2 => "volume-2",
    VolumeX => "volume-x",
    Wallet => "wallet",
    WandSparkles => "wand-sparkles",
    WholeWord => "whole-word",
    WifiOff => "wifi-off",
    Wifi => "wifi",
    Workflow => "workflow",
    Wrench => "wrench",
    X => "x",
    Zap => "zap",
    ZoomIn => "zoom-in",
    ZoomOut => "zoom-out",
}

#[derive(IntoElement)]
pub struct Icon {
    path: SharedString,
    bundled: bool,
    size: IconSize,
    color: Option<Hsla>,
    hover: Option<(SharedString, Hsla)>,
    rotation: Option<Radians>,
    scale: f32,
}

impl Icon {
    pub fn new(name: IconName) -> Self {
        Self {
            bundled: true,
            ..Self::from_path(name.path())
        }
    }

    /// Draws an SVG supplied by the application's asset source; a missing one logs and shows `broken`.
    pub fn from_path(path: impl Into<SharedString>) -> Self {
        Self {
            path: path.into(),
            bundled: false,
            size: IconSize::Md,
            color: None,
            hover: None,
            rotation: None,
            scale: 1.0,
        }
    }

    pub fn size(mut self, size: IconSize) -> Self {
        self.size = size;
        self
    }

    /// Defaults to `colors.fg`.
    pub fn color(mut self, color: impl Into<Hsla>) -> Self {
        self.color = Some(color.into());
        self
    }

    /// Color while the pointer is over `group`.
    pub fn group_hover_color(
        mut self,
        group: impl Into<SharedString>,
        color: impl Into<Hsla>,
    ) -> Self {
        self.hover = Some((group.into(), color.into()));
        self
    }

    pub fn rotate(mut self, angle: impl Into<Radians>) -> Self {
        self.rotation = Some(angle.into());
        self
    }

    /// Draws the glyph larger or smaller inside the same box.
    pub fn scale(mut self, factor: f32) -> Self {
        self.scale = factor;
        self
    }
}

impl From<IconName> for Icon {
    fn from(name: IconName) -> Self {
        Icon::new(name)
    }
}

/// App paths already looked up, and whether the source held them.
#[derive(Default)]
struct FoundIcons(HashMap<SharedString, bool>);

impl Global for FoundIcons {}

fn found(source: &dyn AssetSource, path: &str) -> bool {
    match source.load(path) {
        Ok(Some(_)) => true,
        Ok(None) => {
            log::error!("ely: icon {path} is not in the app's asset source");
            false
        }
        Err(error) => {
            log::error!("ely: icon {path} failed to load: {error:#}");
            false
        }
    }
}

impl Icon {
    /// What a missing app icon shows: a broken picture in danger.
    fn broken(self, cx: &App) -> Self {
        Self {
            path: IconName::ImageOff.path().into(),
            bundled: true,
            color: Some(cx.theme().colors.danger),
            hover: None,
            ..self
        }
    }
}

impl RenderOnce for Icon {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let source = cx.asset_source().clone();
        let missing = !self.bundled
            && !*cx
                .default_global::<FoundIcons>()
                .0
                .entry(self.path.clone())
                .or_insert_with(|| found(source.as_ref(), &self.path));
        let icon = if missing { self.broken(cx) } else { self };
        let theme = cx.theme();
        let color = icon.color.unwrap_or(theme.colors.fg);
        svg()
            .path(icon.path)
            .size(theme.icon_size(icon.size))
            .flex_none()
            .text_color(color)
            .when_some(icon.hover, |svg, (group, hover)| {
                svg.group_hover(group, |style| style.text_color(hover))
            })
            .when(icon.rotation.is_some() || icon.scale != 1.0, |svg| {
                svg.with_transformation(
                    Transformation::rotate(icon.rotation.unwrap_or(Radians(0.0)))
                        .with_scaling(size(icon.scale, icon.scale)),
                )
            })
    }
}

#[cfg(test)]
mod tests;
