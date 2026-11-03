use std::rc::Rc;

use gpui::{
    App, ElementId, Entity, FontWeight, IntoElement, ParentElement, RenderOnce, SharedString,
    Styled, Window, div,
};

use crate::{
    data_display::{Badge, Tone},
    feedback::EmptyState,
    forms::{Input, TextInput},
    lists::{ListItem, SelectableList},
    primitives::{Icon, IconName},
    tables::Table,
    theme::{ActiveTheme, IconSize, TextSize},
    typography::{Ellipsis, format},
};

/// What a Redis key holds.
#[derive(Clone, Debug, PartialEq)]
pub enum RedisValue {
    Text(SharedString),
    Hash(Vec<(SharedString, SharedString)>),
    List(Vec<SharedString>),
    Set(Vec<SharedString>),
    Sorted(Vec<(SharedString, f64)>),
}

impl RedisValue {
    /// The type's name as Redis gives it.
    pub fn kind(&self) -> &'static str {
        match self {
            RedisValue::Text(_) => "string",
            RedisValue::Hash(_) => "hash",
            RedisValue::List(_) => "list",
            RedisValue::Set(_) => "set",
            RedisValue::Sorted(_) => "zset",
        }
    }

    fn tone(&self) -> Tone {
        match self {
            RedisValue::Text(_) => Tone::Neutral,
            RedisValue::Hash(_) => Tone::Accent,
            RedisValue::List(_) => Tone::Info,
            RedisValue::Set(_) => Tone::Success,
            RedisValue::Sorted(_) => Tone::Warning,
        }
    }
}

/// A key, what it holds, and the seconds it has left, or none when it keeps.
#[derive(Clone, Debug, PartialEq)]
pub struct RedisKey {
    pub key: SharedString,
    pub value: RedisValue,
    pub ttl: Option<u64>,
}

/// Whether `key` matches a Redis glob: `*` for any run, `?` for one character.
pub(crate) fn matches(pattern: &str, key: &str) -> bool {
    fn go(pattern: &[char], key: &[char]) -> bool {
        match (pattern.first(), key.first()) {
            (None, None) => true,
            (Some('*'), _) => go(&pattern[1..], key) || (!key.is_empty() && go(pattern, &key[1..])),
            (Some('?'), Some(_)) => go(&pattern[1..], &key[1..]),
            (Some(p), Some(k)) if p == k => go(&pattern[1..], &key[1..]),
            _ => false,
        }
    }
    let (pattern, key): (Vec<char>, Vec<char>) = (pattern.chars().collect(), key.chars().collect());
    go(&pattern, &key)
}

type OnKey = Rc<dyn Fn(&SharedString, &mut Window, &mut App)>;

/// Keys found by a glob, each with its type and time left; the one selected shows its value laid out for its type: text, a hash's fields, a list's items by index, a set's members, a sorted set's members by score.
#[derive(IntoElement)]
pub struct RedisKeyBrowser {
    id: ElementId,
    keys: Vec<RedisKey>,
    pattern: Entity<TextInput>,
    selected: Option<SharedString>,
    on_select: Option<OnKey>,
}

impl RedisKeyBrowser {
    /// `pattern` is the glob field's text, which the owner keeps; empty finds every key.
    pub fn new(
        id: impl Into<ElementId>,
        keys: impl IntoIterator<Item = RedisKey>,
        pattern: &Entity<TextInput>,
    ) -> Self {
        Self {
            id: id.into(),
            keys: keys.into_iter().collect(),
            pattern: pattern.clone(),
            selected: None,
            on_select: None,
        }
    }

    pub fn selected(mut self, key: Option<impl Into<SharedString>>) -> Self {
        self.selected = key.map(Into::into);
        self
    }

    pub fn on_select(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_select = Some(Rc::new(handler));
        self
    }
}

