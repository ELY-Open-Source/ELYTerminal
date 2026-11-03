use gpui::{AnyElement, Entity, IntoElement, ParentElement, Styled, TestAppContext, div, px};

use super::{Bench, bench, said, say, settle, tab, tap, write};
use crate::account::{
    BillingHistory, Invitation, InvitationList, Invoice, InvoiceState, Member, PaymentMethodForm,
    Quota, Role, Standing, Subscription, SubscriptionCard, TeamMemberTable, UpgradePrompt,
    UsageQuota,
};

fn plan(bench: &Bench, owner: Entity<Bench>) -> AnyElement {
    let standing = match bench.sent {
        true => Standing::Canceled {
            ends: "1 Oct 2026".into(),
        },
        false => Standing::Active {
            renews: "1 Oct 2026".into(),
        },
    };
    let [cancel, resume] = [(); 2].map(|_| owner.clone());
    SubscriptionCard::new(
        "plan",
        Subscription {
            plan: "Pro".into(),
            price: 20.0,
            currency: "USD".into(),
            yearly: false,
            standing,
        },
    )
    .on_cancel(move |_, cx| say(&cancel, "cancel".into(), cx))
    .on_resume(move |_, cx| say(&resume, "resume".into(), cx))
    .into_any_element()
}

/// Stops: Cancel plan while it runs; Keep plan once canceled.
#[gpui::test]
fn a_plan_cancels_on_the_second_press_and_comes_back(cx: &mut TestAppContext) {
    let (host, cx) = bench(plan, cx);
    tab(1, cx);
    tap("space", cx);
    assert!(said(&host, cx).is_empty(), "the first press only arms");
    tap("space", cx);
    host.update(cx, |bench, cx| {
        bench.sent = true;
        cx.notify();
    });
    settle(cx);
    tab(1, cx);
    tap("space", cx);
    assert_eq!(said(&host, cx), ["cancel", "resume"]);
}

fn card(_: &Bench, owner: Entity<Bench>) -> AnyElement {
    PaymentMethodForm::new("card", move |card, _, cx| {
        say(
            &owner,
            format!(
                "{} {} {}/{} {} {}",
                card.brand.name(),
                card.number,
                card.month,
                card.year,
                card.cvc,
                card.name
            ),
            cx,
        )
    })
    .into_any_element()
}

/// Stops: number, expiry, code, name, then Save once the card reads.
#[gpui::test]
fn a_card_saves_once_its_number_checks_out(cx: &mut TestAppContext) {
    let (host, cx) = bench(card, cx);
    tab(1, cx);
    write("4242424242424241", cx);
    tab(2, cx);
    write("1240", cx);
    tab(3, cx);
    write("123", cx);
    tab(4, cx);
    write("Ada Lovelace", cx);
    tap("enter", cx);
    assert!(
        said(&host, cx).is_empty(),
        "a number that fails its check keeps Save at rest"
    );
    tab(1, cx);
    tap("backspace", cx);
    write("2", cx);
    tab(4, cx);
    tap("enter", cx);
    assert_eq!(
        said(&host, cx),
        ["Visa 4242424242424242 12/2040 123 Ada Lovelace"]
    );
}

fn roles() -> Vec<Role> {
    let role = |key: &str, name: &str| Role {
        key: key.to_string().into(),
        name: name.to_string().into(),
        description: "Can do things".into(),
    };
    vec![role("member", "Member"), role("admin", "Admin")]
}

fn invites(_: &Bench, owner: Entity<Bench>) -> AnyElement {
    let sent = Invitation {
        key: "bo".into(),
        email: "bo@example.com".into(),
        role: "admin".into(),
        sent: "Sent 2 days ago".into(),
        expired: false,
    };
    let [invited, again, gone] = [(); 3].map(|_| owner.clone());
    InvitationList::new(
        "invites",
        [sent],
        roles(),
        move |email, role, _, cx| say(&invited, format!("invite {email} {role}"), cx),
        move |key, _, cx| say(&again, format!("resend {key}"), cx),
        move |key, _, cx| say(&gone, format!("revoke {key}"), cx),
    )
    .into_any_element()
}

