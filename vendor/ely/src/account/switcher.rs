use std::rc::Rc;

use gpui::{
    AnyElement, App, Div, ElementId, FontWeight, ImageSource, InteractiveElement, IntoElement,
    MouseButton, ParentElement, RenderOnce, SharedString, Stateful, StatefulInteractiveElement,
    Styled, Window, div, transparent_black,
};

use super::login::Run;
use crate::{
    data_display::{Avatar, Presence},
    menus::{Menu, MenuItem, menu_under},
    primitives::{FocusRing, Icon, IconName},
    theme::{ActiveTheme, AvatarSize, IconSize, Radius, TextSize},
    typography::Ellipsis,
};

type OnKey = Rc<dyn Fn(&SharedString, &mut Window, &mut App)>;

/// Someone signed in: a key, a name and an email, and a picture when there is one.
#[derive(Clone)]
pub struct Account {
    pub key: SharedString,
    pub name: SharedString,
    pub email: SharedString,
    pub image: Option<ImageSource>,
}

/// A workspace or an organization: a key, a name, and a line such as its plan.
#[derive(Clone, Debug, PartialEq)]
pub struct Workspace {
    pub key: SharedString,
    pub name: SharedString,
    pub detail: SharedString,
}

/// A name over a muted line, each cut to the room there is.
fn named(title: SharedString, detail: SharedString, cx: &App) -> Div {
    let theme = cx.theme();
    div()
        .flex_1()
        .min_w_0()
        .child(
            div()
                .font_weight(FontWeight::MEDIUM)
                .child(Ellipsis::new(title)),
        )
        .child(
            div()
                .text_size(theme.text_size(TextSize::Xs))
                .text_color(theme.colors.fg_muted)
                .child(Ellipsis::new(detail)),
        )
}

/// A row that opens a menu: what it shows, then a chevron. A press leaves focus where it was; Tab reaches it.
fn row(id: impl Into<ElementId>, lead: impl IntoElement, text: Div, cx: &App) -> Stateful<Div> {
    let theme = cx.theme();
    let hover = theme.colors.hover;
    div()
        .id(id)
        .flex()
        .items_center()
        .gap_2()
        .px_2()
        .py_1p5()
        .rounded(theme.radius(Radius::Md))
        .border_1()
        .border_color(transparent_black())
        .text_size(theme.text_size(TextSize::Sm))
        .tab_index(0)
        .focus_ring(cx)
        .hover(move |style| style.bg(hover))
        .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
        .child(div().flex_none().child(lead))
        .child(text)
        .child(
            Icon::new(IconName::ChevronsUpDown)
                .size(IconSize::Sm)
                .color(theme.colors.fg_subtle),
        )
}

/// `trigger` opening `menu` under it.
fn opening(
    id: ElementId,
    menu: &Menu,
    trigger: Stateful<Div>,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    menu_under(
        id,
        menu,
        move |click| {
            trigger
                .on_click(move |event, window, cx| click(event, window, cx))
                .into_any_element()
        },
        window,
        cx,
    )
    .into_any_element()
}

/// The accounts signed in, the current one showing; its menu picks another or adds one.
#[derive(IntoElement)]
pub struct AccountSwitcher {
    id: ElementId,
    accounts: Vec<Account>,
    current: SharedString,
    on_select: OnKey,
    on_add: Option<Run>,
}

impl AccountSwitcher {
    pub fn new(
        id: impl Into<ElementId>,
        accounts: impl IntoIterator<Item = Account>,
        current: impl Into<SharedString>,
        on_select: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        let (accounts, current): (Vec<_>, SharedString) =
            (accounts.into_iter().collect(), current.into());
        Self {
            id: id.into(),
            accounts,
            current,
            on_select: Rc::new(on_select),
            on_add: None,
        }
    }

