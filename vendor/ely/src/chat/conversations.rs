use std::rc::Rc;

use gpui::{
    App, ElementId, FontWeight, InteractiveElement, IntoElement, MouseButton, ParentElement,
    RenderOnce, SharedString, StatefulInteractiveElement, Styled, Window, div, prelude::*,
    transparent_black,
};
use jiff::{Timestamp, civil::Date, tz::TimeZone};

use crate::{
    buttons::{Button, ButtonVariant},
    forms::OnValue,
    menus::{Menu, MenuItem, OverflowMenu},
    primitives::{FocusRing, Icon, IconName, tab_stop},
    theme::{ActiveTheme, ControlSize, IconSize, Radius, TextSize},
    typography::{Ellipsis, format::system_zone},
};

/// A conversation in a list: its key, its title, when it last moved, and whether it is pinned.
#[derive(Clone, Debug, PartialEq)]
pub struct Conversation {
    pub key: SharedString,
    pub title: SharedString,
    pub at: Timestamp,
    pub pinned: bool,
}

/// The heading a conversation files under by the day it last moved, seen from `today`.
pub(crate) fn group_name(day: Date, today: Date) -> String {
    let days = (today - day).get_days();
    match days {
        ..=0 => "Today".to_string(),
        1 => "Yesterday".to_string(),
        2..=7 => "Previous 7 days".to_string(),
        8..=30 => "Previous 30 days".to_string(),
        _ if day.year() == today.year() => day.strftime("%B").to_string(),
        _ => day.strftime("%B %Y").to_string(),
    }
}

/// Whether every word of `query` is in `title`, ignoring case.
pub(crate) fn matches(title: &str, query: &str) -> bool {
    let title = title.to_lowercase();
    query
        .split_whitespace()
        .all(|word| title.contains(&word.to_lowercase()))
}

/// The conversations `query` keeps, pinned first, then under their day's heading, newest first; each group holds indexes.
pub(crate) fn grouped(
    conversations: &[Conversation],
    query: &str,
    today: Date,
    zone: &TimeZone,
) -> Vec<(String, Vec<usize>)> {
    let mut kept: Vec<usize> = (0..conversations.len())
        .filter(|ix| matches(&conversations[*ix].title, query))
        .collect();
    kept.sort_by_key(|ix| std::cmp::Reverse(conversations[*ix].at));
    let mut groups: Vec<(String, Vec<usize>)> = Vec::new();
    let pinned: Vec<usize> = kept
        .iter()
        .copied()
        .filter(|ix| conversations[*ix].pinned)
        .collect();
    if !pinned.is_empty() {
        groups.push(("Pinned".to_string(), pinned));
    }
    for ix in kept.into_iter().filter(|ix| !conversations[*ix].pinned) {
        let name = group_name(conversations[ix].at.to_zoned(zone.clone()).date(), today);
        match groups.last_mut() {
            Some((last, items)) if *last == name => items.push(ix),
            _ => groups.push((name, vec![ix])),
        }
    }
    groups
}

/// Opens a new conversation; it shows Command-N, which the host binds.
pub fn new_chat_button(id: impl Into<ElementId>) -> Button {
    Button::new(id, "New chat")
        .variant(ButtonVariant::Secondary)
        .icon(IconName::SquarePen)
        .shortcut("secondary-n")
}

type OnKey = OnValue;

/// One conversation in the list: its title, a pin when pinned, the one open marked with its menu of rename, pin and delete, which the others show on hover or focus.
#[derive(IntoElement)]
pub struct ConversationItem {
    id: ElementId,
    conversation: Conversation,
    active: bool,
    on_select: Option<OnKey>,
    on_rename: Option<OnKey>,
    on_pin: Option<OnKey>,
    on_delete: Option<OnKey>,
}

impl ConversationItem {
    pub fn new(id: impl Into<ElementId>, conversation: Conversation) -> Self {
        Self {
            id: id.into(),
            conversation,
            active: false,
            on_select: None,
            on_rename: None,
            on_pin: None,
            on_delete: None,
        }
    }

    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    pub fn on_select(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_select = Some(Rc::new(handler));
        self
    }

