use std::rc::Rc;

use gpui::{
    App, AppContext as _, ElementId, Entity, FontWeight, IntoElement, ParentElement, Rems,
    RenderOnce, SharedString, Styled, Window, canvas, div, rems,
};

use super::schema::{Field, FieldKey};
use crate::{
    buttons::{Button, ButtonVariant, IconButton},
    canvas::{OnEdit, caption, editing},
    data_display::{Badge, Tone},
    forms::{Checkbox, Choice, InlineEdit, Input, MultiSelect, Select, Switch, TextInput},
    primitives::IconName,
    theme::{ActiveTheme, Radius, TextSize},
    typography::Ellipsis,
};

/// The types a field's select offers; a field of any other keeps its own.
const TYPES: [&str; 10] = [
    "int",
    "bigint",
    "text",
    "varchar(255)",
    "boolean",
    "numeric",
    "date",
    "timestamptz",
    "uuid",
    "jsonb",
];

/// Each column's share of the table's least width: name, type, may be empty, primary key, default, and the remove button.
const SHARES: [f32; 6] = [0.26, 0.26, 0.1, 0.1, 0.2, 0.08];

/// A row of the table, each cell a fixed share of `least`, so what a cell holds is measured at its final width.
fn cells(parts: [gpui::AnyElement; 6], least: Rems) -> gpui::Div {
    parts
        .into_iter()
        .zip(SHARES)
        .fold(div().flex().items_center(), |row, (part, share)| {
            row.child(
                div()
                    .flex_none()
                    .w(rems(least.0 * share))
                    .min_w_0()
                    .pr_2()
                    .child(part),
            )
        })
}

/// A field stacked to fit a narrow box: its name and remove button, then its type with its flags, then its default.
fn stacked(parts: [gpui::AnyElement; 6], cx: &App) -> gpui::Div {
    let theme = cx.theme();
    let [name, ty, empty, key, default, remove] = parts;
    let flag = |part: gpui::AnyElement, words: &'static str| {
        div()
            .flex()
            .items_center()
            .gap_1()
            .child(part)
            .child(caption(words, cx))
    };
    div()
        .flex()
        .flex_col()
        .gap_2()
        .p_2()
        .rounded(theme.radius(Radius::Md))
        .border_1()
        .border_color(theme.colors.border)
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(div().flex_1().min_w_0().child(name))
                .child(div().flex_none().child(remove)),
        )
        .child(
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap_3()
                .child(div().w(theme.label_width()).child(ty))
                .child(flag(empty, "Empty"))
                .child(flag(key, "Key")),
        )
        .child(default)
}

/// The editor's own measure of its width, in pixels.
#[derive(Default)]
struct Room(f32);

/// A table's fields to edit in place, as a table where there is room for its columns and each field stacked on its own where there is not: a name and a default rewritten on a press, a type from the common ones, whether it may be empty, and the primary key. A foreign key shows where it points. Minus drops a field and Add field appends one; each edit hands the owner every field.
#[derive(IntoElement)]
pub struct TableStructureEditor {
    id: ElementId,
    fields: Vec<Field>,
    on_change: OnEdit<Vec<Field>>,
}

impl TableStructureEditor {
    pub fn new(
        id: impl Into<ElementId>,
        fields: impl IntoIterator<Item = Field>,
        on_change: impl Fn(Vec<Field>, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            fields: fields.into_iter().collect(),
            on_change: Rc::new(on_change),
        }
    }
}

