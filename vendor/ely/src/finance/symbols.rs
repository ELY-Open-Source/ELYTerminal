use std::rc::Rc;

use gpui::{
    App, ElementId, Entity, FontWeight, IntoElement, ParentElement, RenderOnce, SharedString,
    Styled, Window, div,
};

use super::quotes::{PriceChangeBadge, moves, price};
use crate::{
    data_display::{Avatar, Sparkline},
    forms::{Choice, Combobox, TextInput},
    lists::{ListItem, SelectableList},
    tables::{Column, DataTable, FilterBuilder, FilterRule, Row},
    theme::{ActiveTheme, AvatarSize, TextSize},
    typography::tabular,
};

/// A symbol as a small tile of its letters, toned steadily from its name, beside the symbol and what it trades on.
#[derive(IntoElement)]
pub struct SymbolBadge {
    id: ElementId,
    symbol: SharedString,
    venue: Option<SharedString>,
}

impl SymbolBadge {
    pub fn new(id: impl Into<ElementId>, symbol: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            symbol: symbol.into(),
            venue: None,
        }
    }

    /// Where it trades, or its name, read under the symbol.
    pub fn venue(mut self, venue: impl Into<SharedString>) -> Self {
        self.venue = Some(venue.into());
        self
    }
}

impl RenderOnce for SymbolBadge {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        div()
            .flex()
            .items_center()
            .gap_2()
            .child(
                Avatar::new(self.id, self.symbol.clone())
                    .square()
                    .size(AvatarSize::Sm),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .text_size(theme.text_size(TextSize::Sm))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(theme.colors.fg)
                            .child(self.symbol),
                    )
                    .children(self.venue.map(|venue| {
                        div()
                            .text_size(theme.text_size(TextSize::Xs))
                            .text_color(theme.colors.fg_muted)
                            .child(venue)
                    })),
            )
    }
}

/// A search for a symbol by its letters or its name, each result with where it trades.
#[derive(IntoElement)]
pub struct SymbolSearch {
    id: ElementId,
    field: Entity<TextInput>,
    symbols: Vec<(SharedString, SharedString, SharedString)>,
    on_pick: Option<OnSymbol>,
}

impl SymbolSearch {
    /// Symbols to find, each with its name and where it trades; `field` holds the query.
    pub fn new(
        id: impl Into<ElementId>,
        field: &Entity<TextInput>,
        symbols: impl IntoIterator<Item = (SharedString, SharedString, SharedString)>,
    ) -> Self {
        Self {
            id: id.into(),
            field: field.clone(),
            symbols: symbols.into_iter().collect(),
            on_pick: None,
        }
    }

    pub fn on_pick(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_pick = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for SymbolSearch {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let choices = self.symbols.iter().map(|(symbol, name, venue)| {
            Choice::new(symbol.clone(), format!("{symbol} · {name}")).note(venue.clone())
        });
        let search = Combobox::new(self.id, &self.field, choices);
        match self.on_pick {
            Some(on_pick) => search.on_change(move |symbol, window, cx| {
                log::info!("symbol search: {symbol}");
                on_pick(symbol, window, cx)
            }),
            None => search,
        }
    }
}

/// A symbol on a watchlist: its name, last price, the last session's close, and today's prices so far.
#[derive(Clone, Debug, PartialEq)]
pub struct Watch {
    pub symbol: SharedString,
    pub name: SharedString,
    pub last: f64,
    pub close: f64,
    pub day: Vec<f32>,
}

type OnSymbol = Rc<dyn Fn(&SharedString, &mut Window, &mut App)>;

/// Symbols to keep an eye on, each with its tile, today's line, its price and move; a press picks one, and the keys walk the list.
#[derive(IntoElement)]
pub struct Watchlist {
    id: ElementId,
    watches: Vec<Watch>,
    selected: Option<SharedString>,
    on_select: Option<OnSymbol>,
    red_up: bool,
}

impl Watchlist {
    pub fn new(id: impl Into<ElementId>, watches: impl IntoIterator<Item = Watch>) -> Self {
        Self {
            id: id.into(),
            watches: watches.into_iter().collect(),
            selected: None,
            on_select: None,
            red_up: false,
        }
    }

