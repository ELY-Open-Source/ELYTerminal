use ely_gpui_component::{
    forms::{Copy, Paste, SelectAll},
    i18n::text,
};
use gpui::{App, InteractiveElement, KeyBinding, Menu, MenuItem, OsAction, actions};

use crate::{
    settings::window::OpenSettings,
    term::{input::Find, zoom},
    workspace::{CloseSession, NewSession, TogglePanel, ToggleSessions},
};

actions!(
    elyterminal,
    [Quit, Bigger, Smaller, ResetSize, Minimize, Zoom]
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

/// Binds app commands before building their menus.
pub fn init(cx: &mut App) {
    let (quit, modifier) = if cfg!(target_os = "macos") {
        ("cmd-q", "cmd")
    } else {
        ("ctrl-shift-q", "ctrl")
    };
    cx.bind_keys([
        KeyBinding::new(quit, Quit, None),
        KeyBinding::new(&format!("{modifier}-="), Bigger, None),
        KeyBinding::new(&format!("{modifier}-+"), Bigger, None),
        KeyBinding::new(&format!("{modifier}--"), Smaller, None),
        KeyBinding::new(&format!("{modifier}-0"), ResetSize, None),
    ]);
    if cfg!(target_os = "macos") {
        cx.bind_keys([KeyBinding::new("cmd-m", Minimize, None)]);
    }
    cx.set_global(zoom::Enlarged::default());
    cx.on_action(|_: &Quit, cx| {
        log::info!("app: quit asked");
        cx.quit();
    });
    cx.on_action(|_: &Bigger, cx| zoom::step(Some(1.0), cx));
    cx.on_action(|_: &Smaller, cx| zoom::step(Some(-1.0), cx));
    cx.on_action(|_: &ResetSize, cx| zoom::step(None, cx));
    cx.set_menus(menus(cx));
}

/// Minimize and Zoom, which only a window answers.
pub fn window_actions<E: InteractiveElement>(root: E) -> E {
    root.on_action(|_: &Minimize, window, _| {
        log::info!("window: minimized");
        window.minimize_window();
    })
    .on_action(|_: &Zoom, window, _| {
        log::info!("window: zoomed");
        window.zoom_window();
    })
}
