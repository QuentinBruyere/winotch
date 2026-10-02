//! Settings window and the commands it calls, see
//! docs/fonctionnel/DF-0005-fenetre-de-parametres.md. Every change is applied
//! immediately and saved to config.json.

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_autostart::ManagerExt;

use crate::resistance::Strength;
use crate::{AppState, claude_settings, config, notch, platform, server};

pub const SETTINGS_LABEL: &str = "settings";

/// Everything the settings window displays.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    sound_enabled: bool,
    hide_in_fullscreen: bool,
    cursor_resistance: bool,
    resistance_strength: Strength,
    /// Cursor resistance only exists on Windows for now.
    resistance_available: bool,
    server_port: u16,
    session_timeout_minutes: u64,
    autostart: bool,
    hooks_installed: bool,
    server_error: Option<String>,
    claude_settings_path: String,
    config_dir: String,
}

pub fn current(app: &AppHandle) -> Settings {
    let state = app.state::<AppState>();
    let config = state.config.lock().unwrap().clone();
    Settings {
        sound_enabled: config.sound_enabled,
        hide_in_fullscreen: config.hide_in_fullscreen,
        cursor_resistance: config.cursor_resistance,
        resistance_strength: config.cursor_resistance_strength,
        resistance_available: cfg!(windows),
        server_port: config.server_port,
        session_timeout_minutes: config.session_timeout_minutes,
        autostart: app.autolaunch().is_enabled().unwrap_or(false),
        hooks_installed: claude_settings::is_installed(&state.claude_settings),
        server_error: state.server_error.lock().unwrap().clone(),
        claude_settings_path: state.claude_settings.display().to_string(),
        config_dir: state.config_dir.display().to_string(),
    }
}

/// Opens the settings window, or brings it back to the front.
pub fn open(app: &AppHandle) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window(SETTINGS_LABEL) {
        window.unminimize()?;
        window.show()?;
        return platform::bring_to_front(&window);
    }
    let window =
        WebviewWindowBuilder::new(app, SETTINGS_LABEL, WebviewUrl::App("index.html".into()))
            .title("Paramètres de winotch")
            .inner_size(480.0, 760.0)
            .min_inner_size(420.0, 480.0)
            .center()
            .build()?;
    platform::bring_to_front(&window)
}

/// Tells every window (notch included) that settings changed.
fn changed(app: &AppHandle) {
    let _ = app.emit("settings-changed", current(app));
    crate::emit_status(app);
}

fn update_config(app: &AppHandle, edit: impl FnOnce(&mut config::Config)) {
    let state = app.state::<AppState>();
    let mut config = state.config.lock().unwrap();
    edit(&mut config);
    if let Err(e) = config::save(&state.config_dir, &config) {
        log::error!("cannot save config: {e}");
    }
}

#[tauri::command]
pub fn get_settings(app: AppHandle) -> Settings {
    current(&app)
}

#[tauri::command]
pub fn set_sound(app: AppHandle, enabled: bool) {
    update_config(&app, |c| c.sound_enabled = enabled);
    changed(&app);
}

/// Applied by the watcher within a second (DF-0002).
#[tauri::command]
pub fn set_hide_in_fullscreen(app: AppHandle, enabled: bool) {
    update_config(&app, |c| c.hide_in_fullscreen = enabled);
    changed(&app);
}

#[tauri::command]
pub fn set_cursor_resistance(app: AppHandle, enabled: bool, strength: Strength) {
    update_config(&app, |c| {
        c.cursor_resistance = enabled;
        c.cursor_resistance_strength = strength;
    });
    platform::set_resistance_strength(strength);
    notch::update_cursor_resistance(&app);
    changed(&app);
}

#[tauri::command]
pub fn set_session_timeout(app: AppHandle, minutes: u64) -> Result<(), String> {
    if !(5..=1440).contains(&minutes) {
        return Err("Entre 5 et 1440 minutes".into());
    }
    update_config(&app, |c| c.session_timeout_minutes = minutes);
    changed(&app);
    Ok(())
}

#[tauri::command]
pub fn set_autostart(app: AppHandle, enabled: bool) -> Result<(), String> {
    let launcher = app.autolaunch();
    let result = if enabled {
        launcher.enable()
    } else {
        launcher.disable()
    };
    changed(&app);
    result.map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_claude_connected(app: AppHandle, connected: bool) -> Result<(), String> {
    let result = connect_claude_code(&app, connected);
    changed(&app);
    result
}

/// Moves the hook server to another port without restarting winotch: the new
/// port is opened first, so a busy port leaves everything as it was.
#[tauri::command]
pub fn set_server_port(app: AppHandle, port: u16) -> Result<(), String> {
    if port < 1024 {
        return Err("Choisis un port entre 1024 et 65535".into());
    }
    let state = app.state::<AppState>();
    if port == state.config.lock().unwrap().server_port && state.server.lock().unwrap().is_some() {
        return Ok(());
    }
    let new_server = server::start(app.clone(), port, state.token.clone())
        .map_err(|e| format!("Port {port} indisponible : {e}"))?;
    if let Some(old) = state.server.lock().unwrap().replace(new_server) {
        old.unblock();
    }
    *state.server_error.lock().unwrap() = None;
    update_config(&app, |c| c.server_port = port);

    // The hooks contain the port: follow the change if Claude Code is connected.
    let result = if claude_settings::is_installed(&state.claude_settings) {
        connect_claude_code(&app, true)
    } else {
        Ok(())
    };
    changed(&app);
    result
}

/// Adds (`true`) or removes (`false`) winotch's hooks in the Claude Code settings.
pub fn connect_claude_code(app: &AppHandle, connect: bool) -> Result<(), String> {
    let state = app.state::<AppState>();
    let result = if connect {
        let port = state.config.lock().unwrap().server_port;
        claude_settings::install(&state.claude_settings, port, &state.token)
    } else {
        claude_settings::uninstall(&state.claude_settings)
    };
    let message = match &result {
        Ok(()) if connect => "Claude Code connecté",
        Ok(()) => "Claude Code déconnecté",
        Err(e) => {
            log::error!("cannot update {}: {e}", state.claude_settings.display());
            "Échec : voir les logs"
        }
    };
    let _ = app.emit("notice", message);
    result.map_err(|e| e.to_string())
}
