//! Session groups: named blocks of rows, the ungrouped rows first, then each group in the order made.

use std::ops::Range;

use ely_gpui_component::{
    forms::{Input, InputEvent, TextInput},
    i18n::text,
    menus::ContextMenu,
    theme::{ActiveTheme, ControlSize, TextSize},
};
use gpui::{
    AnyElement, AppContext, Context, EntityId, Focusable, FontWeight, InteractiveElement,
    IntoElement, MouseButton, ParentElement, StatefulInteractiveElement, Styled, Subscription,
    Window, div,
};

use super::Workspace;
use crate::{sessions::menu, settings::SettingsStore};

/// A group of sessions, its name the user's.
pub struct Group {
    pub id: u64,
    pub name: String,
    /// The field its header shows while the user renames it.
    pub editing: Option<(gpui::Entity<TextInput>, Subscription)>,
}

/// Where rows of `group` sit among `groups`: the ungrouped first.
fn rank(groups: &[u64], group: Option<u64>) -> usize {
    group.map_or(0, |group| {
        1 + (groups.iter())
            .position(|id| *id == group)
            .expect("a session's group is listed")
    })
}

impl Workspace {
    /// Each block of rows: its group, none for the ungrouped, and its places.
    pub(super) fn blocks(&self) -> Vec<(Option<u64>, Range<usize>)> {
        let mut blocks: Vec<(Option<u64>, Range<usize>)> = Vec::new();
        for (index, session) in self.sessions.iter().enumerate() {
            match blocks.last_mut() {
                Some((group, places)) if *group == session.group => places.end = index + 1,
                _ => blocks.push((session.group, index..index + 1)),
            }
        }
        blocks
    }

    /// Sorts the rows into their blocks, each block in its own order, the shown session kept.
    pub(super) fn arrange(&mut self) {
        let shown = self.sessions.get(self.active).map(|session| session.id);
        let groups: Vec<u64> = self.groups.iter().map(|group| group.id).collect();
        self.sessions
            .sort_by_key(|session| rank(&groups, session.group));
        if let Some(shown) = shown {
            self.active = (self.sessions.iter())
                .position(|session| session.id == shown)
                .expect("the shown session stays");
        }
    }

    /// Drops the groups no session holds any more; the keys a dropped name field held go to the shown terminal.
    pub(super) fn prune_groups(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let sessions = &self.sessions;
        let mut held_keys = false;
        self.groups.retain(|group| {
            let held = sessions
                .iter()
                .any(|session| session.group == Some(group.id));
            if !held {
                log::info!("app: group {:?} empties and goes", group.name);
                held_keys |= (group.editing.as_ref())
                    .is_some_and(|(input, _)| input.focus_handle(cx).is_focused(window));
            }
            held
        });
        if held_keys && let Some(shown) = self.sessions.get(self.active) {
            window.focus(&shown.terminal().focus_handle(cx), cx);
        }
    }

    /// Session `id` goes into `group`, or out of every group with none.
    pub fn join_group(
        &mut self,
        id: EntityId,
        group: Option<u64>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if group.is_some_and(|group| self.groups.iter().all(|each| each.id != group)) {
            return log::info!("app: session {id:?} joins no group: the group went");
        }
        let Some(session) = self.sessions.iter_mut().find(|session| session.id == id) else {
            return log::info!("app: session {id:?} joins no group: it went");
        };
        session.group = group;
        let name = group.and_then(|group| self.groups.iter().find(|each| each.id == group));
        match name {
            Some(group) => log::info!("app: session {id:?} joins group {:?}", group.name),
            None => log::info!("app: session {id:?} leaves its group"),
        }
        self.prune_groups(window, cx);
        self.arrange();
        cx.notify();
    }

    /// A new group holding session `id`, its name open to type.
    pub fn new_group(&mut self, id: EntityId, window: &mut Window, cx: &mut Context<Self>) {
        if self.sessions.iter().all(|session| session.id != id) {
            return log::info!("app: no group made: session {id:?} went");
        }
        let group = self.groups.iter().map(|group| group.id).max().unwrap_or(0) + 1;
        let name = text(cx, "et.group.default", &[("number", &group.to_string())]);
        log::info!("app: group {name:?} is made");
        self.groups.push(Group {
            id: group,
            name: name.to_string(),
            editing: None,
        });
        self.join_group(id, Some(group), window, cx);
        self.rename_group(group, window, cx);
    }