/// Stops: the address, the role, then Resend and Revoke while Invite rests.
#[gpui::test]
fn an_invitation_goes_as_a_member_and_one_out_resends(cx: &mut TestAppContext) {
    let (host, cx) = bench(invites, cx);
    tab(3, cx);
    tap("space", cx);
    tab(1, cx);
    write("cy@example.com", cx);
    tap("enter", cx);
    assert_eq!(
        said(&host, cx),
        ["resend bo", "invite cy@example.com member"]
    );
}

fn team(_: &Bench, owner: Entity<Bench>) -> AnyElement {
    let member = |key: &str, owner: bool| Member {
        key: key.to_string().into(),
        name: key.to_string().into(),
        email: format!("{key}@example.com").into(),
        role: "member".into(),
        seen: "Active today".into(),
        image: None,
        owner,
    };
    let removed = owner.clone();
    TeamMemberTable::new(
        "team",
        [member("ada", true), member("bo", false)],
        roles(),
        move |key, role, _, cx| say(&owner, format!("{key} {role}"), cx),
        move |key, _, cx| say(&removed, format!("remove {key}"), cx),
    )
    .into_any_element()
}

/// Stops: bo's role, then bo's Remove; the owner has neither.
#[gpui::test]
fn a_member_leaves_on_the_second_press_and_the_owner_stays(cx: &mut TestAppContext) {
    let (host, cx) = bench(team, cx);
    tab(2, cx);
    tap("space", cx);
    tap("space", cx);
    assert_eq!(said(&host, cx), ["remove bo"]);
}

fn wall(_: &Bench, owner: Entity<Bench>) -> AnyElement {
    let later = owner.clone();
    UpgradePrompt::new("wall", "Unlock history", move |_, cx| {
        say(&owner, "upgrade".into(), cx)
    })
    .benefit("Every message, kept")
    .on_dismiss(move |_, cx| say(&later, "later".into(), cx))
    .into_any_element()
}

/// Stops: Upgrade, then Not now.
#[gpui::test]
fn a_paywall_upgrades_or_waits(cx: &mut TestAppContext) {
    let (host, cx) = bench(wall, cx);
    tab(1, cx);
    tap("space", cx);
    tab(2, cx);
    tap("space", cx);
    assert_eq!(said(&host, cx), ["upgrade", "later"]);
}

fn busy_card(_: &Bench, owner: Entity<Bench>) -> AnyElement {
    PaymentMethodForm::new("card", move |card, _, cx| {
        say(&owner, card.number.to_string(), cx)
    })
    .busy(true)
    .into_any_element()
}

/// Stops: number, expiry, code, name; Save rests while the owner checks the card.
#[gpui::test]
fn a_card_waits_while_the_owner_checks_it(cx: &mut TestAppContext) {
    let (host, cx) = bench(busy_card, cx);
    tab(1, cx);
    write("4242424242424242", cx);
    tab(2, cx);
    write("1240", cx);
    tab(3, cx);
    write("123", cx);
    tab(4, cx);
    write("Ada Lovelace", cx);
    tap("enter", cx);
    assert!(said(&host, cx).is_empty());
}

/// Stops: the address, the role; Invite waits for an address that reads.
#[gpui::test]
fn an_invitation_waits_for_an_address(cx: &mut TestAppContext) {
    let (host, cx) = bench(invites, cx);
    tab(1, cx);
    write("cy@example", cx);
    tap("enter", cx);
    assert!(said(&host, cx).is_empty());
}

/// Stops: bo's role first; Enter opens its list, Down reaches Admin and Enter picks it.
#[gpui::test]
fn a_members_role_changes_from_its_row(cx: &mut TestAppContext) {
    let (host, cx) = bench(team, cx);
    tab(1, cx);
    for key in ["enter", "down", "enter"] {
        tap(key, cx);
    }
    assert_eq!(said(&host, cx), ["bo admin"]);
}

