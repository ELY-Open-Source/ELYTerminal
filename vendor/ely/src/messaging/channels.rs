use std::rc::Rc;

use gpui::{
    App, ElementId, ImageSource, IntoElement, ParentElement, RenderOnce, SharedString, Styled,
    Window, div, prelude::*,
};

use crate::{
    data_display::{Avatar, CountBadge, Presence},
    forms::OnValue,
    lists::{ListItem, Sections},
    primitives::{Icon, IconName},
    theme::{ActiveTheme, AvatarSize, IconSize},
};

/// What unread messages add at a row's end: a count, and a pin before it when pinned.
fn trail(id: &ElementId, unread: u32, pinned: bool, cx: &App) -> gpui::Div {
    let colors = &cx.theme().colors;
    div()
        .flex()
        .items_center()
        .gap_2()
        .when(pinned, |trail| {
            trail.child(
                Icon::new(IconName::Pin)
                    .size(IconSize::Xs)
                    .color(colors.fg_subtle),
            )
        })
        .when(unread > 0, |trail| {
            trail.child(CountBadge::new((id.clone(), "unread"), unread as usize).max(99))
        })
}

/// A channel as a row: a hash, or a lock when it is private, its name, and how many messages wait unread. Unread, its name stands out; muted, it goes quiet and counts nothing; pinned, it keeps a pin. As a `ListItem` it goes into a `ChannelList`.
#[derive(IntoElement)]
pub struct ChannelItem {
    id: ElementId,
    name: SharedString,
    private: bool,
    unread: u32,
    muted: bool,
    pinned: bool,
}

impl ChannelItem {
    pub fn new(id: impl Into<ElementId>, name: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            private: false,
            unread: 0,
            muted: false,
            pinned: false,
        }
    }

    pub fn private(mut self) -> Self {
        self.private = true;
        self
    }

    /// Messages waiting unread.
    pub fn unread(mut self, count: u32) -> Self {
        self.unread = count;
        self
    }

    pub fn muted(mut self, muted: bool) -> Self {
        self.muted = muted;
        self
    }

    pub fn pinned(mut self, pinned: bool) -> Self {
        self.pinned = pinned;
        self
    }

    fn item(self, cx: &App) -> ListItem {
        let colors = &cx.theme().colors;
        let unread = if self.muted { 0 } else { self.unread };
        let mark = if self.private {
            IconName::Lock
        } else {
            IconName::Hash
        };
        let trail = trail(&self.id, unread, self.pinned, cx);
        ListItem::new(self.id, self.name)
            .leading(Icon::new(mark).size(IconSize::Sm).color(colors.fg_muted))
            .strong(unread > 0)
            .quiet(self.muted)
            .trailing(trail)
    }
}

impl RenderOnce for ChannelItem {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        self.item(cx)
    }
}

/// A chat with one person as a row: their picture or initials with their presence, their name, and how many messages wait unread; unread, the name stands out. As a `ListItem` it goes into a `ChannelList`.
#[derive(IntoElement)]
pub struct DirectMessageItem {
    id: ElementId,
    name: SharedString,
    picture: Option<ImageSource>,
    presence: Presence,
    unread: u32,
}

impl DirectMessageItem {
    pub fn new(
        id: impl Into<ElementId>,
        name: impl Into<SharedString>,
        presence: Presence,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            picture: None,
            presence,
            unread: 0,
        }
    }

    pub fn picture(mut self, source: impl Into<ImageSource>) -> Self {
        self.picture = Some(source.into());
        self
    }

    /// Messages waiting unread.
    pub fn unread(mut self, count: u32) -> Self {
        self.unread = count;
        self
    }

    fn item(self, cx: &App) -> ListItem {
        let avatar = Avatar::new((self.id.clone(), "avatar"), self.name.clone())
            .size(AvatarSize::Sm)
            .presence(self.presence);
        let avatar = match self.picture {
            Some(picture) => avatar.image(picture),
            None => avatar,
        };
        let trail = trail(&self.id, self.unread, false, cx);
        ListItem::new(self.id, self.name)
            .leading(avatar)
            .strong(self.unread > 0)
            .quiet(self.presence == Presence::Offline && self.unread == 0)
            .trailing(trail)
    }
}

impl RenderOnce for DirectMessageItem {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        self.item(cx)
    }
}

/// Rows a channel list takes: a channel or a direct message.
pub enum ChatRow {
    Channel(ChannelItem),
    Direct(DirectMessageItem),
}

impl From<ChannelItem> for ChatRow {
    fn from(item: ChannelItem) -> Self {
        Self::Channel(item)
    }
}

impl From<DirectMessageItem> for ChatRow {
    fn from(item: DirectMessageItem) -> Self {
        Self::Direct(item)
    }
}

/// Channels and chats in sections down a column, such as Starred, Channels and Direct messages, each section a list: a press, an arrow or Enter opens a chat, and the one open is lit. Tab moves between sections.
#[derive(IntoElement)]
pub struct ChannelList {
    id: ElementId,
    sections: Vec<(SharedString, Vec<(SharedString, ChatRow)>)>,
    open: Option<SharedString>,
    on_open: Option<OnValue>,
}

impl ChannelList {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            sections: Vec::new(),
            open: None,
            on_open: None,
        }
    }

    /// A section under `title`, its chats by key; a title names one section, and a section holds a chat.
    pub fn section(
        mut self,
        title: impl Into<SharedString>,
        rows: impl IntoIterator<Item = (impl Into<SharedString>, impl Into<ChatRow>)>,
    ) -> Self {
        let title = title.into();
        let rows: Vec<(SharedString, ChatRow)> = rows
            .into_iter()
            .map(|(key, row)| (key.into(), row.into()))
            .collect();
        assert!(!rows.is_empty(), "section {title} holds no chats");
        let named = self.sections.iter().any(|(other, _)| *other == title);
        assert!(!named, "section {title} twice");
        for (key, _) in &rows {
            let seen = self
                .sections
                .iter()
                .flat_map(|(_, rows)| rows)
                .any(|(other, _)| other == key);
            assert!(!seen, "chat {key} twice");
        }
        self.sections.push((title, rows));
        self
    }

    /// The chat open now, by key.
    pub fn open(mut self, key: impl Into<SharedString>) -> Self {
        self.open = Some(key.into());
        self
    }

    /// Gets the key of the chat to open.
    pub fn on_open(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_open = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for ChannelList {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        if let Some(open) = &self.open {
            let listed = self
                .sections
                .iter()
                .flat_map(|(_, rows)| rows)
                .any(|(key, _)| key == open);
            if !listed {
                log::error!("channel list: no chat {open}; none open");
            }
        }
        let open: OnValue = {
            let (id, on_open) = (self.id.clone(), self.on_open.clone());
            Rc::new(move |key, window, cx| {
                log::info!("channel list {id:?}: open {key}");
                if let Some(on_open) = &on_open {
                    on_open(key, window, cx);
                }
            })
        };
        let sections = self.sections.into_iter().fold(
            Sections::new(self.id.clone()),
            |sections, (title, rows)| {
                let rows = rows
                    .into_iter()
                    .map(|(key, row)| {
                        let item = match row {
                            ChatRow::Channel(item) => item.item(cx),
                            ChatRow::Direct(item) => item.item(cx),
                        };
                        (key, item)
                    })
                    .collect();
                sections.section(title, rows)
            },
        );
        div().debug_selector(|| "channel-list".into()).child(
            sections
                .selected(self.open)
                .on_select(open.clone())
                .on_activate(open),
        )
    }
}
