use gpui::{Bounds, Decorations, Pixels, Size, Window, WindowBackgroundAppearance, point};

use crate::settings::{Backdrop, Settings};

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "linux")]
mod wayland;
#[cfg(windows)]
mod windows;
#[cfg(target_os = "linux")]
mod x11;

/// Whether a primary selection exists; Wayland may lack one.
#[cfg(target_os = "linux")]
pub fn primary_selection() -> bool {
    wayland::PRIMARY.load(std::sync::atomic::Ordering::Relaxed)
}

/// What the window shows of the desktop: blurred or clear while the background lets it through.
pub fn appearance(settings: &Settings) -> WindowBackgroundAppearance {
    if settings.background_opacity >= 1.0 {
        WindowBackgroundAppearance::Opaque
    } else if settings.background_blur {
        blurred(settings.backdrop)
    } else {
        WindowBackgroundAppearance::Transparent
    }
}

/// Whether the window can wear Windows 11's Mica.
pub fn mica() -> bool {
    #[cfg(windows)]
    return windows::mica();
    #[cfg(not(windows))]
    false
}

/// The backdrop's blur: Mica where Windows draws it, else Acrylic.
fn blurred(backdrop: Backdrop) -> WindowBackgroundAppearance {
    match backdrop {
        Backdrop::Mica if mica() => WindowBackgroundAppearance::MicaBackdrop,
        Backdrop::MicaAlt if mica() => WindowBackgroundAppearance::MicaAltBackdrop,
        _ => WindowBackgroundAppearance::Blurred,
    }
}

/// What the main window shows of the desktop, as last set, and on Linux the compositor's blur behind its pane.
pub struct Blur {
    shown: WindowBackgroundAppearance,
    #[cfg(target_os = "linux")]
    wayland: Option<wayland::Effect>,
    #[cfg(target_os = "linux")]
    x11: Option<x11::Effect>,
    #[cfg(windows)]
    moves: Option<windows::Moves>,
}

impl Blur {
    /// The window as `WindowOptions` opened it from these settings.
    pub fn new(window: &Window, settings: &Settings) -> Self {
        let shown = appearance(settings);
        #[cfg(target_os = "macos")]
        if shown == WindowBackgroundAppearance::Blurred {
            macos::blur(window);
        }
        #[cfg(windows)]
        let moves = windows::Moves::on(window).unwrap_or_else(|error| {
            log::error!("{error:#}");
            None
        });
        #[cfg(windows)]
        if let Some(moves) = &moves {
            moves.blurred(shown == WindowBackgroundAppearance::Blurred);
        }
        Self {
            shown,
            #[cfg(target_os = "linux")]
            wayland: wayland::Effect::on(window),
            #[cfg(target_os = "linux")]
            x11: x11::Effect::on(window),
            #[cfg(windows)]
            moves,
        }
    }

    /// Whether the desktop shows through the window.
    pub fn see_through(&self) -> bool {
        self.shown != WindowBackgroundAppearance::Opaque
    }

    /// Sets the window to what the settings ask when it differs from what it shows; logs the way taken.
    pub fn follow(&mut self, window: &Window, settings: &Settings) {
        let wanted = appearance(settings);
        if wanted == self.shown {
            return;
        }
        self.shown = wanted;
        window.set_background_appearance(wanted);
        log::info!("blur: the window is {wanted:?} on {}", std::env::consts::OS);
        #[cfg(target_os = "macos")]
        if wanted == WindowBackgroundAppearance::Blurred {
            macos::blur(window);
        }
        #[cfg(windows)]
        if let Some(moves) = &self.moves {
            moves.blurred(wanted == WindowBackgroundAppearance::Blurred);
        }
    }

    /// Keeps a Linux compositor's blur on the window's pane while Blurred; each frame, sending only a change.
    pub fn fit(&mut self, window: &Window) {
        #[cfg(not(target_os = "linux"))]
        let _ = window;
        #[cfg(target_os = "linux")]
        {
            let blurred = self.shown == WindowBackgroundAppearance::Blurred;
            let size = window.viewport_size();
            let pane =
                blurred.then(|| pane(size, window.client_inset(), window.window_decorations()));
            if let Some(effect) = &mut self.wayland {
                effect.blur(pane.map(|pane| rect(pane, 1.0)));
            }
            if let Some(effect) = &mut self.x11 {
                effect.blur(pane.map(|pane| rect(pane, window.scale_factor())));
            }
        }
    }
}

