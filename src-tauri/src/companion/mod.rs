//! Companions (DF-0020): little animated creatures an item may show instead
//! of its dot (`Item::companion`). The front draws them (src/lib/companions/);
//! the core only knows their ids, to offer them in menus. Besides the
//! built-in ones, the user may import their own (DF-0025): `library` reads
//! them from the config folder, `pack` turns them into pictures.

pub(crate) mod library;
mod pack;

use std::sync::RwLock;

use serde::{Deserialize, Serialize};

pub use library::load as load_custom;

/// The companions shipped with Minim Notch, in menu order. Each needs its
/// drawing in src/lib/companions/ and a `companion.<id>` translation (its
/// name).
pub const BUILT_IN: &[&str] = &["molf", "bloop", "bit", "miso"];

/// The colors a companion may be given instead of its state's, in menu
/// order. The front holds their values (`--companion-<id>` in src/app.css,
/// for each notch theme); each needs a `companion.color.<id>` translation.
pub const COLORS: &[&str] = &[
    "ink",
    "pink",
    "purple",
    "sky",
    "blue",
    "teal",
    "emerald",
    "lemon",
    "terracotta",
];

/// A companion shown instead of an item's dot (`Item::companion`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Choice {
    /// One of `all()`.
    pub id: String,
    pub size: Size,
    /// One of `COLORS` or a `#rrggbb` (the Companion module's settings),
    /// worn while working (and darker at rest); the other
    /// states keep their own color. `None` = always the state's.
    pub color: Option<String>,
    /// Drawn darker while working too, as at rest (a color too bright for
    /// the user); the other states keep their color.
    pub dimmed: bool,
    /// Its mood follows the rest of the notch and the mouse, not the item's
    /// tone (the Companion module, DF-0024); the front picks it.
    pub reactive: bool,
}

/// How big a companion is drawn; the front picks the pixels.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Size {
    Small,
    #[default]
    Medium,
    Large,
}

/// An imported companion (DF-0025), as the front receives it: its
/// pictures, or why its pack is refused.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Custom {
    /// `custom:<folder or zip name>`.
    pub id: String,
    pub name: String,
    pub definition: Option<pack::Definition>,
    pub error: Option<String>,
}

/// The imported companions, refused packs included.
static CUSTOM: RwLock<Vec<Custom>> = RwLock::new(Vec::new());

fn set_custom(list: Vec<Custom>) {
    *CUSTOM.write().unwrap() = list;
}

/// The imported companions, refused packs included.
pub fn custom() -> Vec<Custom> {
    CUSTOM.read().unwrap().clone()
}

/// Every companion that can be shown, in menu order: the built-in ones, then
/// the imported ones.
pub fn all() -> Vec<String> {
    let custom = CUSTOM.read().unwrap();
    BUILT_IN
        .iter()
        .map(|id| (*id).to_owned())
        .chain(
            custom
                .iter()
                .filter(|c| c.definition.is_some())
                .map(|c| c.id.clone()),
        )
        .collect()
}

/// `id` can be shown (built in, or imported and valid).
pub fn exists(id: &str) -> bool {
    BUILT_IN.contains(&id)
        || CUSTOM
            .read()
            .unwrap()
            .iter()
            .any(|c| c.id == id && c.definition.is_some())
}

/// A companion's name, in the current language (an imported one's is its
/// pack's).
pub fn name(id: &str) -> String {
    if let Some(custom) = CUSTOM.read().unwrap().iter().find(|c| c.id == id) {
        return custom.name.clone();
    }
    crate::i18n::translate(&format!("companion.{id}"), &[])
}

/// A companion color's name, in the current language.
pub fn color_name(id: &str) -> String {
    crate::i18n::translate(&format!("companion.color.{id}"), &[])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::{LANGUAGES, translate_in};

    #[test]
    fn every_companion_and_color_has_a_name_in_every_language() {
        let keys = BUILT_IN
            .iter()
            .map(|id| format!("companion.{id}"))
            .chain(COLORS.iter().map(|id| format!("companion.color.{id}")));
        for key in keys {
            for (language, _) in LANGUAGES {
                assert_ne!(
                    translate_in(language, &key, &[]),
                    key,
                    "{language}: no {key}"
                );
            }
        }
    }
}
