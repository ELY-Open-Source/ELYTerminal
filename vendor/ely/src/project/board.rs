use std::{collections::HashMap, rc::Rc};

use gpui::{
    AnyElement, App, Axis, Bounds, ElementId, EmptyView, EntityId, FocusHandle, InteractiveElement,
    IntoElement, ParentElement, Pixels, Point, RenderOnce, ScrollHandle, SharedString,
    StatefulInteractiveElement, Styled, Window, anchored, canvas, div, prelude::*,
};

use super::moves::{landing, stepped};
use crate::{
    layout::{on_axis, reveal_when_focused},
    motion::Flip,
    primitives::{FocusRing, raise, tab_stop},
    theme::{ActiveTheme, Elevation, Radius, TextSize},
    typography::tabular,
};

type OnMove = Rc<dyn Fn(&SharedString, &SharedString, usize, &mut Window, &mut App)>;
type OnCard = Rc<dyn Fn(&SharedString, &mut Window, &mut App)>;

/// A card on a board: its key and what it shows.
pub struct KanbanCard {
    key: SharedString,
    content: AnyElement,
}

impl KanbanCard {
    pub fn new(key: impl Into<SharedString>, content: impl IntoElement) -> Self {
        Self {
            key: key.into(),
            content: content.into_any_element(),
        }
    }
}

/// A board's column: its key, its title, and its cards in order.
pub struct KanbanColumn {
    key: SharedString,
    title: SharedString,
    cards: Vec<KanbanCard>,
}

impl KanbanColumn {
    pub fn new(key: impl Into<SharedString>, title: impl Into<SharedString>) -> Self {
        Self {
            key: key.into(),
            title: title.into(),
            cards: Vec::new(),
        }
    }

    pub fn card(mut self, card: KanbanCard) -> Self {
        self.cards.push(card);
        self
    }

    pub fn cards(mut self, cards: impl IntoIterator<Item = KanbanCard>) -> Self {
        self.cards.extend(cards);
        self
    }
}

/// A card on its way, by key, and whose board it belongs to.
struct CardDrag {
    owner: EntityId,
    key: SharedString,
}

/// The card held: its key, where the pointer took it, the pointer now, and where it would land.
#[derive(Clone)]
struct Hold {
    key: SharedString,
    grab: Point<Pixels>,
    pointer: Point<Pixels>,
    column: SharedString,
    index: usize,
}

/// Last frame's measures: each card's row height and box, each column's body.
#[derive(Default)]
struct Measures {
    rows: HashMap<SharedString, Pixels>,
    cards: HashMap<SharedString, Bounds<Pixels>>,
    bodies: HashMap<SharedString, Bounds<Pixels>>,
}

