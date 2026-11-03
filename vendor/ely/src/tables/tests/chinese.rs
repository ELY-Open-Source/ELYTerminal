//! The sort and filter builders read in Chinese under zh-CN.

use gpui::{IntoElement, Render, TestAppContext, Window, div, prelude::*};

use crate::{
    i18n::{self, I18n},
    tables::{FilterBuilder, FilterRule, SortBuilder, SortKey, Test},
    theme::Theme,
};

struct Builders;

impl Render for Builders {
    fn render(&mut self, _: &mut Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
        let columns = [("a", "第一列"), ("b", "第二列")];
        let rule = FilterRule {
            column: "a".into(),
            ..FilterRule::default()
        };
        let key = SortKey {
            column: "a".into(),
            rising: true,
        };
        div()
            .size_full()
            .child(FilterBuilder::new("filter", columns).rules([rule.clone(), rule], true))
            .child(SortBuilder::new("sort", columns).keys([key]))
    }
}

#[gpui::test]
fn the_builders_read_in_chinese(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        cx.set_global(I18n::new("zh-CN").catalog("zh-CN", &[]));
    });
    cx.update(|cx| {
        for test in Test::ALL {
            let shown = i18n::text(cx, test.key(), &[]);
            assert!(
                !shown.chars().any(|ch| ch.is_ascii_alphabetic()),
                "{test:?}: {shown}"
            );
        }
        for key in [
            "builder.add_rule",
            "builder.all",
            "builder.any",
            "builder.add_sort",
            "builder.up",
            "builder.down",
        ] {
            let shown = i18n::text(cx, key, &[]);
            assert!(
                !shown.chars().any(|ch| ch.is_ascii_alphabetic()),
                "{key}: {shown}"
            );
        }
    });
    let (_, cx) = cx.add_window_view(|_, _| Builders);
    cx.run_until_parked();
}
