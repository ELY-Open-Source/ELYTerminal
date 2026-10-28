//! Theme cards, each a small window in its theme's colors.

use ely_gpui_component::{
    forms::RadioCard,
    layout::SimpleGrid,
    theme::{ActiveTheme, Mix, Radius, TextSize},
};
use gpui::{
    App, Div, Entity, Hsla, ParentElement, SharedString, Styled, Window, div, relative, rems,
};

use super::super::{SettingsWindow, set};
use crate::{
    settings::Settings,
    themes::{self, TerminalTheme},
};

/// Omarchy's theme where followed, then every built-in, as cards; `dark` picks the dark system's.
pub(super) fn grid(
    this: &Entity<SettingsWindow>,
    title: Option<SharedString>,
    dark: bool,
    settings: &Settings,
    window: &Window,
    cx: &App,
) -> Div {
    let theme = cx.theme();
    let (field, chosen) = if dark {
        ("dark_theme", settings.dark_theme.as_deref())
    } else {
        ("theme", Some(settings.theme.as_str()))
    };
    let omarchy = cx.try_global::<themes::Omarchy>().map(|omarchy| &omarchy.0);
    let cards = omarchy.into_iter().chain(themes::all()).map(|each| {
        let (id, this) = (each.id, this.clone());
        let key = SharedString::from(format!("{field}-{id}"));
        RadioCard::new(key, chosen == Some(id), each.name)
            .preview(preview(each, cx))
            .on_select(move |_, cx| {
                set(&this, format!("{field} is {id}"), cx, |settings| {
                    if dark {
                        settings.dark_theme = Some(id.into());
                    } else {
                        settings.theme = id.into();
                    }
                });
            })
    });
    let rem = window.rem_size();
    let (min, gap) = (
        theme.label_width().to_pixels(rem),
        rems(0.75).to_pixels(rem),
    );
    div()
        .flex()
        .flex_col()
        .gap_2()
        .py_3()
        .border_b_1()
        .border_color(theme.colors.border)
        .children(title.map(|title| div().text_size(theme.text_size(TextSize::Sm)).child(title)))
        .child(
            SimpleGrid::new(SharedString::from(format!("{field}-grid")), min, gap).children(cards),
        )
}

/// A small window in `theme`'s colors: the sessions strip, then a prompt, its output and the cursor.
fn preview(theme: &TerminalTheme, cx: &App) -> Div {
    let (bg, fg) = (theme.background, theme.foreground);
    let bar = |color: Hsla, width: f32| div().h_1().w(relative(width)).rounded_full().bg(color);
    let prompt = |end: Div| {
        let dot = div()
            .flex_none()
            .size_1p5()
            .rounded_full()
            .bg(theme.ansi[2]);
        div().flex().items_center().gap_1().child(dot).child(end)
    };
    let strip = div()
        .w(relative(0.2))
        .h_full()
        .flex()
        .flex_col()
        .items_center()
        .gap_1()
        .pt_2()
        .bg(bg.mix(&fg, 0.06))
        .children([0.6, 0.3, 0.3].map(|alpha| bar(fg.opacity(alpha), 0.6)));
    let lines = div()
        .flex_1()
        .flex()
        .flex_col()
        .justify_center()
        .gap_1()
        .px_2()
        .child(prompt(bar(fg, 0.5)))
        .child(bar(theme.ansi[4], 0.75))
        .child(bar(theme.ansi[5], 0.55))
        .child(prompt(div().flex_none().w_1p5().h_2p5().bg(theme.cursor)));
    let chrome = cx.theme();
    div()
        .w_full()
        .aspect_ratio(2.)
        .flex()
        .overflow_hidden()
        .rounded(chrome.radius(Radius::Sm))
        .border_1()
        .border_color(chrome.colors.border)
        .bg(bg)
        .child(strip)
        .child(lines)
}
