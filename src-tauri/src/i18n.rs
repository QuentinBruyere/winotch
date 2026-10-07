//! Translations (ADR-0012): one flat JSON file per language in `locales/`,
//! shared with the front. `t!("key")` gives the text in the current
//! language, `t!("key", name = value)` fills its `{name}` placeholders.

use std::collections::HashMap;
use std::fmt::Display;
use std::sync::{LazyLock, RwLock};

/// Languages with a translation: code and own name, as the settings list them
/// (in this order).
pub const LANGUAGES: [(&str, &str); 5] = [
    ("de", "Deutsch"),
    ("en", "English"),
    ("es", "Español"),
    ("fr", "Français"),
    ("ja", "日本語"),
];

/// For a system language without translation, and for a missing key.
pub const FALLBACK: &str = "en";

/// English first: the reference of the tests.
const BUILT_IN: [(&str, &str); 5] = [
    ("en", include_str!("../../locales/en.json")),
    ("de", include_str!("../../locales/de.json")),
    ("es", include_str!("../../locales/es.json")),
    ("fr", include_str!("../../locales/fr.json")),
    ("ja", include_str!("../../locales/ja.json")),
];

type Catalog = HashMap<String, String>;

struct State {
    language: &'static str,
    catalogs: HashMap<&'static str, Catalog>,
}

static STATE: LazyLock<RwLock<State>> = LazyLock::new(|| {
    let mut catalogs = HashMap::new();
    for (language, json) in BUILT_IN {
        catalogs.insert(language, parse(json).expect("built-in locales are valid"));
    }
    RwLock::new(State {
        language: FALLBACK,
        catalogs,
    })
});

/// Gives the text of `key` in the current language, see the module docs.
#[macro_export]
macro_rules! t {
    ($key:expr) => {
        $crate::i18n::translate($key, &[])
    };
    ($key:expr, $($name:ident = $value:expr),+ $(,)?) => {
        $crate::i18n::translate(
            $key,
            &[$((stringify!($name), &$value as &dyn ::std::fmt::Display)),+],
        )
    };
}

fn parse(json: &str) -> Result<Catalog, serde_json::Error> {
    serde_json::from_str(json)
}

/// Adds a module's own translations (`Module::locales`), for a language
/// winotch knows.
pub fn register(language: &str, json: &str) {
    let Some(code) = known(language) else {
        log::warn!("translations for unknown language {language} ignored");
        return;
    };
    match parse(json) {
        Ok(entries) => STATE
            .write()
            .unwrap()
            .catalogs
            .entry(code)
            .or_default()
            .extend(entries),
        Err(e) => log::warn!("invalid {language} translations: {e}"),
    }
}

/// The language to show: the chosen one, else the system's, if translated;
/// else English.
pub fn resolve(chosen: Option<&str>) -> &'static str {
    match chosen {
        Some(code) => pick(Some(code)),
        None => pick(sys_locale::get_locale().as_deref()),
    }
}

/// From a language tag ("fr-FR", "en_US", "fr"), the translated language.
fn pick(tag: Option<&str>) -> &'static str {
    tag.and_then(|tag| known(tag.split(['-', '_']).next().unwrap_or(tag)))
        .unwrap_or(FALLBACK)
}

fn known(code: &str) -> Option<&'static str> {
    LANGUAGES
        .iter()
        .find(|(c, _)| c.eq_ignore_ascii_case(code))
        .map(|(c, _)| *c)
}

pub fn set_language(language: &'static str) {
    STATE.write().unwrap().language = language;
}

pub fn language() -> &'static str {
    STATE.read().unwrap().language
}

/// What `t!` calls. A key missing from the language falls back on English,
/// then on the key itself: never a crash.
pub fn translate(key: &str, args: &[(&str, &dyn Display)]) -> String {
    translate_in(language(), key, args)
}

