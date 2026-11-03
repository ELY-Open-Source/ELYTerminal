use std::rc::Rc;

use gpui::{
    App, ElementId, ImageSource, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    SharedString, Styled, Window, div, prelude::*,
};

use super::login::field;
use crate::{
    buttons::{Button, ButtonVariant, ConfirmButton, ConfirmMode},
    data_display::{Avatar, Badge, Tone},
    forms::{Choice, EmailInput, Enter, Select, is_email},
    theme::{ActiveTheme, AvatarSize, ControlSize, TextSize},
    typography::Ellipsis,
};

type OnKey = Rc<dyn Fn(&SharedString, &mut Window, &mut App)>;
type OnPair = Rc<dyn Fn(&SharedString, &SharedString, &mut Window, &mut App)>;

/// A role: its key, its name and what it may do.
#[derive(Clone, Debug, PartialEq)]
pub struct Role {
    pub key: SharedString,
    pub name: SharedString,
    pub description: SharedString,
}

/// The role of `key`; none, logged for `owner`, when the roles lost it.
fn role_named<'a>(roles: &'a [Role], key: &SharedString, owner: &str) -> Option<&'a Role> {
    let role = roles.iter().find(|role| role.key == *key);
    if role.is_none() {
        log::error!("{owner}: no role {key}; none shown");
    }
    role
}

/// A role to pick, each with what it may do beneath its name.
#[derive(IntoElement)]
pub struct RoleSelector {
    id: ElementId,
    roles: Vec<Role>,
    selected: SharedString,
    disabled: bool,
    on_change: OnKey,
}

impl RoleSelector {
    pub fn new(
        id: impl Into<ElementId>,
        roles: impl IntoIterator<Item = Role>,
        selected: impl Into<SharedString>,
        on_change: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        let (roles, selected): (Vec<_>, SharedString) =
            (roles.into_iter().collect(), selected.into());
        role_named(&roles, &selected, "role selector");
        Self {
            id: id.into(),
            roles,
            selected,
            disabled: false,
            on_change: Rc::new(on_change),
        }
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl RenderOnce for RoleSelector {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let id = self.id;
        let on_change = self.on_change;
        let choices = self
            .roles
            .into_iter()
            .map(|role| Choice::new(role.key, role.name).note(role.description));
        Select::new(id, choices)
            .selected(self.selected)
            .disabled(self.disabled)
            .size(ControlSize::Sm)
            .on_change(move |key, window, cx| {
                log::info!("role selector: {key}");
                on_change(key, window, cx);
            })
    }
}

/// Someone on a team: their key, name and email, their role's key, when they were last seen as words, a picture when there is one, and whether they own the team.
#[derive(Clone)]
pub struct Member {
    pub key: SharedString,
    pub name: SharedString,
    pub email: SharedString,
    pub role: SharedString,
    pub seen: SharedString,
    pub image: Option<ImageSource>,
    pub owner: bool,
}

/// A team's members: each with their picture, name and email and when last seen, a role to change and a Remove that asks twice. The owner's role stays, and the owner stays.
#[derive(IntoElement)]
pub struct TeamMemberTable {
    id: ElementId,
    members: Vec<Member>,
    roles: Vec<Role>,
    on_role: OnPair,
    on_remove: OnKey,
}

impl TeamMemberTable {
    pub fn new(
        id: impl Into<ElementId>,
        members: impl IntoIterator<Item = Member>,
        roles: impl IntoIterator<Item = Role>,
        on_role: impl Fn(&SharedString, &SharedString, &mut Window, &mut App) + 'static,
        on_remove: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        let (members, roles): (Vec<Member>, Vec<Role>) =
            (members.into_iter().collect(), roles.into_iter().collect());
        Self {
            id: id.into(),
            members,
            roles,
            on_role: Rc::new(on_role),
            on_remove: Rc::new(on_remove),
        }
    }
}

impl RenderOnce for TeamMemberTable {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id;
        let on_role = self.on_role;
        let on_remove = self.on_remove;
        let theme = cx.theme();
        let rows = self.members.into_iter().enumerate().map(|(ix, member)| {
            let mut avatar = Avatar::new(
                (id.clone(), format!("avatar-{}", member.key)),
                member.name.clone(),
            )
            .size(AvatarSize::Sm);
            if let Some(image) = member.image {
                avatar = avatar.image(image);
            }
            let (role, remove) = match member.owner {
                true => (
                    div()
                        .text_color(theme.colors.fg_muted)
                        .child("Owner")
                        .into_any_element(),
                    None,
                ),
                false => {
                    let [changed, removed] = [(); 2].map(|_| member.key.clone());
                    let (set, drop) = (on_role.clone(), on_remove.clone());
                    let select = RoleSelector::new(
                        (id.clone(), format!("role-{}", member.key)),
                        self.roles.clone(),
                        member.role.clone(),
                        move |role, window, cx| set(&changed, role, window, cx),
                    );
                    let button = ConfirmButton::new(
                        (id.clone(), format!("remove-{}", member.key)),
                        "Remove",
                        ConfirmMode::Twice,
                    )
                    .size(ControlSize::Sm)
                    .on_confirm(move |window, cx| {
                        log::info!("team member table: remove {removed}");
                        drop(&removed, window, cx);
                    });
                    (select.into_any_element(), Some(button))
                }
            };
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap_x_3()
                .gap_y_2()
                .py_2p5()
                .when(ix > 0, |row| {
                    row.border_t_1().border_color(theme.colors.border)
                })
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_3()
                        .flex_1()
                        .min_w(theme.label_width())
                        .child(div().flex_none().child(avatar))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .child(Ellipsis::new(member.name))
                                .child(
                                    div()
                                        .text_size(theme.text_size(TextSize::Xs))
                                        .text_color(theme.colors.fg_muted)
                                        .child(Ellipsis::new(format!(
                                            "{} · {}",
                                            member.email, member.seen
                                        ))),
                                ),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .flex_none()
                        .child(div().w(theme.label_width()).child(role))
                        .children(remove),
                )
        });
        div()
            .flex()
            .flex_col()
            .text_size(theme.text_size(TextSize::Sm))
            .children(rows)
    }
}

