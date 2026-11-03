use gpui::{
    App, Entity, FontWeight, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window,
    div,
};

use crate::{
    forms::{Input, TextInput},
    primitives::{Icon, IconName},
    theme::{ActiveTheme, IconSize, TextSize},
    typography::{Ellipsis, KbdCombo},
};

/// A shortcut: the group it sits in, what it does, and its keys as written, such as "cmd-shift-p".
#[derive(Clone, Debug, PartialEq)]
pub struct Shortcut {
    pub group: SharedString,
    pub name: SharedString,
    pub keys: SharedString,
}

/// Shortcuts in their groups, each with what it does and its keys drawn as caps; a search keeps those whose name or keys hold its words. Editing them is `editor::KeybindingsEditor`.
#[derive(IntoElement)]
pub struct KeyboardShortcutsList {
    shortcuts: Vec<Shortcut>,
    search: Entity<TextInput>,
}

impl KeyboardShortcutsList {
    /// `search` is the find field's text, which the owner keeps.
    pub fn new(shortcuts: impl IntoIterator<Item = Shortcut>, search: &Entity<TextInput>) -> Self {
        Self {
            shortcuts: shortcuts.into_iter().collect(),
            search: search.clone(),
        }
    }
}

/// The shortcuts whose name or keys hold every word of `query`.
pub(crate) fn kept<'a>(shortcuts: &'a [Shortcut], query: &str) -> Vec<&'a Shortcut> {
    let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
    shortcuts
        .iter()
        .filter(|shortcut| {
            let (name, keys) = (shortcut.name.to_lowercase(), shortcut.keys.to_lowercase());
            words
                .iter()
                .all(|word| name.contains(word) || keys.contains(word))
        })
        .collect()
}

impl RenderOnce for KeyboardShortcutsList {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let query = self.search.read(cx).text().to_string();
        let theme = cx.theme();
        let kept = kept(&self.shortcuts, &query);
        let mut groups: Vec<SharedString> = Vec::new();
        for shortcut in &kept {
            if !groups.contains(&shortcut.group) {
                groups.push(shortcut.group.clone());
            }
        }
        let body =
            groups.into_iter().map(|group| {
                div()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .pt_3()
                            .pb_1()
                            .text_size(theme.text_size(TextSize::Xs))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(theme.colors.fg_subtle)
                            .child(group.clone()),
                    )
                    .children(kept.iter().filter(|shortcut| shortcut.group == group).map(
                        |shortcut| {
                            div()
                                .flex()
                                .items_center()
                                .justify_between()
                                .gap_2()
                                .py_1p5()
                                .border_b_1()
                                .border_color(theme.colors.border)
                                .text_size(theme.text_size(TextSize::Sm))
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .child(Ellipsis::new(shortcut.name.clone())),
                                )
                                .child(div().flex_none().child(KbdCombo::new(&shortcut.keys)))
                        },
                    ))
            });
        div()
            .flex()
            .flex_col()
            .child(Input::new(&self.search).prefix(Icon::new(IconName::Search).size(IconSize::Sm)))
            .children(body)
    }
}

#[cfg(test)]
mod tests {
    use super::{Shortcut, kept};

    #[test]
    fn every_word_answers_in_the_name_or_the_keys() {
        let shortcut = |name: &str, keys: &str| Shortcut {
            group: "General".into(),
            name: name.to_string().into(),
            keys: keys.to_string().into(),
        };
        let all = [shortcut("Undo", "cmd-z"), shortcut("Find", "cmd-f")];
        let names = |query| {
            kept(&all, query)
                .iter()
                .map(|each| each.name.to_string())
                .collect::<Vec<_>>()
        };
        assert_eq!(names("cmd z"), ["Undo"], "each word may answer in the keys");
        assert_eq!(
            names("find cmd"),
            ["Find"],
            "or across the name and the keys"
        );
        assert_eq!(names(""), ["Undo", "Find"]);
    }
}
