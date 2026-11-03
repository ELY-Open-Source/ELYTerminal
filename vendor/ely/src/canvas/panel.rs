use std::{fmt::Debug, rc::Rc};

use gpui::{
    App, ElementId, FontWeight, Hsla, IntoElement, ParentElement, SharedString, Styled, Window, div,
};

use crate::{
    forms::{ColorPicker, ColorSwatch},
    overlays::Popover,
    theme::{ActiveTheme, TextSize},
};

/// A panel's handler for its whole value.
pub(crate) type OnEdit<V> = Rc<dyn Fn(V, &mut Window, &mut App)>;

/// A handler that hands `on_change` a copy of `value` changed by `edit`.
pub(crate) fn editing<V, T>(
    what: &'static str,
    value: &V,
    on_change: &OnEdit<V>,
    edit: impl Fn(&mut V, T) + 'static,
) -> impl Fn(T, &mut Window, &mut App) + 'static
where
    V: Clone + Debug + 'static,
    T: 'static,
{
    let (value, on_change) = (value.clone(), on_change.clone());
    move |input, window, cx| {
        let mut next = value.clone();
        edit(&mut next, input);
        log::info!("{what}: {next:?}");
        on_change(next, window, cx);
    }
}

/// Two fields side by side, sharing the width.
pub(crate) fn pair(left: impl IntoElement, right: impl IntoElement) -> gpui::Div {
    div()
        .flex()
        .gap_2()
        .child(div().flex_1().min_w_0().child(left))
        .child(div().flex_1().min_w_0().child(right))
}

/// A quiet line naming what follows.
pub(crate) fn caption(text: impl Into<SharedString>, cx: &App) -> gpui::Div {
    let theme = cx.theme();
    div()
        .text_size(theme.text_size(TextSize::Xs))
        .text_color(theme.colors.fg_subtle)
        .child(text.into())
}

/// A panel part's title, with an action at the end of its line.
pub(crate) fn heading(
    title: impl Into<SharedString>,
    action: impl IntoElement,
    cx: &App,
) -> gpui::Div {
    let theme = cx.theme();
    div()
        .flex()
        .items_center()
        .justify_between()
        .gap_2()
        .child(
            div()
                .text_size(theme.text_size(TextSize::Sm))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme.colors.fg)
                .child(title.into()),
        )
        .child(action)
}

/// A swatch that opens a color picker.
pub(crate) fn color_well(
    id: ElementId,
    color: Hsla,
    on_pick: impl Fn(Hsla, &mut Window, &mut App) + 'static,
) -> Popover {
    let swatch = (id.clone(), "swatch");
    Popover::with_opener(
        id.clone(),
        move |toggle| {
            ColorSwatch::new(swatch, color)
                .tooltip("Change the color")
                .on_click(move |window, cx| toggle(window, cx))
        },
        move |_, _, _| ColorPicker::new((id, "picker"), color).on_change(on_pick),
    )
}