    /// Group `group`'s rows go back among the ungrouped, and it goes.
    pub fn ungroup(&mut self, group: u64, window: &mut Window, cx: &mut Context<Self>) {
        for session in self
            .sessions
            .iter_mut()
            .filter(|session| session.group == Some(group))
        {
            session.group = None;
        }
        self.prune_groups(window, cx);
        self.arrange();
        cx.notify();
    }

    /// Opens a field over group `group`'s name, selected.
    pub fn rename_group(&mut self, group: u64, window: &mut Window, cx: &mut Context<Self>) {
        let Some(found) = self.groups.iter().find(|each| each.id == group) else {
            return;
        };
        if found.editing.is_some() {
            return;
        }
        let name = found.name.clone();
        let input = cx.new(|cx| {
            let mut input = TextInput::new(window, cx);
            input.set_text(name.clone(), cx);
            input.select(0..name.len(), cx);
            input
        });
        let events = cx.subscribe_in(&input, window, move |this, _, event, window, cx| {
            if matches!(event, InputEvent::Submit | InputEvent::Blur) {
                this.end_group_rename(group, true, window, cx);
            }
        });
        window.focus(&input.focus_handle(cx), cx);
        let found = (self.groups.iter_mut())
            .find(|each| each.id == group)
            .expect("the group renamed");
        found.editing = Some((input, events));
        cx.notify();
    }

    /// Closes group `group`'s name field, `keep`ing a name it holds; the keys go back to the shown terminal.
    pub fn end_group_rename(
        &mut self,
        group: u64,
        keep: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(found) = self.groups.iter_mut().find(|each| each.id == group) else {
            return log::debug!("app: the group renamed is gone");
        };
        let Some((input, _)) = found.editing.take() else {
            return;
        };
        let held = input.focus_handle(cx).is_focused(window);
        let name = input.read(cx).text().trim().to_string();
        match (keep, name.is_empty()) {
            (true, false) => {
                log::info!("app: group {:?} is named {name:?}", found.name);
                found.name = name;
            }
            _ => log::debug!("app: group {:?} keeps its name", found.name),
        }
        if held && let Some(shown) = self.sessions.get(self.active) {
            window.focus(&shown.terminal().focus_handle(cx), cx);
        }
        cx.notify();
    }

    /// Group `group`'s header: its name, which a double press renames, and its menu.
    pub(super) fn header(&self, group: &Group, cx: &mut Context<Self>) -> AnyElement {
        let workspace = cx.entity();
        let id = group.id;
        let theme = cx.theme();
        let name = match &group.editing {
            Some((input, _)) => {
                let workspace = workspace.clone();
                div()
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .on_key_down(move |event, window, cx| {
                        if event.keystroke.key == "escape" {
                            cx.stop_propagation();
                            workspace.update(cx, |this, cx| {
                                this.end_group_rename(id, false, window, cx)
                            });
                        }
                    })
                    .child(Input::new(input).size(ControlSize::Sm))
                    .into_any_element()
            }
            None => {
                let workspace = workspace.clone();
                div()
                    .id(("group", id as usize))
                    .child(group.name.clone())
                    .on_click(move |event, window, cx| {
                        if event.click_count() >= 2 {
                            workspace.update(cx, |this, cx| this.rename_group(id, window, cx));
                        }
                    })
                    .into_any_element()
            }
        };
        let header = div()
            .w_full()
            .px_3()
            .pt_3()
            .pb_1()
            .text_size(theme.text_size(TextSize::Xs))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(theme.colors.fg_muted)
            .child(name);
        if !SettingsStore::get(cx).session_menu || group.editing.is_some() {
            return header.into_any_element();
        }
        ContextMenu::new(("group-menu", id as usize), menu::group(&workspace, id, cx))
            .w_full()
            .child(header)
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::rank;

    #[test]
    fn the_ungrouped_rows_come_first_then_each_group_in_the_order_made() {
        let groups = [7, 3];
        let mut rows = [Some(3), None, Some(7), Some(3), None];
        rows.sort_by_key(|group| rank(&groups, *group));
        assert_eq!(rows, [None, None, Some(7), Some(3), Some(3)]);
    }
}