/// The window's pane in its surface: all of it, less the inset on each free edge of a client-drawn window, as Ely's `ResizeBorder` pads it.
pub(crate) fn pane(
    size: Size<Pixels>,
    inset: Option<Pixels>,
    decorations: Decorations,
) -> Bounds<Pixels> {
    let mut pane = Bounds::new(point(Pixels::ZERO, Pixels::ZERO), size);
    let (Decorations::Client { tiling }, Some(inset)) = (decorations, inset) else {
        return pane;
    };
    if !tiling.top {
        pane.origin.y += inset;
        pane.size.height -= inset;
    }
    if !tiling.bottom {
        pane.size.height -= inset;
    }
    if !tiling.left {
        pane.origin.x += inset;
        pane.size.width -= inset;
    }
    if !tiling.right {
        pane.size.width -= inset;
    }
    pane.size = pane.size.max(&Size::default());
    pane
}

/// `pane` in whole pixels at `scale`: x, y, width, height.
#[cfg(target_os = "linux")]
fn rect(pane: Bounds<Pixels>, scale: f32) -> [i32; 4] {
    let whole = |length: Pixels| (f32::from(length) * scale).round() as i32;
    [
        whole(pane.origin.x),
        whole(pane.origin.y),
        whole(pane.size.width),
        whole(pane.size.height),
    ]
}

#[cfg(test)]
mod tests {
    use gpui::WindowBackgroundAppearance::{
        Blurred, MicaAltBackdrop, MicaBackdrop, Opaque, Transparent,
    };

    use super::*;

    #[test]
    fn the_desktop_shows_only_through_a_background_it_can_pass() {
        let at = |opacity, blur| {
            appearance(&Settings {
                background_opacity: opacity,
                background_blur: blur,
                ..Settings::default()
            })
        };
        assert_eq!(at(1.0, true), Opaque);
        assert_eq!(at(1.0, false), Opaque);
        assert_eq!(at(0.88, true), Blurred);
        assert_eq!(at(0.3, false), Transparent);
    }

    #[test]
    fn mica_blurs_only_where_windows_draws_it() {
        let with = |backdrop| {
            appearance(&Settings {
                background_opacity: 0.88,
                backdrop,
                ..Settings::default()
            })
        };
        assert_eq!(with(Backdrop::Acrylic), Blurred);
        let (wanted, alt) = if mica() {
            (MicaBackdrop, MicaAltBackdrop)
        } else {
            (Blurred, Blurred)
        };
        assert_eq!(with(Backdrop::Mica), wanted);
        assert_eq!(with(Backdrop::MicaAlt), alt);
        // Clear or opaque, the backdrop changes nothing.
        let clear = Settings {
            background_opacity: 0.88,
            background_blur: false,
            backdrop: Backdrop::Mica,
            ..Settings::default()
        };
        assert_eq!(appearance(&clear), Transparent);
        assert!(cfg!(windows) || !mica(), "Mica is Windows' alone");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn the_pane_leaves_the_margin_on_each_free_edge() {
        use gpui::{Tiling, px, size};
        let surface = size(px(1220.), px(780.));
        let inset = Some(px(10.));
        let at = |decorations| rect(pane(surface, inset, decorations), 2.0);
        assert_eq!(at(Decorations::Server), [0, 0, 2440, 1560]);
        let floating = Decorations::Client {
            tiling: Tiling::default(),
        };
        assert_eq!(at(floating), [20, 20, 2400, 1520]);
        let left = Tiling {
            left: true,
            top: true,
            bottom: true,
            right: false,
        };
        let half = Decorations::Client { tiling: left };
        assert_eq!(at(half), [0, 0, 2420, 1560]);
        assert_eq!(rect(pane(surface, None, floating), 1.0), [0, 0, 1220, 780]);
        let tiny = size(px(15.), px(15.));
        assert_eq!(rect(pane(tiny, inset, floating), 1.0), [10, 10, 0, 0]);
    }
}
