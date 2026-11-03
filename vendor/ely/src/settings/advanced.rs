use std::rc::Rc;

use gpui::{
    App, ElementId, Entity, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window,
    div,
};

use super::section::SettingsRow;
use crate::{
    data_display::{Badge, Tone},
    feedback::Callout,
    forms::{Input, Switch, TextInput},
    primitives::{Icon, IconName, Severity},
    theme::{ActiveTheme, IconSize, TextSize},
};

/// How far along a feature is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    Stable,
    Beta,
    Experimental,
}

/// A feature flag: its key and name, what it does, how far along it is, and whether it is on.
#[derive(Clone, Debug, PartialEq)]
pub struct Flag {
    pub key: SharedString,
    pub name: SharedString,
    pub description: SharedString,
    pub stage: Stage,
    pub on: bool,
}

type OnFlag = Rc<dyn Fn(&SharedString, bool, &mut Window, &mut App)>;

/// Features to turn on or off, each with what it does and how far along it is, found by a search over their names and words.
#[derive(IntoElement)]
pub struct FeatureFlags {
    id: ElementId,
    flags: Vec<Flag>,
    search: Entity<TextInput>,
    on_toggle: OnFlag,
}

impl FeatureFlags {
    /// `search` is the find field's text, which the owner keeps.
    pub fn new(
        id: impl Into<ElementId>,
        flags: impl IntoIterator<Item = Flag>,
        search: &Entity<TextInput>,
        on_toggle: impl Fn(&SharedString, bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            flags: flags.into_iter().collect(),
            search: search.clone(),
            on_toggle: Rc::new(on_toggle),
        }
    }
}

/// The flags whose name or description holds every word of `query`.
fn kept_flags<'a>(flags: &'a [Flag], query: &str) -> Vec<&'a Flag> {
    let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
    flags
        .iter()
        .filter(|flag| {
            let (name, about) = (flag.name.to_lowercase(), flag.description.to_lowercase());
            words
                .iter()
                .all(|word| name.contains(word) || about.contains(word))
        })
        .collect()
}

impl RenderOnce for FeatureFlags {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let on_toggle = self.on_toggle;
        let query = self.search.read(cx).text().to_string();
        let theme = cx.theme();
        let rows = kept_flags(&self.flags, &query).into_iter().map(|flag| {
            let (key, on_toggle) = (flag.key.clone(), on_toggle.clone());
            let (words, tone) = match flag.stage {
                Stage::Stable => ("Stable", Tone::Neutral),
                Stage::Beta => ("Beta", Tone::Info),
                Stage::Experimental => ("Experimental", Tone::Warning),
            };
            SettingsRow::new(flag.name.clone())
                .description(flag.description.clone())
                .control(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(Badge::new(words).tone(tone))
                        .child(
                            Switch::new((self.id.clone(), format!("flag-{}", flag.key)), flag.on)
                                .on_change(move |on, window, cx| {
                                    log::info!("feature flag: {key} {on}");
                                    on_toggle(&key, on, window, cx);
                                }),
                        ),
                )
        });
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(Input::new(&self.search).prefix(Icon::new(IconName::Search).size(IconSize::Sm)))
            .children(rows)
            .child(
                div()
                    .text_size(theme.text_size(TextSize::Xs))
                    .text_color(theme.colors.fg_subtle)
                    .child("Experimental features may change or go away."),
            )
    }
}

type OnBool = Rc<dyn Fn(bool, &mut Window, &mut App)>;

/// Developer mode's switch, with a warning of what it shows while on.
#[derive(IntoElement)]
pub struct DeveloperModeToggle {
    id: ElementId,
    on: bool,
    on_change: OnBool,
}

impl DeveloperModeToggle {
    pub fn new(
        id: impl Into<ElementId>,
        on: bool,
        on_change: impl Fn(bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            on,
            on_change: Rc::new(on_change),
        }
    }
}

impl RenderOnce for DeveloperModeToggle {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let on_change = self.on_change;
        let on_change = on_change;
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                SettingsRow::new("Developer mode")
                    .description("Shows the inspector, logs and debug menus.")
                    .control(
                        Switch::new(self.id, self.on).on_change(move |on, window, cx| {
                            log::info!("developer mode: {on}");
                            on_change(on, window, cx);
                        }),
                    ),
            )
            .children(self.on.then(|| {
                Callout::new(Severity::Warning)
                    .title("Developer mode is on")
                    .child("Internal tools show, and some of them can change what the app stores.")
            }))
    }
}

#[cfg(test)]
mod tests {
    use super::{Flag, Stage, kept_flags};

    #[test]
    fn a_flag_answers_in_its_name_or_what_it_does() {
        let flag = |key: &str, name: &str, description: &str| Flag {
            key: key.to_string().into(),
            name: name.to_string().into(),
            description: description.to_string().into(),
            stage: Stage::Beta,
            on: false,
        };
        let all = [
            flag("tabs", "Vertical tabs", "Tabs down the side."),
            flag("sync", "Settings sync", "The same settings everywhere."),
        ];
        let keys = |query| {
            kept_flags(&all, query)
                .iter()
                .map(|each| each.key.to_string())
                .collect::<Vec<_>>()
        };
        assert_eq!(keys("side tabs"), ["tabs"]);
        assert_eq!(keys("everywhere"), ["sync"]);
        assert_eq!(keys(""), ["tabs", "sync"]);
    }
}
