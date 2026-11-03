use gpui::{
    Context, IntoElement, ParentElement, Render, TestAppContext, Window, div, prelude::*, px,
};

use crate::{lists::GitStatus, theme::Theme};

const LONG: &str = "developerID_application_certificate_for_signing.cer";
const NAME: &str = "change-name-developerID_application_certificate_for_signing.cer";
const ACTIONS: &str = "change-actions-developerID_application_certificate_for_signing.cer";

struct Narrow;

impl Render for Narrow {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let file = super::super::Changed {
            path: LONG.into(),
            status: GitStatus::Untracked,
            lines: None,
        };
        div()
            .w(px(260.0))
            .child(super::super::ChangesList::new("narrow", [], []).untracked([file]))
    }
}

/// A long name gives way before the row's buttons; it never runs under them.
#[gpui::test]
fn a_long_name_ends_before_the_buttons(cx: &mut TestAppContext) {
    cx.update(Theme::init);
    let (_, cx) = cx.add_window_view(|_, _| Narrow);
    cx.run_until_parked();
    let name = cx.debug_bounds(NAME).expect("the name");
    let actions = cx.debug_bounds(ACTIONS).expect("the buttons");
    assert!(
        name.right() <= actions.left(),
        "{name:?} runs into {actions:?}"
    );
    assert!(actions.right() <= px(260.0), "{actions:?}");
}