    /// Shows "Add account".
    pub fn on_add(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_add = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for AccountSwitcher {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id;
        let on_select = self.on_select;
        let items = self.accounts.iter().map(|account| {
            let (key, pick) = (account.key.clone(), on_select.clone());
            MenuItem::radio(account.email.clone(), account.key == self.current).on_click(
                move |window, cx| {
                    log::info!("account switcher: {key}");
                    pick(&key, window, cx);
                },
            )
        });
        let mut menu = Menu::new().group("Accounts", items.collect::<Vec<_>>());
        if let Some(add) = self.on_add {
            menu = menu.separator().item(
                MenuItem::new("Add account")
                    .icon(IconName::UserPlus)
                    .on_click(move |window, cx| add(window, cx)),
            );
        }
        let now = self
            .accounts
            .into_iter()
            .find(|account| account.key == self.current);
        if now.is_none() {
            log::error!(
                "account switcher {id:?}: no account {}; none current",
                self.current
            );
        }
        let name = now
            .as_ref()
            .map_or_else(|| SharedString::from("No account"), |now| now.name.clone());
        let mut avatar = Avatar::new((id.clone(), "avatar"), name.clone()).size(AvatarSize::Sm);
        if let Some(image) = now.as_ref().and_then(|now| now.image.clone()) {
            avatar = avatar.image(image);
        }
        let email = now.map(|now| now.email).unwrap_or_default();
        let trigger = row((id.clone(), "trigger"), avatar, named(name, email, cx), cx);
        opening(id, &menu, trigger, window, cx)
    }
}

/// The workspaces one belongs to, the current one showing; its menu picks another or makes one.
#[derive(IntoElement)]
pub struct WorkspaceSwitcher {
    id: ElementId,
    workspaces: Vec<Workspace>,
    current: SharedString,
    on_select: OnKey,
    on_create: Option<Run>,
}

impl WorkspaceSwitcher {
    pub fn new(
        id: impl Into<ElementId>,
        workspaces: impl IntoIterator<Item = Workspace>,
        current: impl Into<SharedString>,
        on_select: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        let (workspaces, current): (Vec<_>, SharedString) =
            (workspaces.into_iter().collect(), current.into());
        Self {
            id: id.into(),
            workspaces,
            current,
            on_select: Rc::new(on_select),
            on_create: None,
        }
    }

    /// Shows "Create workspace".
    pub fn on_create(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_create = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for WorkspaceSwitcher {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id;
        let on_select = self.on_select;
        let items: Vec<_> = self
            .workspaces
            .iter()
            .map(|each| {
                let (key, pick) = (each.key.clone(), on_select.clone());
                MenuItem::radio(each.name.clone(), each.key == self.current).on_click(
                    move |window, cx| {
                        log::info!("workspace switcher: {key}");
                        pick(&key, window, cx);
                    },
                )
            })
            .collect();
        let mut menu = Menu::new().group("Workspaces", items);
        if let Some(create) = self.on_create {
            menu = menu.separator().item(
                MenuItem::new("Create workspace")
                    .icon(IconName::Plus)
                    .on_click(move |window, cx| create(window, cx)),
            );
        }
        let now = self
            .workspaces
            .into_iter()
            .find(|each| each.key == self.current);
        if now.is_none() {
            log::error!(
                "workspace switcher {id:?}: no workspace {}; none current",
                self.current
            );
        }
        let name = now.as_ref().map_or_else(
            || SharedString::from("No workspace"),
            |now| now.name.clone(),
        );
        let mark = Avatar::new((id.clone(), "mark"), name.clone())
            .square()
            .size(AvatarSize::Sm);
        let detail = now.map(|now| now.detail).unwrap_or_default();
        let trigger = row((id.clone(), "trigger"), mark, named(name, detail, cx), cx);
        opening(id, &menu, trigger, window, cx)
    }
}

/// Someone's avatar that opens their menu: who is signed in over the owner's rows, and Sign out last.
#[derive(IntoElement)]
pub struct UserMenu {
    id: ElementId,
    name: SharedString,
    email: SharedString,
    image: Option<ImageSource>,
    presence: Option<Presence>,
    items: Vec<MenuItem>,
    on_sign_out: Run,
}

impl UserMenu {
    pub fn new(
        id: impl Into<ElementId>,
        name: impl Into<SharedString>,
        email: impl Into<SharedString>,
        on_sign_out: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            email: email.into(),
            image: None,
            presence: None,
            items: Vec::new(),
            on_sign_out: Rc::new(on_sign_out),
        }
    }

    pub fn image(mut self, source: impl Into<ImageSource>) -> Self {
        self.image = Some(source.into());
        self
    }

    pub fn presence(mut self, presence: Presence) -> Self {
        self.presence = Some(presence);
        self
    }

    /// A row of the owner's, such as Profile or Settings.
    pub fn item(mut self, item: MenuItem) -> Self {
        self.items.push(item);
        self
    }
}

impl RenderOnce for UserMenu {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id;
        let sign_out = self.on_sign_out;
        let menu =
            Menu::new()
                .group(format!("Signed in as {}", self.email), self.items)
                .separator()
                .item(MenuItem::new("Sign out").icon(IconName::LogOut).on_click(
                    move |window, cx| {
                        log::info!("user menu: sign out");
                        sign_out(window, cx);
                    },
                ));
        let mut avatar = Avatar::new((id.clone(), "avatar"), self.name).size(AvatarSize::Sm);
        if let Some(image) = self.image {
            avatar = avatar.image(image);
        }
        if let Some(presence) = self.presence {
            avatar = avatar.presence(presence);
        }
        let trigger = div()
            .id((id.clone(), "trigger"))
            .rounded_full()
            .border_1()
            .border_color(transparent_black())
            .tab_index(0)
            .focus_ring(cx)
            .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
            .child(avatar);
        menu_under(
            id,
            &menu,
            move |click| {
                trigger
                    .on_click(move |event, window, cx| click(event, window, cx))
                    .into_any_element()
            },
            window,
            cx,
        )
    }
}