/// Like `translate`, in a given language.
pub fn translate_in(language: &str, key: &str, args: &[(&str, &dyn Display)]) -> String {
    let state = STATE.read().unwrap();
    let text = [language, FALLBACK]
        .iter()
        .find_map(|language| state.catalogs.get(language)?.get(key));
    match text {
        Some(text) => fill(text, args),
        None => {
            log::warn!("missing translation: {key}");
            key.to_owned()
        }
    }
}

fn fill(text: &str, args: &[(&str, &dyn Display)]) -> String {
    let mut text = text.to_owned();
    for (name, value) in args {
        text = text.replace(&format!("{{{name}}}"), &value.to_string());
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    use std::path::Path;

    fn keys(json: &str) -> BTreeSet<String> {
        parse(json).unwrap().into_keys().collect()
    }

    #[test]
    fn every_language_has_the_same_keys() {
        let reference = keys(BUILT_IN[0].1);
        for (language, json) in BUILT_IN {
            let these = keys(json);
            let missing: Vec<_> = reference.difference(&these).collect();
            let extra: Vec<_> = these.difference(&reference).collect();
            assert!(
                missing.is_empty() && extra.is_empty(),
                "{language}.json: missing {missing:?}, extra {extra:?}"
            );
        }
    }

    #[test]
    fn translations_keep_the_placeholders() {
        let placeholders = |text: &str| -> BTreeSet<String> {
            text.split('{')
                .skip(1)
                .filter_map(|part| part.split_once('}').map(|(name, _)| name.to_owned()))
                .collect()
        };
        let reference = parse(BUILT_IN[0].1).unwrap();
        let mut wrong = Vec::new();
        for (language, json) in BUILT_IN {
            for (key, text) in parse(json).unwrap() {
                if placeholders(&text) != placeholders(&reference[&key]) {
                    wrong.push(format!("{language}: {key}"));
                }
            }
        }
        assert!(
            wrong.is_empty(),
            "placeholders differ from English: {wrong:#?}"
        );
    }

    #[test]
    fn every_built_in_language_is_listed() {
        for (language, _) in BUILT_IN {
            assert!(
                known(language).is_some(),
                "{language} missing from LANGUAGES"
            );
        }
        assert_eq!(BUILT_IN.len(), LANGUAGES.len());
    }

    #[test]
    fn every_key_used_in_the_code_exists() {
        let known = keys(BUILT_IN[0].1);
        let mut missing = Vec::new();
        let mut stack = vec![Path::new(env!("CARGO_MANIFEST_DIR")).join("src")];
        while let Some(dir) = stack.pop() {
            for entry in std::fs::read_dir(dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    stack.push(path);
                } else if path.extension().is_some_and(|e| e == "rs")
                    // Its docs and this test quote `t!` without a real key.
                    && !path.ends_with("i18n.rs")
                {
                    let code = std::fs::read_to_string(&path).unwrap();
                    for (at, _) in code.match_indices("t!(\"") {
                        // `format!(`, `print!(`… are not `t!(`.
                        let before = code[..at].chars().next_back();
                        if before.is_some_and(|c| c.is_alphanumeric() || c == '_') {
                            continue;
                        }
                        let rest = &code[at + 4..];
                        let key = &rest[..rest.find('"').unwrap()];
                        if !known.contains(key) {
                            missing.push(format!("{}: {key}", path.display()));
                        }
                    }
                }
            }
        }
        assert!(
            missing.is_empty(),
            "keys missing from en.json: {missing:#?}"
        );
    }

    #[test]
    fn a_tag_picks_its_language_or_english() {
        assert_eq!(pick(Some("fr-FR")), "fr");
        assert_eq!(pick(Some("fr_CA")), "fr");
        assert_eq!(pick(Some("EN-us")), "en");
        assert_eq!(pick(Some("de-DE")), "de");
        assert_eq!(pick(Some("it-IT")), "en");
        assert_eq!(pick(None), "en");
    }

    #[test]
    fn placeholders_are_filled() {
        assert_eq!(
            fill("Port {port} busy, {port}!", &[("port", &8080)]),
            "Port 8080 busy, 8080!"
        );
    }
}
