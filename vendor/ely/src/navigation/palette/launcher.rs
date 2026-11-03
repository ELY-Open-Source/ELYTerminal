use std::rc::Rc;

use anyhow::Context as _;
use gpui::{
    App, AppContext, Bounds, Context, IntoElement, ParentElement, Render, SharedString, Styled,
    Subscription, Window, WindowBackgroundAppearance, WindowBounds, WindowHandle, WindowKind,
    WindowOptions, div, point, size,
};

use super::kinds::{Search, SearchPalette};
use crate::{
    forms::{Choice, OnValue},
    theme::ActiveTheme,
};

/// A search palette alone in a window above all others, as Spotlight is. A pick, Escape or a click elsewhere closes it.
pub struct QuickLauncher {
    search: Search,
    on_open: OnValue,
    focused: bool,
    _away: Subscription,
}

impl QuickLauncher {
    /// Opens centered across the main display, a fifth of the way down.
    pub fn open(
        search: impl Fn(&str) -> Vec<(SharedString, Vec<Choice>)> + 'static,
        on_open: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
        cx: &mut App,
    ) -> anyhow::Result<WindowHandle<QuickLauncher>> {
        let theme = cx.theme();
        let (card, rem) = (theme.palette_size(), theme.base_rem());
        let card = size(card.width.to_pixels(rem), card.height.to_pixels(rem));
        let area = cx
            .primary_display()
            .context("a launcher needs a display")?
            .bounds();
        let origin = point(
            area.center().x - card.width / 2.0,
            area.top() + area.size.height / 5.0,
        );
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::new(origin, card))),
            titlebar: None,
            kind: WindowKind::PopUp,
            is_resizable: false,
            is_minimizable: false,
            window_background: WindowBackgroundAppearance::Transparent,
            ..Default::default()
        };
        let (search, on_open): (Search, OnValue) = (Rc::new(search), Rc::new(on_open));
        let handle = cx.open_window(options, |window, cx| {
            cx.new(|cx: &mut Context<QuickLauncher>| {
                let away = cx.observe_window_activation(window, |_, window, _| {
                    if !window.is_window_active() {
                        log::info!("quick launcher: closed on leaving");
                        window.remove_window();
                    }
                });
                QuickLauncher {
                    search,
                    on_open,
                    focused: false,
                    _away: away,
                }
            })
        })?;
        log::info!("quick launcher: opened");
        Ok(handle)
    }
}

impl Render for QuickLauncher {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = SearchPalette::from_parts(
            "quick-launcher".into(),
            self.search.clone(),
            Some(self.on_open.clone()),
            Rc::new(|window, _| window.remove_window()),
        )
        .palette(window, cx);
        if !self.focused {
            self.focused = true;
            window.focus(&palette.input.read(cx).focus().clone(), cx);
        }
        div()
            .size_full()
            .font_family(cx.theme().font_family.clone())
            .child(palette.card(window, cx))
    }
}
