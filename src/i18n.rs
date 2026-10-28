//! The interface's words, English and Simplified Chinese, via `I18n`.

use ely_gpui_component::i18n::I18n;
use gpui::App;

use crate::{
    menus,
    settings::{Language, SettingsStore},
};

mod en;
mod zh;

/// Every catalog, `language` shown; each must hold English's keys.
pub fn catalogs(language: Language) -> I18n {
    I18n::new(language.tag())
        .catalog(Language::English.tag(), en::MESSAGES)
        .catalog(Language::SimplifiedChinese.tag(), zh::MESSAGES)
}

/// Speaks the settings' language everywhere, and follows its changes.
pub fn init(cx: &mut App) {
    let mut spoken = SettingsStore::get(cx).language;
    log::info!("i18n: the interface speaks {}", spoken.tag());
    cx.set_global(catalogs(spoken));
    cx.observe_global::<SettingsStore>(move |cx| {
        let language = SettingsStore::get(cx).language;
        if language != spoken {
            spoken = language;
            I18n::set_locale(language.tag(), cx);
            cx.set_menus(menus::menus(cx));
        }
    })
    .detach();
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeSet, fs, path::Path};

    use super::*;

    /// The `{name}`s a message fills.
    fn names(message: &str) -> BTreeSet<&str> {
        message
            .split('{')
            .skip(1)
            .map(|rest| rest.split_once('}').expect("a { ends").0)
            .collect()
    }

    /// The keys the source at `path` names, the catalogs aside.
    fn named(path: &Path, keys: &mut BTreeSet<String>) {
        if path.ends_with("i18n") || path.ends_with("i18n.rs") {
            return;
        }
        if path.is_dir() {
            for entry in fs::read_dir(path).expect("a source folder reads") {
                named(&entry.expect("an entry reads").path(), keys);
            }
            return;
        }
        let source = fs::read_to_string(path).expect("a source file reads");
        for rest in source.split("\"et.").skip(1) {
            let (key, _) = rest.split_once('"').expect("a key ends");
            keys.insert(format!("et.{key}"));
        }
    }

    #[test]
    fn each_catalog_holds_english_s_keys_and_fills_the_same_names() {
        for language in [Language::English, Language::SimplifiedChinese] {
            assert_eq!(catalogs(language).locale().tag, language.tag());
        }
        for ((key, english), (other, chinese)) in en::MESSAGES.iter().zip(zh::MESSAGES) {
            assert_eq!(key, other, "both catalogs list their keys in one order");
            assert_eq!(names(english), names(chinese), "{key}");
        }
        let chinese = catalogs(Language::SimplifiedChinese);
        assert_eq!(chinese.text("et.session.new", &[]).as_ref(), "新建会话");
    }

    #[test]
    fn every_key_the_source_names_is_in_the_catalogs_and_every_key_is_named() {
        let mut keys = BTreeSet::new();
        named(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
            &mut keys,
        );
        let listed: BTreeSet<String> = en::MESSAGES
            .iter()
            .map(|(key, _)| key.to_string())
            .collect();
        assert_eq!(keys, listed);
    }
}
