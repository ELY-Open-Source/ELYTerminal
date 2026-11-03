use gpui::{AnyElement, Entity, IntoElement, SharedString, TestAppContext};

use super::{Bench, bench, said, say, settle, tab, tap, write};
use crate::account::{
    Account, AccountSwitcher, ApiKey, ApiKeyManager, Profile, ProfileEditor, Session, SessionList,
    UserMenu, Workspace, WorkspaceSwitcher,
};
use crate::{menus::MenuItem, primitives::IconName};

fn accounts(_: &Bench, owner: Entity<Bench>) -> AnyElement {
    let account = |key: &str, name: &str, email: &str| Account {
        key: key.to_string().into(),
        name: name.to_string().into(),
        email: email.to_string().into(),
        image: None,
    };
    AccountSwitcher::new(
        "accounts",
        [
            account("ada", "Ada", "ada@work.com"),
            account("home", "Ada", "ada@home.com"),
        ],
        "ada",
        move |key, _, cx| say(&owner, key.to_string(), cx),
    )
    .into_any_element()
}

/// Stops: the switcher; its menu marks the first row on a keyboard open.
#[gpui::test]
fn the_account_menu_picks_another(cx: &mut TestAppContext) {
    let (host, cx) = bench(accounts, cx);
    tab(1, cx);
    tap("enter", cx);
    tap("down", cx);
    tap("enter", cx);
    assert_eq!(said(&host, cx), ["home"]);
}

fn sessions(_: &Bench, owner: Entity<Bench>) -> AnyElement {
    let session = |key: &str, current: bool| Session {
        key: key.to_string().into(),
        icon: IconName::Monitor,
        device: "MacBook".into(),
        client: "Safari".into(),
        place: "Lisbon".into(),
        seen: "Active now".into(),
        current,
    };
    let everyone = owner.clone();
    SessionList::new(
        "sessions",
        [session("phone", false), session("mac", true)],
        move |key, _, cx| say(&owner, format!("out {key}"), cx),
    )
    .on_sign_out_others(move |_, cx| say(&everyone, "others".into(), cx))
    .into_any_element()
}

/// Stops: the phone's Sign out, then Sign out of all other devices; this device has none.
#[gpui::test]
fn a_session_signs_out_and_the_rest_ask_twice(cx: &mut TestAppContext) {
    let (host, cx) = bench(sessions, cx);
    tab(1, cx);
    tap("space", cx);
    assert_eq!(
        said(&host, cx),
        ["out phone"],
        "this device lists first and has no Sign out"
    );
    tab(2, cx);
    tap("space", cx);
    assert_eq!(said(&host, cx), ["out phone"], "the first press only arms");
    tap("space", cx);
    assert_eq!(said(&host, cx), ["out phone", "others"]);
}

fn keys(bench: &Bench, owner: Entity<Bench>) -> AnyElement {
    let key = ApiKey {
        key: "ci".into(),
        name: "CI".into(),
        hint: "sk_…4f2a".into(),
        created: "Made 3 Sep".into(),
        used: "Never used".into(),
    };
    let [made, revoked, done] = [(); 3].map(|_| owner.clone());
    let manager = ApiKeyManager::new(
        "keys",
        [key],
        move |name, _, cx| say(&made, format!("make {name}"), cx),
        move |key, _, cx| say(&revoked, format!("revoke {key}"), cx),
        move |_, cx| say(&done, "done".into(), cx),
    );
    match bench.sent {
        true => manager.secret("sk_live_0123456789").into_any_element(),
        false => manager.into_any_element(),
    }
}

/// Stops: the name, then Revoke while Create rests; with a secret, Copy and Done lead.
#[gpui::test]
fn a_key_is_made_by_name_revoked_on_the_second_press_and_its_secret_dismissed(
    cx: &mut TestAppContext,
) {
    let (host, cx) = bench(keys, cx);
    tab(2, cx);
    tap("space", cx);
    tap("space", cx);
    tab(1, cx);
    write("Deploys", cx);
    tap("enter", cx);
    assert_eq!(said(&host, cx), ["revoke ci", "make Deploys"]);
    cx.executor()
        .advance_clock(std::time::Duration::from_secs(10));
    settle(cx);
    tab(2, cx);
    tap("space", cx);
    tap("space", cx);
    assert_eq!(
        said(&host, cx),
        ["revoke ci", "make Deploys", "revoke ci"],
        "the name field emptied, so Create rests and the second stop is Revoke"
    );
    host.update(cx, |bench, cx| {
        bench.sent = true;
        cx.notify();
    });
    settle(cx);
    tab(2, cx);
    tap("space", cx);
    assert_eq!(
        said(&host, cx),
        ["revoke ci", "make Deploys", "revoke ci", "done"]
    );
}

