use std::rc::Rc;

use gpui::{
    AnyElement, App, ElementId, FontWeight, InteractiveElement as _, IntoElement, ParentElement,
    RenderOnce, Role, SharedString, StatefulInteractiveElement as _, Styled, Window, div,
    prelude::FluentBuilder as _,
};

use crate::{
    lists::{ListItem, SelectableList},
    primitives::{Icon, IconName},
    theme::{ActiveTheme, IconSize, TextSize},
};

type OnKey = Rc<dyn Fn(&SharedString, &mut Window, &mut App)>;

/// A settings page: its sections listed down the side with the one open beside them, the list going above the page in a narrow box. Up and Down walk the list and Enter opens a section.
#[derive(IntoElement)]
pub struct SettingsLayout {
    id: ElementId,
    sections: Vec<(SharedString, SharedString, IconName)>,
    selected: SharedString,
    page: Option<AnyElement>,
    on_select: OnKey,
}

impl SettingsLayout {
    /// `sections` are each a key, a name and an icon; `selected` names the one open.
    pub fn new(
        id: impl Into<ElementId>,
        sections: impl IntoIterator<Item = (impl Into<SharedString>, impl Into<SharedString>, IconName)>,
        selected: impl Into<SharedString>,
        on_select: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        let sections: Vec<_> = sections
            .into_iter()
            .map(|(key, name, icon)| (key.into(), name.into(), icon))
            .collect();
        let selected = selected.into();
        if !sections.iter().any(|(key, _, _)| *key == selected) {
            log::error!("settings layout: no section {selected}; none open");
        }
        Self {
            id: id.into(),
            sections,
            selected,
            page: None,
            on_select: Rc::new(on_select),
        }
    }

    /// What the open section shows.
    pub fn page(mut self, page: impl IntoElement) -> Self {
        self.page = Some(page.into_any_element());
        self
    }
}

impl RenderOnce for SettingsLayout {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let on_select = self.on_select;
        let theme = cx.theme();
        let muted = theme.colors.fg_muted;
        let list = self.sections.iter().fold(
            SelectableList::new((self.id.clone(), "sections")).selected([self.selected.clone()]),
            |list, (key, name, icon)| {
                list.row(
                    key.clone(),
                    ListItem::new((self.id.clone(), format!("section-{key}")), name.clone())
                        .leading(Icon::new(*icon).size(IconSize::Sm).color(muted)),
                )
            },
        );
        let (picked, opened) = (on_select.clone(), on_select);
        let list = list
            .on_change(move |keys, window, cx| {
                if let Some(key) = keys.first() {
                    log::info!("settings: open {key}");
                    picked(key, window, cx);
                }
            })
            .on_activate(move |key, window, cx| {
                opened(key, window, cx);
            });
        div()
            .flex()
            .flex_wrap()
            .items_start()
            .gap_6()
            .child(
                div()
                    .flex_1()
                    .min_w(theme.label_width())
                    .max_w(theme.label_width() * 1.25)
                    .child(list),
            )
            .child(
                div()
                    .flex_1()
                    .min_w(theme.label_width())
                    .children(self.page),
            )
    }
}

/// A titled group of settings rows, with a line of context under the title.
#[derive(IntoElement)]
pub struct SettingsSection {
    title: SharedString,
    description: Option<SharedString>,
    rows: Vec<AnyElement>,
}

impl SettingsSection {
    pub fn new(title: impl Into<SharedString>) -> Self {
        Self {
            title: title.into(),
            description: None,
            rows: Vec::new(),
        }
    }

    pub fn description(mut self, text: impl Into<SharedString>) -> Self {
        self.description = Some(text.into());
        self
    }

    pub fn row(mut self, row: impl IntoElement) -> Self {
        self.rows.push(row.into_any_element());
        self
    }
}

impl RenderOnce for SettingsSection {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(div().font_weight(FontWeight::SEMIBOLD).child(self.title))
            .children(self.description.map(|text| {
                div()
                    .text_size(theme.text_size(TextSize::Sm))
                    .text_color(theme.colors.fg_muted)
                    .child(text)
            }))
            .child(div().flex().flex_col().pt_2().children(self.rows))
    }
}

/// One setting: its name and a line on what it does, and its control at the end of the line, or below them where the line is too narrow.
#[derive(IntoElement)]
pub struct SettingsRow {
    title: SharedString,
    description: Option<SharedString>,
    control: Option<AnyElement>,
}

impl SettingsRow {
    pub fn new(title: impl Into<SharedString>) -> Self {
        Self {
            title: title.into(),
            description: None,
            control: None,
        }
    }

    pub fn description(mut self, text: impl Into<SharedString>) -> Self {
        self.description = Some(text.into());
        self
    }

    pub fn control(mut self, control: impl IntoElement) -> Self {
        self.control = Some(control.into_any_element());
        self
    }
}

impl RenderOnce for SettingsRow {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        // The row names its control for assistive tech.
        let id = SharedString::from(format!("settings-row-{}", self.title));
        div()
            .id(id)
            .role(Role::Group)
            .aria_label(self.title.clone())
            .when_some(self.description.clone(), |row, text| {
                row.aria_description(text)
            })
            .flex()
            .flex_wrap()
            .items_center()
            .justify_between()
            .gap_x_4()
            .gap_y_2()
            .py_3()
            .border_b_1()
            .border_color(theme.colors.border)
            .child(
                div()
                    .flex_1()
                    .min_w(theme.label_width())
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .text_size(theme.text_size(TextSize::Sm))
                            .child(self.title),
                    )
                    .children(self.description.map(|text| {
                        div()
                            .text_size(theme.text_size(TextSize::Xs))
                            .text_color(theme.colors.fg_muted)
                            .child(text)
                    })),
            )
            .children(
                self.control
                    .map(|control| div().flex_none().min_w_0().max_w_full().child(control)),
            )
    }
}
