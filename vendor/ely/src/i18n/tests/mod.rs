use jiff::{Timestamp, tz::TimeZone};

use super::{Direction, I18n, LOCALES, Locale, fill};

mod a11y;
#[cfg(feature = "test-support")]
mod shipped;

fn i18n() -> I18n {
    I18n::new("en-US")
        .catalog(
            "en-US",
            &[("files", "{count} files in {folder}"), ("hi", "Hello")],
        )
        .catalog(
            "de-DE",
            &[("files", "{count} Dateien in {folder}"), ("hi", "Hallo")],
        )
}

/// Each `{name}` takes its argument, in the order the message puts them.
#[test]
fn a_message_fills_its_arguments() {
    let args = [("folder", "Docs"), ("count", "1,204")];
    assert_eq!(i18n().text("files", &args), "1,204 files in Docs");
    assert_eq!(i18n().text("hi", &[]), "Hello");
    assert_eq!(fill("{a}{a}", &[("a", "x")]), Ok("xx".into()));
    assert_eq!(fill("{a", &[("a", "x")]), Err("a { without }".into()));
    assert_eq!(fill("{b}", &[("a", "x")]), Err("no argument \"b\"".into()));
}

/// A right-to-left message leads with its mark, so one that starts with Latin letters or a signed number still reads right to left.
#[test]
fn a_right_to_left_message_leads_with_its_mark() {
    let he = I18n::new("he-IL").catalog(
        "he-IL",
        &[("change", "{change}% מאז אתמול"), ("sent", "{name} שלח")],
    );
    let change = Locale::of("he-IL").number(-3.5, 1);
    assert_eq!(
        he.text("change", &[("change", &change)]),
        "\u{200f}\u{200e}−3.5% מאז אתמול"
    );
    assert_eq!(he.text("sent", &[("name", "Ada")]), "\u{200f}Ada שלח");
    assert_eq!(
        i18n().text("hi", &[]),
        "Hello",
        "a left-to-right message takes no mark"
    );
}

#[test]
#[should_panic(expected = "\"files\" in en-US: no argument \"folder\"")]
fn a_missing_argument_fails() {
    i18n().text("files", &[("count", "3")]);
}

#[test]
#[should_panic(expected = "no message \"bye\" in en-US")]
fn a_missing_key_fails() {
    i18n().text("bye", &[]);
}

#[test]
#[should_panic(expected = "de-DE misses [\"hi\"] from en-US")]
fn a_catalog_missing_a_key_fails() {
    I18n::new("en-US")
        .catalog("en-US", &[("hi", "Hello"), ("files", "{count} files")])
        .catalog("de-DE", &[("files", "{count} Dateien")]);
}

#[test]
#[should_panic(expected = "de-DE has [\"bye\"], which en-US lacks")]
fn a_catalog_with_a_stray_key_fails() {
    I18n::new("en-US")
        .catalog("en-US", &[("hi", "Hello")])
        .catalog("de-DE", &[("hi", "Hallo"), ("bye", "Tschüss")]);
}

#[test]
#[should_panic(expected = "en-US repeats a key")]
fn a_repeated_key_fails() {
    I18n::new("en-US").catalog("en-US", &[("hi", "Hello"), ("hi", "Hey")]);
}

#[test]
#[should_panic(expected = "en-US has a catalog")]
fn a_second_catalog_for_a_locale_fails() {
    I18n::new("en-US")
        .catalog("en-US", &[("hi", "Hello")])
        .catalog("en-US", &[("hi", "Hey")]);
}

#[test]
#[should_panic(expected = "no locale \"xx-XX\"")]
fn an_unlisted_locale_fails() {
    Locale::of("xx-XX");
}

