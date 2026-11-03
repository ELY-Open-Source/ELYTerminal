use std::rc::Rc;

use gpui::{
    AnyElement, App, ElementId, Entity, FontWeight, Hsla, InteractiveElement, IntoElement,
    MouseButton, ParentElement, RenderOnce, SharedString, StatefulInteractiveElement, Styled,
    Window, div, prelude::*,
};

use super::OnIndex;
use crate::{
    buttons::{ButtonVariant, IconButton},
    forms::{Input, TextInput},
    primitives::{Disclosure, IconName},
    theme::{ActiveTheme, ControlSize, Palette, Radius, TextSize},
    typography::Ellipsis,
};

/// What a value is, which picks its color.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValueKind {
    Text,
    Number,
    Bool,
    Null,
    Object,
    List,
    Function,
}

impl ValueKind {
    fn color(self, colors: &Palette) -> Hsla {
        let syntax = &colors.syntax;
        match self {
            Self::Text => syntax.string,
            Self::Number => syntax.number,
            Self::Bool | Self::Null => syntax.keyword,
            Self::Object | Self::List => syntax.type_name,
            Self::Function => syntax.function,
        }
    }
}

/// A variable: its name, its value as the debugger prints it, its kind and type, and its fields or items.
#[derive(Clone, Debug, PartialEq)]
pub struct Variable {
    pub name: SharedString,
    pub value: SharedString,
    pub kind: ValueKind,
    pub type_name: Option<SharedString>,
    pub children: Vec<Variable>,
}

/// Rows for `variables` under `path`, their children when open: each row's key, depth and variable.
pub(crate) fn flat<'a>(
    variables: &'a [Variable],
    path: &str,
    depth: usize,
    open: &[SharedString],
    out: &mut Vec<(SharedString, usize, &'a Variable)>,
) {
    for variable in variables {
        let key: SharedString = format!("{path}/{}", variable.name).into();
        let opened = open.contains(&key);
        out.push((key.clone(), depth, variable));
        if opened {
            flat(&variable.children, &key, depth + 1, open, out);
        }
    }
}

type OnToggle = Rc<dyn Fn(&SharedString, bool, &mut Window, &mut App)>;

/// Variables by scope, as a tree: each with its value colored by kind and its type, fields and items opening under it; values that changed at the last step washed.
#[derive(IntoElement)]
pub struct VariablesPanel {
    id: ElementId,
    scopes: Vec<(SharedString, Vec<Variable>)>,
    open: Vec<SharedString>,
    changed: Vec<SharedString>,
    on_toggle: Option<OnToggle>,
}

impl VariablesPanel {
    pub fn new(
        id: impl Into<ElementId>,
        scopes: impl IntoIterator<Item = (SharedString, Vec<Variable>)>,
    ) -> Self {
        Self {
            id: id.into(),
            scopes: scopes.into_iter().collect(),
            open: Vec::new(),
            changed: Vec::new(),
            on_toggle: None,
        }
    }

    /// Scopes and variables shown open, by key: `scope`, `scope/name`, `scope/name/field`.
    pub fn open(mut self, keys: impl IntoIterator<Item = SharedString>) -> Self {
        self.open = keys.into_iter().collect();
        self
    }

    /// Variables whose values changed at the last step, by key.
    pub fn changed(mut self, keys: impl IntoIterator<Item = SharedString>) -> Self {
        self.changed = keys.into_iter().collect();
        self
    }