    /// Rename, pin and delete, each given the conversation's key.
    pub fn actions(
        mut self,
        on_rename: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
        on_pin: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
        on_delete: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_rename = Some(Rc::new(on_rename));
        self.on_pin = Some(Rc::new(on_pin));
        self.on_delete = Some(Rc::new(on_delete));
        self
    }
}

impl RenderOnce for ConversationItem {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let key = self.conversation.key.clone();
        let group = SharedString::from(format!("conversation-{}", self.id));
        let menu = match (self.on_rename, self.on_pin, self.on_delete) {
            (Some(rename), Some(pin), Some(delete)) => {
                let (a, b, c) = (key.clone(), key.clone(), key.clone());
                Some(
                    Menu::new()
                        .item(
                            MenuItem::new("Rename")
                                .icon(IconName::Pencil)
                                .on_click(move |window, cx| rename(&a, window, cx)),
                        )
                        .item(
                            match self.conversation.pinned {
                                true => MenuItem::new("Unpin").icon(IconName::PinOff),
                                false => MenuItem::new("Pin").icon(IconName::Pin),
                            }
                            .on_click(move |window, cx| pin(&b, window, cx)),
                        )
                        .separator()
                        .item(
                            MenuItem::new("Delete")
                                .icon(IconName::Trash2)
                                .on_click(move |window, cx| delete(&c, window, cx)),
                        ),
                )
            }
            _ => None,
        };
        let select = self.on_select;
        let pressable = select.is_some() || menu.is_some();
        let focus = tab_stop((self.id.clone(), "focus").into(), pressable, window, cx);
        let shown = self.active || focus.contains_focused(window, cx);
        let theme = cx.theme();
        let colors = theme.colors.clone();
        div()
            .id(self.id.clone())
            .group(group.clone())
            .track_focus(&focus)
            .flex()
            .items_center()
            .gap_1()
            .h(theme.control_height(ControlSize::Md))
            .pl_2()
            .pr_0p5()
            .rounded(theme.radius(Radius::Md))
            .border_1()
            .border_color(transparent_black())
            .focus_ring(cx)
            .text_size(theme.text_size(TextSize::Sm))
            .text_color(if self.active {
                colors.fg
            } else {
                colors.fg_muted
            })
            .when(self.active, |row| row.bg(colors.active))
            .when_some(select, |row, select| {
                let key = key.clone();
                row.cursor_pointer()
                    .when(!self.active, |row| row.hover(|row| row.bg(colors.hover)))
                    .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
                    .on_click(move |_, window, cx| {
                        log::info!("conversations: open {key}");
                        select(&key, window, cx)
                    })
            })
            .when(self.conversation.pinned, |row| {
                row.child(
                    Icon::new(IconName::Pin)
                        .size(IconSize::Xs)
                        .color(colors.fg_subtle),
                )
            })
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .child(Ellipsis::new(self.conversation.title.clone())),
            )
            .children(menu.map(|menu| {
                div()
                    .flex_none()
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .when(!shown, |slot| {
                        slot.invisible().group_hover(group, |style| style.visible())
                    })
                    .child(
                        OverflowMenu::new((self.id.clone(), "menu"), menu)
                            .tooltip("Conversation actions"),
                    )
            }))
    }
}

/// Conversations down a column, pinned first, then under their day, newest first; a query keeps those whose titles hold every word.
#[derive(IntoElement)]
pub struct ConversationList {
    id: ElementId,
    conversations: Vec<Conversation>,
    active: Option<SharedString>,
    query: SharedString,
    today: Date,
    on_select: Option<OnKey>,
    actions: Option<(OnKey, OnKey, OnKey)>,
}

impl ConversationList {
    /// `today` names the reader's day, so tests and captures hold still.
    pub fn new(
        id: impl Into<ElementId>,
        conversations: impl IntoIterator<Item = Conversation>,
        today: Date,
    ) -> Self {
        Self {
            id: id.into(),
            conversations: conversations.into_iter().collect(),
            active: None,
            query: SharedString::default(),
            today,
            on_select: None,
            actions: None,
        }
    }

    /// The conversation open, by key, and what a press on another asks.
    pub fn active(
        mut self,
        active: Option<SharedString>,
        on_select: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.active = active;
        self.on_select = Some(Rc::new(on_select));
        self
    }

