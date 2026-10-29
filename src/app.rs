use std::{borrow::Cow, cell::RefCell, rc::Rc, time::Duration};

use anyhow::Context as _;
use ely_gpui_component::theme::ActiveTheme;
use gpui::{
    App, AppContext, Bounds, Context, DisplayId, Pixels, Size, Task, TitlebarOptions, Window,
    WindowBounds, WindowDecorations, WindowHandle, WindowOptions, point, px, size,
};

#[cfg(target_os = "macos")]
use crate::term::mac;
use crate::{
    blur, i18n, menus,
    settings::{self, SettingsStore},
    term::{ICONS, Launch, MONO, config, input, shell::ShellIntegration, spawn},
    themes, updates,
    workspace::{self, TITLE, Workspace},
};

/// How long the window's size holds before it is kept, so a drag saves once.
const SETTLE: Duration = Duration::from_millis(500);

/// The id the macOS bundle and Linux desktop entry share.
pub const APP_ID: &str = "com.zacharyzhang.elyterminal";

/// Fonts, settings, language and scripts, checked before any window.
pub fn start(cx: &mut App) -> anyhow::Result<()> {
    cx.text_system()
        .add_fonts(vec![
            Cow::Borrowed(include_bytes!("../assets/fonts/JetBrainsMono-Bold.ttf")),
            Cow::Borrowed(include_bytes!("../assets/fonts/JetBrainsMono-Italic.ttf")),
            Cow::Borrowed(include_bytes!(
                "../assets/fonts/JetBrainsMono-BoldItalic.ttf"
            )),
            Cow::Borrowed(include_bytes!(
                "../assets/fonts/JetBrainsMonoNerdFontMono-Regular.ttf"
            )),
        ])
        .context("app: the bundled fonts failed to register")?;
    let names = cx.text_system().all_font_names();
    for family in [MONO, ICONS] {
        anyhow::ensure!(
            names.iter().any(|name| name == family),
            "app: {family} is not among the registered fonts"
        );
    }
    SettingsStore::init(cx)?;
    i18n::init(cx);
    updates::start(cx);
    ShellIntegration::init(cx)?;
    input::bind_keys(cx);
    workspace::bind_keys(cx);
    settings::window::bind_keys(cx);
    menus::init(cx);
    cx.set_app_identity(APP_ID, TITLE);
    // A press on a notification goes to the window whose session posted it, or to a newer release's page.
    cx.on_system_notification_response(|response, cx| {
        let tag = response.tag;
        if updates::pressed(&tag, cx) {
            return;
        }
        let pressed = cx
            .windows()
            .into_iter()
            .filter_map(|window| window.downcast::<Workspace>())
            .any(|workspace| {
                workspace
                    .update(cx, |this, window, cx| this.pressed(&tag, window, cx))
                    .unwrap_or(false)
            });
        if !pressed {
            log::info!("app: notification {tag} pressed, but no session holds it");
        }
    });
    // Settings alone hold no terminal, so they go with the last one.
    cx.on_window_closed(|cx, _| {
        let windows = cx.windows();
        if !windows
            .iter()
            .any(|window| window.downcast::<Workspace>().is_some())
        {
            log::info!("app: no terminal window left; quitting");
            cx.quit();
        }
    })
    .detach();
    #[cfg(target_os = "macos")]
    {
        mac::repeat_held_keys();
        mac::deliver_command_key_ups()?;
    }
    #[cfg(target_os = "linux")]
    themes::omarchy::start(cx)?;
    themes::apply(cx);
    let chosen = |settings: &settings::Settings| {
        let theme = (settings.theme.clone(), settings.dark_theme.clone());
        (theme, settings.background_opacity)
    };
    let mut worn = chosen(SettingsStore::get(cx));
    cx.observe_global::<SettingsStore>(move |cx| {
        let now = chosen(SettingsStore::get(cx));
        if now != worn {
            worn = now;
            themes::apply(cx);
        }
    })
    .detach();
    Ok(())
}

/// Opens the main window running `launch`.
pub fn open(launch: Launch, cx: &mut App) -> anyhow::Result<WindowHandle<Workspace>> {
    let (bounds, display_id) = opening_bounds(cx);
    let background = blur::appearance(SettingsStore::get(cx));
    let least = px(*settings::WINDOW.start());
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        display_id,
        // No smaller than the settings keep.
        window_min_size: Some(size(least, least)),
        titlebar: Some(TitlebarOptions {
            title: Some(TITLE.into()),
            appears_transparent: true,
            traffic_light_position: Some(cx.theme().traffic_light_origin()),
        }),
        // Ely's columns draw the controls; only Linux reads this, and must not draw its own.
        window_decorations: Some(WindowDecorations::Client),
        window_background: background,
        ..Default::default()
    };
    let (pty, events) = spawn(&launch, (80, 24), (8, 16), config(SettingsStore::get(cx)))?;
    // Windows' wheel settings, read where gpui reads its own.
    #[cfg(windows)]
    crate::term::keep_wheel()?;
    let window = cx.open_window(options, |window, cx| {
        window.set_app_id(APP_ID);
        window
            .observe_window_appearance(|_, cx| themes::apply(cx))
            .detach();
        cx.new(|cx| {
            keep_bounds(window, cx);
            Workspace::new(pty, events, launch, window, cx)
        })
    })?;
    crate::pet::follow(window, cx);
    log::info!(
        "app: main window open at {:?}, blur: {background:?} on {}",
        bounds,
        std::env::consts::OS
    );
    Ok(window)
}

