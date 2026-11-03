use std::{cell::RefCell, rc::Rc};

use gpui::{
    App, ElementId, Entity, IntoElement, ParentElement, RenderOnce, SharedString, Styled,
    Subscription, Window, div, prelude::*,
};

use super::{FilterRule, SortKey, Test};
use crate::i18n;
use crate::{
    buttons::{Button, ButtonVariant, IconButton, SegmentedControl},
    forms::{Choice, Input, InputEvent, Select, TextInput},
    motion::Reorder,
    primitives::{Icon, IconName},
    theme::{ActiveTheme, ControlSize, IconSize},
};

type OnFilters = Rc<dyn Fn(&[FilterRule], bool, &mut Window, &mut App)>;
type OnSorts = Rc<dyn Fn(&[SortKey], &mut Window, &mut App)>;

/// The newest rules and hearer, for value fields whose edits come later.
type Latest = Rc<RefCell<(Vec<FilterRule>, bool, Option<OnFilters>)>>;

/// A builder's value fields, one per rule in the rules' order, and what their edits report to.
#[derive(Default)]
struct Fields {
    fields: Vec<(Entity<TextInput>, Subscription)>,
    latest: Latest,
}

fn choices(columns: &[(SharedString, SharedString)]) -> Vec<Choice> {
    columns
        .iter()
        .map(|(key, title)| Choice::new(key.clone(), title.clone()))
        .collect()
}

/// The columns a sort row may take: its own, and those no other row holds.
fn open(
    columns: &[(SharedString, SharedString)],
    keys: &[SortKey],
    own: &SharedString,
) -> Vec<(SharedString, SharedString)> {
    columns
        .iter()
        .filter(|(key, _)| key == own || !keys.iter().any(|sort| sort.column == *key))
        .cloned()
        .collect()
}

/// Filter rules to build: each a column, a test and a value, and a switch between keeping rows that pass all and rows that pass any. The owner keeps the rules and hands them to a `DataTable`.
#[derive(IntoElement)]
pub struct FilterBuilder {
    id: ElementId,
    columns: Vec<(SharedString, SharedString)>,
    rules: Vec<FilterRule>,
    any: bool,
    on_change: Option<OnFilters>,
}

impl FilterBuilder {
    /// `columns` are each a key and a title.
    pub fn new(
        id: impl Into<ElementId>,
        columns: impl IntoIterator<Item = (impl Into<SharedString>, impl Into<SharedString>)>,
    ) -> Self {
        let columns: Vec<_> = columns
            .into_iter()
            .map(|(key, title)| (key.into(), title.into()))
            .collect();
        assert!(!columns.is_empty(), "a filter builder needs a column");
        Self {
            id: id.into(),
            columns,
            rules: Vec::new(),
            any: false,
            on_change: None,
        }
    }

    pub fn rules(mut self, rules: impl IntoIterator<Item = FilterRule>, any: bool) -> Self {
        self.rules = rules.into_iter().collect();
        self.any = any;
        self
    }

