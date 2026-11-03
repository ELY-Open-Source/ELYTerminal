use gpui::{
    AnyElement, App, ElementId, IntoElement, ParentElement, Rems, SharedString, Styled, div,
};

use crate::{
    data_display::{Avatar, Badge, Sparkline, Tone},
    motion::ProgressBar,
    theme::{ActiveTheme, AvatarSize},
    typography::{Ellipsis, ExternalLink, format, tabular},
};

/// What a cell holds; its column says how numbers read.
#[derive(Clone, Debug, PartialEq)]
pub enum Cell {
    Empty,
    Text(SharedString),
    Number(f64),
    Tag(SharedString, Tone),
    /// A share from 0 to 1, drawn as a bar with its percent.
    Progress(f32),
    Spark(Vec<f32>),
    /// A person's name beside their initials.
    Person(SharedString),
    /// A label, and the address it opens.
    Link(SharedString, SharedString),
    /// A length of time in seconds, read in its two largest units; it sorts by length.
    Duration(u64),
}

impl Cell {
    /// The words a filter looks through and a sort compares.
    pub(crate) fn words(&self) -> SharedString {
        match self {
            Self::Text(text) | Self::Tag(text, _) | Self::Person(text) | Self::Link(text, _) => {
                text.clone()
            }
            Self::Number(value) => value.to_string().into(),
            Self::Progress(share) => share.to_string().into(),
            Self::Duration(seconds) => {
                format::duration(*seconds, format::DurationStyle::Compact).into()
            }
            Self::Empty | Self::Spark(_) => SharedString::default(),
        }
    }

    /// The value a sort and a figure use, for cells that count.
    pub(crate) fn number(&self) -> Option<f64> {
        match self {
            Self::Number(value) => Some(*value),
            Self::Progress(share) => Some(f64::from(*share)),
            Self::Duration(seconds) => Some(*seconds as f64),
            _ => None,
        }
    }
}

impl From<&'static str> for Cell {
    fn from(text: &'static str) -> Self {
        Self::Text(text.into())
    }
}

impl From<String> for Cell {
    fn from(text: String) -> Self {
        Self::Text(text.into())
    }
}

impl From<f64> for Cell {
    fn from(value: f64) -> Self {
        Self::Number(value)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Align {
    #[default]
    Start,
    End,
}

/// A figure the footer shows for a column.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Aggregate {
    Sum,
    Average,
    Count,
    Min,
    Max,
}

impl Aggregate {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Sum => "Sum",
            Self::Average => "Avg",
            Self::Count => "Count",
            Self::Min => "Min",
            Self::Max => "Max",
        }
    }
}

/// A column: its key and title, its width, and how its cells read.
#[derive(Clone, Debug)]
pub struct Column {
    pub(crate) key: SharedString,
    pub(crate) title: SharedString,
    pub(crate) width: Option<Rems>,
    pub(crate) align: Align,
    pub(crate) sortable: bool,
    pub(crate) decimals: usize,
    pub(crate) prefix: Option<SharedString>,
    pub(crate) suffix: Option<SharedString>,
    pub(crate) aggregate: Option<Aggregate>,
    pub(crate) scale: bool,
    pub(crate) editable: bool,
    pub(crate) pinned: bool,
}

impl Column {
    pub fn new(key: impl Into<SharedString>, title: impl Into<SharedString>) -> Self {
        Self {
            key: key.into(),
            title: title.into(),
            width: None,
            align: Align::Start,
            sortable: true,
            decimals: 0,
            prefix: None,
            suffix: None,
            aggregate: None,
            scale: false,
            editable: false,
            pinned: false,
        }
    }

    /// A fixed width; columns without one share what is left.
    pub fn width(mut self, width: Rems) -> Self {
        self.width = Some(width);
        self
    }

    /// Sets its cells to the end, as numbers read.
    pub fn end(mut self) -> Self {
        self.align = Align::End;
        self
    }

    /// Its header does not sort.
    pub fn unsorted(mut self) -> Self {
        self.sortable = false;
        self
    }

    pub fn decimals(mut self, decimals: usize) -> Self {
        self.decimals = decimals;
        self
    }

    pub fn prefix(mut self, text: impl Into<SharedString>) -> Self {
        self.prefix = Some(text.into());
        self
    }

    pub fn suffix(mut self, text: impl Into<SharedString>) -> Self {
        self.suffix = Some(text.into());
        self
    }

    /// A figure for the footer, over the rows the filter keeps.
    pub fn aggregate(mut self, aggregate: Aggregate) -> Self {
        self.aggregate = Some(aggregate);
        self
    }

    /// Tints each number by where it sits in the column: red low, green high.
    pub fn scale(mut self) -> Self {
        self.scale = true;
        self
    }

    /// A double press on a cell edits its text; the table's `on_edit` gets it.
    pub fn editable(mut self) -> Self {
        self.editable = true;
        self
    }

    /// Holds still at the start while the other columns scroll sideways. Needs a width.
    pub fn pinned(mut self) -> Self {
        self.pinned = true;
        self
    }

    pub fn key(&self) -> &SharedString {
        &self.key
    }

    pub fn title(&self) -> &SharedString {
        &self.title
    }

    /// A number as this column reads it.
    pub(crate) fn reads(&self, value: f64) -> String {
        format!(
            "{}{}{}",
            self.prefix.as_ref().map_or("", |text| text.as_ref()),
            format::number(value, self.decimals, format::Separators::EN),
            self.suffix.as_ref().map_or("", |text| text.as_ref())
        )
    }
}

/// A cell drawn as its kind asks.
pub(crate) fn draw(cell: &Cell, column: &Column, id: ElementId, cx: &App) -> AnyElement {
    let colors = &cx.theme().colors;
    match cell {
        Cell::Empty => div().into_any_element(),
        Cell::Text(text) => Ellipsis::new(text.clone()).into_any_element(),
        Cell::Number(value) => tabular(div())
            .child(column.reads(*value))
            .into_any_element(),
        Cell::Tag(text, tone) => Badge::new(text.clone()).tone(*tone).into_any_element(),
        Cell::Progress(share) => div()
            .flex()
            .items_center()
            .gap_2()
            .w_full()
            .child(
                div()
                    .flex_1()
                    .child(ProgressBar::new(id, share.clamp(0.0, 1.0))),
            )
            .child(
                tabular(div())
                    .text_color(colors.fg_muted)
                    .child(format::percent(f64::from(*share), 0, false)),
            )
            .into_any_element(),
        Cell::Spark(values) => Sparkline::new(values.iter().copied()).into_any_element(),
        Cell::Person(name) => div()
            .flex()
            .items_center()
            .gap_2()
            .min_w_0()
            .child(Avatar::new(id, name.clone()).size(AvatarSize::Xs))
            .child(Ellipsis::new(name.clone()))
            .into_any_element(),
        Cell::Duration(seconds) => tabular(div())
            .child(format::duration(*seconds, format::DurationStyle::Compact))
            .into_any_element(),
        Cell::Link(label, url) => {
            ExternalLink::new(id, label.clone(), url.clone()).into_any_element()
        }
    }
}
