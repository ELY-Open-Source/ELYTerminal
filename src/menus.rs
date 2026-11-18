use ely_gpui_component::{
    forms::{Copy, Paste, SelectAll},
    i18n::text,
};
use gpui::{App, InteractiveElement, Menu, MenuItem, OsAction, actions};

use crate::{
    settings::window::OpenSettings,
    term::{
        input::{Clear, Find},
        zoom,
    },
    workspace::{CloseSession, Leave, NewSession, TogglePanel, ToggleSessions, Workspace},
};

actions!(
    elyterminal,
    [
        Quit,
        Bigger,
        Smaller,
        ResetSize,
        Minimize,
        Zoom,
        ToggleFullScreen
    ]
);

/// The PRD's menu bar, in the interface's language.
pub fn menus(cx: &App) -> Vec<Menu> {
    let word = |key: &str| text(cx, key, &[]);
    let menu = |name, items| Menu {
        name,
        items,
        disabled: false,
    };
    vec![
        menu(
            "ELYTerminal".into(),
            vec![
                MenuItem::action(word("et.menu.settings"), OpenSettings),
                MenuItem::separator(),
                MenuItem::action(word("et.menu.quit"), Quit),
            ],
        ),
        menu(
            word("et.menu.shell"),
            vec![
                MenuItem::action(word("et.menu.new_session"), NewSession),
                MenuItem::action(word("et.menu.close_session"), CloseSession),
            ],
        ),
        menu(
            word("et.menu.edit"),
            vec![
                MenuItem::os_action(word("et.menu.copy"), Copy, OsAction::Copy),
                MenuItem::os_action(word("et.menu.paste"), Paste, OsAction::Paste),
                MenuItem::os_action(word("et.menu.select_all"), SelectAll, OsAction::SelectAll),
                MenuItem::separator(),
                MenuItem::action(word("et.menu.find"), Find),
                MenuItem::action(word("et.menu.clear"), Clear),
            ],
        ),
        menu(
            word("et.menu.view"),
            vec![
                MenuItem::action(word("et.menu.toggle_sessions"), ToggleSessions),
                MenuItem::action(word("et.menu.toggle_panel"), TogglePanel),
                MenuItem::separator(),
                MenuItem::action(word("et.menu.bigger"), Bigger),
                MenuItem::action(word("et.menu.smaller"), Smaller),
                MenuItem::action(word("et.menu.reset_size"), ResetSize),
                MenuItem::separator(),
                MenuItem::action(word("et.menu.full_screen"), ToggleFullScreen),
            ],
        ),
        // macOS lists the open windows here itself.
        menu(
            word("et.menu.window"),
            vec![
                MenuItem::action(word("et.menu.minimize"), Minimize),
                MenuItem::action(word("et.menu.zoom"), Zoom),
            ],
        ),
    ]
}

/// Answers the app's commands; the keymap sets the menu bar.
pub fn init(cx: &mut App) {
    cx.set_global(zoom::Enlarged::default());
    // A key's Quit comes inside its window's update, which another update of that window would fail.
    cx.on_action(|_: &Quit, cx| cx.defer(|cx| leave(Leave::Quit, cx)));
    cx.on_action(|_: &Bigger, cx| zoom::step(Some(1.0), cx));
    cx.on_action(|_: &Smaller, cx| zoom::step(Some(-1.0), cx));
    cx.on_action(|_: &ResetSize, cx| zoom::step(None, cx));
}

/// Quits, or restarts, unless a window asks first.
pub fn leave(leave: Leave, cx: &mut App) {
    log::info!("app: {leave:?} asked");
    for handle in cx.windows() {
        let Some(workspace) = handle.downcast::<Workspace>() else {
            continue;
        };
        let asked = workspace.update(cx, |this, window, cx| {
            let asked = this.asks(leave, cx);
            if asked {
                window.activate_window();
            }
            asked
        });
        match asked {
            Ok(true) => return,
            Ok(false) => {}
            Err(error) => {
                return log::error!(
                    "app: quit stops; a window that would ask does not read: {error:#}"
                );
            }
        }
    }
    match leave {
        Leave::Restart => cx.restart(),
        _ => cx.quit(),
    }
}

/// Minimize, Zoom and full screen, which only a window answers.
pub fn window_actions<E: InteractiveElement>(root: E) -> E {
    root.on_action(|_: &Minimize, window, _| {
        log::info!("window: minimized");
        window.minimize_window();
    })
    .on_action(|_: &Zoom, window, _| {
        log::info!("window: zoomed");
        window.zoom_window();
    })
    .on_action(|_: &ToggleFullScreen, window, _| {
        log::info!("window: full screen {}", !window.is_fullscreen());
        window.toggle_fullscreen();
    })
}
