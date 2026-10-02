//! Per-user configuration, see docs/adr/0006-configuration-et-secrets.md.
//! Lives in the OS config dir; a missing or broken file falls back to defaults.

use std::fs;
use std::io;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::placement::Placement;
use crate::resistance::Strength;

const CONFIG_FILE: &str = "config.json";
const TOKEN_FILE: &str = "hook-token";
const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Config {
    pub schema_version: u32,
    /// Local port the Claude Code hooks post to (127.0.0.1 only).
    pub server_port: u16,
    /// Sessions silent for this long are considered dead and removed.
    pub session_timeout_minutes: u64,
    /// Plays a sound when a session needs attention (DF-0003).
    pub sound_enabled: bool,
    /// Holds the cursor at the notch edge until the user pushes through (DF-0004).
    pub cursor_resistance: bool,
    pub cursor_resistance_strength: Strength,
    /// Hides the notch while an app is fullscreen on its monitor (DF-0002).
    pub hide_in_fullscreen: bool,
    /// Where the notch sits on screen (DF-0006).
    pub placement: Placement,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            server_port: 47821,
            session_timeout_minutes: 180,
            sound_enabled: true,
            cursor_resistance: true,
            cursor_resistance_strength: Strength::default(),
            hide_in_fullscreen: true,
            placement: Placement::default(),
        }
    }
}

/// Reads the config, writing the defaults on first launch.
pub fn load_or_create(dir: &Path) -> Config {
    let path = dir.join(CONFIG_FILE);
    match fs::read_to_string(&path) {
        Ok(text) => serde_json::from_str(&text).unwrap_or_else(|e| {
            // Keep the user's file untouched so they can fix it.
            log::warn!("invalid {}: {e}, using defaults", path.display());
            Config::default()
        }),
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

/// Token guarding the local hook server, generated once per installation.
pub fn load_or_create_token(dir: &Path) -> io::Result<String> {
    let path = dir.join(TOKEN_FILE);
    if let Ok(token) = fs::read_to_string(&path) {
        let token = token.trim();
        if token.len() >= 32 {
            return Ok(token.to_owned());
        }
    }
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).map_err(|e| io::Error::other(e.to_string()))?;
    let token: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    fs::create_dir_all(dir)?;
    fs::write(&path, &token)?;
    Ok(token)
}

fn write_json<T: Serialize>(dir: &Path, name: &str, value: &T) -> io::Result<()> {
    fs::create_dir_all(dir)?;
    let text = serde_json::to_string_pretty(value).map_err(io::Error::other)?;
    fs::write(dir.join(name), text + "\n")
}