impl RenderOnce for TableStructureEditor {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let on_change = self.on_change;
        let (id, fields, on_change) = (self.id, self.fields, &on_change);
        let room = window.use_keyed_state((id.clone(), "room"), cx, |_, _| Room::default());
        let rem = window.rem_size();
        let theme = cx.theme();
        let least = rems(theme.label_width().0 * 3.75);
        let wide = room.read(cx).0 >= f32::from(least.to_pixels(rem));
        let heading = |text: &'static str| {
            div()
                .text_size(theme.text_size(TextSize::Xs))
                .text_color(theme.colors.fg_subtle)
                .child(text)
                .into_any_element()
        };
        let head = wide.then(|| {
            cells(
                [
                    heading("Name"),
                    heading("Type"),
                    heading("Empty"),
                    heading("Key"),
                    heading("Default"),
                    div().into_any_element(),
                ],
                least,
            )
        });
        let rows = fields.iter().enumerate().map(|(ix, field)| {
            let at = |edit: fn(&mut Field, SharedString)| {
                editing(
                    "table structure",
                    &fields,
                    on_change,
                    move |all: &mut Vec<Field>, value| edit(&mut all[ix], value),
                )
            };
            let flag = |edit: fn(&mut Field, bool)| {
                editing(
                    "table structure",
                    &fields,
                    on_change,
                    move |all: &mut Vec<Field>, on: bool| edit(&mut all[ix], on),
                )
            };
            let named = at(|field, name| field.name = name);
            let typed = at(|field, ty| field.ty = ty);
            let defaulted = at(|field, value| field.default = (!value.is_empty()).then_some(value));
            let (empty, primary) = (
                flag(|field, on| field.nullable = on),
                flag(|field, on| field.key = on.then_some(FieldKey::Primary)),
            );
            let remove = editing(
                "table structure",
                &fields,
                on_change,
                move |all: &mut Vec<Field>, _: ()| {
                    all.remove(ix);
                },
            );
            let mut types: Vec<Choice> = TYPES.iter().map(|ty| Choice::new(*ty, *ty)).collect();
            if !TYPES.contains(&field.ty.as_ref()) {
                types.insert(0, Choice::new(field.ty.clone(), field.ty.clone()));
            }
            let key = match &field.key {
                Some(FieldKey::Foreign { table, field: to }) => {
                    Badge::new(format!("→ {table}.{to}"))
                        .tone(Tone::Info)
                        .into_any_element()
                }
                key => Checkbox::new(
                    (id.clone(), format!("primary-{ix}")),
                    *key == Some(FieldKey::Primary),
                )
                .on_change(move |on, window, cx| primary(on, window, cx))
                .into_any_element(),
            };
            let parts = [
                InlineEdit::new((id.clone(), format!("name-{ix}")), field.name.clone())
                    .on_commit(move |name, window, cx| named(name.clone(), window, cx))
                    .into_any_element(),
                Select::new((id.clone(), format!("type-{ix}")), types)
                    .selected(field.ty.clone())
                    .on_change(move |ty, window, cx| typed(ty.clone(), window, cx))
                    .into_any_element(),
                Checkbox::new((id.clone(), format!("empty-{ix}")), field.nullable)
                    .on_change(move |on, window, cx| empty(on, window, cx))
                    .into_any_element(),
                key,
                InlineEdit::new(
                    (id.clone(), format!("default-{ix}")),
                    field.default.clone().unwrap_or_default(),
                )
                .placeholder("None")
                .on_commit(move |value, window, cx| defaulted(value.clone(), window, cx))
                .into_any_element(),
                IconButton::new((id.clone(), format!("remove-{ix}")), IconName::Minus)
                    .tooltip("Remove the field")
                    .on_click(move |_, window, cx| remove((), window, cx))
                    .into_any_element(),
            ];
            match wide {
                true => cells(parts, least),
                false => stacked(parts, cx),
            }
        });
        let add = editing(
            "table structure",
            &fields,
            on_change,
            |all: &mut Vec<Field>, _: ()| {
                let name = (1..)
                    .map(|n| format!("field_{n}"))
                    .find(|name| all.iter().all(|field| field.name != name.as_str()))
                    .expect("a free name");
                all.push(Field::new(name, "text").nullable());
            },
        );
        div()
            .relative()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                canvas(
                    move |bounds, window, cx| {
                        let now = f32::from(bounds.size.width);
                        if room.read(cx).0 != now {
                            room.update(cx, |room, _| room.0 = now);
                            window.request_animation_frame();
                        }
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            )
            .children(head)
            .children(rows)
            .child(
                div().flex().child(
                    Button::new((id, "add"), "Add field")
                        .variant(ButtonVariant::Secondary)
                        .on_click(move |_, window, cx| add((), window, cx)),
                ),
            )
    }
}

/// An index on a table: its name, its fields in order, whether it keeps them unique, and its method.
#[derive(Clone, Debug, PartialEq)]
pub struct Index {
    pub name: SharedString,
    pub fields: Vec<SharedString>,
    pub unique: bool,
    pub method: SharedString,
}

const METHODS: [&str; 4] = ["btree", "hash", "gin", "gist"];

type OnName = Rc<dyn Fn(&SharedString, &mut Window, &mut App)>;
type OnIndex = Rc<dyn Fn(Index, &mut Window, &mut App)>;

/// A new index being drawn up: its name field, fields, uniqueness and method.
struct Draft {
    name: Entity<TextInput>,
    fields: Vec<SharedString>,
    unique: bool,
    method: SharedString,
}

/// A table's indexes, each with its fields, its method and whether it is unique, and a button to drop it; under them, a new one to draw up from the table's fields. Create waits for a name and a field, and asks the owner.
#[derive(IntoElement)]
pub struct IndexManager {
    id: ElementId,
    indexes: Vec<Index>,
    fields: Vec<SharedString>,
    on_drop: OnName,
    on_create: OnIndex,
}

