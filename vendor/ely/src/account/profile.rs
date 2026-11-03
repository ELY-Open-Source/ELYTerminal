use std::{path::PathBuf, rc::Rc};

use gpui::{
    AnyElement, App, AppContext as _, ElementId, Entity, FontWeight, ImageSource, IntoElement,
    ParentElement, RenderOnce, SharedString, Styled, Window, div,
};

use crate::{
    buttons::{Button, ButtonVariant},
    data_display::{Avatar, Presence},
    forms::{AvatarUpload, EmailInput, FormField, Input, TextInput, is_email},
    layout::Card,
    primitives::{Icon, IconName},
    theme::{ActiveTheme, AvatarSize, IconSize, TextSize},
    typography::Ellipsis,
};

/// Someone at a glance: their picture or initials, name and title, a line about them, details each with an icon, and the owner's actions.
#[derive(IntoElement)]
pub struct ProfileCard {
    id: ElementId,
    name: SharedString,
    title: Option<SharedString>,
    image: Option<ImageSource>,
    presence: Option<Presence>,
    bio: Option<SharedString>,
    details: Vec<(IconName, SharedString)>,
    actions: Vec<AnyElement>,
}

impl ProfileCard {
    pub fn new(id: impl Into<ElementId>, name: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            title: None,
            image: None,
            presence: None,
            bio: None,
            details: Vec::new(),
            actions: Vec::new(),
        }
    }

    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn image(mut self, source: impl Into<ImageSource>) -> Self {
        self.image = Some(source.into());
        self
    }

    pub fn presence(mut self, presence: Presence) -> Self {
        self.presence = Some(presence);
        self
    }

    pub fn bio(mut self, bio: impl Into<SharedString>) -> Self {
        self.bio = Some(bio.into());
        self
    }

    /// A line such as an email or a place, after its icon.
    pub fn detail(mut self, icon: IconName, text: impl Into<SharedString>) -> Self {
        self.details.push((icon, text.into()));
        self
    }

    pub fn action(mut self, action: impl IntoElement) -> Self {
        self.actions.push(action.into_any_element());
        self
    }
}

impl RenderOnce for ProfileCard {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let mut avatar =
            Avatar::new((self.id.clone(), "avatar"), self.name.clone()).size(AvatarSize::Lg);
        if let Some(image) = self.image {
            avatar = avatar.image(image);
        }
        if let Some(presence) = self.presence {
            avatar = avatar.presence(presence);
        }
        let muted = theme.colors.fg_muted;
        let details = self.details.into_iter().map(|(icon, text)| {
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(Icon::new(icon).size(IconSize::Sm).color(muted))
                .child(div().flex_1().min_w_0().child(Ellipsis::new(text)))
        });
        Card::new().child(
            div()
                .flex()
                .flex_col()
                .gap_4()
                .text_size(theme.text_size(TextSize::Sm))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_3()
                        .child(div().flex_none().child(avatar))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .child(
                                    div()
                                        .text_size(theme.text_size(TextSize::Md))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .child(Ellipsis::new(self.name)),
                                )
                                .children(self.title.map(|title| {
                                    div().text_color(muted).child(Ellipsis::new(title))
                                })),
                        ),
                )
                .children(
                    self.bio
                        .map(|bio| div().flex().child(div().flex_1().min_w_0().child(bio))),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_1p5()
                        .text_color(muted)
                        .children(details),
                )
                .children(
                    (!self.actions.is_empty())
                        .then(|| div().flex().flex_wrap().gap_2().children(self.actions)),
                ),
        )
    }
}

/// What a profile holds.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Profile {
    pub name: SharedString,
    pub username: SharedString,
    pub email: SharedString,
    pub bio: SharedString,
    pub location: SharedString,
    pub picture: Option<PathBuf>,
}

type OnProfile = Rc<dyn Fn(&Profile, &mut Window, &mut App)>;

/// The editor's own fields, filled from the profile they started as.
struct Fields {
    seed: Profile,
    name: Entity<TextInput>,
    username: Entity<TextInput>,
    email: Entity<TextInput>,
    bio: Entity<TextInput>,
    location: Entity<TextInput>,
    picture: Option<PathBuf>,
}

impl Fields {
    fn filled(profile: &Profile, window: &mut Window, cx: &mut App) -> Self {
        let mut field = |text: &SharedString, lines: bool| {
            let text = text.to_string();
            cx.new(|cx| {
                let input = TextInput::new(window, cx);
                let mut input = if lines { input.multi_line(3, 6) } else { input };
                input.set_text(text, cx);
                input
            })
        };
        Self {
            seed: profile.clone(),
            name: field(&profile.name, false),
            username: field(&profile.username, false),
            email: field(&profile.email, false),
            bio: field(&profile.bio, true),
            location: field(&profile.location, false),
            picture: profile.picture.clone(),
        }
    }