/// The kept place and size on the display holding them, else centered on the primary.
fn opening_bounds(cx: &App) -> (Bounds<Pixels>, Option<DisplayId>) {
    let settings = SettingsStore::get(cx);
    let kept = size(px(settings.window_width), px(settings.window_height));
    if let Some([x, y]) = settings.window_place {
        let place = Bounds::new(point(px(x), px(y)), kept);
        let displays = cx.displays();
        let areas: Vec<_> = displays.iter().map(|display| display.bounds()).collect();
        match home(place, &areas) {
            Some(at) => {
                let display = &displays[at];
                let fitted = fit(kept, Some(display.visible_bounds().size));
                log::info!(
                    "app: the window opens where it was left, on {:?}",
                    areas[at]
                );
                return (Bounds::new(place.origin, fitted), Some(display.id()));
            }
            None => log::info!("app: the kept place {x}, {y} lies on no display; centered"),
        }
    }
    let room = cx
        .primary_display()
        .map(|display| display.visible_bounds().size);
    let fitted = fit(kept, room);
    // macOS grows a window by a point on a side whose edge falls on a half point.
    let centered = Bounds::centered(None, fitted, cx);
    (
        Bounds::new(centered.origin.map(|at| at.round()), fitted),
        None,
    )
}

/// The display, of `areas`, that holds the middle of `place`.
fn home(place: Bounds<Pixels>, areas: &[Bounds<Pixels>]) -> Option<usize> {
    areas.iter().position(|area| area.contains(&place.center()))
}

/// `kept`, each side no wider than `room` where that is known.
fn fit(kept: Size<Pixels>, room: Option<Size<Pixels>>) -> Size<Pixels> {
    let side = |want: Pixels, side: Option<Pixels>| match side {
        Some(side) if side > px(0.) && want > side => {
            log::info!(
                "app: a kept side of {} px passes the screen's {}; the window takes it",
                f32::from(want),
                f32::from(side)
            );
            side
        }
        _ => want,
    };
    size(
        side(kept.width, room.map(|area| area.width)),
        side(kept.height, room.map(|area| area.height)),
    )
}

/// Keeps the window's place and size once settled, unless maximized, full screen or hidden.
fn keep_bounds(window: &mut Window, cx: &mut Context<Workspace>) {
    let settling: Rc<RefCell<Option<Task<()>>>> = Rc::default();
    let settle = move |window: &mut Window, cx: &mut Context<Workspace>| {
        let task = cx.spawn_in(window, async move |_, cx| {
            cx.background_executor().timer(SETTLE).await;
            // Read once settled: X11 marks a window maximized late, and unmaximized while hidden.
            let kept = cx.update(|window, cx| {
                if !window.is_visible() || window.is_maximized() || window.is_fullscreen() {
                    log::debug!(
                        "app: the window is hidden, maximized or full screen; its kept bounds stay"
                    );
                    return Ok(());
                }
                let bounds = window.window_bounds().get_bounds();
                let [x, y, width, height] = [
                    bounds.origin.x,
                    bounds.origin.y,
                    bounds.size.width,
                    bounds.size.height,
                ]
                .map(|side| f32::from(side).round());
                SettingsStore::update(cx, |settings| {
                    settings.window_width = width;
                    settings.window_height = height;
                    settings.window_place = Some([x, y]);
                })
                .with_context(|| {
                    format!("app: the window at {x}, {y}, {width} by {height}, is not kept")
                })
            });
            match kept {
                Ok(Ok(())) => {}
                Ok(Err(error)) => log::error!("{error:#}"),
                Err(_) => log::debug!("app: the window closed before its bounds settled"),
            }
        });
        settling.replace(Some(task));
    };
    let shown = settle.clone();
    cx.observe_window_bounds(window, move |_, window, cx| settle(window, cx))
        .detach();
    // Bounds skipped while hidden are read once it shows.
    cx.observe_window_visibility(window, move |_, visibility, window, cx| {
        if visibility.is_visible() {
            shown(window, cx);
        }
    })
    .detach();
}

#[cfg(test)]
mod tests {
    use gpui::{Bounds, point, px, size};

    use super::home;

    fn area(x: f32, y: f32, width: f32, height: f32) -> Bounds<gpui::Pixels> {
        Bounds::new(point(px(x), px(y)), size(px(width), px(height)))
    }

    #[test]
    fn a_window_opens_on_the_display_holding_its_middle() {
        let left = area(-2560., -200., 2560., 1440.);
        let main = area(0., 0., 1920., 1080.);
        let displays = [main, left];
        assert_eq!(home(area(-1800., 100., 1200., 760.), &displays), Some(1));
        assert_eq!(home(area(100., 100., 1200., 760.), &displays), Some(0));
        assert_eq!(home(area(1300., 100., 1200., 760.), &displays), Some(0));
        assert_eq!(home(area(1800., 100., 1200., 760.), &displays), None);
        assert_eq!(home(area(4000., 100., 1200., 760.), &[main]), None);
    }
}
