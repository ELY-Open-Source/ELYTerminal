//! A start that fails says why in a dialog, since a release build on Windows logs to no console.

use ely_gpui_component::i18n::{I18n, text};
use gpui::{
    App, AppContext as _, Bounds, Context, IntoElement, PromptLevel, Render, Window, WindowBounds,
    WindowOptions, div, px, size,
};

use crate::{i18n, settings::Language};

/// The dialog's window; the dialog is all it shows.
struct Refused;

impl Render for Refused {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

/// Logs `error`, shows it, and quits once the dialog is answered.
pub fn refused(error: &anyhow::Error, cx: &mut App) -> anyhow::Result<()> {
    log::error!("{error:#}");
    // The settings may be what failed, so their language may be unknown.
    if !cx.has_global::<I18n>() {
        cx.set_global(i18n::catalogs(Language::English));
    }
    let title = text(cx, "et.start.failed", &[]);
    let quit = text(cx, "et.start.quit", &[]);
    let detail = format!("{error:#}");
    let bounds = Bounds::centered(None, size(px(480.), px(240.)), cx);
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        ..WindowOptions::default()
    };
    cx.open_window(options, |window, cx| {
        let answer = window.prompt(
            PromptLevel::Critical,
            &title,
            Some(&detail),
            &[quit.as_ref()],
            cx,
        );
        cx.spawn(async move |cx| {
            let _ = answer.await;
            log::info!("app: the refused start is read; quitting");
            cx.update(|cx| cx.quit());
        })
        .detach();
        window.on_window_should_close(cx, |_, cx| {
            log::info!("app: the refused start's window closes; quitting");
            cx.quit();
            true
        });
        cx.new(|_| Refused)
    })?;
    Ok(())
}
