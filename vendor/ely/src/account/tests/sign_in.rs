use gpui::{AnyElement, Entity, IntoElement, TestAppContext};

use super::{Bench, bench, said, say, settle, tab, tap, write};
use crate::account::{
    ForgotPassword, LoginForm, MagicLinkForm, OAuthButtons, SignupForm, TwoFactorInput,
};

fn login(_: &Bench, owner: Entity<Bench>) -> AnyElement {
    LoginForm::new("login", move |login, _, cx| {
        say(
            &owner,
            format!("{} {} {}", login.email, login.password, login.remember),
            cx,
        )
    })
    .on_forgot(|_, _| {})
    .into_any_element()
}

/// Stops: email, password, its eye, the box, the forgotten password, Sign in.
#[gpui::test]
fn sign_in_waits_for_an_address_and_a_password_and_enter_sends(cx: &mut TestAppContext) {
    let (host, cx) = bench(login, cx);
    tab(1, cx);
    write("ada@example", cx);
    tab(2, cx);
    write("secret", cx);
    tap("enter", cx);
    assert!(
        said(&host, cx).is_empty(),
        "an address without a dot is no address"
    );
    tab(1, cx);
    write(".com", cx);
    tab(4, cx);
    tap("space", cx);
    tab(2, cx);
    tap("enter", cx);
    assert_eq!(said(&host, cx), ["ada@example.com secret true"]);
}

fn signup(_: &Bench, owner: Entity<Bench>) -> AnyElement {
    SignupForm::new("signup", move |signup, _, cx| {
        say(
            &owner,
            format!("{} {} {}", signup.name, signup.email, signup.password),
            cx,
        )
    })
    .terms("I agree to the terms")
    .into_any_element()
}

/// Stops: name, email, password, its eye, the terms, then Create account once it holds.
#[gpui::test]
fn sign_up_waits_for_every_rule_and_the_terms(cx: &mut TestAppContext) {
    let (host, cx) = bench(signup, cx);
    tab(1, cx);
    write("Ada", cx);
    tab(2, cx);
    write("ada@example.com", cx);
    tab(3, cx);
    write("abcdefg1", cx);
    tap("enter", cx);
    assert!(said(&host, cx).is_empty(), "the terms are not yet accepted");
    tab(5, cx);
    tap("space", cx);
    tab(3, cx);
    tap("backspace", cx);
    tap("enter", cx);
    assert!(
        said(&host, cx).is_empty(),
        "seven characters and no number meet no rule but one"
    );
    write("1", cx);
    tap("enter", cx);
    assert_eq!(said(&host, cx), ["Ada ada@example.com abcdefg1"]);
}

fn providers(bench: &Bench, owner: Entity<Bench>) -> AnyElement {
    let buttons = OAuthButtons::new(
        "oauth",
        [("github", "GitHub"), ("google", "Google")],
        move |key, _, cx| say(&owner, key.to_string(), cx),
    );
    match bench.sent {
        true => buttons.busy("github").into_any_element(),
        false => buttons.into_any_element(),
    }
}

/// Stops: GitHub, Google; while GitHub is on its way, neither.
#[gpui::test]
fn a_provider_hands_its_key_and_rests_while_one_is_on_its_way(cx: &mut TestAppContext) {
    let (host, cx) = bench(providers, cx);
    tab(2, cx);
    tap("space", cx);
    assert_eq!(said(&host, cx), ["google"]);
    host.update(cx, |bench, cx| {
        bench.sent = true;
        cx.notify();
    });
    settle(cx);
    for stop in [1, 2] {
        tab(stop, cx);
        tap("space", cx);
    }
    assert_eq!(
        said(&host, cx),
        ["google"],
        "no provider takes a press while one is on its way"
    );
}

fn magic(bench: &Bench, owner: Entity<Bench>) -> AnyElement {
    let other = owner.clone();
    MagicLinkForm::new("magic", move |address, _, cx| {
        say(&owner, format!("send {address}"), cx);
        owner.update(cx, |bench, _| bench.sent = true);
    })
    .sent(bench.sent)
    .on_other(move |_, cx| {
        other.update(cx, |bench, cx| {
            bench.sent = false;
            cx.notify();
        })
    })
    .into_any_element()
}

