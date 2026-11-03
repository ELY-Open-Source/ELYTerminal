use std::rc::Rc;

use gpui::{
    AnyElement, App, ElementId, FontWeight, IntoElement, ParentElement, RenderOnce, SharedString,
    Styled, Window, div,
};

use crate::{
    buttons::{Button, ButtonVariant, CopyButton, IconButton},
    data_display::Avatar,
    documents::source,
    forms::{Choice, OnValues, Pick, Run, Select, TagInput, is_email},
    overlays::Dialog,
    primitives::{Icon, IconName},
    theme::{ActiveTheme, AvatarSize, ControlSize, IconSize, TextSize},
    typography::Ellipsis,
};

/// What someone may do with a shared document.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    View,
    Comment,
    Edit,
}

const ROLES: [(Role, &str, &str); 3] = [
    (Role::View, "view", "Can view"),
    (Role::Comment, "comment", "Can comment"),
    (Role::Edit, "edit", "Can edit"),
];

type OnRole = Rc<dyn Fn(Role, &mut Window, &mut App)>;

/// The words for what `role` may do.
fn role_label(role: Role) -> &'static str {
    let (_, _, label) = ROLES
        .into_iter()
        .find(|(listed, ..)| *listed == role)
        .expect("every role is listed");
    label
}

/// Picks what someone may do: view, comment or edit.
#[derive(IntoElement)]
pub struct PermissionSelect {
    id: ElementId,
    role: Role,
    on_change: OnRole,
}

impl PermissionSelect {
    pub fn new(
        id: impl Into<ElementId>,
        role: Role,
        on_change: impl Fn(Role, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            role,
            on_change: Rc::new(on_change),
        }
    }
}

impl RenderOnce for PermissionSelect {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let (_, value, _) = ROLES
            .into_iter()
            .find(|(role, ..)| *role == self.role)
            .expect("every role is listed");
        let on_change = self.on_change;
        Select::new(
            self.id,
            ROLES.map(|(_, value, label)| Choice::new(value, label)),
        )
        .selected(value)
        .size(ControlSize::Sm)
        .on_change(move |value, window, cx| {
            let (role, ..) = ROLES
                .into_iter()
                .find(|(_, named, _)| *named == value.as_ref())
                .expect("a listed role");
            log::info!("permission: {role:?}");
            on_change(role, window, cx)
        })
    }
}

/// Someone with access: name, email, picture, and what they may do; the owner's access stays.
#[derive(Clone, Debug, PartialEq)]
pub struct Member {
    pub name: SharedString,
    pub email: SharedString,
    pub picture: Option<SharedString>,
    pub role: Role,
    pub owner: bool,
}

type OnMemberRole = Rc<dyn Fn(usize, Role, &mut Window, &mut App)>;

/// Who has access, each with what they may do: change it or take it away; the owner stays.
#[derive(IntoElement)]
pub struct AccessList {
    id: ElementId,
    members: Vec<Member>,
    on_role: Option<OnMemberRole>,
    on_remove: Option<Pick>,
}

impl AccessList {
    pub fn new(id: impl Into<ElementId>, members: impl IntoIterator<Item = Member>) -> Self {
        Self {
            id: id.into(),
            members: members.into_iter().collect(),
            on_role: None,
            on_remove: None,
        }
    }

