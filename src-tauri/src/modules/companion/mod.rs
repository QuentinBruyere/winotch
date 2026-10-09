//! Companion module (DF-0024): a companion (DF-0020) living in the notch on
//! its own, reacting to what the notch shows (music playing, a timer
//! ringing…) and to the mouse. The front picks its mood; this module only
//! keeps the user's choice of companion, color and size.

use std::sync::{Mutex, OnceLock};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::companion;
use crate::module::{Host, Item, ItemMenu, MenuChoice, Module, Tone};

/// Item menu choices: `companion:<id>`, then `color:<id>` or the default,
/// and the darker switch.
const COMPANION: &str = "companion:";
const COLOR: &str = "color:";
const DEFAULT_COLOR: &str = "default-color";
const DIMMED: &str = "dimmed";
/// Ticked when the color was picked in the settings: picking it does nothing.
const CUSTOM_COLOR: &str = "custom-color";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct Settings {
    /// One of `companion::all()`.
    companion: String,
    /// One of `companion::COLORS`, or any `#rrggbb` picked in the settings;
    /// `None` = the usual state colors.
    color: Option<String>,
    /// Drawn darker, for a color too bright.
    dimmed: bool,
    size: companion::Size,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            companion: companion::BUILT_IN[0].into(),
            color: None,
            dimmed: false,
            size: companion::Size::default(),
        }
    }
}

#[derive(Default)]
pub struct DeskCompanion {
    host: OnceLock<Host>,
    settings: Mutex<Settings>,
}

impl Module for DeskCompanion {
    fn id(&self) -> &'static str {
        "companion"
    }

    fn name(&self) -> String {
        crate::t!("companion.module.name")
    }

    fn description(&self) -> String {
        crate::t!("companion.module.description")
    }

    fn enabled_by_default(&self) -> bool {
        false
    }

    fn start(&self, host: &Host) {
        let _ = self.host.set(host.clone());
        *self.settings.lock().unwrap() = host.load_settings();
        host.refresh();
    }

    fn stop(&self) {
        if let Some(host) = self.host.get() {
            host.refresh();
        }
    }

    fn items(&self) -> Vec<Item> {
        let settings = self.settings.lock().unwrap().clone();
        vec![item(&settings)]
    }

    /// Pinned, it stays small: hovering only wakes the companion up.
    fn pin_opens(&self) -> bool {
        false
    }

    /// The companion and its color, as in the Claude Code module.
    fn item_menu(&self, _item: &str) -> Vec<ItemMenu> {
        let settings = self.settings.lock().unwrap().clone();
        menus(&settings)
    }

    fn item_action(&self, _item: &str, action: &str) {
        let args = if let Some(id) = action.strip_prefix(COMPANION) {
            ("set_companion", json!({ "id": id }))
        } else if let Some(color) = action.strip_prefix(COLOR) {
            ("set_color", json!({ "color": color }))
        } else if action == DEFAULT_COLOR {
            ("set_color", json!({ "color": null }))
        } else if action == DIMMED {
            let dimmed = self.settings.lock().unwrap().dimmed;
            ("set_dimmed", json!({ "dimmed": !dimmed }))
        } else {
            return;
        };
        if let Err(e) = self.call(args.0, args.1) {
            log::warn!("companion menu: {e}");
        }
    }

    fn settings(&self) -> Value {
        let settings = self.settings.lock().unwrap().clone();
        json!({
            "companion": settings.companion,
            "color": settings.color,
            "dimmed": settings.dimmed,
            "size": settings.size,
            "companions": companion::all()
                .iter()
                .map(|id| json!({ "id": id, "name": companion::name(id) }))
                .collect::<Vec<_>>(),
            "colors": companion::COLORS
                .iter()
                .map(|id| json!({ "id": id, "name": companion::color_name(id) }))
                .collect::<Vec<_>>(),
        })
    }

    fn call(&self, action: &str, args: Value) -> Result<Value, String> {
        let changed = |edit: &dyn Fn(&mut Settings)| {
            let mut settings = self.settings.lock().unwrap();
            edit(&mut settings);
            if let Some(host) = self.host.get() {
                host.save_settings(&*settings);
            }
        };
        match action {
            "set_companion" => {
                let id = args["id"].as_str().unwrap_or_default();
                if !companion::exists(id) {
                    return Err(crate::t!("companion.module.unknown"));
                }
                changed(&|s| s.companion = id.to_owned());
            }
            "set_color" => {
                let color = match args["color"].as_str() {
                    Some(color) => Some(
                        known_color(color).ok_or_else(|| crate::t!("companion.module.unknown"))?,
                    ),
                    None => None,
                };
                changed(&|s| s.color = color.clone());
            }
            "set_dimmed" => {
                let dimmed = args["dimmed"].as_bool().unwrap_or_default();
                changed(&|s| s.dimmed = dimmed);
            }
            "set_size" => {
                let size = serde_json::from_value(args["size"].clone())
                    .map_err(|_| crate::t!("companion.module.unknown"))?;
                changed(&|s| s.size = size);
            }
            _ => return Err(format!("unknown action {action}")),
        }
        // Not holding the settings lock: these call back into the module.
        if let Some(host) = self.host.get() {
            host.settings_changed();
            host.refresh();
        }
        Ok(Value::Null)
    }
}

