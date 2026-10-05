//! Claude Code module: shows the activity of Claude Code sessions, received
//! through HTTP hooks (docs/adr/0002-integration-claude-code-via-hooks.md).

mod hooks;
mod server;
mod sessions;

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::module::{Host, Item, Module, Tone};
use sessions::{Session, SessionState, SessionStore};

const TOKEN_FILE: &str = "hook-token";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct Settings {
    /// Local port the hooks post to (127.0.0.1 only).
    server_port: u16,
    /// Sessions silent for this long are considered dead and removed.
    session_timeout_minutes: u64,
    /// The hooks were removed when the module was disabled: put them back
    /// when it is enabled again.
    reconnect: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            server_port: 47821,
            session_timeout_minutes: 180,
            reconnect: false,
        }
    }
}

#[derive(Default)]
pub struct ClaudeCode {
    inner: Arc<Inner>,
}

#[derive(Default)]
struct Inner {
    host: OnceLock<Host>,
    /// Guards the hook server, generated once per installation.
    token: OnceLock<String>,
    /// Claude Code's settings.json, where the hooks are installed.
    hooks_path: OnceLock<PathBuf>,
    settings: Mutex<Settings>,
    sessions: Mutex<SessionStore>,
    /// Running hook server; replaced when the port changes.
    server: Mutex<Option<Arc<tiny_http::Server>>>,
    server_error: Mutex<Option<String>>,
    running: AtomicBool,
    pruner_started: AtomicBool,
}

impl Module for ClaudeCode {
    fn id(&self) -> &'static str {
        "claude-code"
    }

    fn name(&self) -> &'static str {
        "Claude Code"
    }

    fn description(&self) -> &'static str {
        "L'activité de tes sessions Claude Code, en temps réel."
    }

    fn start(&self, host: &Host) {
        let inner = &self.inner;
        let _ = inner.host.set(host.clone());
        if inner.token.get().is_none() {
            match load_or_create_token(&host.config_dir()) {
                Ok(token) => {
                    let _ = inner.token.set(token);
                }
                Err(e) => log::error!("cannot create hook token: {e}"),
            }
        }
        if let Some(home) = host.home_dir() {
            let _ = inner.hooks_path.set(hooks::settings_path(&home));
        }
        let settings: Settings = host.load_settings();
        *inner.settings.lock().unwrap() = settings.clone();

        let error = match Inner::start_server(inner, settings.server_port) {
            Ok(server) => {
                *inner.server.lock().unwrap() = Some(server);
                None
            }
            Err(e) => {
                log::error!("hook server: {e}");
                Some(format!("Port {} occupé", settings.server_port))
            }
        };
        *inner.server_error.lock().unwrap() = error;
        inner.running.store(true, Ordering::Relaxed);

        if settings.reconnect {
            let _ = inner.connect(true);
            inner.update_settings(|s| s.reconnect = false);
        }
        if !inner.pruner_started.swap(true, Ordering::Relaxed) {
            spawn_pruner(Arc::clone(inner));
        }
        host.refresh();
    }

    fn stop(&self) {
        let inner = &self.inner;
        inner.running.store(false, Ordering::Relaxed);
        if let Some(server) = inner.server.lock().unwrap().take() {
            server.unblock();
        }
        *inner.server_error.lock().unwrap() = None;
        *inner.sessions.lock().unwrap() = SessionStore::default();
        // Without a server, the hooks would fail on every Claude Code event.
        if inner.hooks_installed() && inner.connect(false).is_ok() {
            inner.update_settings(|s| s.reconnect = true);
        }
        if let Some(host) = inner.host.get() {
            host.refresh();
        }
    }

    fn items(&self) -> Vec<Item> {
        if !self.inner.running.load(Ordering::Relaxed) {
            return Vec::new();
        }
        self.inner
            .sessions
            .lock()
            .unwrap()
            .list()
            .iter()
            .map(item)
            .collect()
    }

    fn placeholder(&self) -> Option<String> {
        let error = self.inner.server_error.lock().unwrap().clone();
        Some(error.unwrap_or_else(|| {
            if self.inner.hooks_installed() {
                "Aucune session".into()
            } else {
                "Claude Code non connecté".into()
            }
        }))
    }

    fn acknowledge(&self) {
        let changed = self.inner.sessions.lock().unwrap().acknowledge();
        if changed && let Some(host) = self.inner.host.get() {
            host.refresh();
        }
    }

    fn settings(&self) -> Value {
        let inner = &self.inner;
        let settings = inner.settings.lock().unwrap().clone();
        json!({
            "hooksInstalled": inner.hooks_installed(),
            "hooksPath": inner.hooks_path.get().map(|p| p.display().to_string()),
            "serverPort": settings.server_port,
            "serverError": inner.server_error.lock().unwrap().clone(),
            "sessionTimeoutMinutes": settings.session_timeout_minutes,
        })
    }

    fn call(&self, action: &str, args: Value) -> Result<Value, String> {
        let inner = &self.inner;
        let result = match action {
            "connect" => {
                let connected = args["connected"].as_bool().ok_or("connected manquant")?;
                inner.connect(connected)
            }
            "set_port" => {
                let port = args["port"].as_u64().ok_or("port manquant")?;
                inner.set_port(u16::try_from(port).unwrap_or(0))
            }
            "set_session_timeout" => {
                let minutes = args["minutes"].as_u64().ok_or("minutes manquant")?;
                if !(5..=1440).contains(&minutes) {
                    return Err("Entre 5 et 1440 minutes".into());
                }
                inner.update_settings(|s| s.session_timeout_minutes = minutes);
                Ok(())
            }
            _ => return Err(format!("unknown action {action}")),
        };
        if let Some(host) = inner.host.get() {
            host.settings_changed();
            host.refresh();
        }
        result.map(|_| Value::Null)
    }
}

