//! A session renamed in place, as a double-click on its row asks.

use ely_gpui_component::{
    forms::{InputEvent, TextInput},
    i18n::text,
    menus::{Menu, MenuItem},
    primitives::IconName,
};
use gpui::{App, AppContext, Context, Entity, EntityId, Focusable, Window};

use super::Workspace;
use crate::settings::SettingsStore;

/// What a right press on the terminal's title offers: renaming the session shown.
pub(super) fn header(workspace: &Entity<Workspace>, cx: &App) -> Menu {
    let workspace = workspace.clone();
    Menu::new().item(
        MenuItem::new(text(cx, "et.session.rename", &[]))
            .icon(IconName::Pencil)
            .on_click(move |window, cx| {
                workspace.update(cx, |this, cx| this.rename_shown(window, cx))
            }),
    )
}

impl Workspace {
    /// Renames the session shown in its row, showing the sessions column first.
    fn rename_shown(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !SettingsStore::get(cx).show_sessions {
            self.toggle(|settings| settings.show_sessions = true, cx);
        }
        self.rename(self.active, window, cx);
    }

    /// Opens a field over session `index`'s title, what it is called selected.
    pub fn rename(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(session) = self.sessions.get(index) else {
            return;
        };
        if session.editing.is_some() {
            return;
        }
        let named = session.named().to_string();
        let id = session.id;
        let input = cx.new(|cx| {
            let mut input = TextInput::new(window, cx);
            input.set_text(named.clone(), cx);
            input.select(0..named.len(), cx);
            input
        });
        let events = cx.subscribe_in(
            &input,
            window,
            move |this, _, event, window, cx| match event {
                InputEvent::Submit | InputEvent::Blur => this.end_rename(id, true, window, cx),
                InputEvent::Changed | InputEvent::Focus => {}
            },
        );
        window.focus(&input.focus_handle(cx), cx);
        log::debug!("app: session {} is renamed in place", index + 1);
        self.sessions[index].editing = Some((input, events));
        cx.notify();
    }

    /// Closes the field renaming the session `id` names, `keep`ing what it holds or not; a focus it still holds goes to the shown terminal.
    pub fn end_rename(
        &mut self,
        id: EntityId,
        keep: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(session) = self.sessions.iter_mut().find(|session| session.id == id) else {
            log::debug!("app: the session renamed is gone");
            return;
        };
        let Some((input, _)) = session.editing.take() else {
            return;
        };
        let pid = session.terminal().read(cx).pid();
        // A window that goes inactive blurs the field yet keeps its focus there.
        let held = input.focus_handle(cx).is_focused(window);
        if !keep {
            log::debug!("app: session {pid} keeps its name");
        } else {
            session.name = input.read(cx).text().trim().to_string();
            if session.name.is_empty() {
                log::info!("app: session {pid} takes its program's title again");
            } else {
                log::info!("app: session {pid} is named {:?}", session.name);
            }
        }
        if held && let Some(shown) = self.sessions.get(self.active) {
            window.focus(&shown.terminal().focus_handle(cx), cx);
        }
        self.retitle(window);
        cx.notify();
    }
}