    fn read(&self, cx: &App) -> Profile {
        let text = |input: &Entity<TextInput>| {
            SharedString::from(input.read(cx).text().trim().to_string())
        };
        Profile {
            name: text(&self.name),
            username: text(&self.username),
            email: text(&self.email),
            bio: text(&self.bio),
            location: text(&self.location),
            picture: self.picture.clone(),
        }
    }
}

/// Whether an edited profile may be saved: it changed, it has a name, and its email reads.
fn savable(edited: &Profile, seed: &Profile) -> bool {
    edited != seed && !edited.name.is_empty() && is_email(&edited.email)
}

/// A profile to edit: the picture, name, username, email, a line about oneself and a place. A new profile from the owner refills it. Save hands the owner the changes and rests while nothing changed, a name is missing or the email does not read; Discard puts the owner's profile back.
#[derive(IntoElement)]
pub struct ProfileEditor {
    id: ElementId,
    profile: Profile,
    on_save: OnProfile,
}

impl ProfileEditor {
    pub fn new(
        id: impl Into<ElementId>,
        profile: Profile,
        on_save: impl Fn(&Profile, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            profile,
            on_save: Rc::new(on_save),
        }
    }
}

impl RenderOnce for ProfileEditor {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let (id, profile) = (self.id, self.profile);
        let on_save = self.on_save;
        let fields = window.use_keyed_state((id.clone(), "fields"), cx, |window, cx| {
            Fields::filled(&profile, window, cx)
        });
        if fields.read(cx).seed != profile {
            log::info!("profile editor: filled from the owner's profile");
            let fresh = Fields::filled(&profile, window, cx);
            fields.update(cx, |fields, _| *fields = fresh);
        }
        let now = fields.read(cx);
        let edited = now.read(cx);
        let ready = savable(&edited, &profile);
        let changed = edited != profile;
        let (name, username, email, bio, location) = (
            now.name.clone(),
            now.username.clone(),
            now.email.clone(),
            now.bio.clone(),
            now.location.clone(),
        );
        let mut avatar =
            Avatar::new((id.clone(), "avatar"), edited.name.clone()).size(AvatarSize::Xl);
        if let Some(picture) = &edited.picture {
            avatar = avatar.image(picture.as_path());
        }
        let [pictured, discarded] = [(); 2].map(|_| fields.clone());
        let seed = profile.clone();
        div()
            .flex()
            .flex_col()
            .gap_5()
            .child(
                AvatarUpload::new((id.clone(), "picture"), avatar).on_change(move |path, _, cx| {
                    log::info!("profile editor: a new picture");
                    pictured.update(cx, |fields, cx| {
                        fields.picture = Some(path);
                        cx.notify();
                    })
                }),
            )
            .child(
                FormField::new((id.clone(), "name-field"), "Name")
                    .required()
                    .child(Input::new(&name)),
            )
            .child(
                FormField::new((id.clone(), "username-field"), "Username")
                    .child(Input::new(&username).prefix("@")),
            )
            .child(
                FormField::new((id.clone(), "email-field"), "Email")
                    .required()
                    .child(EmailInput::new(&email)),
            )
            .child(
                FormField::new((id.clone(), "bio-field"), "About")
                    .description("A line or two others see.")
                    .child(Input::new(&bio)),
            )
            .child(
                FormField::new((id.clone(), "location-field"), "Location")
                    .child(Input::new(&location)),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .justify_end()
                    .gap_2()
                    .child(
                        Button::new((id.clone(), "discard"), "Discard")
                            .variant(ButtonVariant::Ghost)
                            .disabled(!changed)
                            .on_click(move |_, window, cx| {
                                log::info!("profile editor: discard");
                                let fresh = Fields::filled(&seed, window, cx);
                                discarded.update(cx, |fields, cx| {
                                    *fields = fresh;
                                    cx.notify();
                                })
                            }),
                    )
                    .child(
                        Button::new((id.clone(), "save"), "Save")
                            .variant(ButtonVariant::Primary)
                            .disabled(!ready)
                            .on_click(move |_, window, cx| {
                                log::info!("profile editor: save");
                                on_save(&edited, window, cx);
                            }),
                    ),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::{Profile, savable};

    #[test]
    fn a_profile_saves_once_changed_named_and_reachable() {
        let seed = Profile {
            name: "Ada".into(),
            email: "ada@example.com".into(),
            ..Profile::default()
        };
        let with = |edit: fn(&mut Profile)| {
            let mut edited = seed.clone();
            edit(&mut edited);
            savable(&edited, &seed)
        };
        assert!(!with(|_| {}), "nothing changed");
        assert!(with(|profile| profile.location = "London".into()));
        assert!(
            !with(|profile| profile.name = "".into()),
            "a name is needed"
        );
        assert!(
            !with(|profile| profile.email = "ada".into()),
            "the email must read"
        );
    }
}
