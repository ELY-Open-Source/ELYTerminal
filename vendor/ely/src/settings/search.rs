use std::rc::Rc;

use gpui::{
    App, ElementId, Entity, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window,
    div,
};

use crate::{
    forms::{Input, TextInput},
    lists::{ListItem, SelectableList},
    primitives::{Icon, IconName},
    theme::{ActiveTheme, IconSize, TextSize},
};

/// A setting a search can find: its key, the section it lives in, its name, and other words it answers to.
#[derive(Clone, Debug, PartialEq)]
pub struct SettingEntry {
    pub key: SharedString,
    pub section: SharedString,
    pub name: SharedString,
    pub words: Vec<SharedString>,
}

/// The settings whose name, section or words hold every word of `query`, in order.
pub fn matching<'a>(entries: &'a [SettingEntry], query: &str) -> Vec<&'a SettingEntry> {
    let terms: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
    entries
        .iter()
        .filter(|entry| {
            let haystack = std::iter::once(&entry.name)
                .chain(std::iter::once(&entry.section))
                .chain(&entry.words)
                .map(|word| word.to_lowercase())
                .collect::<Vec<_>>()
                .join(" ");
            terms.iter().all(|term| haystack.contains(term.as_str()))
        })
        .collect()
}

type OnKey = Rc<dyn Fn(&SharedString, &mut Window, &mut App)>;

/// A search across every setting: while it holds words, the settings that answer to them, each under its section; Enter or a double press goes to one.
#[derive(IntoElement)]
pub struct SettingsSearch {
    id: ElementId,
    entries: Vec<SettingEntry>,
    search: Entity<TextInput>,
    on_pick: OnKey,
}

impl SettingsSearch {
    /// `search` is the field's text, which the owner keeps.
    pub fn new(
        id: impl Into<ElementId>,
        entries: impl IntoIterator<Item = SettingEntry>,
        search: &Entity<TextInput>,
        on_pick: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            entries: entries.into_iter().collect(),
            search: search.clone(),
            on_pick: Rc::new(on_pick),
        }
    }
}

impl RenderOnce for SettingsSearch {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let on_pick = self.on_pick;
        let query = self.search.read(cx).text().trim().to_string();
        let theme = cx.theme();
        let found = if query.is_empty() {
            Vec::new()
        } else {
            matching(&self.entries, &query)
        };
        let none = !query.is_empty() && found.is_empty();
        let list = found.iter().fold(
            SelectableList::new((self.id.clone(), "found")),
            |list, entry| {
                list.row(
                    entry.key.clone(),
                    ListItem::new(
                        (self.id.clone(), format!("found-{}", entry.key)),
                        entry.name.clone(),
                    )
                    .description(entry.section.clone()),
                )
            },
        );
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(Input::new(&self.search).prefix(Icon::new(IconName::Search).size(IconSize::Sm)))
            .children((!found.is_empty()).then(|| {
                list.on_activate(move |key, window, cx| {
                    log::info!("settings search: go to {key}");
                    on_pick(key, window, cx);
                })
            }))
            .children(none.then(|| {
                div()
                    .text_size(theme.text_size(TextSize::Sm))
                    .text_color(theme.colors.fg_subtle)
                    .child(format!("No setting answers to \"{query}\"."))
            }))
    }
}

#[cfg(test)]
mod tests {
    use super::{SettingEntry, matching};

    #[test]
    fn every_word_must_answer_in_the_name_the_section_or_the_words() {
        let entry = |key: &str, section: &str, name: &str, words: &[&str]| SettingEntry {
            key: key.to_string().into(),
            section: section.to_string().into(),
            name: name.to_string().into(),
            words: words.iter().map(|word| word.to_string().into()).collect(),
        };
        let entries = [
            entry("theme", "Appearance", "Theme", &["dark", "light"]),
            entry("proxy", "Network", "Proxy", &["http", "socks"]),
            entry("size", "Appearance", "Font size", &["text", "zoom"]),
        ];
        let keys = |query: &str| {
            matching(&entries, query)
                .iter()
                .map(|entry| entry.key.to_string())
                .collect::<Vec<_>>()
        };
        assert_eq!(keys("DARK"), ["theme"]);
        assert_eq!(keys("appearance zoom"), ["size"]);
        assert_eq!(keys("network dark"), Vec::<String>::new());
    }
}
