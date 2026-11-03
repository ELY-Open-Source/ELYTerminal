use std::rc::Rc;

use gpui::{
    App, ElementId, Entity, InteractiveElement, IntoElement, KeyDownEvent, ParentElement,
    RenderOnce, SharedString, Styled, Window, div,
};

use crate::{
    forms::{Enter, Input, TextInput},
    menus::{Menu, MenuItem, OverflowMenu},
    primitives::IconName,
    theme::{ActiveTheme, ControlSize, Elevation, Radius, TextSize},
};

/// What a code action does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionKind {
    QuickFix,
    Refactor,
    Source,
}

/// A change the tools offer at the cursor; a preferred one leads its group.
#[derive(Clone, Debug, PartialEq)]
pub struct CodeAction {
    pub title: SharedString,
    pub kind: ActionKind,
    pub preferred: bool,
}

impl CodeAction {
    pub fn new(title: impl Into<SharedString>, kind: ActionKind) -> Self {
        Self {
            title: title.into(),
            kind,
            preferred: false,
        }
    }

    pub fn preferred(mut self) -> Self {
        self.preferred = true;
        self
    }
}

type OnAction = Rc<dyn Fn(usize, &mut Window, &mut App)>;

/// A lightbulb that opens the actions at the cursor: fixes, refactors and source actions, each group led by its preferred one.
#[derive(IntoElement)]
pub struct CodeActionMenu {
    id: ElementId,
    actions: Vec<CodeAction>,
    on_pick: Option<OnAction>,
}

impl CodeActionMenu {
    pub fn new(id: impl Into<ElementId>, actions: impl IntoIterator<Item = CodeAction>) -> Self {
        Self {
            id: id.into(),
            actions: actions.into_iter().collect(),
            on_pick: None,
        }
    }

    /// Gets the chosen action's place among the actions given.
    pub fn on_pick(mut self, handler: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_pick = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for CodeActionMenu {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let groups = [
            (ActionKind::QuickFix, "Quick fixes"),
            (ActionKind::Refactor, "Refactor"),
            (ActionKind::Source, "Source"),
        ];
        let menu = groups.into_iter().fold(Menu::new(), |menu, (kind, title)| {
            let mut found: Vec<(usize, &CodeAction)> = self
                .actions
                .iter()
                .enumerate()
                .filter(|(_, action)| action.kind == kind)
                .collect();
            if found.is_empty() {
                return menu;
            }
            found.sort_by_key(|(_, action)| !action.preferred);
            let items = found.into_iter().map(|(ix, action)| {
                let pick = self.on_pick.clone();
                let item = MenuItem::new(action.title.clone());
                let item = if action.preferred {
                    item.icon(IconName::Sparkles)
                } else {
                    item
                };
                item.on_click(move |window, cx| {
                    log::info!("code action: {ix}");
                    if let Some(pick) = &pick {
                        pick(ix, window, cx);
                    }
                })
            });
            menu.group(title, items)
        });
        OverflowMenu::new(self.id, menu)
            .icon(IconName::Lightbulb)
            .tooltip("Show code actions")
    }
}

type OnName = Rc<dyn Fn(SharedString, &mut Window, &mut App)>;
type OnCancel = Rc<dyn Fn(&mut Window, &mut App)>;

/// A small box over a symbol to rename it: Enter renames everywhere, Escape keeps the name.
#[derive(IntoElement)]
pub struct RenameInput {
    id: ElementId,
    field: Entity<TextInput>,
    on_rename: Option<OnName>,
    on_cancel: Option<OnCancel>,
}

impl RenameInput {
    /// `field` holds the new name; seed it with the current one.
    pub fn new(id: impl Into<ElementId>, field: &Entity<TextInput>) -> Self {
        Self {
            id: id.into(),
            field: field.clone(),
            on_rename: None,
            on_cancel: None,
        }
    }

    pub fn on_rename(
        mut self,
        handler: impl Fn(SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_rename = Some(Rc::new(handler));
        self
    }

    pub fn on_cancel(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_cancel = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for RenameInput {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let (field, rename, cancel) = (self.field.clone(), self.on_rename, self.on_cancel);
        div()
            .id(self.id)
            .w(theme.label_width() * 2.5)
            .flex()
            .flex_col()
            .gap_1()
            .p_1p5()
            .rounded(theme.radius(Radius::Md))
            .border_1()
            .border_color(colors.focus)
            .bg(colors.overlay)
            .shadow(theme.elevation(Elevation::Floating))
            .capture_action(move |_: &Enter, window, cx| {
                cx.stop_propagation();
                let name = field.read(cx).text().trim().to_string();
                if name.is_empty() {
                    return;
                }
                log::info!("rename: {name}");
                if let Some(rename) = &rename {
                    rename(name.into(), window, cx);
                }
            })
            .on_key_down(move |event: &KeyDownEvent, window, cx| {
                if event.keystroke.key == "escape" {
                    cx.stop_propagation();
                    if let Some(cancel) = &cancel {
                        cancel(window, cx);
                    }
                }
            })
            .child(Input::new(&self.field).size(ControlSize::Sm))
            .child(
                div()
                    .px_0p5()
                    .text_size(theme.text_size(TextSize::Xs))
                    .text_color(colors.fg_subtle)
                    .child("Enter to rename · Escape to cancel"),
            )
    }
}