/// Stops: email, then the link button; once sent, Send again and Use another email.
#[gpui::test]
fn a_link_goes_to_the_address_and_again_on_request(cx: &mut TestAppContext) {
    let (host, cx) = bench(magic, cx);
    tab(1, cx);
    write("ada@example.com", cx);
    tap("enter", cx);
    tab(1, cx);
    tap("space", cx);
    assert_eq!(
        said(&host, cx),
        ["send ada@example.com", "send ada@example.com"]
    );
    tab(2, cx);
    tap("space", cx);
    assert!(
        !host.read_with(cx, |bench, _| bench.sent),
        "Use another email goes back to the address"
    );
}

fn forgot(bench: &Bench, owner: Entity<Bench>) -> AnyElement {
    let back = owner.clone();
    ForgotPassword::new("forgot", move |address, _, cx| {
        say(&owner, format!("reset {address}"), cx)
    })
    .sent(bench.sent)
    .on_back(move |_, cx| say(&back, "back".into(), cx))
    .into_any_element()
}

/// Stops: email, then Back to sign in while the send rests.
#[gpui::test]
fn a_reset_waits_for_an_address_and_offers_the_way_back(cx: &mut TestAppContext) {
    let (host, cx) = bench(forgot, cx);
    tab(2, cx);
    tap("enter", cx);
    assert_eq!(
        said(&host, cx),
        ["back"],
        "with no address the send rests, so the second stop is the way back"
    );
}

fn factor(bench: &Bench, owner: Entity<Bench>) -> AnyElement {
    TwoFactorInput::new("factor", move |code, _, cx| {
        say(&owner, code.to_string(), cx)
    })
    .failed(bench.failed)
    .into_any_element()
}

/// Stops: the code.
#[gpui::test]
fn a_failed_code_clears_the_boxes_for_the_next(cx: &mut TestAppContext) {
    let (host, cx) = bench(factor, cx);
    tab(1, cx);
    write("123456", cx);
    host.update(cx, |bench, cx| {
        bench.failed = 1;
        cx.notify();
    });
    settle(cx);
    write("654321", cx);
    assert_eq!(said(&host, cx), ["123456", "654321"]);
}

/// Stops: email, password, its eye, the box, Forgot password.
#[gpui::test]
fn sign_in_waits_for_a_password(cx: &mut TestAppContext) {
    let (host, cx) = bench(login, cx);
    tab(1, cx);
    write("ada@example.com", cx);
    tap("enter", cx);
    assert!(
        said(&host, cx).is_empty(),
        "an address alone signs nobody in"
    );
}

fn busy_login(_: &Bench, owner: Entity<Bench>) -> AnyElement {
    LoginForm::new("login", move |login, _, cx| {
        say(&owner, login.email.to_string(), cx)
    })
    .busy(true)
    .into_any_element()
}

/// Stops: email, password, its eye, the box; Sign in rests while busy, so a fifth Tab wraps to the email.
#[gpui::test]
fn sign_in_rests_while_busy(cx: &mut TestAppContext) {
    let (host, cx) = bench(busy_login, cx);
    tab(1, cx);
    write("ada@example.com", cx);
    tab(2, cx);
    write("secret", cx);
    tap("enter", cx);
    tab(5, cx);
    tap("space", cx);
    assert!(said(&host, cx).is_empty());
}

/// Stops: name, email, password, its eye, the terms.
#[gpui::test]
fn sign_up_waits_for_a_name(cx: &mut TestAppContext) {
    let (host, cx) = bench(signup, cx);
    tab(2, cx);
    write("ada@example.com", cx);
    tab(3, cx);
    write("abcdefg1", cx);
    tab(5, cx);
    tap("space", cx);
    tab(3, cx);
    tap("enter", cx);
    assert!(said(&host, cx).is_empty(), "no name, no account");
    tab(1, cx);
    write("Ada", cx);
    tap("enter", cx);
    assert_eq!(said(&host, cx), ["Ada ada@example.com abcdefg1"]);
}

fn busy_sent(_: &Bench, owner: Entity<Bench>) -> AnyElement {
    let other = owner.clone();
    MagicLinkForm::new("magic", move |address, _, cx| {
        say(&owner, format!("send {address}"), cx)
    })
    .sent(true)
    .busy(true)
    .on_other(move |_, cx| say(&other, "other".into(), cx))
    .into_any_element()
}

/// While a link is on its way Send again rests, so the first stop is Use another email.
#[gpui::test]
fn send_again_rests_while_busy(cx: &mut TestAppContext) {
    let (host, cx) = bench(busy_sent, cx);
    tab(1, cx);
    tap("space", cx);
    assert_eq!(said(&host, cx), ["other"]);
}
