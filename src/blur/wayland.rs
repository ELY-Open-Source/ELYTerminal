use std::{
    ffi::c_void,
    mem::ManuallyDrop,
    ptr::NonNull,
    sync::atomic::{AtomicBool, Ordering},
};

use anyhow::Context as _;
use gpui::Window;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle, RawDisplayHandle, RawWindowHandle};
use wayland_backend::client::{Backend, ObjectId};
use wayland_client::{
    Connection, Dispatch, Proxy, QueueHandle, delegate_noop,
    globals::{BindError, GlobalListContents, registry_queue_init},
    protocol::{
        wl_compositor::WlCompositor, wl_region::WlRegion, wl_registry::WlRegistry,
        wl_surface::WlSurface,
    },
};
use wayland_protocols::ext::background_effect::v1::client::{
    ext_background_effect_manager_v1::{self, Capability, ExtBackgroundEffectManagerV1},
    ext_background_effect_surface_v1::ExtBackgroundEffectSurfaceV1,
};

/// The compositor's blur behind a window's surface, through `ext_background_effect_v1`.
pub struct Effect {
    // Never dropped: gpui may close the display first, and a drop touches it.
    connection: ManuallyDrop<Connection>,
    queue: QueueHandle<Offers>,
    compositor: WlCompositor,
    effect: ExtBackgroundEffectSurfaceV1,
    /// The rect the compositor blurs behind, as last sent.
    blurs: Option<[i32; 4]>,
}

/// Whether the compositor offers a primary selection; X11 always does.
pub(super) static PRIMARY: AtomicBool = AtomicBool::new(true);

/// What the compositor offers, as its events say.
#[derive(Default)]
struct Offers {
    blur: bool,
}

impl Effect {
    /// The effect on `window`'s surface, blurring nothing yet; none off Wayland or where the compositor lacks it.
    pub fn on(window: &Window) -> Option<Self> {
        let handles = (
            HasWindowHandle::window_handle(window),
            window.display_handle(),
        );
        let (Ok(surface), Ok(display)) = handles else {
            log::error!("blur: gpui gave no handle to the window");
            return None;
        };
        let (RawWindowHandle::Wayland(surface), RawDisplayHandle::Wayland(display)) =
            (surface.as_raw(), display.as_raw())
        else {
            log::info!("blur: no Wayland surface, so no ext_background_effect_v1");
            return None;
        };
        // SAFETY: gpui's live display, and the window's live surface on it.
        match unsafe { Self::new(display.display, surface.surface) } {
            Ok(effect) => effect,
            Err(error) => {
                log::error!("blur: ext_background_effect_v1 failed: {error:#}");
                None
            }
        }
    }

    /// An effect on `surface`, none where the compositor lacks the protocol; `display` must outlive every call on it.
    unsafe fn new(
        display: NonNull<c_void>,
        surface: NonNull<c_void>,
    ) -> anyhow::Result<Option<Self>> {
        // SAFETY: the caller's live display.
        let backend = unsafe { Backend::from_foreign_display(display.as_ptr().cast()) };
        let connection = Connection::from_backend(backend);
        let (globals, mut queue) =
            registry_queue_init::<Offers>(&connection).context("blur: Wayland's globals")?;
        let qh = queue.handle();
        let primary = globals.contents().with_list(|list| {
            list.iter()
                .any(|global| global.interface == "zwp_primary_selection_device_manager_v1")
        });
        PRIMARY.store(primary, Ordering::Relaxed);
        log::info!("clipboard: the compositor offers a primary selection: {primary}");
        let kde = globals.contents().with_list(|list| {
            list.iter()
                .any(|global| global.interface == "org_kde_kwin_blur_manager")
        });
        let manager: ExtBackgroundEffectManagerV1 = match globals.bind(&qh, 1..=1, ()) {
            Ok(manager) => manager,
            Err(BindError::NotPresent) => {
                if kde {
                    log::info!(
                        "blur: the compositor offers no ext_background_effect_v1; gpui blurs through org_kde_kwin_blur"
                    );
                } else {
                    log::info!(
                        "blur: the compositor offers neither ext_background_effect_v1 nor org_kde_kwin_blur; a see-through window shows the desktop clear"
                    );
                }
                return Ok(None);
            }
            Err(error) => {
                return Err(error).context("blur: ext_background_effect_manager_v1 did not bind");
            }
        };
        let compositor: WlCompositor =
            (globals.bind(&qh, 1..=1, ())).context("blur: wl_compositor did not bind")?;
        let mut offers = Offers::default();
        (queue.roundtrip(&mut offers)).context("blur: the compositor's capabilities")?;
        // SAFETY: the caller's live surface.
        let id = unsafe { ObjectId::from_ptr(WlSurface::interface(), surface.as_ptr().cast()) }
            .context("blur: the window's surface is no wl_surface")?;
        let surface =
            WlSurface::from_id(&connection, id).context("blur: the window's surface is gone")?;
        let effect = manager.get_background_effect(&surface, &qh, ());
        log::info!(
            "blur: ext_background_effect_v1 on the window's surface; the compositor blurs: {}, org_kde_kwin_blur too: {kde}",
            offers.blur
        );
        Ok(Some(Self {
            connection: ManuallyDrop::new(connection),
            queue: qh,
            compositor,
            effect,
            blurs: None,
        }))
    }

    /// Blurs behind `rect`, in surface pixels, or nothing, from the surface's next commit; sends only a change.
    pub fn blur(&mut self, rect: Option<[i32; 4]>) {
        if rect == self.blurs {
            return;
        }
        let turned = rect.is_some() != self.blurs.is_some();
        self.blurs = rect;
        let region = rect.map(|[x, y, width, height]| {
            let region = self.compositor.create_region(&self.queue, ());
            region.add(x, y, width, height);
            region
        });
        self.effect.set_blur_region(region.as_ref());
        // The effect copies the region.
        if let Some(region) = region {
            region.destroy();
        }
        let what = rect.map_or("nothing".into(), |rect| format!("{rect:?}"));
        if let Err(error) = self.connection.flush() {
            log::error!("blur: ext_background_effect_v1 did not reach the compositor: {error}");
        } else if turned {
            log::info!("blur: ext_background_effect_v1 blurs {what}");
        } else {
            log::debug!("blur: ext_background_effect_v1 blurs {what}");
        }
    }
}

impl Dispatch<WlRegistry, GlobalListContents> for Offers {
    fn event(
        _: &mut Self,
        _: &WlRegistry,
        _: <WlRegistry as Proxy>::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ExtBackgroundEffectManagerV1, ()> for Offers {
    fn event(
        offers: &mut Self,
        _: &ExtBackgroundEffectManagerV1,
        event: ext_background_effect_manager_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let ext_background_effect_manager_v1::Event::Capabilities { flags } = event {
            // Bits this version does not know leave the blur bit readable.
            offers.blur = u32::from(flags) & Capability::Blur.bits() != 0;
        }
    }
}

delegate_noop!(Offers: WlCompositor);
delegate_noop!(Offers: WlRegion);
delegate_noop!(Offers: ExtBackgroundEffectSurfaceV1);

#[cfg(test)]
mod tests;
