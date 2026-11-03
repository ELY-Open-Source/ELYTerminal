use std::rc::Rc;

use gpui::{App, ElementId, Entity, SharedString, Window};

use crate::forms::{
    Choice, Pick, Suggestions, TextInput, active_trigger, emoji_found, handles_matching, one_word,
    replace_trigger,
};

/// A trigger of the owner's: from the query and where it runs, the rows and what a pick does.
pub(crate) type Offer<'a> = &'a dyn Fn(&str, usize, usize) -> (Vec<Choice>, Pick);

/// An emoji name needs this many letters before `:` suggests, so a colon in prose stays quiet.
const EMOJI_QUERY: usize = 2;

/// Emoji shown for a `:` query.
const EMOJI_SHOWN: usize = 6;

/// An offer of nothing: no rows, so no pick can come.
fn nothing() -> (Vec<Choice>, Pick) {
    (
        Vec::new(),
        Rc::new(|_: usize, _: &mut Window, _: &mut App| unreachable!("an empty offer has no row")),
    )
}

/// What `@` and `:` offer at the field's caret: people by handle, emoji by name; `extra` adds a trigger of the owner's, such as `/` in blocks.
pub(crate) fn offers(
    id: ElementId,
    field: &Entity<TextInput>,
    people: &[SharedString],
    extra: Option<(char, Offer)>,
    cx: &App,
) -> Suggestions {
    let input = field.read(cx);
    let (text, caret) = (input.text().to_string(), input.cursor());
    let mut marks = vec!['@', ':'];
    marks.extend(extra.as_ref().map(|(mark, _)| *mark));
    let active = active_trigger(&text, caret, &marks, one_word);
    let query = active.map_or("", |(at, mark)| &text[at + mark.len_utf8()..caret]);
    let (rows, pick): (Vec<Choice>, Pick) = match active {
        Some((at, '@')) => {
            let found = handles_matching(people, query);
            let rows = found
                .iter()
                .map(|name| Choice::new(name.clone(), format!("@{name}")))
                .collect();
            let field = field.clone();
            (
                rows,
                Rc::new(move |pick: usize, _: &mut Window, cx: &mut App| {
                    log::info!("mention: @{}", found[pick]);
                    replace_trigger(&field, at, caret, &format!("@{} ", found[pick]), cx);
                }),
            )
        }
        Some((at, ':')) if query.len() >= EMOJI_QUERY => {
            let found: Vec<&'static emojis::Emoji> = emoji_found(&query.to_lowercase())
                .into_iter()
                .take(EMOJI_SHOWN)
                .collect();
            let rows = found
                .iter()
                .map(|emoji| {
                    let name = emoji.shortcode().unwrap_or(emoji.name());
                    Choice::new(name, format!("{}  {name}", emoji.as_str()))
                })
                .collect();
            let field = field.clone();
            (
                rows,
                Rc::new(move |pick: usize, _: &mut Window, cx: &mut App| {
                    log::info!("emoji: {}", found[pick].name());
                    replace_trigger(&field, at, caret, found[pick].as_str(), cx);
                }),
            )
        }
        Some((at, mark)) => match &extra {
            Some((extra_mark, build)) if *extra_mark == mark => build(query, at, caret),
            _ => nothing(),
        },
        None => nothing(),
    };
    Suggestions {
        id,
        state: field.clone(),
        trigger: active.map(|(at, _)| at),
        rows,
        pick,
    }
}