    pub fn on_change(
        mut self,
        handler: impl Fn(&[FilterRule], bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for FilterBuilder {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let fields =
            window.use_keyed_state((self.id.clone(), "fields"), cx, |_, _| Fields::default());
        let latest = fields.read(cx).latest.clone();
        *latest.borrow_mut() = (self.rules.clone(), self.any, self.on_change.clone());
        while fields.read(cx).fields.len() < self.rules.len() {
            let field = cx.new(|cx| TextInput::new(window, cx));
            let (latest, owner) = (latest.clone(), fields.downgrade());
            let heard = window.subscribe(&field, cx, move |field, event, window, cx| {
                if !matches!(event, InputEvent::Changed) {
                    return;
                }
                let Some(ix) = owner.upgrade().and_then(|owner| {
                    let held = &owner.read(cx).fields;
                    held.iter()
                        .position(|(held, _)| held.entity_id() == field.entity_id())
                }) else {
                    return;
                };
                let (mut rules, any, on_change) = latest.borrow().clone();
                let Some(rule) = rules.get_mut(ix) else {
                    return;
                };
                rule.value = field.read(cx).text().to_string().into();
                if let Some(on_change) = on_change {
                    on_change(&rules, any, window, cx);
                }
            });
            fields.update(cx, |fields, _| fields.fields.push((field, heard)));
        }
        fields.update(cx, |fields, _| fields.fields.truncate(self.rules.len()));
        let entities: Vec<Entity<TextInput>> = fields
            .read(cx)
            .fields
            .iter()
            .map(|(field, _)| field.clone())
            .collect();
        for (field, rule) in entities.iter().zip(&self.rules) {
            let typing = field.read(cx).focus().is_focused(window);
            if !typing && field.read(cx).text() != rule.value.as_ref() {
                let value = rule.value.to_string();
                field.update(cx, |field, cx| field.set_text(value, cx));
            }
        }
        let emit = move |rules: Vec<FilterRule>, any: bool, window: &mut Window, cx: &mut App| {
            let hearer = latest.borrow().2.clone();
            log::info!(
                "filter builder: {} rules, {}",
                rules.len(),
                if any { "any" } else { "all" }
            );
            if let Some(on_change) = hearer {
                on_change(&rules, any, window, cx);
            }
        };
        let emit = Rc::new(emit);
        let tests: Vec<Choice> = Test::ALL
            .iter()
            .map(|test| Choice::new(test.key(), i18n::text(cx, test.key(), &[])))
            .collect();
        let theme = cx.theme();
        let (rules, any, id) = (self.rules.clone(), self.any, self.id.clone());
        let rows: Vec<_> = self
            .rules
            .iter()
            .enumerate()
            .map(|(ix, rule)| {
                let (column, test, remove, held) =
                    (emit.clone(), emit.clone(), emit.clone(), fields.clone());
                let (for_column, for_test, for_remove) =
                    (rules.clone(), rules.clone(), rules.clone());
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div().w(theme.label_width()).child(
                            Select::new(
                                (id.clone(), format!("column-{ix}")),
                                choices(&self.columns),
                            )
                            .selected(rule.column.clone())
                            .size(ControlSize::Sm)
                            .on_change(move |key, window, cx| {
                                let mut next = for_column.clone();
                                next[ix].column = key.clone();
                                column(next, any, window, cx)
                            }),
                        ),
                    )
                    .child(
                        div().w(theme.label_width() * 0.75).child(
                            Select::new((id.clone(), format!("test-{ix}")), tests.clone())
                                .selected(rule.test.key())
                                .size(ControlSize::Sm)
                                .on_change(move |label, window, cx| {
                                    let mut next = for_test.clone();
                                    next[ix].test = *Test::ALL
                                        .iter()
                                        .find(|test| test.key() == label.as_ref())
                                        .expect("a listed test");
                                    test(next, any, window, cx)
                                }),
                        ),
                    )
                    .child(div().flex_1().when(rule.test.takes_value(), |slot| {
                        slot.child(Input::new(&entities[ix]).size(ControlSize::Sm))
                    }))
                    .child(
                        IconButton::new((id.clone(), format!("remove-{ix}")), IconName::X)
                            .variant(ButtonVariant::Ghost)
                            .size(ControlSize::Sm)
                            .tooltip(i18n::text(cx, "builder.remove", &[]))
                            .on_click(move |_, window, cx| {
                                let mut next = for_remove.clone();
                                next.remove(ix);
                                held.update(cx, |held, _| drop(held.fields.remove(ix)));
                                remove(next, any, window, cx)
                            }),
                    )
            })
            .collect();
        let (add, join, first) = (emit.clone(), emit, self.columns[0].0.clone());
        let (to_add, to_join) = (self.rules.clone(), self.rules.clone());
        div().flex().flex_col().gap_2().children(rows).child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(
                    Button::new(
                        (self.id.clone(), "add"),
                        i18n::text(cx, "builder.add_rule", &[]),
                    )
                    .icon(IconName::Plus)
                    .variant(ButtonVariant::Ghost)
                    .size(ControlSize::Sm)
                    .on_click(move |_, window, cx| {
                        let mut next = to_add.clone();
                        next.push(FilterRule {
                            column: first.clone(),
                            ..FilterRule::default()
                        });
                        add(next, any, window, cx)
                    }),
                )
                .when(self.rules.len() > 1, |footer| {
                    footer.child(
                        SegmentedControl::new(
                            (self.id.clone(), "join"),
                            if any { "any" } else { "all" },
                        )
                        .segment("all", i18n::text(cx, "builder.all", &[]), None)
                        .segment("any", i18n::text(cx, "builder.any", &[]), None)
                        .size(ControlSize::Sm)
                        .on_change(move |value, window, cx| {
                            join(to_join.clone(), value.as_ref() == "any", window, cx)
                        }),
                    )
                }),
        )
    }
}

/// Sort keys to build, the first deciding most: each a column and a direction. Add one, flip it, remove it, or drag it to a new place. The owner keeps the keys and hands them to a `DataTable`.
#[derive(IntoElement)]
pub struct SortBuilder {
    id: ElementId,
    columns: Vec<(SharedString, SharedString)>,
    keys: Vec<SortKey>,
    on_change: Option<OnSorts>,
}

impl SortBuilder {
    /// `columns` are each a key and a title.
    pub fn new(
        id: impl Into<ElementId>,
        columns: impl IntoIterator<Item = (impl Into<SharedString>, impl Into<SharedString>)>,
    ) -> Self {
        Self {
            id: id.into(),
            columns: columns
                .into_iter()
                .map(|(key, title)| (key.into(), title.into()))
                .collect(),
            keys: Vec::new(),
            on_change: None,
        }
    }

