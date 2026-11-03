use std::rc::Rc;

use gpui::{AnyWindowHandle, App, AsyncApp, ElementId, Hsla, IntoElement, RenderOnce, Window};

use crate::{
    buttons::{ButtonVariant, IconButton},
    primitives::IconName,
};

type OnColor = Rc<dyn Fn(Hsla, &mut Window, &mut App)>;

/// A button that picks a color from anywhere on screen. macOS only.
#[derive(IntoElement)]
pub struct EyeDropper {
    id: ElementId,
    on_pick: Option<OnColor>,
}

impl EyeDropper {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            on_pick: None,
        }
    }

    pub fn on_pick(mut self, handler: impl Fn(Hsla, &mut Window, &mut App) + 'static) -> Self {
        self.on_pick = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for EyeDropper {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let on_pick = self.on_pick;
        let available = cfg!(target_os = "macos");
        IconButton::new(self.id, IconName::Pipette)
            .variant(ButtonVariant::Ghost)
            .tooltip(if available {
                "Pick a color from the screen"
            } else {
                "Picking from the screen needs macOS"
            })
            .disabled(!available)
            .on_click(move |_, window, cx| {
                sample(window.window_handle(), cx.to_async(), on_pick.clone())
            })
    }
}

/// An `NSColor` as sRGB, or `None` when AppKit cannot convert it.
#[cfg(target_os = "macos")]
unsafe fn read_srgb(color: cocoa::base::id) -> Option<gpui::Rgba> {
    use cocoa::base::{id, nil};
    use objc::{class, msg_send, sel, sel_impl};
    unsafe {
        let space: id = msg_send![class!(NSColorSpace), sRGBColorSpace];
        let srgb: id = msg_send![color, colorUsingColorSpace: space];
        if srgb == nil {
            return None;
        }
        let (r, g, b, a): (f64, f64, f64, f64) = (
            msg_send![srgb, redComponent],
            msg_send![srgb, greenComponent],
            msg_send![srgb, blueComponent],
            msg_send![srgb, alphaComponent],
        );
        Some(gpui::Rgba {
            r: r as f32,
            g: g as f32,
            b: b as f32,
            a: a as f32,
        })
    }
}

/// Opens AppKit's color sampler; its answer comes back on the main thread.
#[cfg(target_os = "macos")]
fn sample(window: AnyWindowHandle, cx: AsyncApp, on_pick: Option<OnColor>) {
    use block::ConcreteBlock;
    use cocoa::base::{id, nil};
    use objc::{class, msg_send, sel, sel_impl};

    log::info!("eye dropper: sampling");
    unsafe {
        let sampler: id = msg_send![class!(NSColorSampler), new];
        let handler = ConcreteBlock::new(move |color: id| {
            let _: () = msg_send![sampler, release];
            if color == nil {
                log::info!("eye dropper: cancelled");
                return;
            }
            let Some(rgba) = read_srgb(color) else {
                log::error!("eye dropper: the color has no sRGB form");
                return;
            };
            log::info!("eye dropper: picked {}", super::hex(rgba));
            let Some(on_pick) = on_pick.clone() else {
                return;
            };
            let mut cx = cx.clone();
            if let Err(error) =
                window.update(&mut cx, |_, window, cx| on_pick(rgba.into(), window, cx))
            {
                log::error!("eye dropper: the window closed first: {error:#}");
            }
        })
        .copy();
        let _: () = msg_send![sampler, showSamplerWithSelectionHandler: &*handler];
    }
}

#[cfg(not(target_os = "macos"))]
fn sample(_: AnyWindowHandle, _: AsyncApp, _: Option<OnColor>) {
    unreachable!("the eye dropper stays disabled off macOS");
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use cocoa::base::id;
    use objc::{class, msg_send, sel, sel_impl};

    use super::read_srgb;

    #[test]
    fn a_display_p3_color_reads_back_as_srgb() {
        let color: id = unsafe {
            msg_send![class!(NSColor), colorWithDisplayP3Red: 1.0f64 green: 0.0f64 blue: 0.0f64 alpha: 0.5f64]
        };
        let srgb = unsafe { read_srgb(color) }.expect("P3 red converts to sRGB");
        assert!(srgb.r > 0.99 && srgb.g < 0.01 && srgb.b < 0.01, "{srgb:?}");
        assert!((srgb.a - 0.5).abs() < 1e-6);
    }
}