/// An invitation on its way: its key, the address, the role's key, when it went as words, and whether it has lapsed.
#[derive(Clone, Debug, PartialEq)]
pub struct Invitation {
    pub key: SharedString,
    pub email: SharedString,
    pub role: SharedString,
    pub sent: SharedString,
    pub expired: bool,
}

/// Invitations out, each with its role and when it went, and a Resend and a Revoke; above them, an address and a role to invite someone new. Invite waits for an address that reads.
#[derive(IntoElement)]
pub struct InvitationList {
    id: ElementId,
    invitations: Vec<Invitation>,
    roles: Vec<Role>,
    on_invite: OnPair,
    on_resend: OnKey,
    on_revoke: OnKey,
}

impl InvitationList {
    pub fn new(
        id: impl Into<ElementId>,
        invitations: impl IntoIterator<Item = Invitation>,
        roles: impl IntoIterator<Item = Role>,
        on_invite: impl Fn(&SharedString, &SharedString, &mut Window, &mut App) + 'static,
        on_resend: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
        on_revoke: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        let (invitations, roles): (Vec<Invitation>, Vec<Role>) = (
            invitations.into_iter().collect(),
            roles.into_iter().collect(),
        );
        assert!(!roles.is_empty(), "invitation list: no role to invite as");
        Self {
            id: id.into(),
            invitations,
            roles,
            on_invite: Rc::new(on_invite),
            on_resend: Rc::new(on_resend),
            on_revoke: Rc::new(on_revoke),
        }
    }
}