    /// Gets a member's index and their new role.
    pub fn on_role(
        mut self,
        handler: impl Fn(usize, Role, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_role = Some(Rc::new(handler));
        self
    }

    /// Gets the index of the member to take away.
    pub fn on_remove(mut self, handler: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_remove = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for AccessList {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        div()
            .flex()
            .flex_col()
            .gap_1()
            .children(self.members.iter().enumerate().map(|(ix, member)| {
                let id = |what: &str| (self.id.clone(), format!("{what}-{ix}"));
                let avatar = Avatar::new(id("avatar"), member.name.clone()).size(AvatarSize::Sm);
                let avatar = match &member.picture {
                    Some(picture) => avatar.image(source(picture)),
                    None => avatar,
                };
                let access: AnyElement = if member.owner {
                    div()
                        .px_2()
                        .text_size(theme.text_size(TextSize::Sm))
                        .text_color(colors.fg_muted)
                        .child("Owner")
                        .into_any_element()
                } else {
                    let (on_role, on_remove) = (self.on_role.clone(), self.on_remove.clone());
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .child(match on_role {
                            Some(on_role) => PermissionSelect::new(
                                id("role"),
                                member.role,
                                move |role, window, cx| on_role(ix, role, window, cx),
                            )
                            .into_any_element(),
                            None => div()
                                .px_2()
                                .text_size(theme.text_size(TextSize::Sm))
                                .text_color(colors.fg_muted)
                                .child(role_label(member.role))
                                .into_any_element(),
                        })
                        .children(on_remove.map(|on_remove| {
                            IconButton::new(id("remove"), IconName::X)
                                .variant(ButtonVariant::Ghost)
                                .size(ControlSize::Sm)
                                .tooltip("Remove access")
                                .on_click(move |_, window, cx| {
                                    log::info!("access list: remove {ix}");
                                    on_remove(ix, window, cx)
                                })
                        }))
                        .into_any_element()
                };
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .py_1()
                    .child(avatar)
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .text_size(theme.text_size(TextSize::Sm))
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(Ellipsis::new(member.name.clone())),
                            )
                            .child(
                                div()
                                    .text_size(theme.text_size(TextSize::Xs))
                                    .text_color(colors.fg_subtle)
                                    .child(Ellipsis::new(member.email.clone())),
                            ),
                    )
                    .child(access)
            }))
    }
}

/// Whether every email reads as an address, and there is one.
pub(crate) fn ready(emails: &[SharedString]) -> bool {
    !emails.is_empty() && emails.iter().all(|email| is_email(email))
}

/// People to invite by email, each a chip that reads as danger until it is an address, what they may do, and a button that sends once every one is.
#[derive(IntoElement)]
pub struct InviteInput {
    id: ElementId,
    emails: Vec<SharedString>,
    on_change: OnValues,
    on_invite: Run,
    role: Option<(Role, OnRole)>,
}

impl InviteInput {
    /// `on_change` gets the emails as they are added and removed; `on_invite` sends them.
    pub fn new(
        id: impl Into<ElementId>,
        emails: impl IntoIterator<Item = impl Into<SharedString>>,
        on_change: impl Fn(&[SharedString], &mut Window, &mut App) + 'static,
        on_invite: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            emails: emails.into_iter().map(Into::into).collect(),
            on_change: Rc::new(on_change),
            on_invite: Rc::new(on_invite),
            role: None,
        }
    }

    /// What the people invited may do, and what a new choice asks.
    pub fn role(
        mut self,
        role: Role,
        on_role: impl Fn(Role, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.role = Some((role, Rc::new(on_role)));
        self
    }
}

impl RenderOnce for InviteInput {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let (on_change, invite) = (self.on_change, self.on_invite);
        let count = self.emails.len();
        let label = cx.theme().label_width();
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                TagInput::new((self.id.clone(), "emails"), self.emails.clone())
                    .placeholder("Add people by email")
                    .check(is_email)
                    .on_change(move |emails, window, cx| on_change(&emails, window, cx)),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_end()
                    .gap_2()
                    .children(self.role.map(|(role, on_role)| {
                        div().w(label).child(PermissionSelect::new(
                            (self.id.clone(), "role"),
                            role,
                            move |role, window, cx| on_role(role, window, cx),
                        ))
                    }))
                    .child(
                        Button::new((self.id.clone(), "invite"), "Invite")
                            .variant(ButtonVariant::Primary)
                            .icon(IconName::UserPlus)
                            .disabled(!ready(&self.emails))
                            .on_click(move |_, window, cx| {
                                log::info!("invite: {count} people");
                                invite(window, cx)
                            }),
                    ),
            )
    }
}

type OnAccess = Rc<dyn Fn(Option<Role>, &mut Window, &mut App)>;

const RESTRICTED: &str = "restricted";
const ANYONE: &str = "anyone";

/// Who a link lets in: only the people added, or anyone who has it, at a role; and the link to copy.
#[derive(IntoElement)]
pub struct LinkAccess {
    id: ElementId,
    link: SharedString,
    anyone: Option<Role>,
    on_change: OnAccess,
}

