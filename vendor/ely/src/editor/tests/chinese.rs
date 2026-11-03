//! The find box reads in Chinese under zh-CN.

use gpui::{Entity, IntoElement, Render, TestAppContext, Window, div, prelude::*};

use crate::{forms::TextInput, i18n::I18n, theme::Theme};

struct Host(Entity<TextInput>, Entity<TextInput>);

impl Render for Host {
    fn render(&mut self, _: &mut Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
        let find = super::super::FindWidget::new("find", &self.0, Some(0), 2)
            .replace(&self.1)
            .on_options(|_, _, _| {})
            .on_step(|_, _, _| {})
            .on_toggle_replace(|_, _, _| {})
            .on_close(|_, _| {});
        div().size_full().child(find)
    }
}

#[gpui::test]
fn the_find_box_reads_in_chinese(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        cx.set_global(I18n::new("zh-CN").catalog("zh-CN", &[]));
    });
    cx.update(|cx| {
        let count = |matches| super::super::find::count(&matches, cx);
        assert_eq!(count(Ok((None, 0))), ("无结果".into(), true));
        assert_eq!(count(Ok((Some(0), 2))), ("第 1 项，共 2 项".into(), false));
        assert_eq!(count(Ok((None, 3))), ("找到 3 项".into(), false));
    });
    let (_, cx) = cx.add_window_view(|window, cx| {
        Host(
            cx.new(|cx| TextInput::new(window, cx)),
            cx.new(|cx| TextInput::new(window, cx)),
        )
    });
    cx.run_until_parked();
}
