//! Per-user configuration, see docs/adr/0006-configuration-et-secrets.md.
//! Lives in the OS config dir; a missing or broken file falls back to defaults.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::placement::Placement;
use crate::resistance::Strength;

const CONFIG_FILE: &str = "config.json";
/// 2: module settings moved under `modules` (ADR-0009).
const SCHEMA_VERSION: u32 = 2;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Config {
    pub schema_version: u32,
    /// Plays a sound when a session needs attention (DF-0003).
    pub sound_enabled: bool,
    /// Holds the cursor at the notch edge until the user pushes through (DF-0004).
    pub cursor_resistance: bool,
    pub cursor_resistance_strength: Strength,
    /// Hides the notch while an app is fullscreen on its monitor (DF-0002).
    pub hide_in_fullscreen: bool,
    /// Where the notch sits on screen (DF-0006).
    pub placement: Placement,
    /// Per module, by module id (ADR-0009).
    pub modules: BTreeMap<String, ModuleEntry>,
    /// Module ids in the user's display order (DF-0011); modules missing
    /// from it come after, in their declaration order.
    pub module_order: Vec<String>,
    pub module_layout: ModuleLayout,
    /// How fast the notch opens and closes.
    pub notch_speed: NotchSpeed,
    /// Pinned modules and their side, by module id (DF-0012).
    pub pins: BTreeMap<String, PinSide>,
}

/// Side of a pinned module's mini-notch, next to the notch (DF-0012).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PinSide {
    /// The side with fewer pins.
    #[default]
    Auto,
    Left,
    Right,
}

/// Speed of the notch's opening and closing animation.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NotchSpeed {
    Slow,
    #[default]
    Normal,
    Fast,
}

/// How several modules share the open notch (DF-0011).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ModuleLayout {
    /// All in the notch, a thin line between two modules.
    #[default]
    Joined,
    /// One card per module, the others under the notch.
    Separate,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ModuleEntry {
    /// Chosen by the user; `None` = the module's own default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    /// The module's own settings, read and written by the module.
    #[serde(flatten)]
    pub settings: Map<String, Value>,
}

impl Config {
    /// The user's choice, else `default` (`Module::enabled_by_default`).
    pub fn module_enabled(&self, id: &str, default: bool) -> bool {
        self.modules
            .get(id)
            .and_then(|m| m.enabled)
            .unwrap_or(default)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            sound_enabled: true,
            cursor_resistance: true,
            cursor_resistance_strength: Strength::default(),
            hide_in_fullscreen: true,
            placement: Placement::default(),
            modules: BTreeMap::new(),
            module_order: Vec::new(),
            module_layout: ModuleLayout::default(),
            notch_speed: NotchSpeed::default(),
            pins: BTreeMap::new(),
        }
    }
}

/// Reads the config, writing the defaults on first launch.
pub fn load_or_create(dir: &Path) -> Config {
    let path = dir.join(CONFIG_FILE);
    match fs::read_to_string(&path) {
        Ok(text) => match serde_json::from_str::<Value>(&text).and_then(|v| {
            let outdated = schema_version(&v) < SCHEMA_VERSION;
            serde_json::from_value::<Config>(migrate(v)).map(|c| (c, outdated))
        }) {
            Ok((config, outdated)) => {
                if outdated && let Err(e) = save(dir, &config) {
                    log::warn!("cannot save migrated config: {e}");
                }
                config
            }
            // Keep the user's file untouched so they can fix it.
            Err(e) => {
                log::warn!("invalid {}: {e}, using defaults", path.display());
                Config::default()
            }
        },
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            let config = Config::default();
            if let Err(e) = write_json(dir, CONFIG_FILE, &config) {
                log::warn!("cannot write default config: {e}");
            }
            config
        }
        Err(e) => {
            log::warn!("cannot read {}: {e}, using defaults", path.display());
            Config::default()
        }
    }
}

pub fn save(dir: &Path, config: &Config) -> io::Result<()> {
    write_json(dir, CONFIG_FILE, config)
}

/// Files written before `schemaVersion` existed count as version 1.
fn schema_version(value: &Value) -> u32 {
    value
        .get("schemaVersion")
        .and_then(Value::as_u64)
        .map_or(1, |v| v as u32)
}

/// Brings an older config.json up to the current schema.
fn migrate(mut value: Value) -> Value {
    let version = schema_version(&value);
    let Some(root) = value.as_object_mut() else {
        return value;
    };
    if version < 2 {
        // Claude Code became a module: its settings left the root.
        let mut claude = Map::new();
        for key in ["serverPort", "sessionTimeoutMinutes"] {
            if let Some(v) = root.remove(key) {
                claude.insert(key.into(), v);
            }
        }
        if !claude.is_empty() {
            let modules = root
                .entry("modules")
                .or_insert_with(|| Value::Object(Map::new()));
            if let Some(modules) = modules.as_object_mut() {
                modules.insert("claude-code".into(), Value::Object(claude));
            }
        }
    }
    root.insert("schemaVersion".into(), SCHEMA_VERSION.into());
    value
}

fn write_json<T: Serialize>(dir: &Path, name: &str, value: &T) -> io::Result<()> {
    fs::create_dir_all(dir)?;
    let text = serde_json::to_string_pretty(value).map_err(io::Error::other)?;
    fs::write(dir.join(name), text + "\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn version_1_moves_claude_code_settings_to_its_module() {
        let old = json!({
            "schemaVersion": 1,
            "serverPort": 50000,
            "sessionTimeoutMinutes": 60,
            "soundEnabled": false
        });
        let config: Config = serde_json::from_value(migrate(old)).unwrap();
        assert_eq!(config.schema_version, SCHEMA_VERSION);
        assert!(!config.sound_enabled);
        let claude = &config.modules["claude-code"];
        assert_eq!(claude.enabled, None);
        assert_eq!(claude.settings["serverPort"], json!(50000));
        assert_eq!(claude.settings["sessionTimeoutMinutes"], json!(60));
    }

    #[test]
    fn current_version_is_left_alone() {
        let current = json!({
            "schemaVersion": SCHEMA_VERSION,
            "serverPort": 50000,
            "modules": { "claude-code": { "enabled": false } }
        });
        let config: Config = serde_json::from_value(migrate(current)).unwrap();
        assert!(!config.module_enabled("claude-code", true));
        assert!(config.modules["claude-code"].settings.is_empty());
        assert!(config.module_enabled("other", true));
        assert!(!config.module_enabled("other", false));
    }
}