impl IndexManager {
    /// `fields` are the table's, which a new index picks from.
    pub fn new(
        id: impl Into<ElementId>,
        indexes: impl IntoIterator<Item = Index>,
        fields: impl IntoIterator<Item = impl Into<SharedString>>,
        on_create: impl Fn(Index, &mut Window, &mut App) + 'static,
        on_drop: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            indexes: indexes.into_iter().collect(),
            fields: fields.into_iter().map(Into::into).collect(),
            on_drop: Rc::new(on_drop),
            on_create: Rc::new(on_create),
        }
    }
}

impl RenderOnce for IndexManager {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let on_create = self.on_create;
        let on_drop = self.on_drop;
        let id = self.id;
        let draft = window.use_keyed_state((id.clone(), "draft"), cx, |window, cx| Draft {
            name: cx.new(|cx| TextInput::new(window, cx).placeholder("Index name")),
            fields: Vec::new(),
            unique: false,
            method: "btree".into(),
        });
        let theme = cx.theme();
        let rows = self.indexes.iter().map(|index| {
            let (name, on_drop) = (index.name.clone(), on_drop.clone());
            div()
                .flex()
                .items_center()
                .gap_2()
                .py_1()
                .border_b_1()
                .border_color(theme.colors.border)
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .font_weight(FontWeight::MEDIUM)
                                .child(Ellipsis::new(index.name.clone())),
                        )
                        .child(
                            div()
                                .text_size(theme.text_size(TextSize::Sm))
                                .text_color(theme.colors.fg_muted)
                                .child(Ellipsis::new(index.fields.join(", "))),
                        ),
                )
                .child(
                    div()
                        .flex_none()
                        .flex()
                        .items_center()
                        .gap_1()
                        .children(
                            index
                                .unique
                                .then(|| Badge::new("unique").tone(Tone::Accent)),
                        )
                        .child(Badge::new(index.method.clone()))
                        .child(
                            IconButton::new(
                                (id.clone(), format!("drop-{}", index.name)),
                                IconName::Trash2,
                            )
                            .tooltip("Drop the index")
                            .on_click(move |_, window, cx| {
                                log::info!("index manager: drop {name}");
                                on_drop(&name, window, cx);
                            }),
                        ),
                )
        });
        let now = draft.read(cx);
        let (name, picked, unique, method) = (
            now.name.clone(),
            now.fields.clone(),
            now.unique,
            now.method.clone(),
        );
        let ready = !name.read(cx).text().trim().is_empty() && !picked.is_empty();
        let [chose, flipped, pick_method, made] = [(); 4].map(|_| draft.clone());
        let methods = Select::new(
            (id.clone(), "method"),
            METHODS.iter().map(|method| Choice::new(*method, *method)),
        )
        .selected(method)
        .on_change(move |value, _, cx| {
            pick_method.update(cx, |draft, cx| {
                draft.method = value.clone();
                cx.notify();
            })
        });
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(div().flex().flex_col().children(rows))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Input::new(&name))
                    .child(
                        MultiSelect::new(
                            (id.clone(), "fields"),
                            self.fields
                                .iter()
                                .map(|field| Choice::new(field.clone(), field.clone())),
                        )
                        .selected(picked)
                        .placeholder("Fields")
                        .on_change(move |values, _, cx| {
                            chose.update(cx, |draft, cx| {
                                draft.fields = values.to_vec();
                                cx.notify();
                            })
                        }),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_2()
                            .child(
                                Switch::new((id.clone(), "unique"), unique)
                                    .label("Unique")
                                    .on_change(move |on, _, cx| {
                                        flipped.update(cx, |draft, cx| {
                                            draft.unique = on;
                                            cx.notify();
                                        })
                                    }),
                            )
                            .child(div().flex_1().min_w_0().child(methods)),
                    )
                    .child(
                        Button::new((id, "create"), "Create index")
                            .variant(ButtonVariant::Primary)
                            .disabled(!ready)
                            .on_click(move |_, window, cx| {
                                let index = made.update(cx, |draft, cx| {
                                    let name = SharedString::from(
                                        draft.name.read(cx).text().trim().to_string(),
                                    );
                                    draft
                                        .name
                                        .update(cx, |input, cx| input.set_text(String::new(), cx));
                                    let index = Index {
                                        name,
                                        fields: std::mem::take(&mut draft.fields),
                                        unique: draft.unique,
                                        method: draft.method.clone(),
                                    };
                                    cx.notify();
                                    index
                                });
                                log::info!("index manager: create {}", index.name);
                                on_create(index, window, cx);
                            }),
                    ),
            )
    }
}