fn bills(_: &Bench, owner: Entity<Bench>) -> AnyElement {
    let invoice = Invoice {
        key: "sep".into(),
        date: "1 Sep 2026".into(),
        description: "Pro, monthly".into(),
        amount: 20.0,
        currency: "USD".into(),
        state: InvoiceState::Paid,
    };
    BillingHistory::new("bills", [invoice])
        .on_download(move |key, _, cx| say(&owner, format!("download {key}"), cx))
        .into_any_element()
}

/// Stops: the bill's download.
#[gpui::test]
fn a_bill_downloads_from_its_row(cx: &mut TestAppContext) {
    let (host, cx) = bench(bills, cx);
    tab(1, cx);
    tap("space", cx);
    assert_eq!(said(&host, cx), ["download sep"]);
}

/// Stops: number, expiry, code, name, then Save once the card reads.
#[gpui::test]
fn a_card_waits_for_a_whole_code_and_a_name(cx: &mut TestAppContext) {
    let (host, cx) = bench(card, cx);
    tab(1, cx);
    write("4242424242424242", cx);
    tab(2, cx);
    write("1240", cx);
    tab(3, cx);
    write("12", cx);
    tab(4, cx);
    write("Ada", cx);
    tap("enter", cx);
    assert!(said(&host, cx).is_empty(), "a code of two digits");
    tab(3, cx);
    write("3", cx);
    tab(4, cx);
    tap("secondary-a", cx);
    tap("backspace", cx);
    tap("enter", cx);
    assert!(said(&host, cx).is_empty(), "no name on the card");
    write("Ada", cx);
    tap("enter", cx);
    assert_eq!(said(&host, cx), ["Visa 4242424242424242 12/2040 123 Ada"]);
}

/// Stops: number, expiry, code, name; an American Express number takes fifteen digits and a code of four.
#[gpui::test]
fn an_american_express_card_takes_its_own_shape(cx: &mut TestAppContext) {
    let (host, cx) = bench(card, cx);
    tab(1, cx);
    write("378282246310005", cx);
    tab(2, cx);
    write("1240", cx);
    tab(3, cx);
    write("1234", cx);
    tab(4, cx);
    write("Ada", cx);
    tap("enter", cx);
    assert_eq!(
        said(&host, cx),
        ["American Express 378282246310005 12/2040 1234 Ada"]
    );
}

/// Stops: the address, the role, then Invite once the address reads; after it goes, Invite rests and the third stop is Resend.
#[gpui::test]
fn an_invitation_empties_the_address_once_it_goes(cx: &mut TestAppContext) {
    let (host, cx) = bench(invites, cx);
    tab(1, cx);
    write("cy@example.com", cx);
    tap("enter", cx);
    tab(3, cx);
    tap("space", cx);
    assert_eq!(
        said(&host, cx),
        ["invite cy@example.com member", "resend bo"]
    );
}

/// One quota in a 280px box.
fn usage(name: &'static str, used: f64, limit: f64) -> AnyElement {
    let quota = Quota {
        key: "seats".into(),
        name: name.into(),
        used,
        limit,
        unit: "seats".into(),
    };
    div()
        .w(px(280.0))
        .child(UsageQuota::new("usage", [quota]))
        .into_any_element()
}

fn long_name(_: &Bench, _: Entity<Bench>) -> AnyElement {
    usage("Seats across every workspace you own", 9.0, 12.0)
}

/// A long name gives way, so the count stays whole inside a 280px box.
#[gpui::test]
fn a_long_quota_name_leaves_its_count_inside_the_box(cx: &mut TestAppContext) {
    let (_, cx) = bench(long_name, cx);
    let count = cx.debug_bounds("meter-detail").expect("the count");
    assert!(
        count.right() <= px(280.0),
        "the count ends inside the box: {count:?}"
    );
}

fn past_limit(_: &Bench, _: Entity<Bench>) -> AnyElement {
    usage("Seats", 12.4, 10.0)
}