fn profile() -> Profile {
    Profile {
        name: "Ada".into(),
        username: "ada".into(),
        email: "ada@example.com".into(),
        ..Profile::default()
    }
}

fn editor(_: &Bench, owner: Entity<Bench>) -> AnyElement {
    ProfileEditor::new("profile", profile(), move |profile, _, cx| {
        say(&owner, format!("{} {}", profile.name, profile.email), cx)
    })
    .into_any_element()
}

/// Stops: the picture, name, username, email, about, location; Discard and Save once changed.
#[gpui::test]
fn a_profile_saves_only_what_changed_and_discard_puts_it_back(cx: &mut TestAppContext) {
    let (host, cx) = bench(editor, cx);
    tab(2, cx);
    write(" Lovelace", cx);
    tab(8, cx);
    tap("space", cx);
    assert_eq!(said(&host, cx), ["Ada Lovelace ada@example.com"]);
    tab(7, cx);
    tap("space", cx);
    tab(8, cx);
    tap("space", cx);
    assert_eq!(
        said(&host, cx),
        ["Ada Lovelace ada@example.com"],
        "after Discard nothing changed, so Save rests"
    );
}

fn menu(_: &Bench, owner: Entity<Bench>) -> AnyElement {
    let [profile, out] = [(); 2].map(|_| owner.clone());
    UserMenu::new("user", "Ada", "ada@example.com", move |_, cx| {
        say(&out, "out".into(), cx)
    })
    .item(MenuItem::new("Profile").on_click(move |_, cx| say(&profile, "profile".into(), cx)))
    .into_any_element()
}

/// Stops: the avatar; the menu's rows are Profile, then Sign out.
#[gpui::test]
fn the_user_menu_holds_the_owners_rows_and_sign_out_last(cx: &mut TestAppContext) {
    let (host, cx) = bench(menu, cx);
    tab(1, cx);
    tap("enter", cx);
    tap("down", cx);
    tap("enter", cx);
    assert_eq!(said(&host, cx), ["out"]);
}

fn workspaces(_: &Bench, owner: Entity<Bench>) -> AnyElement {
    let space = |key: &str| Workspace {
        key: key.to_string().into(),
        name: SharedString::from(key.to_string()),
        detail: "Pro".into(),
    };
    let made = owner.clone();
    WorkspaceSwitcher::new(
        "spaces",
        [space("north"), space("south")],
        "south",
        move |key, _, cx| say(&owner, key.to_string(), cx),
    )
    .on_create(move |_, cx| say(&made, "create".into(), cx))
    .into_any_element()
}

/// Stops: the switcher; its rows are north, south, then Create workspace.
#[gpui::test]
fn the_workspace_menu_picks_or_makes_one(cx: &mut TestAppContext) {
    let (host, cx) = bench(workspaces, cx);
    tab(1, cx);
    tap("enter", cx);
    tap("enter", cx);
    tab(1, cx);
    tap("enter", cx);
    tap("up", cx);
    tap("enter", cx);
    assert_eq!(said(&host, cx), ["north", "create"]);
}

fn alone(_: &Bench, owner: Entity<Bench>) -> AnyElement {
    let here = Session {
        key: "mac".into(),
        icon: IconName::Monitor,
        device: "MacBook".into(),
        client: "Safari".into(),
        place: "Lisbon".into(),
        seen: "Active now".into(),
        current: true,
    };
    let everyone = owner.clone();
    SessionList::new("sessions", [here], move |key, _, cx| {
        say(&owner, format!("out {key}"), cx)
    })
    .on_sign_out_others(move |_, cx| say(&everyone, "others".into(), cx))
    .into_any_element()
}

/// With this device alone there is nothing to sign out, so the list has no stop.
#[gpui::test]
fn this_device_alone_offers_no_sign_out(cx: &mut TestAppContext) {
    let (host, cx) = bench(alone, cx);
    tab(1, cx);
    tap("space", cx);
    tap("space", cx);
    assert!(said(&host, cx).is_empty());
}

/// The phone comes first in what the owner hands over, this device second; this device lists first.
#[gpui::test]
fn this_device_lists_first(cx: &mut TestAppContext) {
    let (_, cx) = bench(sessions, cx);
    let here = cx.debug_bounds("session-mac").expect("this device");
    let phone = cx.debug_bounds("session-phone").expect("the phone");
    assert!(here.top() < phone.top(), "{here:?} over {phone:?}");
}
