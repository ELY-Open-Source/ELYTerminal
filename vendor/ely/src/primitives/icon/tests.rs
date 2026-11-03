use gpui::{Context, IntoElement, Render, TestAppContext, Window, black};

use super::{FoundIcons, Icon, IconName, found};
use crate::theme::{ActiveTheme, Theme};

#[test]
fn every_icon_ships_in_the_bundle() {
    for icon in IconName::ALL {
        assert!(
            crate::Assets::get(icon.path()).is_some(),
            "{icon:?} missing at {}",
            icon.path()
        );
    }
}

#[test]
fn an_app_path_draws_as_given_and_a_name_as_its_bundled_file() {
    let custom = Icon::from_path("app-icons/mark.svg");
    assert_eq!(custom.path.as_ref(), "app-icons/mark.svg");
    assert!(!custom.bundled);
    let named = Icon::new(IconName::Check);
    assert_eq!(named.path.as_ref(), IconName::Check.path());
    assert!(named.bundled);
}

#[test]
fn the_source_answers_whether_it_holds_a_path() {
    assert!(found(&crate::Assets, IconName::Check.path()));
    assert!(!found(&crate::Assets, "app-icons/missing.svg"));
}

struct Missing;

impl Render for Missing {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Icon::from_path("app-icons/missing.svg")
    }
}

#[gpui::test]
fn a_missing_app_icon_draws_its_mark(cx: &mut TestAppContext) {
    cx.update(Theme::init);
    let (_, cx) = cx.add_window_view(|_, _| Missing);
    cx.run_until_parked();
    let seen = cx.update(|_, cx| cx.global::<FoundIcons>().0.clone());
    assert_eq!(seen.get("app-icons/missing.svg"), Some(&false));
}

#[gpui::test]
fn a_broken_icon_shows_a_broken_picture_in_danger(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        let icon = Icon::from_path("app-icons/missing.svg")
            .color(black())
            .group_hover_color("row", black())
            .broken(cx);
        assert_eq!(icon.path.as_ref(), IconName::ImageOff.path());
        assert_eq!(icon.color, Some(cx.theme().colors.danger));
        assert!(icon.hover.is_none(), "hover keeps the danger color");
    });
}

#[gpui::test]
fn init_without_ely_assets_returns_an_error(cx: &mut TestAppContext) {
    let error = cx
        .update(crate::init)
        .expect_err("the test source holds no Ely assets");
    assert!(format!("{error:#}").contains("pass `ely_gpui_component::Assets`"));
}