    /// Keeps the conversations whose titles hold every word of `query`, such as a `forms::SearchInput`'s.
    pub fn query(mut self, query: impl Into<SharedString>) -> Self {
        self.query = query.into();
        self
    }

    /// Rename, pin and delete on each conversation.
    pub fn actions(
        mut self,
        on_rename: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
        on_pin: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
        on_delete: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.actions = Some((Rc::new(on_rename), Rc::new(on_pin), Rc::new(on_delete)));
        self
    }
}

impl RenderOnce for ConversationList {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let groups = grouped(
            &self.conversations,
            &self.query,
            self.today,
            &system_zone("conversation list"),
        );
        let empty = groups.is_empty();
        div()
            .id(self.id.clone())
            .size_full()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap_3()
            .p_2()
            .when(empty, |list| {
                list.child(
                    div()
                        .px_2()
                        .text_size(theme.text_size(TextSize::Sm))
                        .text_color(colors.fg_subtle)
                        .child(if self.query.trim().is_empty() {
                            "No conversations yet"
                        } else {
                            "No conversations match"
                        }),
                )
            })
            .children(groups.into_iter().map(|(name, items)| {
                div()
                    .flex()
                    .flex_col()
                    .gap_0p5()
                    .child(
                        div()
                            .px_2()
                            .pb_1()
                            .text_size(theme.text_size(TextSize::Xs))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(colors.fg_subtle)
                            .child(name),
                    )
                    .children(items.into_iter().map(|ix| {
                        let conversation = self.conversations[ix].clone();
                        let active = self.active.as_ref() == Some(&conversation.key);
                        let item = ConversationItem::new(
                            (self.id.clone(), format!("item-{}", conversation.key)),
                            conversation,
                        )
                        .active(active);
                        let item = match self.on_select.clone() {
                            Some(select) => {
                                item.on_select(move |key, window, cx| select(key, window, cx))
                            }
                            None => item,
                        };
                        match self.actions.clone() {
                            Some((rename, pin, delete)) => item.actions(
                                move |key, window, cx| rename(key, window, cx),
                                move |key, window, cx| pin(key, window, cx),
                                move |key, window, cx| delete(key, window, cx),
                            ),
                            None => item,
                        }
                    }))
            }))
    }
}

#[cfg(test)]
mod tests {
    use jiff::{ToSpan, civil::date};

    use super::*;

    fn conversation(key: &str, days_ago: i64, pinned: bool) -> Conversation {
        let noon = date(2026, 9, 26)
            .at(12, 0, 0, 0)
            .to_zoned(TimeZone::UTC)
            .expect("noon exists");
        Conversation {
            key: key.to_string().into(),
            title: format!("About {key}").into(),
            at: (noon - days_ago.days()).timestamp(),
            pinned,
        }
    }

    #[test]
    fn days_name_their_groups() {
        let today = date(2026, 9, 26);
        assert_eq!(group_name(today, today), "Today");
        assert_eq!(group_name(date(2026, 9, 25), today), "Yesterday");
        assert_eq!(group_name(date(2026, 9, 21), today), "Previous 7 days");
        assert_eq!(group_name(date(2026, 9, 1), today), "Previous 30 days");
        assert_eq!(group_name(date(2026, 3, 4), today), "March");
        assert_eq!(group_name(date(2025, 12, 31), today), "December 2025");
    }

    #[test]
    fn pinned_come_first_then_each_day_newest_first() {
        let all = [
            conversation("lift", 0, false),
            conversation("gamma", 3, true),
            conversation("dark", 1, false),
            conversation("type", 0, false),
        ];
        let groups = grouped(&all, "", date(2026, 9, 26), &TimeZone::UTC);
        let names: Vec<(&str, Vec<usize>)> = groups
            .iter()
            .map(|(name, items)| (name.as_str(), items.clone()))
            .collect();
        assert_eq!(
            names,
            [
                ("Pinned", vec![1]),
                ("Today", vec![0, 3]),
                ("Yesterday", vec![2])
            ]
        );
        let found = grouped(&all, "ABOUT dark", date(2026, 9, 26), &TimeZone::UTC);
        assert_eq!(found, [("Yesterday".to_string(), vec![2])]);
    }
}
