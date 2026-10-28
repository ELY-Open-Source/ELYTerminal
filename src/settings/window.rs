use std::{fmt::Debug, path::PathBuf};

use ely_gpui_component::{
    buttons::SegmentedControl,
    feedback::InlineMessage,
    forms::{InputEvent, Switch, TextInput},
    i18n::{I18n, text},
    lists::{ListItem, SelectableList},
    primitives::{FocusScope, Icon, IconName, Severity},
    shell::{ColumnShell, ResizeBorder, ShellColumn},
    theme::{ActiveTheme, ControlSize, IconSize},
};
use gpui::{
    App, AppContext, Bounds, Context, Entity, FocusHandle, InteractiveElement, IntoElement,
    KeyBinding, ParentElement, Render, SharedString, StatefulInteractiveElement, Styled,
    Subscription, Task, TitlebarOptions, Window, WindowBounds, WindowDecorations, WindowOptions,
    actions, div, prelude::FluentBuilder, px, size,
};

use super::{Settings, SettingsStore};
use crate::{app::APP_ID, menus, term::BLINK, updates::Updates};

mod appearance;
mod terminal;

actions!(elyterminal, [OpenSettings, CloseSettings]);

/// The key context of the settings window's content.
const CONTEXT: &str = "Settings";

/// Each category's key, its name's key and its icon.
const SECTIONS: [(&str, &str, IconName); 2] = [
    ("appearance", "et.settings.appearance", IconName::Palette),
    ("terminal", "et.settings.terminal", IconName::Terminal),
];

/// Cmd-, opens the settings, Cmd-W closes them; off macOS Ctrl-, and Ctrl-Shift-W, as Ghostty's.
pub fn bind_keys(cx: &mut App) {
    let (open, close) = if cfg!(target_os = "macos") {
        ("cmd-,", "cmd-w")
    } else {
        ("ctrl-,", "ctrl-shift-w")
    };
    cx.bind_keys([
        KeyBinding::new(open, OpenSettings, None),
        KeyBinding::new(close, CloseSettings, Some(CONTEXT)),
    ]);
    cx.on_action(|_: &OpenSettings, cx| open_window(cx));
}

/// Opens the settings window, or brings the open one forward.
fn open_window(cx: &mut App) {
    let open = cx
        .windows()
        .into_iter()
        .find_map(|window| window.downcast::<SettingsWindow>());
    if let Some(window) = open {
        log::info!("settings: the window comes forward");
        let forward = window.update(cx, |_, window, _| window.activate_window());
        if let Err(error) = forward {
            log::error!("settings: the window did not come forward: {error:#}");
        }
        return;
    }
    let bounds = Bounds::centered(None, size(px(900.0), px(640.0)), cx);
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        titlebar: Some(TitlebarOptions {
            title: Some(text(cx, "et.settings.title", &[])),
            appears_transparent: true,
            traffic_light_position: Some(cx.theme().traffic_light_origin()),
        }),
        // Ely's column draws the controls; only Linux reads this.
        window_decorations: Some(WindowDecorations::Client),
        ..Default::default()
    };
    match cx.open_window(options, |window, cx| {
        window.set_app_id(APP_ID);
        cx.new(|cx| SettingsWindow::new(window, cx))
    }) {
        Ok(_) => log::info!("settings: the window opens"),
        Err(error) => log::error!("settings: the window did not open: {error:#}"),
    }
}

/// The settings window: categories in one column, the open one's rows in the next, each scrolling on its own.
pub struct SettingsWindow {
    focus: FocusHandle,
    section: SharedString,
    shell: Entity<TextInput>,
    /// Start in's own folder, typed or browsed.
    folder: Entity<TextInput>,
    /// The shortcut list's search.
    search: Entity<TextInput>,
    /// Why the typed shell and folder are not saved.
    refused: Option<SharedString>,
    refused_folder: Option<SharedString>,
    /// Why the last change did not save.
    failed: Option<SharedString>,
    /// Refusals so far; a field keyed by it drops its draft and shows the settings again.
    drafts: usize,
    font: Entity<TextInput>,
    /// A picker's input for each fallback family, and one for the family being added.
    fallbacks: Vec<Entity<TextInput>>,
    /// The empty picker that adds a fallback shows.
    adding: bool,
    /// Whether the cursor preview shows, as it blinks.
    lit: bool,
    _typed: [Subscription; 2],
    _settings: Subscription,
    _updates: Subscription,
    _language: Subscription,
    _blink: Task<()>,
}

