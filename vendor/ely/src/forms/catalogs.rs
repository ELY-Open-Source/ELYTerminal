use std::rc::Rc;

use gpui::{
    App, ElementId, Entity, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window,
    div, prelude::*,
};
use iso_currency::{Currency, IntoEnumIterator};
use isocountry::CountryCode;

use super::{Choice, Combobox, TextInput, options::OnValue};
use crate::{
    i18n,
    theme::{ActiveTheme, TextSize},
};

/// The flag emoji of a two-letter region code.
pub(crate) fn flag(region: &str) -> String {
    region
        .chars()
        .map(|letter| {
            assert!(letter.is_ascii_uppercase(), "region code {region:?}");
            char::from_u32(0x1F1E6 + (letter as u32 - 'A' as u32)).expect("a regional indicator")
        })
        .collect()
}

/// `text` with its first letter capitalized, as list items read.
fn titled(text: &str) -> String {
    let mut chars = text.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(chars).collect()
    })
}

fn by_label(mut choices: Vec<Choice>) -> Vec<Choice> {
    choices.sort_by_key(|choice| choice.label.to_lowercase());
    choices
}

/// ISO 639-1 languages under their own names, with the English name and code beside.
fn languages() -> Vec<Choice> {
    let choices = isolang::languages()
        .filter_map(|language| {
            let (code, english) = (language.to_639_1()?, language.to_name());
            Some(match language.to_autonym() {
                Some(own) => Choice::new(code, titled(own)).note(format!("{english} · {code}")),
                None => Choice::new(code, english).note(code),
            })
        })
        .collect();
    by_label(choices)
}

impl Combobox {
    /// ISO 639-1 languages under their own names; a name in either tongue or the code finds one.
    pub fn languages(id: impl Into<ElementId>, state: &Entity<TextInput>) -> Self {
        Self::new(id, state, languages())
    }

    /// ISO 3166-1 countries and territories, flagged, with their codes.
    pub fn countries(id: impl Into<ElementId>, state: &Entity<TextInput>) -> Self {
        let mut named: Vec<(&str, &str)> = CountryCode::iter()
            .map(|country| (country.name(), country.alpha2()))
            .collect();
        named.sort_by_key(|(name, _)| name.to_lowercase());
        let choices = named
            .into_iter()
            .map(|(name, code)| Choice::new(code, format!("{} {name}", flag(code))).note(code));
        Self::new(id, state, choices)
    }

    /// ISO 4217 currencies in use, without funds, metals or retired codes.
    pub fn currencies(id: impl Into<ElementId>, state: &Entity<TextInput>) -> Self {
        let choices = Currency::iter()
            .filter(|currency| {
                !currency.is_fund() && !currency.is_special() && currency.is_superseded().is_none()
            })
            .map(|currency| {
                Choice::new(currency.code(), currency.name().to_string()).note(currency.code())
            })
            .collect();
        Self::new(id, state, by_label(choices))
    }
}

/// The system's font families in a combobox, and a line set in the chosen one.
#[derive(IntoElement)]
pub struct FontPicker {
    id: ElementId,
    state: Entity<TextInput>,
    selected: Option<SharedString>,
    on_change: Option<OnValue>,
    monospace: bool,
}

impl FontPicker {
    pub fn new(id: impl Into<ElementId>, state: &Entity<TextInput>) -> Self {
        Self {
            id: id.into(),
            state: state.clone(),
            selected: None,
            on_change: None,
            monospace: false,
        }
    }

    /// Offers only families the system calls monospaced.
    pub fn monospace(mut self, monospace: bool) -> Self {
        self.monospace = monospace;
        self
    }

    pub fn selected(mut self, family: impl Into<SharedString>) -> Self {
        self.selected = Some(family.into());
        self
    }

    pub fn on_change(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for FontPicker {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let cache = if self.monospace { "families-mono" } else { "families" };
        let families = window.use_keyed_state((self.id.clone(), cache), cx, |_, cx| {
            let found: Vec<Choice> = cx
                .text_system()
                .all_font_names()
                .into_iter()
                .filter(|family| !family.starts_with('.'))
                .filter(|family| !self.monospace || cx.text_system().is_monospaced(family))
                .map(|family| Choice::new(family.clone(), family))
                .collect();
            log::info!("font picker: {} families", found.len());
            found
        });
        let theme = cx.theme();
        let sample = i18n::text(cx, "theme.size.sample", &[]);
        let mut combobox = Combobox::new(
            self.id.clone(),
            &self.state,
            families.read(cx).iter().cloned(),
        );
        if let Some(family) = &self.selected {
            combobox = combobox.selected(family.clone());
        }
        if let Some(on_change) = self.on_change {
            combobox = combobox.on_change(move |family, window, cx| on_change(family, window, cx));
        }
        div().flex().flex_col().gap_2().child(combobox).when_some(
            self.selected,
            |picker, family| {
                picker.child(
                    div()
                        .font_family(family)
                        .text_size(theme.text_size(TextSize::Lg))
                        .text_color(theme.colors.fg_muted)
                        .child(sample),
                )
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{flag, languages, titled};
    use crate::forms::combobox::matching;

    #[test]
    fn languages_answer_to_their_code_and_english_name() {
        let found = |query| {
            matching(&languages(), query)
                .into_iter()
                .map(|choice| choice.value)
                .collect::<Vec<_>>()
        };
        assert!(found("zh").iter().any(|code| code == "zh"));
        assert!(found("chinese").iter().any(|code| code == "zh"));
        assert!(found("中文").iter().any(|code| code == "zh"));
        assert!(found("aymara").iter().any(|code| code == "ay"));
    }

    #[test]
    fn flags_come_from_region_letters() {
        assert_eq!(flag("CN"), "🇨🇳");
        assert_eq!(flag("US"), "🇺🇸");
    }

    #[test]
    fn titles_capitalize_the_first_letter_only() {
        assert_eq!(titled("français"), "Français");
        assert_eq!(titled("русский"), "Русский");
        assert_eq!(titled("中文"), "中文");
        assert_eq!(titled(""), "");
    }
}
