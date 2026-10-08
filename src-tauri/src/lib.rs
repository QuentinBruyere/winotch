mod config;
mod context_menu;
pub mod i18n;
pub mod module;
pub mod modules;
mod notch;
mod placement;
mod platform;
mod resistance;
mod settings;
mod tray;

use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, WindowEvent};

use config::Config;
use module::{Content, Host, Module};

pub const NOTCH_LABEL: &str = "notch";

pub struct AppState {
    pub modules: Vec<Box<dyn Module>>,
    pub config: Mutex<Config>,
    pub config_dir: PathBuf,
    /// Hidden from the tray menu: fullscreen detection must not show it back.
    pub user_hidden: AtomicBool,
    /// Move mode, on while the settings ask for it (DF-0006, step 3).
    pub movable: AtomicBool,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    sound_enabled: bool,
    /// Screen edge the notch is attached to: drives its orientation.
    edge: placement::Edge,
    /// Notch glued to the edge, or pill detached from it: drives its corners.
    style: placement::Style,
    /// Move mode: the notch can be dragged along its edge.
    movable: bool,
    /// Modules in the notch itself, or one card each (DF-0011).
    layout: config::ModuleLayout,
    speed: config::NotchSpeed,
    /// Light or dark (DF-0014), applied by the front.
    appearance: config::Appearance,
    /// The language shown (ADR-0012), the system's already resolved.
    language: &'static str,
    /// Center of the compact notch along the edge, from the start of the
    /// window, in logical pixels: every shape is centered on it, then kept
    /// inside the window. `None` = the middle of the window.
    anchor: Option<f64>,
}

#[tauri::command]
fn get_content(state: tauri::State<AppState>) -> Content {
    content(&state)
}

#[tauri::command]
fn get_status(state: tauri::State<AppState>) -> Status {
    status(&state)
}

/// Clicking the notch means the user has seen what it shows.
#[tauri::command]
fn acknowledge(state: tauri::State<AppState>) {
    let config = state.config.lock().unwrap().clone();
    for module in state
        .modules
        .iter()
        .filter(|m| module::is_enabled(m.as_ref(), &config))
    {
        module.acknowledge();
    }
}

/// A button of an item in the open notch; `item` is `<module id>:<item id>`.
#[tauri::command]
fn item_action(state: tauri::State<AppState>, item: String, action: String) {
    let Some((module_id, item_id)) = item.split_once(':') else {
        return;
    };
    let config = state.config.lock().unwrap().clone();
    if let Some(module) = state
        .modules
        .iter()
        .find(|m| m.id() == module_id && module::is_enabled(m.as_ref(), &config))
    {
        module.item_action(item_id, &action);
    }
}

/// The front decides the visible shapes: the notch first, compact or
/// expanded (DF-0003), then the other cards (DF-0011) and pins (DF-0012).
#[tauri::command]
fn set_hit_area(window: tauri::WebviewWindow, shapes: Vec<notch::Shape>) -> Result<(), String> {
    notch::set_hit_area(&window, shapes).map_err(|e| e.to_string())
}

/// The cursor left an open shape (`area`, logical pixels in the window): it
/// closes once the cursor is also past the safety margin around it, or not
/// if the cursor comes back first.
#[tauri::command]
fn watch_leave(window: tauri::WebviewWindow, area: notch::Bounds) -> Result<(), String> {
    notch::watch_leave(&window, area).map_err(|e| e.to_string())
}

/// The cursor is back on the notch: stops `watch_leave`.
#[tauri::command]
fn cancel_leave() {
    notch::cancel_leave();
}

/// Dragging the notch in move mode (DF-0006, step 3).
#[tauri::command]
fn start_drag(window: tauri::WebviewWindow) -> Result<(), String> {
    notch::start_drag(&window).map_err(|e| e.to_string())
}

#[tauri::command]
fn drag(window: tauri::WebviewWindow) -> Result<(), String> {
    notch::drag(&window).map_err(|e| e.to_string())
}

#[tauri::command]
fn end_drag(app: AppHandle) {
    notch::end_drag(&app);
}