    pub fn selected(mut self, symbol: impl Into<SharedString>) -> Self {
        self.selected = Some(symbol.into());
        self
    }

    pub fn on_select(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_select = Some(Rc::new(handler));
        self
    }

    pub fn red_up(mut self) -> Self {
        self.red_up = true;
        self
    }
}

impl RenderOnce for Watchlist {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let (rise, fall) = moves(self.red_up, cx);
        let theme = cx.theme();
        let spark = theme.spark_size();
        let list = self
            .watches
            .iter()
            .fold(SelectableList::new(self.id.clone()), |list, watch| {
                let ink = if watch.last >= watch.close {
                    rise
                } else {
                    fall
                };
                let change = if self.red_up {
                    PriceChangeBadge::new(watch.close, watch.last).red_up()
                } else {
                    PriceChangeBadge::new(watch.close, watch.last)
                };
                let figures = div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(
                        Sparkline::new(watch.day.clone())
                            .color(ink)
                            .w(spark.width)
                            .h(spark.height),
                    )
                    .child(
                        tabular(div())
                            .w(theme.label_width() * 0.45)
                            .text_right()
                            .text_color(theme.colors.fg)
                            .child(price(watch.last, 2)),
                    )
                    .child(
                        div()
                            .w(theme.label_width() * 0.8)
                            .flex()
                            .justify_end()
                            .child(change),
                    );
                let item = ListItem::new(
                    (self.id.clone(), watch.symbol.clone()),
                    watch.symbol.clone(),
                )
                .description(watch.name.clone())
                .leading(
                    Avatar::new(
                        (self.id.clone(), format!("tile-{}", watch.symbol)),
                        watch.symbol.clone(),
                    )
                    .square()
                    .size(AvatarSize::Sm),
                )
                .trailing(figures);
                list.row(watch.symbol.clone(), item)
            });
        let list = list.selected(self.selected.clone());
        match self.on_select {
            Some(on_select) => list.on_change(move |keys, window, cx| {
                if let Some(symbol) = keys.first() {
                    log::info!("watchlist: {symbol}");
                    on_select(symbol, window, cx);
                }
            }),
            None => list,
        }
    }
}

type OnRules = Rc<dyn Fn(&[FilterRule], bool, &mut Window, &mut App)>;

/// Securities narrowed by rules: the rules to build above, and the table of what passes them below.
#[derive(IntoElement)]
pub struct Screener {
    id: ElementId,
    columns: Vec<Column>,
    rows: Vec<Row>,
    rules: Vec<FilterRule>,
    any: bool,
    on_rules: Option<OnRules>,
}

impl Screener {
    pub fn new(
        id: impl Into<ElementId>,
        columns: impl IntoIterator<Item = Column>,
        rows: impl IntoIterator<Item = Row>,
    ) -> Self {
        Self {
            id: id.into(),
            columns: columns.into_iter().collect(),
            rows: rows.into_iter().collect(),
            rules: Vec::new(),
            any: false,
            on_rules: None,
        }
    }

    /// The rules the owner keeps, and whether a row passes any rather than all.
    pub fn rules(mut self, rules: impl IntoIterator<Item = FilterRule>, any: bool) -> Self {
        self.rules = rules.into_iter().collect();
        self.any = any;
        self
    }

    pub fn on_rules(
        mut self,
        handler: impl Fn(&[FilterRule], bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_rules = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for Screener {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let keys = self
            .columns
            .iter()
            .map(|column| (column.key().clone(), column.title().clone()));
        let builder = FilterBuilder::new((self.id.clone(), "rules"), keys)
            .rules(self.rules.clone(), self.any);
        let builder = match self.on_rules {
            Some(on_rules) => {
                builder.on_change(move |rules, any, window, cx| on_rules(rules, any, window, cx))
            }
            None => builder,
        };
        let table = DataTable::new((self.id.clone(), "table"), self.columns)
            .rows(self.rows)
            .filters(self.rules, self.any);
        div().flex().flex_col().gap_3().child(builder).child(table)
    }
}