/// A quota past its limit draws a full bar and says how far past.
#[gpui::test]
fn a_quota_past_its_limit_draws_full(cx: &mut TestAppContext) {
    let (_, cx) = bench(past_limit, cx);
    assert!(cx.debug_bounds("meter-detail").is_some(), "the quota drew");
}

fn line(_: &Bench, owner: Entity<Bench>) -> AnyElement {
    UpgradePrompt::new("line", "Unlock history", move |_, cx| {
        say(&owner, "upgrade".into(), cx)
    })
    .into_any_element()
}

/// With benefits the prompt is a paywall that lists them.
#[gpui::test]
fn a_paywall_lists_its_benefits(cx: &mut TestAppContext) {
    let (_, cx) = bench(wall, cx);
    assert!(cx.debug_bounds("paywall-benefits").is_some());
}

/// Without benefits the prompt stands as one line, with no list.
#[gpui::test]
fn a_prompt_without_benefits_stands_as_a_line(cx: &mut TestAppContext) {
    let (_, cx) = bench(line, cx);
    assert!(cx.debug_bounds("paywall-benefits").is_none());
}

/// Accounts, workspaces, a provider and roles that left their lists.
fn departed(_: &Bench, _: Entity<Bench>) -> AnyElement {
    let account = crate::account::Account {
        key: "ada".into(),
        name: "Ada".into(),
        email: "ada@work.com".into(),
        image: None,
    };
    let space = crate::account::Workspace {
        key: "north".into(),
        name: "North".into(),
        detail: "Pro".into(),
    };
    let invitation = Invitation {
        key: "bo".into(),
        email: "bo@example.com".into(),
        role: "gone".into(),
        sent: "Sent 2 days ago".into(),
        expired: false,
    };
    let member = Member {
        key: "bo".into(),
        name: "Bo".into(),
        email: "bo@example.com".into(),
        role: "gone".into(),
        seen: "Active today".into(),
        image: None,
        owner: false,
    };
    div()
        .w(px(360.0))
        .child(crate::account::AccountSwitcher::new(
            "accounts",
            [account],
            "gone",
            |_, _, _| {},
        ))
        .child(crate::account::WorkspaceSwitcher::new(
            "spaces",
            [space],
            "gone",
            |_, _, _| {},
        ))
        .child(
            crate::account::OAuthButtons::new("oauth", [("github", "GitHub")], |_, _, _| {})
                .busy("gone"),
        )
        .child(InvitationList::new(
            "invites",
            [invitation],
            roles(),
            |_, _, _, _| {},
            |_, _, _| {},
            |_, _, _| {},
        ))
        .child(TeamMemberTable::new(
            "team",
            [member],
            roles(),
            |_, _, _, _| {},
            |_, _, _| {},
        ))
        .into_any_element()
}

#[gpui::test]
fn accounts_workspaces_and_roles_that_left_draw(cx: &mut TestAppContext) {
    let (_, cx) = bench(departed, cx);
    settle(cx);
}

/// Invitations whose first role leaves once the bench says sent.
fn departing_role(state: &Bench, owner: Entity<Bench>) -> AnyElement {
    let offered: Vec<Role> = roles()
        .into_iter()
        .filter(|role| !state.sent || role.key != "member")
        .collect();
    InvitationList::new(
        "invites",
        [],
        offered,
        move |email, role, _, cx| say(&owner, format!("invite {email} {role}"), cx),
        |_, _, _| {},
        |_, _, _| {},
    )
    .into_any_element()
}

#[gpui::test]
fn an_invitation_as_a_role_that_left_sends_nothing(cx: &mut TestAppContext) {
    let (host, cx) = bench(departing_role, cx);
    tab(1, cx);
    write("cy@example.com", cx);
    host.update(cx, |state, cx| {
        state.sent = true;
        cx.notify();
    });
    settle(cx);
    tap("enter", cx);
    tab(3, cx);
    tap("space", cx);
    assert_eq!(said(&host, cx), Vec::<String>::new());
}
