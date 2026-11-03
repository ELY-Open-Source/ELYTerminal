use std::rc::Rc;

use gpui::{App, SharedString};

/// One entry in a tray menu.
#[derive(Clone, Debug)]
pub enum TrayItem {
    Action(SharedString),
    Separator,
}

type OnPick = Rc<dyn Fn(usize, &mut App)>;

/// An icon in the system menu bar with a menu. Dropping it removes it.
pub struct TrayIcon {
    #[cfg(target_os = "macos")]
    inner: mac::Tray,
}

impl TrayIcon {
    /// `symbol` names an SF Symbol. `on_pick` gets the index of the chosen item.
    pub fn new(
        symbol: &str,
        items: Vec<TrayItem>,
        on_pick: impl Fn(usize, &mut App) + 'static,
        cx: &mut App,
    ) -> anyhow::Result<Self> {
        let on_pick: OnPick = Rc::new(on_pick);
        #[cfg(target_os = "macos")]
        return Ok(Self {
            inner: mac::Tray::new(symbol, &items, on_pick, cx)?,
        });
        #[cfg(not(target_os = "macos"))]
        {
            let _ = (symbol, items, on_pick, cx);
            anyhow::bail!("tray icons need macOS in this build")
        }
    }

    /// Chooses item `index` as a click would.
    pub fn pick(&self, index: usize) {
        #[cfg(target_os = "macos")]
        self.inner.pick(index);
        #[cfg(not(target_os = "macos"))]
        let _ = index;
    }
}

/// Sets or clears the badge on the app's Dock icon.
pub fn set_dock_badge(label: Option<&str>) -> anyhow::Result<()> {
    #[cfg(target_os = "macos")]
    {
        mac::set_badge(label);
        log::info!("dock badge: {label:?}");
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = label;
        anyhow::bail!("dock badges need macOS in this build")
    }
}

/// The badge the Dock shows now.
pub fn dock_badge() -> anyhow::Result<Option<String>> {
    #[cfg(target_os = "macos")]
    return Ok(mac::badge());
    #[cfg(not(target_os = "macos"))]
    anyhow::bail!("dock badges need macOS in this build")
}

#[cfg(target_os = "macos")]
mod mac {
    use std::{ffi::CStr, sync::Once};

    use cocoa::{
        appkit::{
            NSApp, NSMenu, NSMenuItem, NSStatusBar, NSStatusItem, NSVariableStatusItemLength,
        },
        base::{id, nil},
        foundation::{NSAutoreleasePool, NSString},
    };
    use gpui::{App, AsyncApp};
    use objc::{
        class,
        declare::ClassDecl,
        msg_send,
        runtime::{Class, Object, Sel},
        sel, sel_impl,
    };

    use super::{OnPick, TrayItem};

    const TARGET: &str = "ElyTrayTarget";
    const ROUTE: &str = "elyRoute";

    struct Route {
        app: AsyncApp,
        on_pick: OnPick,
    }

    impl Route {
        fn pick(&self, index: usize) {
            log::info!("tray: picked item {index}");
            let on_pick = self.on_pick.clone();
            // The app outlives its run loop, the only place this task is polled.
            self.app
                .spawn(async move |cx| cx.update(|cx| on_pick(index, cx)))
                .detach();
        }
    }

    extern "C" fn picked(this: &Object, _: Sel, item: id) {
        unsafe {
            let route = *this.get_ivar::<*const std::ffi::c_void>(ROUTE) as *const Route;
            let tag: isize = msg_send![item, tag];
            (*route).pick(tag as usize);
        }
    }

    fn target_class() -> &'static Class {
        static REGISTER: Once = Once::new();
        REGISTER.call_once(|| {
            let mut decl = ClassDecl::new(TARGET, class!(NSObject))
                .unwrap_or_else(|| panic!("objc class {TARGET} already exists"));
            decl.add_ivar::<*const std::ffi::c_void>(ROUTE);
            unsafe {
                decl.add_method(sel!(elyPicked:), picked as extern "C" fn(&Object, Sel, id));
            }
            decl.register();
        });
        Class::get(TARGET).expect("registered above")
    }

    fn string(text: &str) -> id {
        unsafe { NSString::alloc(nil).init_str(text).autorelease() }
    }

    pub struct Tray {
        item: id,
        menu: id,
        target: id,
        _route: Box<Route>,
    }

    impl Tray {
        pub fn new(
            symbol: &str,
            items: &[TrayItem],
            on_pick: OnPick,
            cx: &mut App,
        ) -> anyhow::Result<Self> {
            let route = Box::new(Route {
                app: cx.to_async(),
                on_pick,
            });
            unsafe {
                let image: id = msg_send![class!(NSImage),
                    imageWithSystemSymbolName: string(symbol)
                    accessibilityDescription: nil];
                anyhow::ensure!(image != nil, "no SF Symbol named {symbol}");
                let target: id = msg_send![target_class(), new];
                (*target).set_ivar(ROUTE, &*route as *const Route as *const std::ffi::c_void);
                let menu = NSMenu::new(nil);
                for (index, entry) in items.iter().enumerate() {
                    let entry = match entry {
                        TrayItem::Separator => NSMenuItem::separatorItem(nil),
                        TrayItem::Action(label) => {
                            let entry = NSMenuItem::alloc(nil)
                                .initWithTitle_action_keyEquivalent_(
                                    string(label),
                                    sel!(elyPicked:),
                                    string(""),
                                )
                                .autorelease();
                            entry.setTarget_(target);
                            entry
                        }
                    };
                    let _: () = msg_send![entry, setTag: index as isize];
                    menu.addItem_(entry);
                }
                let item = NSStatusBar::systemStatusBar(nil)
                    .statusItemWithLength_(NSVariableStatusItemLength);
                let item: id = msg_send![item, retain];
                let button = item.button();
                anyhow::ensure!(button != nil, "status item has no button");
                let _: () = msg_send![button, setImage: image];
                let _: () = msg_send![item, setMenu: menu];
                log::info!("tray: shown with {} items", items.len());
                Ok(Self {
                    item,
                    menu,
                    target,
                    _route: route,
                })
            }
        }

        pub fn pick(&self, index: usize) {
            unsafe {
                let count: isize = msg_send![self.menu, numberOfItems];
                assert!((index as isize) < count, "tray has no item {index}");
                let _: () = msg_send![self.menu, performActionForItemAtIndex: index as isize];
            }
        }
    }

    impl Drop for Tray {
        fn drop(&mut self) {
            unsafe {
                NSStatusBar::systemStatusBar(nil).removeStatusItem_(self.item);
                let _: () = msg_send![self.item, release];
                let _: () = msg_send![self.menu, release];
                let _: () = msg_send![self.target, release];
            }
            log::info!("tray: removed");
        }
    }

    fn tile() -> id {
        unsafe { msg_send![NSApp(), dockTile] }
    }

    pub fn set_badge(label: Option<&str>) {
        unsafe {
            let text = label.map_or(nil, string);
            let _: () = msg_send![tile(), setBadgeLabel: text];
        }
    }

    pub fn badge() -> Option<String> {
        unsafe {
            let label: id = msg_send![tile(), badgeLabel];
            (label != nil).then(|| {
                CStr::from_ptr(label.UTF8String())
                    .to_string_lossy()
                    .into_owned()
            })
        }
    }
}
