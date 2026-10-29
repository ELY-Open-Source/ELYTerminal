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
    settings::{self, Place, SettingsStore},
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

/// The kept size and place on its display, else centered on the primary.
fn opening_bounds(cx: &App) -> (Bounds<Pixels>, Option<DisplayId>) {
    let settings = SettingsStore::get(cx);
    let kept = size(px(settings.window_width), px(settings.window_height));
    if let Some(place) = &settings.window_place {
        let displays = cx.displays();
        let ids: Vec<_> = displays
            .iter()
            .map(|display| display.uuid().ok().map(|id| id.to_string()))
            .collect();
        if let Some(at) = named(&ids, &place.display) {
            let display = &displays[at];
            let area = display.visible_bounds();
            let origin = display.bounds().origin + point(px(place.x), px(place.y));
            match placed(Bounds::new(origin, kept), display.bounds(), area) {
                Some(bounds) => {
                    log::info!("app: the window opens where it was left, on {area:?}");
                    return (bounds, Some(display.id()));
                }
                None => log::info!("app: the kept place lies off its display; centered"),
            }
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

/// The one display of `ids` named `kept`; none when gone or shared.
fn named(ids: &[Option<String>], kept: &str) -> Option<usize> {
    let mut found = (0..ids.len()).filter(|&at| ids[at].as_deref() == Some(kept));
    match (found.next(), found.next()) {
        (Some(at), None) => Some(at),
        (None, _) => {
            log::info!("app: display {kept} is gone; centered");
            None
        }
        (Some(_), Some(_)) => {
            log::info!("app: display {kept} names more than one display; centered");
            None
        }
    }
}

/// `kept` fitted and moved inside `area`; none when its middle left `display`.
fn placed(
    kept: Bounds<Pixels>,
    display: Bounds<Pixels>,
    area: Bounds<Pixels>,
) -> Option<Bounds<Pixels>> {
    let fitted = Bounds::new(kept.origin, fit(kept.size, Some(area.size)));
    display
        .contains(&kept.center())
        .then(|| inside(fitted, area))
}

/// `bounds` moved inside `area`, which holds its size.
fn inside(bounds: Bounds<Pixels>, area: Bounds<Pixels>) -> Bounds<Pixels> {
    let clamp =
        |at: Pixels, low: Pixels, side: Pixels, room: Pixels| at.min(low + room - side).max(low);
    let origin = point(
        clamp(
            bounds.origin.x,
            area.origin.x,
            bounds.size.width,
            area.size.width,
        ),
        clamp(
            bounds.origin.y,
            area.origin.y,
            bounds.size.height,
            area.size.height,
        ),
    );
    Bounds::new(origin, bounds.size)
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
                let place = window.display(cx).and_then(|display| {
                    let at = bounds.origin - display.bounds().origin;
                    match display.uuid() {
                        Ok(id) => Some(Place {
                            display: id.to_string(),
                            x: f32::from(at.x).round(),
                            y: f32::from(at.y).round(),
                        }),
                        Err(error) => {
                            log::warn!("app: the window's display has no uuid: {error:#}");
                            None
                        }
                    }
                });
                let [width, height] =
                    [bounds.size.width, bounds.size.height].map(|side| f32::from(side).round());
                SettingsStore::update(cx, |settings| {
                    settings.window_width = width;
                    settings.window_height = height;
                    if place.is_some() {
                        settings.window_place = place.clone();
                    }
                })
                .with_context(|| {
                    format!("app: the window, {width} by {height} at {place:?}, is not kept")
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

    use super::{inside, named, placed};

    fn area(x: f32, y: f32, width: f32, height: f32) -> Bounds<gpui::Pixels> {
        Bounds::new(point(px(x), px(y)), size(px(width), px(height)))
    }

    #[test]
    fn a_place_whose_monitor_went_opens_nowhere() {
        let screen = area(0., 0., 1920., 1080.);
        let window = |x, y| area(x, y, 900., 600.);
        assert_eq!(placed(window(2500., 100.), screen, screen), None);
        assert_eq!(
            placed(window(400., -200.), screen, screen),
            Some(window(400., 0.))
        );
        assert_eq!(
            placed(window(400., 100.), screen, screen),
            Some(window(400., 100.))
        );
        let (display, visible) = (area(0., 0., 3008., 1269.), area(0., 30., 3008., 1148.));
        let tall = area(300., -600., 1200., 1269.);
        let fitted = area(300., 30., 1200., 1148.);
        assert_eq!(placed(tall, display, visible), Some(fitted));
    }

    #[test]
    fn a_window_opens_on_the_one_display_its_uuid_names() {
        let ids = |all: &[&str]| {
            all.iter()
                .map(|id| Some(id.to_string()))
                .collect::<Vec<_>>()
        };
        assert_eq!(named(&ids(&["a", "b"]), "b"), Some(1));
        assert_eq!(named(&ids(&["a"]), "z"), None);
        assert_eq!(
            named(&ids(&["0", "0"]), "0"),
            None,
            "X screens share a nil uuid"
        );
        assert_eq!(named(&[None, Some("b".into())], "b"), Some(1));
    }

    #[test]
    fn a_window_opens_inside_its_display() {
        let area = area(-2560., 25., 2560., 1415.);
        let kept = |x, y| {
            inside(
                super::Bounds::new(point(px(x), px(y)), size(px(900.), px(600.))),
                area,
            )
            .origin
        };
        assert_eq!(kept(-2000., 300.), point(px(-2000.), px(300.)));
        assert_eq!(kept(-2000., -300.), point(px(-2000.), px(25.)));
        assert_eq!(kept(-100., 1200.), point(px(-900.), px(840.)));
        assert_eq!(kept(-9000., 300.), point(px(-2560.), px(300.)));
    }
}
