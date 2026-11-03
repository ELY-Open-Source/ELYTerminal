use gpui::{
    App, Bounds, ElementId, IntoElement, ParentElement, Pixels, RenderOnce, SharedString, Styled,
    Window, canvas, div,
};

use super::IconButton;
use crate::primitives::IconName;

/// What to share: text, a link, or both.
#[derive(Clone, Debug, Default)]
struct Items {
    text: Option<SharedString>,
    url: Option<SharedString>,
}

struct Anchor {
    bounds: Bounds<Pixels>,
    #[cfg(target_os = "macos")]
    picker: Option<mac::Picker>,
}

/// Opens the system share picker beside itself. macOS only.
#[derive(IntoElement)]
pub struct ShareButton {
    id: ElementId,
    items: Items,
}

impl ShareButton {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            items: Items::default(),
        }
    }

    pub fn text(mut self, text: impl Into<SharedString>) -> Self {
        self.items.text = Some(text.into());
        self
    }

    pub fn url(mut self, url: impl Into<SharedString>) -> Self {
        self.items.url = Some(url.into());
        self
    }
}

impl RenderOnce for ShareButton {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        assert!(
            self.items.text.is_some() || self.items.url.is_some(),
            "share button {:?} has nothing to share",
            self.id
        );
        let anchor = window.use_keyed_state(self.id.clone(), cx, |_, _| Anchor {
            bounds: Bounds::default(),
            #[cfg(target_os = "macos")]
            picker: None,
        });
        let (measure, open) = (anchor.clone(), anchor);
        let items = self.items;
        let button = IconButton::new(self.id, IconName::Share2).tooltip("Share");
        #[cfg(target_os = "macos")]
        let button = button.on_click(move |_, window, cx| {
            let bounds = open.read(cx).bounds;
            match mac::Picker::show(&items, bounds, window) {
                Ok(picker) => open.update(cx, |anchor, _| anchor.picker = Some(picker)),
                Err(error) => log::error!("share button: {error:#}"),
            }
        });
        #[cfg(not(target_os = "macos"))]
        let button = {
            let _ = (open, items);
            button
                .disabled(true)
                .tooltip("Sharing needs macOS in this build")
        };
        div().relative().child(button).child(
            canvas(
                move |bounds, _, cx| {
                    if measure.read(cx).bounds != bounds {
                        measure.update(cx, |anchor, _| anchor.bounds = bounds);
                    }
                },
                |_, _, _, _| {},
            )
            .absolute()
            .top_0()
            .left_0()
            .size_full(),
        )
    }
}

#[cfg(target_os = "macos")]
mod mac {
    use anyhow::{Context as _, bail};
    use cocoa::{
        base::{id, nil},
        foundation::{NSArray, NSAutoreleasePool, NSPoint, NSRect, NSSize, NSString},
    };
    use gpui::{Bounds, Pixels, Window};
    use objc::{class, msg_send, sel, sel_impl};
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};

    use super::Items;

    /// A shown picker; kept alive until the next share.
    pub struct Picker(id);

    impl Drop for Picker {
        fn drop(&mut self) {
            unsafe {
                let _: () = msg_send![self.0, release];
            }
        }
    }

    fn string(text: &str) -> id {
        unsafe { NSString::alloc(nil).init_str(text).autorelease() }
    }

    impl Picker {
        pub fn show(
            items: &Items,
            bounds: Bounds<Pixels>,
            window: &Window,
        ) -> anyhow::Result<Self> {
            let handle = HasWindowHandle::window_handle(window)
                .map_err(|error| anyhow::anyhow!("window has no native handle: {error:?}"))?;
            let RawWindowHandle::AppKit(appkit) = handle.as_raw() else {
                bail!("share needs an AppKit window");
            };
            let height = f64::from(window.viewport_size().height);
            unsafe {
                let view = appkit.ns_view.as_ptr() as id;
                let mut list: Vec<id> = Vec::new();
                if let Some(text) = &items.text {
                    list.push(string(text));
                }
                if let Some(url) = &items.url {
                    let link: id = msg_send![class!(NSURL), URLWithString: string(url)];
                    anyhow::ensure!(link != nil, "share url {url} does not parse");
                    list.push(link);
                }
                let array = NSArray::arrayWithObjects(nil, &list);
                let picker: id = msg_send![class!(NSSharingServicePicker), alloc];
                let picker: id = msg_send![picker, initWithItems: array];
                let picker = (picker != nil)
                    .then_some(picker)
                    .context("AppKit refused a share picker")?;
                let rect = NSRect::new(
                    NSPoint::new(
                        f64::from(bounds.left()),
                        height - f64::from(bounds.bottom()),
                    ),
                    NSSize::new(f64::from(bounds.size.width), f64::from(bounds.size.height)),
                );
                let _: () =
                    msg_send![picker, showRelativeToRect: rect ofView: view preferredEdge: 1u64];
                log::info!("share button: picker shown for {} items", list.len());
                Ok(Self(picker))
            }
        }
    }
}