impl LinkAccess {
    /// `anyone` is the role the link gives, none when only people added get in.
    pub fn new(
        id: impl Into<ElementId>,
        link: impl Into<SharedString>,
        anyone: Option<Role>,
        on_change: impl Fn(Option<Role>, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            link: link.into(),
            anyone,
            on_change: Rc::new(on_change),
        }
    }
}

impl RenderOnce for LinkAccess {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let (open, role) = (self.on_change.clone(), self.on_change);
        let anyone = self.anyone;
        div()
            .flex()
            .items_center()
            .gap_3()
            .child(
                div()
                    .flex_none()
                    .size(theme.avatar_size(AvatarSize::Sm))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_full()
                    .bg(colors.hover)
                    .child(
                        Icon::new(if anyone.is_some() {
                            IconName::Globe
                        } else {
                            IconName::Lock
                        })
                        .size(IconSize::Sm)
                        .color(colors.fg_muted),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap_0p5()
                    .child(
                        Select::new(
                            (self.id.clone(), "who"),
                            [
                                Choice::new(RESTRICTED, "Only people added"),
                                Choice::new(ANYONE, "Anyone with the link"),
                            ],
                        )
                        .selected(if anyone.is_some() { ANYONE } else { RESTRICTED })
                        .size(ControlSize::Sm)
                        .on_change(move |value, window, cx| {
                            let next =
                                (value.as_ref() == ANYONE).then_some(anyone.unwrap_or(Role::View));
                            log::info!("link access: {next:?}");
                            open(next, window, cx)
                        }),
                    )
                    .child(
                        div()
                            .text_size(theme.text_size(TextSize::Xs))
                            .text_color(colors.fg_subtle)
                            .child(if anyone.is_some() {
                                "Anyone on the internet with the link"
                            } else {
                                "Only people with access can open it"
                            }),
                    ),
            )
            .children(anyone.map(|now| {
                PermissionSelect::new((self.id.clone(), "role"), now, move |next, window, cx| {
                    role(Some(next), window, cx)
                })
            }))
            .child(CopyButton::new(
                (self.id.clone(), "copy"),
                self.link.clone(),
            ))
    }
}

/// Sharing a document in one place: invite people, see and change who has access, choose who the link lets in, and copy it.
#[derive(IntoElement)]
pub struct ShareDialog {
    id: ElementId,
    title: SharedString,
    on_close: Run,
    invite: Option<AnyElement>,
    access: Option<AnyElement>,
    link: Option<AnyElement>,
}

impl ShareDialog {
    /// Render it while open; `on_close` runs on Done, Escape or a press outside.
    pub fn new(
        id: impl Into<ElementId>,
        title: impl Into<SharedString>,
        on_close: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            on_close: Rc::new(on_close),
            invite: None,
            access: None,
            link: None,
        }
    }

    pub fn invite(mut self, invite: InviteInput) -> Self {
        self.invite = Some(invite.into_any_element());
        self
    }

    pub fn access(mut self, access: AccessList) -> Self {
        self.access = Some(access.into_any_element());
        self
    }

    pub fn link(mut self, link: LinkAccess) -> Self {
        self.link = Some(link.into_any_element());
        self
    }
}

impl RenderOnce for ShareDialog {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let heading = |text: &'static str| {
            div()
                .text_size(theme.text_size(TextSize::Sm))
                .font_weight(FontWeight::MEDIUM)
                .text_color(colors.fg_muted)
                .child(text)
        };
        let section = |title: &'static str, body: Option<AnyElement>| {
            body.map(|body| {
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(heading(title))
                    .child(body)
            })
        };
        let (on_close, done) = (self.on_close.clone(), (self.id.clone(), "done"));
        Dialog::new(
            self.id.clone(),
            format!("Share “{}”", self.title),
            move |window, cx| on_close(window, cx),
        )
        .child(
            div()
                .w_full()
                .flex()
                .flex_col()
                .gap_5()
                .children(self.invite)
                .children(section("People with access", self.access))
                .children(section("General access", self.link)),
        )
        .action(move |close| {
            Button::new(done, "Done")
                .variant(ButtonVariant::Primary)
                .on_click(move |_, window, cx| close(window, cx))
        })
    }
}
