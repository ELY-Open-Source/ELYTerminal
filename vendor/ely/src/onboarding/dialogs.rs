use std::rc::Rc;

use gpui::{
    App, ElementId, Entity, InteractiveElement, IntoElement, ParentElement, RenderOnce, Window, div,
};

use crate::{
    buttons::Button,
    data_display::{Changelog, Release},
    forms::{Run, TextInput},
    overlays::Dialog,
    settings::{KeyboardShortcutsList, Shortcut},
};

/// Every shortcut in a dialog: settings::KeyboardShortcutsList with its search, which takes focus as the dialog opens. Escape closes it.
#[derive(IntoElement)]
pub struct KeyboardShortcutCheatsheet {
    id: ElementId,
    shortcuts: Vec<Shortcut>,
    search: Entity<TextInput>,
    on_close: Run,
}

impl KeyboardShortcutCheatsheet {
    /// Render it while open; `search` is the find field, which the owner keeps.
    pub fn new(
        id: impl Into<ElementId>,
        shortcuts: impl IntoIterator<Item = Shortcut>,
        search: &Entity<TextInput>,
        on_close: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            shortcuts: shortcuts.into_iter().collect(),
            search: search.clone(),
            on_close: Rc::new(on_close),
        }
    }
}

impl RenderOnce for KeyboardShortcutCheatsheet {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let first = self.search.read(cx).focus().clone();
        let on_close = self.on_close;
        Dialog::new(self.id, "Keyboard shortcuts", move |window, cx| {
            log::info!("shortcut cheatsheet: closed");
            on_close(window, cx)
        })
        .focus_first(first)
        .child(KeyboardShortcutsList::new(self.shortcuts, &self.search))
    }
}

/// What changed, in a dialog: the releases on data_display::Changelog, newest first, then Got it.
#[derive(IntoElement)]
pub struct WhatsNewDialog {
    id: ElementId,
    releases: Vec<Release>,
    on_close: Run,
}

impl WhatsNewDialog {
    /// Render it while open; `on_close` runs on Got it, Escape or a press on the scrim.
    pub fn new(
        id: impl Into<ElementId>,
        on_close: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            releases: Vec::new(),
            on_close: Rc::new(on_close),
        }
    }

    /// A release, newest first.
    pub fn release(mut self, release: Release) -> Self {
        self.releases.push(release);
        self
    }
}

impl RenderOnce for WhatsNewDialog {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let id = self.id;
        assert!(
            !self.releases.is_empty(),
            "what's new {id:?} has no release"
        );
        let changelog = self
            .releases
            .into_iter()
            .fold(Changelog::new(), Changelog::release);
        let on_close = self.on_close;
        let done = (id.clone(), "done");
        Dialog::new(id, "What's new", move |window, cx| {
            log::info!("what's new: closed");
            on_close(window, cx)
        })
        .child(
            div()
                .debug_selector(|| "whats-new-releases".into())
                .child(changelog),
        )
        .action(move |close| {
            div().debug_selector(|| "whats-new-done".into()).child(
                Button::new(done, "Got it")
                    .primary()
                    .on_click(move |_, window, cx| close(window, cx)),
            )
        })
    }
}