/// Measures the box it lies over into `measures` when it changes.
fn measure(
    measures: &gpui::Entity<Measures>,
    save: impl Fn(&mut Measures, Bounds<Pixels>) -> bool + 'static,
) -> impl IntoElement {
    let measures = measures.clone();
    canvas(
        move |bounds, window, cx| {
            let changed = measures.update(cx, |measures, _| save(measures, bounds));
            if changed {
                measures.update(cx, |_, cx| cx.notify());
                window.request_animation_frame();
            }
        },
        |_, _, _, _| {},
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full()
}

/// Columns of cards: a card drags to a place in any column, the others gliding aside, and Option with an arrow moves the focused one. Each move asks the owner through `on_move(card, column, index)`, the index counted without the card.
#[derive(IntoElement)]
pub struct KanbanBoard {
    id: ElementId,
    columns: Vec<KanbanColumn>,
    on_move: Option<OnMove>,
    on_open: Option<OnCard>,
}

impl KanbanBoard {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            columns: Vec::new(),
            on_move: None,
            on_open: None,
        }
    }

    pub fn column(mut self, column: KanbanColumn) -> Self {
        self.columns.push(column);
        self
    }

    /// Gets a card's key, the column it goes to, and its place there.
    pub fn on_move(
        mut self,
        handler: impl Fn(&SharedString, &SharedString, usize, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_move = Some(Rc::new(handler));
        self
    }

    /// A press, Enter or Space on a card; gets its key.
    pub fn on_open(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_open = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for KanbanBoard {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id.clone();
        let mut places: HashMap<SharedString, (usize, usize)> = HashMap::new();
        for (column_ix, column) in self.columns.iter().enumerate() {
            let twice = self.columns[..column_ix]
                .iter()
                .any(|other| other.key == column.key);
            assert!(!twice, "board {id:?}: column {} twice", column.key);
            for (ix, card) in column.cards.iter().enumerate() {
                let known = places.insert(card.key.clone(), (column_ix, ix));
                assert!(known.is_none(), "board {id:?}: card {} twice", card.key);
            }
        }
        let hold = window.use_keyed_state((id.clone(), "hold"), cx, |_, _| None::<Hold>);
        let measures =
            window.use_keyed_state((id.clone(), "measures"), cx, |_, _| Measures::default());
        let scroll = window
            .use_keyed_state((id.clone(), "scroll"), cx, |_, _| ScrollHandle::new())
            .read(cx)
            .clone();
        let owner = hold.entity_id();
        let stale = hold
            .read(cx)
            .as_ref()
            .is_some_and(|held| !cx.has_active_drag() || !places.contains_key(&held.key));
        if stale {
            log::debug!("board {id:?}: the held card let go");
            hold.update(cx, |hold, _| *hold = None);
        }
        let held = hold.read(cx).clone();
        let focus: HashMap<SharedString, FocusHandle> = places
            .keys()
            .map(|key| {
                let stop = tab_stop((id.clone(), format!("card-{key}")).into(), true, window, cx);
                (key.clone(), stop)
            })
            .collect();
        let keys: Vec<SharedString> = self
            .columns
            .iter()
            .map(|column| column.key.clone())
            .collect();
        let orders: HashMap<SharedString, Vec<SharedString>> = self
            .columns
            .iter()
            .map(|column| {
                let cards = column.cards.iter().map(|card| card.key.clone()).collect();
                (column.key.clone(), cards)
            })
            .collect();
        let origins: HashMap<SharedString, (SharedString, usize)> = places
            .iter()
            .map(|(key, (column, ix))| (key.clone(), (keys[*column].clone(), *ix)))
            .collect();
        let lens: Rc<Vec<usize>> = Rc::new(
            self.columns
                .iter()
                .map(|column| column.cards.len())
                .collect(),
        );
        let mut reveals: HashMap<SharedString, AnyElement> = places
            .iter()
            .map(|(key, (column, _))| {
                let place = (id.clone(), format!("reveal-{key}-{}", keys[*column]));
                let reveal =
                    reveal_when_focused(place, &scroll, &focus[key], Axis::Horizontal, window, cx);
                (key.clone(), reveal.into_any_element())
            })
            .collect();
        let theme = cx.theme();
        let colors = &theme.colors;
        let width = theme.project().column;
        let mut lifted: Option<AnyElement> = None;
        let mut columns = Vec::new();
        for (column_ix, column) in self.columns.into_iter().enumerate() {
            let mut flip = Flip::new((id.clone(), format!("column-{}", column.key)));
            let count = column.cards.len();
            let mut shown = 0;
            let slot = held
                .as_ref()
                .filter(|held| held.column == column.key)
                .map(|held| held.index);
            let placeholder = |measures: &gpui::Entity<Measures>, key: &SharedString, cx: &App| {
                let card = measures.read(cx).cards[key];
                div().pb_2().child(
                    div()
                        .h(card.size.height)
                        .rounded(cx.theme().radius(Radius::Md))
                        .bg(cx.theme().colors.hover),
                )
            };
            for (ix, card) in column.cards.into_iter().enumerate() {
                let key = card.key.clone();
                if held.as_ref().is_some_and(|held| held.key == key) {
                    lifted = Some(card.content);
                    continue;
                }
                if slot == Some(shown) {
                    let held_key = &held.as_ref().expect("a slot holds a card").key;
                    flip = flip.row("+slot", placeholder(&measures, held_key, cx));
                }
                shown += 1;
                let handle = focus[&key].clone();
                let (drag, moves, opens) =
                    (hold.clone(), self.on_move.clone(), self.on_open.clone());
                let (column_key, row_key, card_key, open_key) =
                    (column.key.clone(), key.clone(), key.clone(), key.clone());
                let (lens, keys, from) = (lens.clone(), keys.clone(), (column_ix, ix));
                let (row_measures, card_measures, grab_measures) =
                    (measures.clone(), measures.clone(), measures.clone());
                let face = div()
                    .id((id.clone(), format!("card-{key}")))
                    .debug_selector(move || format!("card {card_key}"))
                    .relative()
                    .track_focus(&handle)
                    .rounded(theme.radius(Radius::Md))
                    .border_1()
                    .border_color(gpui::transparent_black())
                    .focus_ring(cx)
                    .cursor_grab()
                    .on_click(move |_, window, cx| {
                        log::info!("board: open {open_key}");
                        if let Some(opens) = &opens {
                            opens(&open_key, window, cx);
                        }
                    })
                    .on_key_down(move |event, window, cx| {
                        let modifiers = event.keystroke.modifiers;
                        if !modifiers.alt || modifiers.platform || modifiers.control {
                            return;
                        }
                        let Some((column, index)) =
                            stepped(&lens, from.0, from.1, event.keystroke.key.as_str())
                        else {
                            return;
                        };
                        cx.stop_propagation();
                        log::info!("board: {row_key} to {} at {index}", keys[column]);
                        if let Some(moves) = &moves {
                            moves(&row_key, &keys[column], index, window, cx);
                        }
                    })
                    .on_drag(
                        CardDrag {
                            owner,
                            key: key.clone(),
                        },
                        move |drag_card, offset, _, cx| {
                            let origin = grab_measures.read(cx).cards[&drag_card.key].origin;
                            log::info!("board: {} lifted", drag_card.key);
                            drag.update(cx, |hold, cx| {
                                *hold = Some(Hold {
                                    key: drag_card.key.clone(),
                                    grab: offset,
                                    pointer: origin + offset,
                                    column: column_key.clone(),
                                    index: from.1,
                                });
                                cx.notify();
                            });
                            cx.new(|_| EmptyView)
                        },
                    )
                    .child(card.content)
                    .child(measure(&card_measures, {
                        let key = key.clone();
                        move |measures, bounds| {
                            measures.cards.insert(key.clone(), bounds) != Some(bounds)
                        }
                    }));
                let reveal = reveals.remove(&key).expect("each card has a reveal");
                let row = div()
                    .relative()
                    .pb_2()
                    .child(face)
                    .child(reveal)
                    .child(measure(&row_measures, {
                        let key = key.clone();
                        move |measures, bounds| {
                            measures.rows.insert(key.clone(), bounds.size.height)
                                != Some(bounds.size.height)
                        }
                    }));
                flip = flip.row(key, row);
            }
            if slot.is_some_and(|slot| slot >= shown) {
                let held_key = &held.as_ref().expect("a slot holds a card").key;
                flip = flip.row("+slot", placeholder(&measures, held_key, cx));
            }
            let body_key = column.key.clone();
            columns.push(
                div()
                    .debug_selector({
                        let key = column.key.clone();
                        move || format!("column {key}")
                    })
                    .w(width)
                    .flex_none()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .p_2()
                    .rounded(theme.radius(Radius::Lg))
                    .bg(colors.sunken)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_2()
                            .px_1()
                            .child(
                                div()
                                    .text_size(theme.text_size(TextSize::Sm))
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .child(column.title),
                            )
                            .child(
                                tabular(div())
                                    .text_size(theme.text_size(TextSize::Sm))
                                    .text_color(colors.fg_muted)
                                    .child(count.to_string()),
                            ),
                    )
                    .child(div().relative().min_h_16().child(flip).child(measure(
                        &measures,
                        move |measures, bounds| {
                            measures.bodies.insert(body_key.clone(), bounds) != Some(bounds)
                        },
                    ))),
            );
        }
        let raised = held.as_ref().zip(lifted).map(|(held, content)| {
            raise(
                (id.clone(), "held"),
                anchored().position(held.pointer - held.grab).child(
                    div()
                        .w(width)
                        .rounded(theme.radius(Radius::Md))
                        .shadow(theme.elevation(Elevation::Floating))
                        .child(content),
                ),
            )
        });
        let (moves, drops, move_measures) = (hold.clone(), hold, measures);
        let landings = orders.clone();
        let on_move = self.on_move;
        on_axis(
            div()
                .id(id.clone())
                .debug_selector(|| "kanban-board".into())
                .track_scroll(&scroll)
                .overflow_x_scroll(),
        )
        .flex()
        .items_start()
        .gap_3()
        .on_drag_move::<CardDrag>(move |event, _, cx| {
            if event.drag(cx).owner != owner {
                return;
            }
            let pointer = event.event.position;
            let measures = move_measures.read(cx);
            let Some(held) = moves.read(cx).clone() else {
                return;
            };
            let column = measures
                .bodies
                .iter()
                .filter(|(key, _)| orders.contains_key(*key))
                .min_by(|(_, a), (_, b)| {
                    let distance = |bounds: &Bounds<Pixels>| (bounds.center().x - pointer.x).abs();
                    distance(a)
                        .partial_cmp(&distance(b))
                        .expect("distances are numbers")
                })
                .map(|(key, body)| (key.clone(), *body));
            let Some((column, body)) = column else { return };
            let card = measures.cards[&held.key];
            let heights: Vec<Pixels> = orders[&column]
                .iter()
                .filter(|key| **key != held.key)
                .filter_map(|key| measures.rows.get(key).copied())
                .collect();
            let middle = pointer.y - held.grab.y + card.size.height / 2.0;
            let index = landing(&heights, body.origin.y, middle);
            moves.update(cx, |hold, cx| {
                if let Some(hold) = hold {
                    hold.pointer = pointer;
                    hold.column = column;
                    hold.index = index;
                }
                cx.notify();
            });
        })
        .on_drop(move |drag: &CardDrag, window, cx| {
            if drag.owner != owner {
                return;
            }
            let Some(held) = drops.update(cx, |hold, cx| {
                cx.notify();
                hold.take()
            }) else {
                return;
            };
            let room = landings
                .get(&held.column)
                .map(|order| order.iter().filter(|key| **key != held.key).count());
            let (Some(origin), Some(room)) = (origins.get(&held.key), room) else {
                log::error!(
                    "board: {} or column {} left the board; the drop is dropped",
                    held.key,
                    held.column
                );
                return;
            };
            if held.index > room {
                log::error!(
                    "board: {} at {} past {} in {}; the drop is dropped",
                    held.key,
                    held.index,
                    room,
                    held.column
                );
                return;
            }
            if *origin == (held.column.clone(), held.index) {
                log::info!("board: {} dropped where it was", held.key);
                return;
            }
            log::info!(
                "board: {} dropped in {} at {}",
                held.key,
                held.column,
                held.index
            );
            if let Some(on_move) = &on_move {
                on_move(&held.key, &held.column, held.index, window, cx);
            }
        })
        .children(columns)
        .children(raised)
    }
}