fn value(key: &RedisKey, cx: &App) -> gpui::AnyElement {
    let theme = cx.theme();
    let text = |words: &SharedString| words.clone();
    match &key.value {
        RedisValue::Text(words) => div()
            .p_3()
            .rounded(theme.radius(crate::theme::Radius::Md))
            .bg(theme.colors.sunken)
            .font_family(theme.mono_family.clone())
            .text_size(theme.text_size(TextSize::Sm))
            .child(words.clone())
            .into_any_element(),
        RedisValue::Hash(fields) => fields
            .iter()
            .fold(Table::new(["Field", "Value"]), |table, (field, value)| {
                table.row([text(field), text(value)])
            })
            .into_any_element(),
        RedisValue::List(items) => items
            .iter()
            .enumerate()
            .fold(Table::new(["Index", "Item"]), |table, (ix, item)| {
                table.row([SharedString::from(ix.to_string()), text(item)])
            })
            .into_any_element(),
        RedisValue::Set(members) => members
            .iter()
            .fold(Table::new(["Member"]), |table, member| {
                table.row([text(member)])
            })
            .into_any_element(),
        RedisValue::Sorted(members) => members
            .iter()
            .fold(Table::new(["Member", "Score"]), |table, (member, score)| {
                table.row([
                    text(member),
                    SharedString::from(format::number(*score, 2, format::Separators::EN)),
                ])
            })
            .into_any_element(),
    }
}

impl RenderOnce for RedisKeyBrowser {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id;
        let pattern = self.pattern.read(cx).text().trim().to_string();
        let glob = if pattern.is_empty() {
            "*".to_string()
        } else {
            pattern
        };
        let found: Vec<&RedisKey> = self
            .keys
            .iter()
            .filter(|key| matches(&glob, &key.key))
            .collect();
        let theme = cx.theme();
        let muted = theme.colors.fg_muted;
        let list = found.iter().fold(
            SelectableList::new((id.clone(), "keys")).selected(self.selected.clone()),
            |list, key| {
                let ttl = key.ttl.map_or("No expiry".to_string(), |seconds| {
                    format!(
                        "{} left",
                        format::duration(seconds, format::DurationStyle::Compact)
                    )
                });
                list.row(
                    key.key.clone(),
                    ListItem::new((id.clone(), format!("key-{}", key.key)), key.key.clone())
                        .description(ttl)
                        .leading(Icon::new(IconName::Key).size(IconSize::Sm).color(muted))
                        .trailing(Badge::new(key.value.kind()).tone(key.value.tone())),
                )
            },
        );
        let on_select = self.on_select;
        let list = list.on_change(move |keys, window, cx| {
            if let (Some(key), Some(on_select)) = (keys.first(), &on_select) {
                on_select(key, window, cx);
            }
        });
        let shown = self
            .selected
            .as_ref()
            .and_then(|selected| self.keys.iter().find(|key| key.key == *selected));
        let detail = match shown {
            Some(key) => div()
                .flex()
                .flex_col()
                .gap_3()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(Ellipsis::new(key.key.clone())),
                        )
                        .child(
                            div()
                                .flex_none()
                                .child(Badge::new(key.value.kind()).tone(key.value.tone())),
                        ),
                )
                .child(value(key, cx))
                .into_any_element(),
            None => EmptyState::new((id.clone(), "none"), IconName::Key, "No key selected")
                .body("Pick a key to see what it holds.")
                .into_any_element(),
        };
        div()
            .flex()
            .flex_wrap()
            .gap_4()
            .child(
                div()
                    .flex_1()
                    .min_w(theme.label_width())
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        Input::new(&self.pattern)
                            .prefix(Icon::new(IconName::Search).size(IconSize::Sm)),
                    )
                    .child(format!("{} of {} keys", found.len(), self.keys.len()))
                    .child(list),
            )
            .child(div().flex_1().min_w(theme.label_width()).child(detail))
    }
}

#[cfg(test)]
mod tests {
    use super::matches;

    #[test]
    fn globs_match_runs_and_single_characters() {
        assert!(matches("user:*", "user:42"));
        assert!(matches("user:*:name", "user:42:name"));
        assert!(!matches("user:*:name", "user:42:mail"));
        assert!(matches("h?llo", "hello"));
        assert!(!matches("h?llo", "hllo"));
        assert!(matches("*", ""));
    }
}
