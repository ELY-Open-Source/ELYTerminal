use std::rc::Rc;

use gpui::{
    App, ElementId, ImageSource, IntoElement, ParentElement, RenderOnce, SharedString, Styled,
    Window, div, prelude::*,
};

use crate::{
    data_display::{Avatar, Badge, Presence},
    forms::OnValue,
    lists::{ListItem, Sections},
    theme::{ActiveTheme, AvatarSize, TextSize},
};

/// Someone in a chat: a key, a name, a picture, where they stand, a role, their own words.
#[derive(Clone)]
pub struct Member {
    pub key: SharedString,
    pub name: SharedString,
    pub picture: Option<ImageSource>,
    pub presence: Presence,
    pub role: Option<SharedString>,
    pub status: Option<SharedString>,
}

impl Member {
    pub fn new(
        key: impl Into<SharedString>,
        name: impl Into<SharedString>,
        presence: Presence,
    ) -> Self {
        Self {
            key: key.into(),
            name: name.into(),
            picture: None,
            presence,
            role: None,
            status: None,
        }
    }

    pub fn picture(mut self, source: impl Into<ImageSource>) -> Self {
        self.picture = Some(source.into());
        self
    }

    pub fn role(mut self, role: impl Into<SharedString>) -> Self {
        self.role = Some(role.into());
        self
    }

    pub fn status(mut self, status: impl Into<SharedString>) -> Self {
        self.status = Some(status.into());
        self
    }
}

/// Members online and offline by name, each group a list under its count. A press or an arrow chooses one, Enter or a double press acts on them; offline members go quiet.
#[derive(IntoElement)]
pub struct MemberList {
    id: ElementId,
    members: Vec<Member>,
    selected: Option<SharedString>,
    on_select: Option<OnValue>,
    on_activate: Option<OnValue>,
}

impl MemberList {
    pub fn new(id: impl Into<ElementId>, members: impl IntoIterator<Item = Member>) -> Self {
        let members: Vec<Member> = members.into_iter().collect();
        for (ix, member) in members.iter().enumerate() {
            let twice = members[..ix].iter().any(|other| other.key == member.key);
            assert!(!twice, "member {} twice", member.key);
        }
        Self {
            id: id.into(),
            members,
            selected: None,
            on_select: None,
            on_activate: None,
        }
    }

    /// The member chosen now, by key.
    pub fn selected(mut self, key: impl Into<SharedString>) -> Self {
        self.selected = Some(key.into());
        self
    }

    pub fn on_select(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_select = Some(Rc::new(handler));
        self
    }

    pub fn on_activate(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_activate = Some(Rc::new(handler));
        self
    }
}

/// The groups `members` fall in, online first, each by name whatever its case; empty groups drop out.
fn grouped(members: &[Member]) -> Vec<(&'static str, Vec<&Member>)> {
    let group = |online: bool| {
        let mut group: Vec<&Member> = members
            .iter()
            .filter(|member| (member.presence != Presence::Offline) == online)
            .collect();
        group.sort_by_key(|member| member.name.to_lowercase());
        group
    };
    [("Online", group(true)), ("Offline", group(false))]
        .into_iter()
        .filter(|(_, group)| !group.is_empty())
        .collect()
}

impl RenderOnce for MemberList {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        if let Some(selected) = &self.selected {
            let listed = self.members.iter().any(|member| member.key == *selected);
            if !listed {
                log::error!("member list: no member {selected}; none selected");
            }
        }
        let theme = cx.theme();
        let colors = &theme.colors;
        let row = |member: &Member| {
            let avatar = Avatar::new(
                (self.id.clone(), format!("avatar-{}", member.key)),
                member.name.clone(),
            )
            .size(AvatarSize::Sm)
            .presence(member.presence);
            let avatar = match member.picture.clone() {
                Some(picture) => avatar.image(picture),
                None => avatar,
            };
            let row = ListItem::new(
                (self.id.clone(), format!("member-{}", member.key)),
                member.name.clone(),
            )
            .leading(avatar)
            .quiet(member.presence == Presence::Offline);
            let row = match member.status.clone() {
                Some(status) => row.description(status),
                None => row,
            };
            let row = match member.role.clone() {
                Some(role) => row.trailing(Badge::new(role)),
                None => row,
            };
            (member.key.clone(), row)
        };
        let groups = grouped(&self.members).into_iter().fold(
            Sections::new(self.id.clone()).counted(),
            |groups, (title, group)| {
                groups.section(title.into(), group.into_iter().map(row).collect())
            },
        );
        let (select, activate): (OnValue, OnValue) = {
            let (id, acted, on_select, on_activate) = (
                self.id.clone(),
                self.id.clone(),
                self.on_select.clone(),
                self.on_activate.clone(),
            );
            (
                Rc::new(move |key, window, cx| {
                    log::info!("member list {id}: select {key}");
                    if let Some(on_select) = &on_select {
                        on_select(key, window, cx);
                    }
                }),
                Rc::new(move |key, window, cx| {
                    log::info!("member list {acted}: activate {key}");
                    if let Some(on_activate) = &on_activate {
                        on_activate(key, window, cx);
                    }
                }),
            )
        };
        let empty = self.members.is_empty().then(|| {
            div()
                .px_3()
                .py_4()
                .text_size(theme.text_size(TextSize::Sm))
                .text_color(colors.fg_muted)
                .child("No members yet.")
        });
        div()
            .debug_selector(|| "member-list".into())
            .children(empty)
            .when(!self.members.is_empty(), |list| {
                list.child(
                    groups
                        .selected(self.selected)
                        .on_select(select)
                        .on_activate(activate),
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each group's title with its members' names.
    fn names(members: &[Member]) -> Vec<(&'static str, Vec<String>)> {
        grouped(members)
            .into_iter()
            .map(|(title, group)| {
                let names = group.iter().map(|member| member.name.to_string()).collect();
                (title, names)
            })
            .collect()
    }

    #[test]
    fn members_group_online_first_by_name_and_empty_groups_drop() {
        let members = [
            Member::new("c", "Chloé", Presence::Busy),
            Member::new("d", "Dev", Presence::Offline),
            Member::new("a", "Ana", Presence::Away),
            Member::new("b", "Ben", Presence::Online),
            Member::new("e", "bea", Presence::Online),
        ];
        assert_eq!(
            names(&members),
            [
                (
                    "Online",
                    vec![
                        "Ana".to_string(),
                        "bea".into(),
                        "Ben".into(),
                        "Chloé".into()
                    ]
                ),
                ("Offline", vec!["Dev".into()])
            ]
        );
        assert_eq!(
            names(&members[1..2]),
            [("Offline", vec!["Dev".to_string()])]
        );
    }
}
