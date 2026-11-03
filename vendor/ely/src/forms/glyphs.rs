use std::rc::Rc;

use emojis::Group;
use gpui::{App, ElementId, IntoElement, RenderOnce, SharedString, Window};

use super::picks::{Grid, Pick, finder, grid, rows};
use crate::{
    primitives::{Icon, IconName},
    theme::{ActiveTheme, ControlSize, IconSize, TextSize},
    typography::Emoji,
};

/// Glyphs across an emoji or icon grid.
const COLUMNS: usize = 8;

type OnIcon = Rc<dyn Fn(IconName, &mut Window, &mut App)>;
type OnEmoji = Rc<dyn Fn(&str, &mut Window, &mut App)>;

/// A searchable grid of the library's icons.
#[derive(IntoElement)]
pub struct IconPicker {
    id: ElementId,
    selected: Option<IconName>,
    on_change: Option<OnIcon>,
}

impl IconPicker {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            selected: None,
            on_change: None,
        }
    }

    pub fn selected(mut self, icon: IconName) -> Self {
        self.selected = Some(icon);
        self
    }

    pub fn on_change(
        mut self,
        handler: impl Fn(IconName, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for IconPicker {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let finder = finder(&self.id, "Search icons", window, cx);
        let query = finder.read(cx).query.clone();
        let found: Rc<Vec<IconName>> = Rc::new(
            IconName::ALL
                .iter()
                .copied()
                .filter(|icon| icon.name().replace('-', " ").contains(&query))
                .collect(),
        );
        let (painted, picked) = (found.clone(), found.clone());
        let (id, on_change) = (self.id.clone(), self.on_change);
        let pick: Pick = Rc::new(move |ix, window, cx| {
            let icon = picked[ix];
            log::info!("icon picker {id:?}: {}", icon.name());
            if let Some(on_change) = &on_change {
                on_change(icon, window, cx);
            }
        });
        let fg = cx.theme().colors.fg;
        grid(
            Grid {
                id: self.id,
                names: Rc::new(found.iter().map(|icon| icon.name().into()).collect()),
                rows: Rc::new(rows([(None, found.len())], COLUMNS)),
                selected: self
                    .selected
                    .and_then(|icon| found.iter().position(|f| *f == icon)),
                draw: Rc::new(move |ix, _| {
                    Icon::new(painted[ix])
                        .size(IconSize::Md)
                        .color(fg)
                        .into_any_element()
                }),
                pick,
                none: "No icons match",
                columns: COLUMNS,
                cell: cx.theme().control_height(ControlSize::Lg),
            },
            &finder,
            window,
            cx,
        )
    }
}

/// Emoji whose name or shortcode holds `query`, a lowercase string; all for an empty one.
pub(crate) fn emoji_found(query: &str) -> Vec<&'static emojis::Emoji> {
    emojis::iter()
        .filter(|emoji| {
            query.is_empty()
                || emoji.name().to_lowercase().contains(query)
                || emoji.shortcodes().any(|code| code.contains(query))
        })
        .collect()
}

fn title(group: Group) -> &'static str {
    match group {
        Group::SmileysAndEmotion => "Smileys & Emotion",
        Group::PeopleAndBody => "People & Body",
        Group::AnimalsAndNature => "Animals & Nature",
        Group::FoodAndDrink => "Food & Drink",
        Group::TravelAndPlaces => "Travel & Places",
        Group::Activities => "Activities",
        Group::Objects => "Objects",
        Group::Symbols => "Symbols",
        Group::Flags => "Flags",
    }
}

/// A searchable grid of emoji, grouped as Unicode groups them.
#[derive(IntoElement)]
pub struct EmojiPicker {
    id: ElementId,
    selected: Option<SharedString>,
    on_change: Option<OnEmoji>,
}

impl EmojiPicker {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            selected: None,
            on_change: None,
        }
    }

    pub fn selected(mut self, emoji: impl Into<SharedString>) -> Self {
        self.selected = Some(emoji.into());
        self
    }

    pub fn on_change(mut self, handler: impl Fn(&str, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for EmojiPicker {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let finder = finder(&self.id, "Search emoji", window, cx);
        let query = finder.read(cx).query.clone();
        let found = emoji_found(&query);
        let layout = if query.is_empty() {
            rows(
                Group::iter().map(|group| (Some(title(group).into()), group.emojis().count())),
                COLUMNS,
            )
        } else {
            rows([(None, found.len())], COLUMNS)
        };
        let found = Rc::new(found);
        let (painted, picked) = (found.clone(), found.clone());
        let (id, on_change) = (self.id.clone(), self.on_change);
        let pick: Pick = Rc::new(move |ix, window, cx| {
            let emoji = picked[ix];
            log::info!("emoji picker {id:?}: {}", emoji.name());
            if let Some(on_change) = &on_change {
                on_change(emoji.as_str(), window, cx);
            }
        });
        grid(
            Grid {
                id: self.id,
                names: Rc::new(found.iter().map(|emoji| emoji.name().into()).collect()),
                rows: Rc::new(layout),
                selected: self.selected.and_then(|glyph| {
                    found
                        .iter()
                        .position(|emoji| emoji.as_str() == glyph.as_ref())
                }),
                draw: Rc::new(move |ix, _| {
                    Emoji::new(painted[ix].as_str())
                        .size(TextSize::Lg)
                        .into_any_element()
                }),
                pick,
                none: "No emoji match",
                columns: COLUMNS,
                cell: cx.theme().control_height(ControlSize::Lg),
            },
            &finder,
            window,
            cx,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::emoji_found;

    #[test]
    fn emoji_are_found_whatever_the_case_of_their_names() {
        let glyphs = |query| {
            emoji_found(query)
                .into_iter()
                .map(|emoji| emoji.as_str())
                .collect::<Vec<_>>()
        };
        assert!(glyphs("united states").contains(&"🇺🇸"));
        assert!(glyphs("sparkles").contains(&"✨"));
        assert_eq!(glyphs("").len(), emojis::iter().count());
    }
}
