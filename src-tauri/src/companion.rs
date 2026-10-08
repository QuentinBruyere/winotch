//! Companions (DF-0020): little animated creatures an item may show instead
//! of its dot (`Item::companion`). The front draws them (src/lib/companions/);
//! the core only knows their ids, to offer them in menus.

use serde::{Deserialize, Serialize};

/// The companions shipped with winotch, in menu order. Each needs its
/// drawing in src/lib/companions/ and a `companion.<id>` translation (its
/// name).
pub const BUILT_IN: &[&str] = &["bloop", "bit", "miso"];

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
    /// One of `BUILT_IN`.
    pub id: String,
    pub size: Size,
    /// One of `COLORS`, worn while working (and darker at rest); the other
    /// states keep their own color. `None` = always the state's.
    pub color: Option<String>,
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

/// A companion's name, in the current language.
pub fn name(id: &str) -> String {
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