fn status(state: &AppState) -> Status {
    // Lock once: temporaries live until the end of the statement, so two
    // `config.lock()` in this struct literal would deadlock the main thread.
    let config = state.config.lock().unwrap().clone();
    Status {
        sound_enabled: config.sound_enabled,
        edge: config.placement.edge,
        style: config.placement.style,
        movable: state.movable.load(Ordering::Relaxed),
        layout: config.module_layout,
        speed: config.notch_speed,
        appearance: config.appearance,
        language: i18n::language(),
        anchor: notch::anchor(),
    }
}

fn content(state: &AppState) -> Content {
    // Clone and release: modules may lock the config while listing items.
    let config = state.config.lock().unwrap().clone();
    module::content(&state.modules, &config)
}

pub fn emit_content(app: &AppHandle) {
    let _ = app.emit("content-changed", content(&app.state::<AppState>()));
}

pub fn emit_status(app: &AppHandle) {
    let _ = app.emit("status-changed", status(&app.state::<AppState>()));
}

/// Starts winotch with the given modules (ADR-0009). `context` comes from
/// `tauri::generate_context!()` in the calling binary: its tauri.conf.json
/// decides the product, the front-end and the bundle.
pub fn run(context: tauri::Context, mut modules: Vec<Box<dyn Module>>) {
    tauri::Builder::default()
        // A second launch would fail to bind the hook port: bring the running
        // one forward instead, notch and settings (e.g. from the Start menu).
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            notch::set_user_visible(app, true);
            if let Err(e) = settings::open(app) {
                log::error!("cannot open settings: {e}");
            }
        }))
        .plugin(tauri_plugin_autostart::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            get_content,
            get_status,
            acknowledge,
            item_action,
            set_hit_area,
            watch_leave,
            cancel_leave,
            start_drag,
            drag,
            end_drag,
            settings::get_settings,
            settings::set_sound,
            settings::set_hide_in_fullscreen,
            settings::set_edge,
            settings::set_style,
            settings::set_gap,
            settings::set_screen,
            settings::set_movable,
            settings::recenter,
            settings::set_cursor_resistance,
            settings::set_autostart,
            settings::set_module_enabled,
            settings::set_module_order,
            settings::set_module_layout,
            settings::set_notch_speed,
            settings::set_appearance,
            settings::set_language,
            settings::set_module_pin,
            settings::module_call,
            context_menu::module_menu
        ])
        .setup(move |app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            let config_dir = app.path().app_config_dir()?;
            let config = config::load_or_create(&config_dir);
            for module in &modules {
                for (language, json) in module.locales() {
                    i18n::register(language, json);
                }
            }
            i18n::set_language(i18n::resolve(config.language.as_deref()));
            platform::set_resistance_strength(config.cursor_resistance_strength);
            let enabled: Vec<bool> = modules
                .iter()
                .map(|m| module::is_enabled(m.as_ref(), &config))
                .collect();
            app.manage(AppState {
                modules: std::mem::take(&mut modules),
                config: Mutex::new(config),
                config_dir,
                user_hidden: AtomicBool::new(false),
                movable: AtomicBool::new(false),
            });
            let state = app.state::<AppState>();
            for (module, enabled) in state.modules.iter().zip(enabled) {
                if enabled {
                    module.start(&Host::new(app.handle().clone(), module.id()));
                }
            }

            let window = app
                .get_webview_window(NOTCH_LABEL)
                .expect("notch window is declared in tauri.conf.json");
            notch::place(&window)?;
            notch::apply_hit_area(&window)?;
            notch::show(&window)?;
            notch::spawn_watcher(app.handle().clone());

            tray::create(app.handle())?;
            // The right-click menu of the modules (the tray has its own).
            app.on_menu_event(|app, event| context_menu::handle(app, &event));
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() != NOTCH_LABEL {
                return;
            }
            // On a DPI change the window still has its old size when this event fires,
            // and Windows then applies its own suggested rect. Re-center afterwards,
            // once the event loop is done with the resize: window calls made from
            // another thread are queued on the event loop instead of running inline.
            if matches!(
                event,
                WindowEvent::ScaleFactorChanged { .. } | WindowEvent::Resized(_)
            ) && let Some(w) = window.app_handle().get_webview_window(NOTCH_LABEL)
            {
                std::thread::spawn(move || {
                    if let Err(e) = notch::place(&w).and_then(|_| notch::apply_hit_area(&w)) {
                        log::warn!("failed to reposition notch: {e}");
                    }
                });
            }
        })
        .run(context)
        .expect("error while running winotch");
}
