//! Settings window and the commands it calls, see
//! docs/fonctionnel/DF-0005-fenetre-de-parametres.md. Every change is applied
//! immediately and saved to config.json.

use std::sync::atomic::Ordering;

use serde::Serialize;
use serde_json::Value;
use tauri::webview::PageLoadEvent;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_autostart::ManagerExt;

use crate::module::Host;
use crate::placement::{self, Edge, GAP_RANGE, Style};
use crate::resistance::Strength;
use crate::{AppState, config, notch, platform};

pub const SETTINGS_LABEL: &str = "settings";

/// A monitor the notch can go on.
#[derive(Clone, Serialize)]
pub struct ScreenChoice {
    id: String,
    label: String,
    primary: bool,
}

/// A module and its section of the settings window (ADR-0009).
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModuleInfo {
    id: String,
    name: String,
    description: String,
    enabled: bool,
    /// The module's own data, read by its settings component.
    settings: Value,
}

/// Everything the settings window displays.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    sound_enabled: bool,
    hide_in_fullscreen: bool,
    edge: Edge,
    /// Notch or pill, and the pill's distance to the edge (ADR-0003).
    style: Style,
    gap: u32,
    /// Chosen screen, `None` = the primary one.
    screen: Option<String>,
    screens: Vec<ScreenChoice>,
    /// Move mode, see `notch::set_movable`.
    movable: bool,
    /// The notch is not centered on its edge.
    off_center: bool,
    cursor_resistance: bool,
    resistance_strength: Strength,
    /// Cursor resistance only exists on Windows for now.
    resistance_available: bool,
    autostart: bool,
    modules: Vec<ModuleInfo>,
    module_layout: crate::config::ModuleLayout,
    notch_speed: crate::config::NotchSpeed,
    config_dir: String,
}

pub fn current(app: &AppHandle) -> Settings {
    let state = app.state::<AppState>();
    let config = state.config.lock().unwrap().clone();
    Settings {
        sound_enabled: config.sound_enabled,
        hide_in_fullscreen: config.hide_in_fullscreen,
        edge: config.placement.edge,
        style: config.placement.style,
        gap: config.placement.gap,
        screen: config.placement.screen.clone(),
        screens: screen_choices(app),
        movable: state.movable.load(Ordering::Relaxed),
        off_center: config.placement.offset != 0.5,
        cursor_resistance: config.cursor_resistance,
        resistance_strength: config.cursor_resistance_strength,
        resistance_available: cfg!(windows),
        autostart: app.autolaunch().is_enabled().unwrap_or(false),
        modules: crate::module::ordered(&state.modules, &config.module_order)
            .into_iter()
            .map(|m| {
                let enabled = crate::module::is_enabled(m, &config);
                ModuleInfo {
                    id: m.id().into(),
                    name: m.name().into(),
                    description: m.description().into(),
                    enabled,
                    settings: if enabled { m.settings() } else { Value::Null },
                }
            })
            .collect(),
        module_layout: config.module_layout,
        notch_speed: config.notch_speed,
        config_dir: state.config_dir.display().to_string(),
    }
}

fn screen_choices(app: &AppHandle) -> Vec<ScreenChoice> {
    let screens = crate::notch::screens(app).unwrap_or_default();
    placement::screen_labels(&screens)
        .into_iter()
        .map(|(id, label)| ScreenChoice {
            primary: screens.iter().any(|s| s.id == id && s.primary),
            id,
            label,
        })
        .collect()
}

