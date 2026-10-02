mod claude_settings;
mod config;
mod notch;
mod platform;
mod server;
mod sessions;
mod tray;

use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, WindowEvent};

use config::Config;
use sessions::{Session, SessionStore};

pub const NOTCH_LABEL: &str = "notch";

pub struct AppState {
    pub sessions: Mutex<SessionStore>,
    pub config: Config,
    pub token: String,
    pub claude_settings: PathBuf,
    pub server_error: Option<String>,
    /// Hidden from the tray menu: fullscreen detection must not show it back.
    pub user_hidden: AtomicBool,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    server_error: Option<String>,
    hooks_installed: bool,
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

fn status(state: &AppState) -> Status {
    Status {
        server_error: state.server_error.clone(),
        hooks_installed: claude_settings::is_installed(&state.claude_settings),
    }
}

pub fn emit_sessions(app: &AppHandle) {
    let list = app.state::<AppState>().sessions.lock().unwrap().list();
    let _ = app.emit("sessions-changed", list);
}

pub fn emit_status(app: &AppHandle) {
    let _ = app.emit("status-changed", status(&app.state::<AppState>()));
}

/// Adds (`true`) or removes (`false`) winotch's hooks in the Claude Code settings.
pub fn connect_claude_code(app: &AppHandle, connect: bool) {
    let state = app.state::<AppState>();
    let result = if connect {
        claude_settings::install(
            &state.claude_settings,
            state.config.server_port,
            &state.token,
        )
    } else {
        claude_settings::uninstall(&state.claude_settings)
    };
    let message = match result {
        Ok(()) if connect => "Claude Code connecté".to_owned(),
        Ok(()) => "Claude Code déconnecté".to_owned(),
        Err(e) => {
            log::error!("cannot update {}: {e}", state.claude_settings.display());
            "Échec : voir les logs".to_owned()
        }
    };
    let _ = app.emit("notice", message);
    emit_status(app);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // A second launch would fail to bind the hook port: show the running one instead.
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            notch::set_user_visible(app, true);
        }))
        .plugin(tauri_plugin_autostart::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            get_sessions,
            get_status,
            acknowledge
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
            let server_error =
                server::spawn(app.handle().clone(), config.server_port, token.clone())
                    .err()
                    .map(|e| {
                        log::error!("hook server: {e}");
                        format!("Port {} occupé", config.server_port)
                    });
            let timeout = Duration::from_secs(config.session_timeout_minutes * 60);
            app.manage(AppState {
                sessions: Mutex::new(SessionStore::default()),
                config,
                token,
                claude_settings,
                server_error,
                user_hidden: AtomicBool::new(false),
            });
            spawn_pruner(app.handle().clone(), timeout);

            let window = app
                .get_webview_window(NOTCH_LABEL)
                .expect("notch window is declared in tauri.conf.json");
            notch::place_on_primary_monitor(&window)?;
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
                    if let Err(e) = notch::place_on_primary_monitor(&w) {
                        log::warn!("failed to reposition notch: {e}");
                    }
                });
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running winotch");
}

/// Removes sessions whose terminal died without sending `SessionEnd`.
fn spawn_pruner(app: AppHandle, timeout: Duration) {
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(Duration::from_secs(60));
            let changed = app
                .state::<AppState>()
                .sessions
                .lock()
                .unwrap()
                .prune(Instant::now(), timeout);
            if changed {
                emit_sessions(&app);
            }
        }
    });
}