impl SettingsWindow {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let shell = cx.new(|cx| {
            let placeholder = text(cx, "et.settings.login_shell", &[]);
            let mut input = TextInput::new(window, cx).placeholder(placeholder);
            input.set_text(SettingsStore::get(cx).shell.clone(), cx);
            input
        });
        let folder = cx.new(|cx| {
            let mut input = TextInput::new(window, cx);
            input.set_text(SettingsStore::get(cx).start_folder.clone(), cx);
            input
        });
        let font = cx.new(|cx| TextInput::new(window, cx));
        let search = cx.new(|cx| {
            let placeholder = text(cx, "et.settings.search_shortcuts", &[]);
            TextInput::new(window, cx).placeholder(placeholder)
        });
        let typed = [
            cx.subscribe(&shell, |this, input, event: &InputEvent, cx| {
                if let InputEvent::Changed = event {
                    let shell = input.read(cx).text().to_string();
                    let put = |settings: &mut Settings, shell| settings.shell = shell;
                    this.refused = this.save_typed("shell", shell, put, cx);
                }
            }),
            cx.subscribe(&folder, |this, input, event: &InputEvent, cx| {
                if let InputEvent::Changed = event {
                    let folder = input.read(cx).text().to_string();
                    let put = |settings: &mut Settings, folder| settings.start_folder = folder;
                    this.refused_folder = this.save_typed("start_folder", folder, put, cx);
                }
            }),
        ];
        // Focus inside, so the window's keys find its context.
        let focus = cx.focus_handle();
        window.focus(&focus, cx);
        Self {
            focus,
            section: SECTIONS[0].0.into(),
            shell,
            folder,
            search,
            refused: None,
            refused_folder: None,
            failed: None,
            drafts: 0,
            font,
            fallbacks: Vec::new(),
            adding: false,
            lit: true,
            _typed: typed,
            _settings: cx.observe_global::<SettingsStore>(|_, cx| cx.notify()),
            _updates: cx.observe_global::<Updates>(|_, cx| cx.notify()),
            _language: cx.observe_global_in::<I18n>(window, Self::speak),
            _blink: Self::blink(cx),
        }
    }

    /// Rewords its title and fields in the chosen language.
    fn speak(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        window.set_window_title(&text(cx, "et.settings.title", &[]));
        let (shell, search) = (self.shell.clone(), self.search.clone());
        shell.update(cx, |input, cx| {
            input.set_placeholder(text(cx, "et.settings.login_shell", &[]), cx)
        });
        search.update(cx, |input, cx| {
            input.set_placeholder(text(cx, "et.settings.search_shortcuts", &[]), cx)
        });
    }

    /// Blinks the cursor preview as a terminal blinks its cursor, while it shows.
    fn blink(cx: &mut Context<Self>) -> Task<()> {
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(BLINK).await;
                let ticked = this.update(cx, |this, cx| {
                    if this.section == "appearance" && SettingsStore::get(cx).cursor_blink {
                        this.lit = !this.lit;
                        cx.notify();
                    }
                });
                if ticked.is_err() {
                    break;
                }
            }
        })
    }

    /// Saves Browse's pick as typed; refuses one not in UTF-8.
    pub(super) fn chosen(&mut self, field: &'static str, path: PathBuf, cx: &mut Context<Self>) {
        let (input, refused) = match field {
            "shell" => (self.shell.clone(), &mut self.refused),
            "start_folder" => (self.folder.clone(), &mut self.refused_folder),
            other => unreachable!("settings: no path field {other}"),
        };
        let Some(text) = path.to_str() else {
            let why = format!(
                "{field} is {}, not in UTF-8, which the settings cannot hold",
                path.display()
            );
            log::error!("settings: {why}");
            *refused = Some(why.into());
            return cx.notify();
        };
        input.update(cx, |input, cx| input.set_text(text, cx));
    }

    /// Saves a field through `put`, or returns the store's reason.
    fn save_typed(
        &mut self,
        field: &str,
        value: String,
        put: fn(&mut Settings, String),
        cx: &mut Context<Self>,
    ) -> Option<SharedString> {
        let saved = SettingsStore::update(cx, |settings| put(settings, value.clone()));
        cx.notify();
        match saved {
            Ok(()) => {
                log::info!("settings: {field} is {value:?}");
                self.failed = None;
                None
            }
            Err(error) => {
                log::debug!("settings: {field} stays: {error:#}");
                Some(format!("{error:#}").into())
            }
        }
    }
}

/// The categories; a press, Up and Down, or Enter opens one.
fn sections(open: &SharedString, cx: &Context<SettingsWindow>) -> SelectableList {
    let muted = cx.theme().colors.fg_muted;
    let list = SECTIONS.iter().fold(
        SelectableList::new("settings-section-list").selected([open.clone()]),
        |list, (key, name, icon)| {
            let name = text(cx, name, &[]);
            let item = ListItem::new(SharedString::from(format!("settings-section-{key}")), name)
                .leading(Icon::new(*icon).size(IconSize::Sm).color(muted));
            list.row(*key, item)
        },
    );
    let show = {
        let this = cx.entity();
        move |key: &SharedString, cx: &mut App| {
            log::info!("settings: open {key}");
            this.update(cx, |this, cx| {
                this.section = key.clone();
                cx.notify();
            });
        }
    };
    let enter = show.clone();
    list.on_change(move |keys, _, cx| {
        if let Some(key) = keys.first() {
            show(key, cx);
        }
    })
    .on_activate(move |key, _, cx| enter(key, cx))
}