    pub fn on_toggle(
        mut self,
        handler: impl Fn(&SharedString, bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_toggle = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for VariablesPanel {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let toggle_row = |key: SharedString, opens: bool, opened: bool, depth: usize| {
            let toggle = self.on_toggle.clone();
            div()
                .id((self.id.clone(), key.clone()))
                .flex()
                .items_center()
                .gap_1()
                .pl(theme.text_size(TextSize::Sm) * (depth as f32 * 1.1 + 0.4))
                .pr_2()
                .py_0p5()
                .rounded(theme.radius(Radius::Sm))
                .hover(|row| row.bg(colors.hover))
                .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
                .when(opens, |row| {
                    row.cursor_pointer().when_some(toggle, |row, toggle| {
                        row.on_click(move |_, window, cx| toggle(&key, !opened, window, cx))
                    })
                })
                .child(div().flex_none().w_4().children(opens.then(|| {
                    Disclosure::new(
                        (self.id.clone(), format!("chevron-{depth}-{opened}")),
                        opened,
                    )
                })))
        };
        let mut rows: Vec<AnyElement> = Vec::new();
        for (scope, variables) in &self.scopes {
            let scope_key = scope.clone();
            let opened = self.open.contains(&scope_key);
            rows.push(
                toggle_row(scope_key.clone(), true, opened, 0)
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(colors.fg_muted)
                    .child(scope.clone())
                    .into_any_element(),
            );
            if !opened {
                continue;
            }
            let mut shown = Vec::new();
            flat(variables, scope, 1, &self.open, &mut shown);
            for (key, depth, variable) in shown {
                let opens = !variable.children.is_empty();
                let changed = self.changed.contains(&key);
                rows.push(
                    toggle_row(key.clone(), opens, self.open.contains(&key), depth)
                        .font_family(theme.mono_family.clone())
                        .child(
                            div()
                                .flex_none()
                                .text_color(colors.syntax.property)
                                .child(variable.name.clone()),
                        )
                        .child(div().flex_none().text_color(colors.fg_subtle).child(":"))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .px_1()
                                .rounded(theme.radius(Radius::Sm))
                                .when(changed, |value| value.bg(colors.warning.opacity(0.18)))
                                .text_color(variable.kind.color(&colors))
                                .child(Ellipsis::new(variable.value.clone())),
                        )
                        .children(
                            variable.type_name.clone().map(|kind| {
                                div().flex_none().text_color(colors.fg_subtle).child(kind)
                            }),
                        )
                        .into_any_element(),
                );
            }
        }
        div()
            .flex()
            .flex_col()
            .text_size(theme.text_size(TextSize::Sm))
            .children(rows)
    }
}

/// A watched expression and what it gives: a value and its kind, or the error it raised.
#[derive(Clone, Debug, PartialEq)]
pub struct Watch {
    pub expression: SharedString,
    pub value: Result<(SharedString, ValueKind), SharedString>,
}

/// Expressions watched at every step, each with its value or its error; a field adds one, and each removes.
#[derive(IntoElement)]
pub struct WatchPanel {
    id: ElementId,
    watches: Vec<Watch>,
    field: Entity<TextInput>,
    on_remove: Option<OnIndex>,
}

impl WatchPanel {
    /// `field` takes a new expression; its owner adds it on Enter.
    pub fn new(
        id: impl Into<ElementId>,
        watches: impl IntoIterator<Item = Watch>,
        field: &Entity<TextInput>,
    ) -> Self {
        Self {
            id: id.into(),
            watches: watches.into_iter().collect(),
            field: field.clone(),
            on_remove: None,
        }
    }

    pub fn on_remove(mut self, handler: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_remove = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for WatchPanel {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        div()
            .flex()
            .flex_col()
            .gap_1()
            .text_size(theme.text_size(TextSize::Sm))
            .children(self.watches.iter().enumerate().map(|(ix, watch)| {
                let group = SharedString::from(format!("watch-{ix}"));
                let remove = self.on_remove.clone();
                let (value, color) = match &watch.value {
                    Ok((value, kind)) => (value.clone(), kind.color(&colors)),
                    Err(error) => (error.clone(), colors.danger),
                };
                div()
                    .id((self.id.clone(), format!("row-{ix}")))
                    .group(group.clone())
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_2()
                    .py_0p5()
                    .rounded(theme.radius(Radius::Sm))
                    .hover(|row| row.bg(colors.hover))
                    .font_family(theme.mono_family.clone())
                    .child(
                        div()
                            .flex_none()
                            .text_color(colors.fg)
                            .child(watch.expression.clone()),
                    )
                    .child(div().flex_none().text_color(colors.fg_subtle).child("="))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_color(color)
                            .child(Ellipsis::new(value)),
                    )
                    .child(
                        div()
                            .opacity(0.)
                            .group_hover(group, |actions| actions.opacity(1.))
                            .child(
                                IconButton::new(
                                    (self.id.clone(), format!("remove-{ix}")),
                                    IconName::X,
                                )
                                .variant(ButtonVariant::Ghost)
                                .size(ControlSize::Sm)
                                .tooltip("Stop watching")
                                .when_some(
                                    remove,
                                    |button, remove| {
                                        button.on_click(move |_, window, cx| remove(ix, window, cx))
                                    },
                                ),
                            ),
                    )
            }))
            .child(Input::new(&self.field).size(ControlSize::Sm))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn variable(name: &str, children: Vec<Variable>) -> Variable {
        Variable {
            name: name.to_string().into(),
            value: "".into(),
            kind: ValueKind::Object,
            type_name: None,
            children,
        }
    }

    #[test]
    fn open_variables_show_their_fields_under_them() {
        let all = [
            variable(
                "palette",
                vec![variable("colors", vec![variable("accent", vec![])])],
            ),
            variable("count", vec![]),
        ];
        let mut rows = Vec::new();
        flat(&all, "Locals", 1, &["Locals/palette".into()], &mut rows);
        let keys: Vec<(&str, usize)> = rows
            .iter()
            .map(|(key, depth, _)| (key.as_ref(), *depth))
            .collect();
        assert_eq!(
            keys,
            [
                ("Locals/palette", 1),
                ("Locals/palette/colors", 2),
                ("Locals/count", 1)
            ]
        );
    }
}