impl Inner {
    fn start_server(inner: &Arc<Inner>, port: u16) -> Result<Arc<tiny_http::Server>, String> {
        let token = inner.token.get().ok_or("no hook token")?;
        let events = Arc::clone(inner);
        server::start(port, token, move |event| {
            let changed = events.sessions.lock().unwrap().apply(event, Instant::now());
            if changed && let Some(host) = events.host.get() {
                host.refresh();
            }
        })
    }

    fn hooks_installed(&self) -> bool {
        self.hooks_path
            .get()
            .is_some_and(|p| hooks::is_installed(p))
    }

    fn update_settings(&self, edit: impl FnOnce(&mut Settings)) {
        let mut settings = self.settings.lock().unwrap();
        edit(&mut settings);
        if let Some(host) = self.host.get() {
            host.save_settings(&*settings);
        }
    }

    /// Adds (`true`) or removes (`false`) winotch's hooks in the Claude Code settings.
    fn connect(&self, connect: bool) -> Result<(), String> {
        let path = self
            .hooks_path
            .get()
            .ok_or("dossier personnel introuvable")?;
        let result = if connect {
            let port = self.settings.lock().unwrap().server_port;
            let token = self.token.get().ok_or("no hook token")?;
            hooks::install(path, port, token)
        } else {
            hooks::uninstall(path)
        };
        let message = match &result {
            Ok(()) if connect => "Claude Code connecté",
            Ok(()) => "Claude Code déconnecté",
            Err(e) => {
                log::error!("cannot update {}: {e}", path.display());
                "Échec : voir les logs"
            }
        };
        if let Some(host) = self.host.get() {
            host.notice(message);
        }
        result.map_err(|e| e.to_string())
    }

    /// Moves the hook server to another port without restarting winotch: the
    /// new port is opened first, so a busy port leaves everything as it was.
    fn set_port(self: &Arc<Self>, port: u16) -> Result<(), String> {
        if port < 1024 {
            return Err("Choisis un port entre 1024 et 65535".into());
        }
        let current = self.settings.lock().unwrap().server_port;
        let running = self.server.lock().unwrap().is_some();
        if port == current && running {
            return Ok(());
        }
        let new_server = Inner::start_server(self, port)
            .map_err(|e| format!("Port {port} indisponible : {e}"))?;
        if let Some(old) = self.server.lock().unwrap().replace(new_server) {
            old.unblock();
        }
        *self.server_error.lock().unwrap() = None;
        self.update_settings(|s| s.server_port = port);

        // The hooks contain the port: follow the change if Claude Code is connected.
        if self.hooks_installed() {
            self.connect(true)
        } else {
            Ok(())
        }
    }
}

fn item(session: &Session) -> Item {
    use SessionState::*;
    let (label, tone) = match session.state {
        Idle => ("En attente", Tone::Neutral),
        Working => ("Au travail", Tone::Active),
        NeedsPermission => ("Permission requise", Tone::Attention),
        WaitingInput => ("Attend ta réponse", Tone::Question),
        Done => ("Terminé", Tone::Success),
        Error => ("Erreur", Tone::Error),
    };
    Item {
        id: session.id.clone(),
        title: project_name(session.cwd.as_deref()),
        label: label.into(),
        detail: session.tool.clone(),
        tone,
        dot: true,
        actions: Vec::new(),
    }
}

/// Last folder of the session's working directory.
fn project_name(cwd: Option<&str>) -> String {
    cwd.and_then(|c| c.split(['/', '\\']).rfind(|s| !s.is_empty()))
        .unwrap_or("session")
        .to_owned()
}

/// Removes sessions whose terminal died without sending `SessionEnd`.
fn spawn_pruner(inner: Arc<Inner>) {
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(Duration::from_secs(60));
            if !inner.running.load(Ordering::Relaxed) {
                continue;
            }
            // Read every time: the delay can change in the settings.
            let minutes = inner.settings.lock().unwrap().session_timeout_minutes;
            let changed = inner
                .sessions
                .lock()
                .unwrap()
                .prune(Instant::now(), Duration::from_secs(minutes * 60));
            if changed && let Some(host) = inner.host.get() {
                host.refresh();
            }
        }
    });
}

/// Token guarding the local hook server, generated once per installation.
fn load_or_create_token(dir: &Path) -> io::Result<String> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_name_is_the_last_folder() {
        assert_eq!(project_name(Some("C:\\work\\winotch\\")), "winotch");
        assert_eq!(project_name(Some("/home/me/app")), "app");
        assert_eq!(project_name(None), "session");
        assert_eq!(project_name(Some("/")), "session");
    }
}