/// A switch that saves through `put`.
fn switch(
    this: &Entity<SettingsWindow>,
    field: &'static str,
    on: bool,
    put: fn(&mut Settings, bool),
) -> Switch {
    let this = this.clone();
    Switch::new(field, on).on_change(move |on, _, cx| {
        set(&this, format!("{field} is {on}"), cx, |settings| {
            put(settings, on)
        });
    })
}

/// Segments for `choices` that save the chosen value through `put`.
fn choice<T: Copy + PartialEq + Debug + 'static>(
    this: &Entity<SettingsWindow>,
    field: &'static str,
    choices: &'static [(T, &'static str, &'static str)],
    now: T,
    put: fn(&mut Settings, T),
    cx: &App,
) -> SegmentedControl {
    let (_, key, _) = choices
        .iter()
        .find(|(each, ..)| *each == now)
        .expect("every choice is offered");
    let this = this.clone();
    choices
        .iter()
        .fold(
            SegmentedControl::new(field, *key),
            |control, (_, key, name)| control.segment(*key, text(cx, name, &[]), None),
        )
        .size(ControlSize::Sm)
        .on_change(move |key, _, cx| {
            let (value, ..) = choices
                .iter()
                .find(|(_, each, _)| *each == key.as_ref())
                .expect("a choice the control offers");
            let value = *value;
            set(&this, format!("{field} is {value:?}"), cx, |settings| {
                put(settings, value)
            });
        })
}

/// Saves one change and logs it, telling whether the settings hold it; a refused one changes nothing, and the window says why.
fn set(
    this: &Entity<SettingsWindow>,
    change: String,
    cx: &mut App,
    edit: impl FnOnce(&mut Settings),
) -> bool {
    let mut next = SettingsStore::get(cx).clone();
    edit(&mut next);
    if next == *SettingsStore::get(cx) {
        return true;
    }
    let failed = match SettingsStore::update(cx, |settings| *settings = next) {
        Ok(()) => {
            log::info!("settings: {change}");
            None
        }
        Err(error) => {
            log::error!("settings: refused {change}: {error:#}");
            Some(format!("{error:#}").into())
        }
    };
    let saved = failed.is_none();
    this.update(cx, |this, cx| {
        this.drafts += usize::from(!saved);
        this.failed = failed;
        cx.notify();
    });
    saved
}

impl Render for SettingsWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let page = match self.section.as_ref() {
            "appearance" => {
                self.grow_fallbacks(window, cx);
                self.appearance(window, cx).into_any_element()
            }
            "terminal" => self.terminal(cx).into_any_element(),
            other => unreachable!("settings: no section {other}"),
        };
        let page = div()
            .flex()
            .flex_col()
            .gap_4()
            .children((self.failed.clone()).map(|why| InlineMessage::new(Severity::Danger, why)))
            .child(page);
        let (_, name, _) = SECTIONS
            .iter()
            .find(|(key, ..)| *key == self.section.as_ref())
            .expect("the open section is listed");
        let list = div()
            .id("settings-list")
            .size_full()
            .overflow_y_scroll()
            .p_2()
            .child(sections(&self.section, cx));
        let body = div()
            .id("settings-body")
            .size_full()
            .overflow_y_scroll()
            .p_6()
            .child(page);
        // The categories and the page scroll apart.
        let shell = ColumnShell::new("settings")
            .column(
                ShellColumn::new("settings-sections")
                    .width(cx.theme().label_width() * 1.25)
                    .title(text(cx, "et.settings.title", &[]))
                    .child(list),
            )
            .column(
                ShellColumn::new("settings-page")
                    .title(text(cx, name, &[]))
                    .child(body),
            );
        // The context sits above the focus root, so its keys work wherever focus is.
        div()
            .size_full()
            .key_context(CONTEXT)
            .map(menus::window_actions)
            .on_action(|_: &CloseSettings, window, _| {
                log::info!("settings: the window closes");
                window.remove_window();
            })
            .on_action(|_: &OpenSettings, window, _| {
                log::info!("settings: the window comes forward");
                window.activate_window();
            })
            .child(
                ResizeBorder::new()
                    .child(FocusScope::new(&self.focus).root().size_full().child(shell)),
            )
    }
}