fn repaint_notch_host(app: &AppHandle) {
    if let Some(notch) = app.get_webview_window(crate::NOTCH_LABEL)
        && let Err(e) = platform::repaint_host(&notch)
    {
        log::warn!("cannot repaint the notch window: {e}");
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
            .inner_size(760.0, 560.0)
            .min_inner_size(600.0, 420.0)
            .center()
            .on_page_load(|window, payload| {
                if payload.event() == PageLoadEvent::Finished {
                    repaint_notch_host(window.app_handle());
                }
            })
            .build()?;
    // Creating the window leaves white pixels behind the notch's webview
    // (see `platform::repaint_host`): wipe them, again once the page is in.
    repaint_notch_host(app);
    // Move mode only lasts while the settings are open.
    let handle = app.clone();
    window.on_window_event(move |event| {
        if matches!(event, tauri::WindowEvent::Destroyed) {
            notch::set_movable(&handle, false);
        }
    });
    platform::bring_to_front(&window)
}

/// Tells every window (notch included) that settings changed.
pub fn changed(app: &AppHandle) {
    let _ = app.emit("settings-changed", current(app));
    crate::emit_status(app);
}

pub(crate) fn update_config(app: &AppHandle, edit: impl FnOnce(&mut config::Config)) {
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

/// Moves the notch to another screen edge, centered on it (DF-0006).
#[tauri::command]
pub fn set_edge(app: AppHandle, edge: Edge) -> Result<(), String> {
    update_config(&app, |c| {
        c.placement.edge = edge;
        c.placement.offset = 0.5;
    });
    notch::reset_hit_area();
    if let Some(window) = app.get_webview_window(crate::NOTCH_LABEL) {
        notch::place(&window)
            .and_then(|_| notch::apply_hit_area(&window))
            .map_err(|e| e.to_string())?;
    }
    changed(&app);
    Ok(())
}

/// Notch glued to the edge, or pill detached from it (ADR-0003).
#[tauri::command]
pub fn set_style(app: AppHandle, style: Style) -> Result<(), String> {
    update_config(&app, |c| c.placement.style = style);
    notch::reset_hit_area();
    replace(&app)
}

/// Distance between the pill and its edge, in logical pixels.
#[tauri::command]
pub fn set_gap(app: AppHandle, gap: u32) -> Result<(), String> {
    if !GAP_RANGE.contains(&gap) {
        return Err(format!(
            "Écart entre {} et {} px",
            GAP_RANGE.start(),
            GAP_RANGE.end()
        ));
    }
    update_config(&app, |c| c.placement.gap = gap);
    replace(&app)
}

/// Puts the notch window back where the placement says, with its shape.
fn replace(app: &AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(crate::NOTCH_LABEL) {
        notch::place(&window)
            .and_then(|_| notch::apply_hit_area(&window))
            .map_err(|e| e.to_string())?;
    }
    changed(app);
    Ok(())
}

/// Moves the notch to another monitor, `None` = the primary one (DF-0006).
/// An unplugged monitor falls back to the primary one until it comes back.
#[tauri::command]
pub fn set_screen(app: AppHandle, screen: Option<String>) -> Result<(), String> {
    update_config(&app, |c| c.placement.screen = screen);
    if let Some(window) = app.get_webview_window(crate::NOTCH_LABEL) {
        notch::place(&window)
            .and_then(|_| notch::apply_hit_area(&window))
            .map_err(|e| e.to_string())?;
    }
    changed(&app);
    Ok(())
}

/// Move mode: the notch can be dragged along its edge while the settings
/// are open (DF-0006, step 3).
#[tauri::command]
pub fn set_movable(app: AppHandle, movable: bool) {
    notch::set_movable(&app, movable);
    changed(&app);
}

/// Puts the notch back in the middle of its edge.
#[tauri::command]
pub fn recenter(app: AppHandle) -> Result<(), String> {
    update_config(&app, |c| c.placement.offset = 0.5);
    if let Some(window) = app.get_webview_window(crate::NOTCH_LABEL) {
        notch::place(&window)
            .and_then(|_| notch::apply_hit_area(&window))
            .map_err(|e| e.to_string())?;
    }
    changed(&app);
    Ok(())
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

/// Turns a module on or off; a disabled module does not run at all.
#[tauri::command]
pub fn set_module_enabled(app: AppHandle, id: String, enabled: bool) -> Result<(), String> {
    let state = app.state::<AppState>();
    let module = state
        .modules
        .iter()
        .find(|m| m.id() == id)
        .ok_or_else(|| format!("module inconnu : {id}"))?;
    let was_enabled = crate::module::is_enabled(module.as_ref(), &state.config.lock().unwrap());
    if was_enabled != enabled {
        update_config(&app, |c| {
            c.modules.entry(id.clone()).or_default().enabled = Some(enabled)
        });
        if enabled {
            module.start(&Host::new(app.clone(), module.id()));
        } else {
            module.stop();
        }
    }
    changed(&app);
    crate::emit_content(&app);
    Ok(())
}

/// The user reordered the modules (DF-0011): `order` lists module ids, the
/// first one closest to the edge. Unknown ids are dropped.
#[tauri::command]
pub fn set_module_order(app: AppHandle, order: Vec<String>) {
    let known: Vec<String> = {
        let state = app.state::<AppState>();
        order
            .into_iter()
            .filter(|id| state.modules.iter().any(|m| m.id() == id))
            .collect()
    };
    update_config(&app, |c| c.module_order = known);
    changed(&app);
    crate::emit_content(&app);
}

/// Modules joined in the notch or one card each (DF-0011).
#[tauri::command]
pub fn set_module_layout(app: AppHandle, layout: crate::config::ModuleLayout) {
    update_config(&app, |c| c.module_layout = layout);
    changed(&app);
    crate::emit_status(&app);
}

/// How fast the notch opens and closes.
#[tauri::command]
pub fn set_notch_speed(app: AppHandle, speed: crate::config::NotchSpeed) {
    update_config(&app, |c| c.notch_speed = speed);
    changed(&app);
    crate::emit_status(&app);
}

/// Action of a module's settings section, handled by the module itself.
#[tauri::command]
pub fn module_call(
    app: AppHandle,
    id: String,
    action: String,
    args: Option<Value>,
) -> Result<Value, String> {
    let state = app.state::<AppState>();
    let module = state
        .modules
        .iter()
        .find(|m| m.id() == id)
        .ok_or_else(|| format!("module inconnu : {id}"))?;
    if !crate::module::is_enabled(module.as_ref(), &state.config.lock().unwrap()) {
        return Err(format!("{} est désactivé", module.name()));
    }
    module.call(&action, args.unwrap_or(Value::Null))
}
