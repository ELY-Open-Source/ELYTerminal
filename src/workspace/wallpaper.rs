//! The painted background's place: the terminal's box or the window's, below the title bar.

use ely_gpui_component::theme::ActiveTheme;
use gpui::{AnyElement, App, Hsla, IntoElement, ParentElement, Styled, Window, div, px};

use crate::{
    background,
    settings::{Scope, Settings},
};

/// The box a background takes in the window's points: x, y, width and height.
fn place(
    scope: Scope,
    across: f32,
    down: f32,
    title: f32,
    sessions: Option<f32>,
    panel: Option<f32>,
) -> (f32, f32, f32, f32) {
    let title = title.clamp(0.0, down);
    let (left, width) = match scope {
        Scope::Window => (0.0, across),
        // The columns' glass leaves the ground to the background, so it takes the terminal's own width.
        Scope::Terminal => {
            let left = sessions.unwrap_or(0.0).clamp(0.0, across);
            let right = panel.unwrap_or(0.0).clamp(0.0, across - left);
            (left, (across - left - right).max(0.0))
        }
    };
    (left, title, width, (down - title).max(0.0))
}

/// The background painted for `settings`, as see-through as the window, or none when it paints nothing.
pub(super) fn painted(
    settings: &Settings,
    base: Hsla,
    window: &mut Window,
    cx: &mut App,
) -> Option<AnyElement> {
    if settings.background.is_empty() {
        return None;
    }
    let style = match settings.background.style() {
        Ok(style) => style,
        Err(error) => {
            log::error!("background: {error:#}");
            return None;
        }
    };
    let viewport = window.viewport_size();
    let (across, down) = (f32::from(viewport.width), f32::from(viewport.height));
    let title = f32::from(cx.theme().titlebar_height().to_pixels(window.rem_size()));
    let sessions = settings.show_sessions.then_some(settings.sessions_width);
    let panel = settings.show_panel.then_some(settings.panel_width);
    let (left, top, width, height) = place(
        settings.background.scope,
        across,
        down,
        title,
        sessions,
        panel,
    );
    let layers = background::render(&style, (width, height), window, cx)?;
    Some(
        div()
            .absolute()
            .inset_0()
            .opacity(settings.background_opacity)
            // The theme grounds the title bar and every column the picture leaves.
            .child(div().absolute().inset_0().bg(base))
            .child(
                div()
                    .absolute()
                    .left(px(left))
                    .top(px(top))
                    .w(px(width))
                    .h(px(height))
                    .overflow_hidden()
                    .child(layers),
            )
            .into_any_element(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_background_takes_the_terminal_or_the_window_below_the_title_bar() {
        let terminal = place(
            Scope::Terminal,
            1000.0,
            800.0,
            40.0,
            Some(160.0),
            Some(200.0),
        );
        assert_eq!(terminal, (160.0, 40.0, 640.0, 760.0));
        let window = place(Scope::Window, 1000.0, 800.0, 40.0, Some(160.0), Some(200.0));
        assert_eq!(window, (0.0, 40.0, 1000.0, 760.0));
        let alone = place(Scope::Terminal, 1000.0, 800.0, 40.0, None, None);
        assert_eq!(alone, (0.0, 40.0, 1000.0, 760.0));
    }

    #[test]
    fn columns_wider_than_the_window_leave_the_picture_no_width() {
        let squeezed = place(
            Scope::Terminal,
            300.0,
            800.0,
            40.0,
            Some(400.0),
            Some(200.0),
        );
        assert_eq!((squeezed.0, squeezed.2), (300.0, 0.0));
    }

    #[test]
    fn a_title_bar_taller_than_the_window_leaves_the_picture_no_height() {
        let short = place(Scope::Window, 1000.0, 20.0, 40.0, None, None);
        assert_eq!((short.1, short.3), (20.0, 0.0));
    }
}
