use std::rc::Rc;

use gpui::{
    App, ElementId, Entity, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window,
    div,
};

use crate::{
    buttons::{Button, ButtonVariant},
    forms::{Run, SearchInput, TextInput},
    menus::{DropdownMenu, Menu, MenuItem},
    primitives::IconName,
    theme::{ActiveTheme, ControlSize, Density},
};

type OnHidden = Rc<dyn Fn(&[SharedString], &mut Window, &mut App)>;
type OnDensity = Rc<dyn Fn(Density, &mut Window, &mut App)>;

/// A table's tools in one row: a search and a filter button that counts its rules, then at the end a menu that shows and hides columns, the density, and an export. Each tool appears once it is given.
#[derive(IntoElement)]
pub struct TableToolbar {
    id: ElementId,
    search: Option<Entity<TextInput>>,
    filter: Option<(usize, Run)>,
    columns: Vec<(SharedString, SharedString)>,
    hidden: Vec<SharedString>,
    on_hidden: Option<OnHidden>,
    density: Option<(Density, OnDensity)>,
    on_export: Option<Run>,
}

impl TableToolbar {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            search: None,
            filter: None,
            columns: Vec::new(),
            hidden: Vec::new(),
            on_hidden: None,
            density: None,
            on_export: None,
        }
    }

    /// A search field over the owner's text; hand its text to the table's `query`.
    pub fn search(mut self, field: &Entity<TextInput>) -> Self {
        self.search = Some(field.clone());
        self
    }

    /// A filter button that counts the rules; a press runs `on_press`, such as opening a `FilterBuilder`.
    pub fn filter(
        mut self,
        rules: usize,
        on_press: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        self.filter = Some((rules, Rc::new(on_press)));
        self
    }

    /// A menu with a check for each column; `on_change` gets the columns now hidden.
    pub fn columns(
        mut self,
        columns: impl IntoIterator<Item = (impl Into<SharedString>, impl Into<SharedString>)>,
        hidden: impl IntoIterator<Item = impl Into<SharedString>>,
        on_change: impl Fn(&[SharedString], &mut Window, &mut App) + 'static,
    ) -> Self {
        self.columns = columns
            .into_iter()
            .map(|(key, title)| (key.into(), title.into()))
            .collect();
        self.hidden = hidden.into_iter().map(Into::into).collect();
        self.on_hidden = Some(Rc::new(on_change));
        self
    }

    pub fn density(
        mut self,
        density: Density,
        on_change: impl Fn(Density, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.density = Some((density, Rc::new(on_change)));
        self
    }

    pub fn export(mut self, on_press: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_export = Some(Rc::new(on_press));
        self
    }
}

impl RenderOnce for TableToolbar {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id;
        let columns = self.on_hidden.map(|on_hidden| {
            let menu = self.columns.iter().fold(Menu::new(), |menu, (key, title)| {
                let shown = !self.hidden.contains(key);
                let (key, hidden, on_hidden) =
                    (key.clone(), self.hidden.clone(), on_hidden.clone());
                menu.item(
                    MenuItem::check(title.clone(), shown).on_click(move |window, cx| {
                        let next: Vec<SharedString> = if shown {
                            hidden.iter().cloned().chain([key.clone()]).collect()
                        } else {
                            hidden
                                .iter()
                                .filter(|known| **known != key)
                                .cloned()
                                .collect()
                        };
                        on_hidden(&next, window, cx)
                    }),
                )
            });
            DropdownMenu::new((id.clone(), "columns"), "Columns", menu)
                .icon(IconName::Table)
                .variant(ButtonVariant::Ghost)
        });
        let density = self.density.map(|(now, on_density)| {
            let menu = [
                (Density::Compact, "Compact"),
                (Density::Standard, "Standard"),
                (Density::Comfortable, "Comfortable"),
            ]
            .into_iter()
            .fold(Menu::new(), |menu, (density, label)| {
                let on_density = on_density.clone();
                menu.item(
                    MenuItem::radio(label, now == density)
                        .on_click(move |window, cx| on_density(density, window, cx)),
                )
            });
            DropdownMenu::new((id.clone(), "density"), "Density", menu)
                .icon(IconName::List)
                .variant(ButtonVariant::Ghost)
        });
        div()
            .flex()
            .items_center()
            .gap_1()
            .children(self.search.map(|field| {
                div()
                    .w(cx.theme().label_width() * 1.6)
                    .child(SearchInput::new((id.clone(), "search"), &field).size(ControlSize::Sm))
            }))
            .children(self.filter.map(|(rules, on_press)| {
                let label: SharedString = if rules == 0 {
                    "Filter".into()
                } else {
                    format!("Filter · {rules}").into()
                };
                Button::new((id.clone(), "filter"), label)
                    .icon(IconName::Filter)
                    .variant(ButtonVariant::Ghost)
                    .size(ControlSize::Sm)
                    .on_click(move |_, window, cx| on_press(window, cx))
            }))
            .child(div().flex_1())
            .children(columns)
            .children(density)
            .children(self.on_export.map(|on_export| {
                Button::new((id.clone(), "export"), "Export")
                    .icon(IconName::Download)
                    .variant(ButtonVariant::Ghost)
                    .size(ControlSize::Sm)
                    .on_click(move |_, window, cx| on_export(window, cx))
            }))
    }
}