impl RenderOnce for InvitationList {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id;
        let on_invite = self.on_invite;
        let on_resend = self.on_resend;
        let on_revoke = self.on_revoke;
        let email = field(&id, "email", "name@example.com", false, window, cx);
        let first = self.roles[0].key.clone();
        let role = window.use_keyed_state((id.clone(), "role"), cx, move |_, _| first);
        let (address, chosen): (SharedString, SharedString) = (
            email.read(cx).text().trim().to_string().into(),
            role.read(cx).clone(),
        );
        let offered = self.roles.iter().any(|role| role.key == chosen);
        let ready = is_email(&address) && offered;
        let clear = email.clone();
        let invite = Rc::new(move |window: &mut Window, cx: &mut App| {
            log::info!("invitation list: invite as {chosen}");
            on_invite(&address, &chosen, window, cx);
            clear.update(cx, |field, cx| field.set_text("", cx));
        });
        let enter = invite.clone();
        let theme = cx.theme();
        let rows =
            self.invitations.iter().map(|invitation| {
                let named = role_named(&self.roles, &invitation.role, "invitation list")
                    .map_or_else(|| SharedString::from("No role"), |role| role.name.clone());
                let [again, gone] = [(); 2].map(|_| invitation.key.clone());
                let (resend, revoke) = (on_resend.clone(), on_revoke.clone());
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap_x_3()
                    .gap_y_2()
                    .py_2p5()
                    .border_t_1()
                    .border_color(theme.colors.border)
                    .child(
                        div()
                            .flex_1()
                            .min_w(theme.label_width())
                            .child(Ellipsis::new(invitation.email.clone()))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .text_size(theme.text_size(TextSize::Xs))
                                    .text_color(theme.colors.fg_muted)
                                    .child(div().flex_1().min_w_0().child(Ellipsis::new(format!(
                                        "{named} · {}",
                                        invitation.sent
                                    ))))
                                    .children(invitation.expired.then(|| {
                                        div()
                                            .flex_none()
                                            .child(Badge::new("Expired").tone(Tone::Warning))
                                    })),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_none()
                            .gap_1()
                            .child(
                                Button::new(
                                    (id.clone(), format!("resend-{}", invitation.key)),
                                    "Resend",
                                )
                                .variant(ButtonVariant::Ghost)
                                .size(ControlSize::Sm)
                                .on_click(move |_, window, cx| {
                                    log::info!("invitation list: resend {again}");
                                    resend(&again, window, cx);
                                }),
                            )
                            .child(
                                Button::new(
                                    (id.clone(), format!("revoke-{}", invitation.key)),
                                    "Revoke",
                                )
                                .variant(ButtonVariant::Ghost)
                                .size(ControlSize::Sm)
                                .on_click(move |_, window, cx| {
                                    log::info!("invitation list: revoke {gone}");
                                    revoke(&gone, window, cx);
                                }),
                            ),
                    )
            });
        let picked = role.clone();
        div()
            .flex()
            .flex_col()
            .gap_3()
            .text_size(theme.text_size(TextSize::Sm))
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_2()
                    .capture_action(move |_: &Enter, window, cx| {
                        if ready {
                            cx.stop_propagation();
                            enter(window, cx);
                        }
                    })
                    .child(
                        div()
                            .flex_1()
                            .min_w(theme.label_width())
                            .child(EmailInput::new(&email)),
                    )
                    .child(div().w(theme.label_width()).child(RoleSelector::new(
                        (id.clone(), "invite-role"),
                        self.roles.clone(),
                        role.read(cx).clone(),
                        move |key, _, cx| {
                            picked.update(cx, |role, cx| {
                                *role = key.clone();
                                cx.notify();
                            })
                        },
                    )))
                    .child(
                        Button::new((id.clone(), "invite"), "Invite")
                            .variant(ButtonVariant::Primary)
                            .disabled(!ready)
                            .on_click(move |_, window, cx| invite(window, cx)),
                    ),
            )
            .child(div().flex().flex_col().children(rows))
    }
}
