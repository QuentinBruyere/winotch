mod claude_settings;
mod config;
mod notch;
mod placement;
mod platform;
mod resistance;
mod server;
mod sessions;
mod settings;
mod tray;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, WindowEvent};

use config::Config;
use sessions::{Session, SessionStore};

pub const NOTCH_LABEL: &str = "notch";

pub struct AppState {
    pub sessions: Mutex<SessionStore>,
    pub config: Mutex<Config>,
    pub config_dir: PathBuf,
    pub token: String,
    pub claude_settings: PathBuf,
    /// Running hook server; replaced when the port changes.
    pub server: Mutex<Option<Arc<tiny_http::Server>>>,
    pub server_error: Mutex<Option<String>>,
    /// Hidden from the tray menu: fullscreen detection must not show it back.
    pub user_hidden: AtomicBool,
    /// Move mode, on while the settings ask for it (DF-0006, step 3).
    pub movable: AtomicBool,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    server_error: Option<String>,
    hooks_installed: bool,
    sound_enabled: bool,
    /// Screen edge the notch is attached to: drives its orientation.
    edge: placement::Edge,
    /// Move mode: the notch can be dragged along its edge.
    movable: bool,
    /// Center of the compact notch along the edge, from the start of the
    /// window, in logical pixels: every shape is centered on it, then kept
    /// inside the window. `None` = the middle of the window.
    anchor: Option<f64>,
}

#[tauri::command]
fn get_sessions(state: tauri::State<AppState>) -> Vec<Session> {
    state.sessions.lock().unwrap().list()
}

#[tauri::command]
fn get_status(state: tauri::State<AppState>) -> Status {
    status(&state)
}

/// Clicking the notch means the user has seen the finished sessions.
#[tauri::command]
fn acknowledge(app: AppHandle, state: tauri::State<AppState>) {
    if state.sessions.lock().unwrap().acknowledge() {
        emit_sessions(&app);
    }
}

/// The front decides the visible notch shape (compact / expanded, DF-0003).
#[tauri::command]
fn set_hit_area(
    window: tauri::WebviewWindow,
    width: f64,
    height: f64,
    radius: f64,
) -> Result<(), String> {
    notch::set_hit_area(&window, width, height, radius).map_err(|e| e.to_string())
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
        server_error: state.server_error.lock().unwrap().clone(),
        hooks_installed: claude_settings::is_installed(&state.claude_settings),
        sound_enabled: config.sound_enabled,
        edge: config.placement.edge,
        movable: state.movable.load(Ordering::Relaxed),
        anchor: notch::anchor(),
    }
}

pub fn emit_sessions(app: &AppHandle) {
    let list = app.state::<AppState>().sessions.lock().unwrap().list();
    let _ = app.emit("sessions-changed", list);
}

pub fn emit_status(app: &AppHandle) {
    let _ = app.emit("status-changed", status(&app.state::<AppState>()));
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
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
            get_sessions,
            get_status,
            acknowledge,
            set_hit_area,
            start_drag,
            drag,
            end_drag,
            settings::get_settings,
            settings::set_sound,
            settings::set_hide_in_fullscreen,
            settings::set_edge,
            settings::set_screen,
            settings::set_movable,
            settings::recenter,
            settings::set_cursor_resistance,
            settings::set_session_timeout,
            settings::set_autostart,
            settings::set_claude_connected,
            settings::set_server_port
        ])
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            let config_dir = app.path().app_config_dir()?;
            let config = config::load_or_create(&config_dir);
            let token = config::load_or_create_token(&config_dir)?;
            let claude_settings = claude_settings::settings_path(&app.path().home_dir()?);
            let (server, server_error) =
                match server::start(app.handle().clone(), config.server_port, token.clone()) {
                    Ok(server) => (Some(server), None),
                    Err(e) => {
                        log::error!("hook server: {e}");
                        (None, Some(format!("Port {} occupé", config.server_port)))
                    }
                };
            platform::set_resistance_strength(config.cursor_resistance_strength);
            app.manage(AppState {
                sessions: Mutex::new(SessionStore::default()),
                config: Mutex::new(config),
                config_dir,
                token,
                claude_settings,
                server: Mutex::new(server),
                server_error: Mutex::new(server_error),
                user_hidden: AtomicBool::new(false),
                movable: AtomicBool::new(false),
            });
            spawn_pruner(app.handle().clone());

            let window = app
                .get_webview_window(NOTCH_LABEL)
                .expect("notch window is declared in tauri.conf.json");
            notch::place(&window)?;
            notch::apply_hit_area(&window)?;
            notch::show(&window)?;
            notch::spawn_watcher(app.handle().clone());

            tray::create(app.handle())?;
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
        .run(tauri::generate_context!())
        .expect("error while running winotch");
}

/// Removes sessions whose terminal died without sending `SessionEnd`.
fn spawn_pruner(app: AppHandle) {
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(Duration::from_secs(60));
            let state = app.state::<AppState>();
            // Read every time: the delay can change in the settings.
            let minutes = state.config.lock().unwrap().session_timeout_minutes;
            let changed = state
                .sessions
                .lock()
                .unwrap()
                .prune(Instant::now(), Duration::from_secs(minutes * 60));
            if changed {
                emit_sessions(&app);
            }
        }
    });
}