    /// The keys, each on its own column.
    pub fn keys(mut self, keys: impl IntoIterator<Item = SortKey>) -> Self {
        self.keys = keys.into_iter().collect();
        assert!(
            self.keys.iter().enumerate().all(|(ix, key)| self.keys[..ix]
                .iter()
                .all(|before| before.column != key.column)),
            "sort keys take a column each"
        );
        self
    }

    pub fn on_change(
        mut self,
        handler: impl Fn(&[SortKey], &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for SortBuilder {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let emit: OnSorts = {
            let on_change = self.on_change.clone();
            Rc::new(move |keys: &[SortKey], window: &mut Window, cx: &mut App| {
                log::info!("sort builder: {} keys", keys.len());
                if let Some(on_change) = &on_change {
                    on_change(keys, window, cx);
                }
            })
        };
        let theme = cx.theme();
        let (keys, id, grip) = (self.keys.clone(), self.id.clone(), theme.colors.fg_subtle);
        let unused = self
            .columns
            .iter()
            .find(|(key, _)| !keys.iter().any(|sort| sort.column == *key))
            .map(|(key, _)| key.clone());
        let moved = (emit.clone(), keys.clone());
        let list = self.keys.iter().enumerate().fold(
            Reorder::new((self.id.clone(), "keys")).on_reorder(move |from, to, window, cx| {
                let mut next = moved.1.clone();
                let key = next.remove(from);
                next.insert(to, key);
                (moved.0)(&next, window, cx)
            }),
            |list, (ix, sort)| {
                let (column, flip, remove) = (emit.clone(), emit.clone(), emit.clone());
                let (for_column, for_flip, for_remove) = (keys.clone(), keys.clone(), keys.clone());
                list.row(
                    sort.column.clone(),
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .py_1()
                        .child(
                            Icon::new(IconName::GripVertical)
                                .size(IconSize::Sm)
                                .color(grip),
                        )
                        .child(
                            div().w(theme.label_width()).child(
                                Select::new(
                                    (id.clone(), format!("sort-column-{ix}")),
                                    choices(&open(&self.columns, &keys, &sort.column)),
                                )
                                .selected(sort.column.clone())
                                .size(ControlSize::Sm)
                                .on_change(
                                    move |key, window, cx| {
                                        let mut next = for_column.clone();
                                        next[ix].column = key.clone();
                                        column(&next, window, cx)
                                    },
                                ),
                            ),
                        )
                        .child(
                            SegmentedControl::new(
                                (id.clone(), format!("sort-way-{ix}")),
                                if sort.rising { "up" } else { "down" },
                            )
                            .segment(
                                "up",
                                i18n::text(cx, "builder.up", &[]),
                                Some(IconName::ArrowUp),
                            )
                            .segment(
                                "down",
                                i18n::text(cx, "builder.down", &[]),
                                Some(IconName::ArrowDown),
                            )
                            .size(ControlSize::Sm)
                            .on_change(move |way, window, cx| {
                                let mut next = for_flip.clone();
                                next[ix].rising = way.as_ref() == "up";
                                flip(&next, window, cx)
                            }),
                        )
                        .child(
                            IconButton::new((id.clone(), format!("sort-remove-{ix}")), IconName::X)
                                .variant(ButtonVariant::Ghost)
                                .size(ControlSize::Sm)
                                .tooltip(i18n::text(cx, "builder.remove", &[]))
                                .on_click(move |_, window, cx| {
                                    let mut next = for_remove.clone();
                                    next.remove(ix);
                                    remove(&next, window, cx)
                                }),
                        ),
                )
            },
        );
        let add = (emit, keys);
        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(list)
            .children(unused.map(|column| {
                Button::new(
                    (self.id.clone(), "add"),
                    i18n::text(cx, "builder.add_sort", &[]),
                )
                .icon(IconName::Plus)
                .variant(ButtonVariant::Ghost)
                .size(ControlSize::Sm)
                .on_click(move |_, window, cx| {
                    let mut next = add.1.clone();
                    next.push(SortKey {
                        column: column.clone(),
                        rising: true,
                    });
                    (add.0)(&next, window, cx)
                })
            }))
    }
}

#[cfg(test)]
mod tests {
    use gpui::SharedString;

    use super::{SortKey, open};

    #[test]
    fn a_sort_row_offers_its_own_column_and_the_unused_ones() {
        let columns: Vec<(SharedString, SharedString)> = ["a", "b", "c"]
            .map(|key| (key.into(), key.to_uppercase().into()))
            .to_vec();
        let keys = [
            SortKey {
                column: "a".into(),
                rising: true,
            },
            SortKey {
                column: "b".into(),
                rising: false,
            },
        ];
        let offered = |own: &'static str| {
            open(&columns, &keys, &own.into())
                .into_iter()
                .map(|(key, _)| key.to_string())
                .collect::<Vec<_>>()
        };
        assert_eq!(offered("a"), ["a", "c"]);
        assert_eq!(offered("b"), ["b", "c"]);
    }
}