/// The right-click submenus: which companion, then its color and, apart,
/// the darker switch; the current choices ticked.
fn menus(settings: &Settings) -> Vec<ItemMenu> {
    let companions = companion::all()
        .into_iter()
        .map(|id| MenuChoice {
            id: format!("{COMPANION}{id}"),
            label: companion::name(&id),
            checked: settings.companion == id,
            ..MenuChoice::default()
        })
        .collect();
    let mut colors = vec![MenuChoice {
        id: DEFAULT_COLOR.into(),
        label: crate::t!("companion.module.default_color"),
        checked: settings.color.is_none(),
        ..MenuChoice::default()
    }];
    colors.extend(companion::COLORS.iter().map(|id| MenuChoice {
        id: format!("{COLOR}{id}"),
        label: companion::color_name(id),
        checked: settings.color.as_deref() == Some(*id),
        ..MenuChoice::default()
    }));
    // Only the settings window picks any color; the menu just shows it.
    if settings
        .color
        .as_deref()
        .is_some_and(|c| c.starts_with('#'))
    {
        colors.push(MenuChoice {
            id: CUSTOM_COLOR.into(),
            label: crate::t!("companion.module.custom_color"),
            checked: true,
            ..MenuChoice::default()
        });
    }
    colors.push(MenuChoice {
        id: DIMMED.into(),
        label: crate::t!("companion.module.dimmed"),
        checked: settings.dimmed,
        separated: true,
    });
    vec![
        ItemMenu {
            title: crate::t!("companion.module.choice"),
            choices: companions,
        },
        ItemMenu {
            title: crate::t!("companion.module.menu_color"),
            choices: colors,
        },
    ]
}

/// A palette color, or `#rrggbb` (lowercase); `None` for anything else.
fn known_color(color: &str) -> Option<String> {
    if companion::COLORS.contains(&color) {
        Some(color.to_owned())
    } else {
        crate::config::color(color)
    }
}

/// The companion's line: its name, and the companion itself, whose mood
/// the front picks from the rest of the notch.
fn item(settings: &Settings) -> Item {
    // An imported companion since removed: the first built-in one.
    let id = if companion::exists(&settings.companion) {
        settings.companion.clone()
    } else {
        companion::BUILT_IN[0].into()
    };
    let color = settings.color.as_deref().and_then(known_color);
    Item {
        id: "companion".into(),
        title: companion::name(&id),
        label: String::new(),
        tone: Tone::Neutral,
        dot: true,
        companion: Some(companion::Choice {
            id,
            size: settings.size,
            color,
            dimmed: settings.dimmed,
            reactive: true,
        }),
        ..Item::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_chosen_companion_reacts_to_the_notch() {
        let settings = Settings {
            companion: "miso".into(),
            color: Some("teal".into()),
            dimmed: true,
            size: companion::Size::Large,
        };
        let choice = item(&settings).companion.unwrap();
        assert_eq!(choice.id, "miso");
        assert_eq!(choice.color.as_deref(), Some("teal"));
        assert!(choice.dimmed);
        assert!(choice.reactive);
    }

    #[test]
    fn the_menus_tick_the_current_companion_and_color() {
        let settings = Settings {
            companion: "bit".into(),
            color: None,
            ..Settings::default()
        };
        let ticked: Vec<Vec<String>> = menus(&settings)
            .into_iter()
            .map(|m| {
                m.choices
                    .into_iter()
                    .filter(|c| c.checked)
                    .map(|c| c.id)
                    .collect()
            })
            .collect();
        assert_eq!(ticked, [vec!["companion:bit"], vec![DEFAULT_COLOR]]);
        let colors = &menus(&settings)[1].choices;
        let last = colors.last().unwrap();
        assert_eq!(last.id, DIMMED);
        assert!(last.separated);
    }

    #[test]
    fn a_custom_color_is_kept_and_shown_ticked_in_the_menu() {
        let settings = Settings {
            color: Some("#A1B2C3".into()),
            ..Settings::default()
        };
        let choice = item(&settings).companion.unwrap();
        assert_eq!(choice.color.as_deref(), Some("#a1b2c3"));
        let settings = Settings {
            color: Some("#a1b2c3".into()),
            ..Settings::default()
        };
        let ticked: Vec<String> = menus(&settings)[1]
            .choices
            .iter()
            .filter(|c| c.checked)
            .map(|c| c.id.clone())
            .collect();
        assert_eq!(ticked, [CUSTOM_COLOR]);
    }

    #[test]
    fn an_unknown_companion_or_color_falls_back() {
        let settings = Settings {
            companion: "gone".into(),
            color: Some("peach".into()),
            ..Settings::default()
        };
        let choice = item(&settings).companion.unwrap();
        assert_eq!(choice.id, companion::BUILT_IN[0]);
        assert_eq!(choice.color, None);
    }
}