/// Separators and the date's order follow each locale.
#[test]
fn locales_write_their_own_numbers_and_dates() {
    let at = Timestamp::from_second(1_788_220_800).unwrap();
    let cases: Vec<_> = LOCALES
        .iter()
        .map(|locale| {
            (
                locale.tag,
                locale.number(-1234567.891, 2),
                locale.date(at, &TimeZone::UTC),
            )
        })
        .collect();
    assert_eq!(
        cases,
        [
            ("en-US", "−1,234,567.89".into(), "9/1/26".into()),
            ("en-GB", "−1,234,567.89".into(), "01/09/2026".into()),
            ("de-DE", "−1.234.567,89".into(), "01.09.26".into()),
            (
                "fr-FR",
                "−1\u{202f}234\u{202f}567,89".into(),
                "01/09/2026".into()
            ),
            ("ja-JP", "−1,234,567.89".into(), "2026/09/01".into()),
            ("zh-CN", "−1,234,567.89".into(), "2026/9/1".into()),
            ("he-IL", "\u{200e}−1,234,567.89".into(), "1.9.2026".into()),
        ]
    );
    let rtl: Vec<_> = LOCALES
        .iter()
        .filter(|locale| locale.direction == Direction::Rtl)
        .collect();
    assert_eq!(rtl, [&Locale::of("he-IL")]);
    assert_eq!(
        Locale::of("he-IL").number(3.5, 1),
        "3.5",
        "a gain takes no mark"
    );
}

#[cfg(feature = "test-support")]
mod shown {
    use gpui::{
        Context, InteractiveElement, IntoElement, ParentElement, Render, Styled, TestAppContext,
        Window, div, px,
    };

    use super::super::{Direction, I18n};

    /// A row of two marks, laid out the way `I18n`'s locale reads.
    struct Row;

    impl Render for Row {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let i18n = cx.global::<I18n>();
            let direction = i18n.locale().direction;
            direction
                .row(div().w(px(280.0)).justify_between())
                .child(
                    div()
                        .debug_selector(|| "first".into())
                        .w(px(40.0))
                        .child(i18n.text("hi", &[])),
                )
                .child(div().debug_selector(|| "second".into()).w(px(40.0)))
        }
    }

    /// A new locale reaches every window, and a right-to-left one starts the row at the right.
    #[gpui::test]
    fn a_right_to_left_locale_mirrors_the_row(cx: &mut TestAppContext) {
        cx.update(|cx| {
            cx.set_global(
                I18n::new("en-US")
                    .catalog("en-US", &[("hi", "Hi")])
                    .catalog("he-IL", &[("hi", "שלום")]),
            )
        });
        let (_, cx) = cx.add_window_view(|_, _| Row);
        cx.run_until_parked();
        let first = cx.debug_bounds("first").expect("the first mark");
        let second = cx.debug_bounds("second").expect("the second mark");
        assert!(first.left() < second.left(), "left to right in en-US");
        cx.update(|_, cx| I18n::set_locale("he-IL", cx));
        cx.run_until_parked();
        assert_eq!(
            cx.read(|cx| cx.global::<I18n>().text("hi", &[])),
            "\u{200f}שלום"
        );
        let first = cx.debug_bounds("first").expect("the first mark");
        let second = cx.debug_bounds("second").expect("the second mark");
        assert!(
            first.left() > second.left(),
            "right to left in he-IL: {first:?} {second:?}"
        );
    }

    #[gpui::test]
    #[should_panic(expected = "no catalog for de-DE")]
    fn a_locale_without_a_catalog_fails(cx: &mut TestAppContext) {
        cx.update(|cx| {
            cx.set_global(I18n::new("en-US").catalog("en-US", &[("hi", "Hi")]));
            I18n::set_locale("de-DE", cx);
        });
    }

    #[test]
    fn text_sits_against_the_reading_side() {
        let side = |direction: Direction| direction.align(div()).style().text.text_align;
        assert_eq!(side(Direction::Ltr), Some(gpui::TextAlign::Left));
        assert_eq!(side(Direction::Rtl), Some(gpui::TextAlign::Right));
    }
}
