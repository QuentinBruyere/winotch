//! Claude Code module: shows the activity of Claude Code sessions, received
//! through HTTP hooks (docs/adr/0002-integration-claude-code-via-hooks.md).

mod hooks;
mod server;
mod sessions;

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::companion;
use crate::module::{Host, Item, ItemMenu, MenuChoice, Module, Tone};
use sessions::{Session, SessionState, SessionStore};

const TOKEN_FILE: &str = "hook-token";
/// Item menu choices (DF-0020): `companion:<id>`, or the dot; then
/// `color:<id>`, or the state's color.
const COMPANION: &str = "companion:";
const DOT: &str = "dot";
const COLOR: &str = "color:";
const STATE_COLOR: &str = "state-color";

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
    /// The companion drawn instead of the dot, by project folder (DF-0020).
    companions: BTreeMap<String, String>,
    /// How big the companions are drawn.
    companion_size: companion::Size,
    /// The companion's own color, by project folder (DF-0020).
    companion_colors: BTreeMap<String, String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            server_port: 47821,
            session_timeout_minutes: 180,
            reconnect: false,
            companions: BTreeMap::new(),
            companion_size: companion::Size::default(),
            companion_colors: BTreeMap::new(),
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

    fn name(&self) -> String {
        crate::t!("claude-code.name")
    }

    fn description(&self) -> String {
        crate::t!("claude-code.description")
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
                Some(crate::t!(
                    "claude-code.port_busy",
                    port = settings.server_port
                ))
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
        let settings = self.inner.settings.lock().unwrap().clone();
        self.inner
            .sessions
            .lock()
            .unwrap()
            .list()
            .iter()
            .map(|session| item(session, &settings))
            .collect()
    }

    fn placeholder(&self) -> Option<String> {
        let error = self.inner.server_error.lock().unwrap().clone();
        Some(error.unwrap_or_else(|| {
            if self.inner.hooks_installed() {
                crate::t!("claude-code.no_session")
            } else {
                crate::t!("claude-code.not_connected")
            }
        }))
    }

    fn acknowledge(&self) {
        let changed = self.inner.sessions.lock().unwrap().acknowledge();
        if changed && let Some(host) = self.inner.host.get() {
            host.refresh();
        }
    }

    /// Only the clicked session: the others keep their result.
    fn acknowledge_item(&self, item: &str) {
        let changed = self.inner.sessions.lock().unwrap().acknowledge_one(item);
        if changed && let Some(host) = self.inner.host.get() {
            host.refresh();
        }
    }

    /// The session's companion and its color, chosen for its whole project
    /// (DF-0020).
    fn item_menu(&self, item: &str) -> Vec<ItemMenu> {
        let Some(cwd) = self.inner.session_cwd(item) else {
            return Vec::new();
        };
        let settings = self.inner.settings.lock().unwrap().clone();
        let current = settings.companions.get(&cwd);
        let mut menus = vec![companion_menu(current.map(String::as_str))];
        if current.is_some() {
            let color = settings.companion_colors.get(&cwd);
            menus.push(color_menu(color.map(String::as_str)));
        }
        menus
    }

    fn item_action(&self, item: &str, action: &str) {
        let inner = &self.inner;
        let Some(cwd) = inner.session_cwd(item) else {
            return;
        };
        if let Some(id) = action.strip_prefix(COMPANION) {
            if !companion::BUILT_IN.contains(&id) {
                return;
            }
            inner.update_settings(|s| {
                s.companions.insert(cwd, id.to_owned());
            });
        } else if action == DOT {
            inner.update_settings(|s| {
                s.companions.remove(&cwd);
            });
        } else if let Some(id) = action.strip_prefix(COLOR) {
            if !companion::COLORS.contains(&id) {
                return;
            }
            inner.update_settings(|s| {
                s.companion_colors.insert(cwd, id.to_owned());
            });
        } else if action == STATE_COLOR {
            inner.update_settings(|s| {
                s.companion_colors.remove(&cwd);
            });
        } else {
            return;
        }
        if let Some(host) = inner.host.get() {
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
            "companionSize": settings.companion_size,
        })
    }

    fn call(&self, action: &str, args: Value) -> Result<Value, String> {
        let inner = &self.inner;
        let result = match action {
            "connect" => {
                let connected = args["connected"]
                    .as_bool()
                    .ok_or_else(|| crate::t!("claude-code.missing_connected"))?;
                inner.connect(connected)
            }
            "set_port" => {
                let port = args["port"]
                    .as_u64()
                    .ok_or_else(|| crate::t!("claude-code.missing_port"))?;
                inner.set_port(u16::try_from(port).unwrap_or(0))
            }
            "set_session_timeout" => {
                let minutes = args["minutes"]
                    .as_u64()
                    .ok_or_else(|| crate::t!("claude-code.missing_minutes"))?;
                if !(5..=1440).contains(&minutes) {
                    return Err(crate::t!("claude-code.timeout_range"));
                }
                inner.update_settings(|s| s.session_timeout_minutes = minutes);
                Ok(())
            }
            "set_companion_size" => {
                let size = serde_json::from_value(args["size"].clone())
                    .map_err(|_| crate::t!("claude-code.missing_size"))?;
                inner.update_settings(|s| s.companion_size = size);
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

    /// The project folder of a session, if Claude Code told it.
    fn session_cwd(&self, id: &str) -> Option<String> {
        self.sessions.lock().unwrap().get(id)?.cwd.clone()
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
            .ok_or_else(|| crate::t!("claude-code.no_home"))?;
        let result = if connect {
            let port = self.settings.lock().unwrap().server_port;
            let token = self.token.get().ok_or("no hook token")?;
            hooks::install(path, port, token)
        } else {
            hooks::uninstall(path)
        };
        let message = match &result {
            Ok(()) if connect => crate::t!("claude-code.connected"),
            Ok(()) => crate::t!("claude-code.disconnected"),
            Err(e) => {
                log::error!("cannot update {}: {e}", path.display());
                crate::t!("claude-code.failed")
            }
        };
        if let Some(host) = self.host.get() {
            host.notice(&message);
        }
        result.map_err(|e| e.to_string())
    }

    /// Moves the hook server to another port without restarting winotch: the
    /// new port is opened first, so a busy port leaves everything as it was.
    fn set_port(self: &Arc<Self>, port: u16) -> Result<(), String> {
        if port < 1024 {
            return Err(crate::t!("claude-code.port_range"));
        }
        let current = self.settings.lock().unwrap().server_port;
        let running = self.server.lock().unwrap().is_some();
        if port == current && running {
            return Ok(());
        }
        let new_server = Inner::start_server(self, port)
            .map_err(|e| crate::t!("claude-code.port_unavailable", port = port, error = e))?;
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

/// The companion submenu: the dot, then each companion; `current` ticked.
fn companion_menu(current: Option<&str>) -> ItemMenu {
    let mut choices = vec![MenuChoice {
        id: DOT.into(),
        label: crate::t!("claude-code.menu.dot"),
        checked: current.is_none(),
    }];
    choices.extend(companion::BUILT_IN.iter().map(|id| MenuChoice {
        id: format!("{COMPANION}{id}"),
        label: companion::name(id),
        checked: current == Some(*id),
    }));
    ItemMenu {
        title: crate::t!("claude-code.menu.companion"),
        choices,
    }
}

/// The color submenu: the state's color, then the palette; `current` ticked.
fn color_menu(current: Option<&str>) -> ItemMenu {
    let mut choices = vec![MenuChoice {
        id: STATE_COLOR.into(),
        label: crate::t!("claude-code.menu.default_color"),
        checked: current.is_none(),
    }];
    choices.extend(companion::COLORS.iter().map(|id| MenuChoice {
        id: format!("{COLOR}{id}"),
        label: companion::color_name(id),
        checked: current == Some(*id),
    }));
    ItemMenu {
        title: crate::t!("claude-code.menu.color"),
        choices,
    }
}

fn item(session: &Session, settings: &Settings) -> Item {
    use SessionState::*;
    let (key, tone) = match session.state {
        Idle => ("claude-code.state.idle", Tone::Neutral),
        Working => ("claude-code.state.working", Tone::Active),
        NeedsPermission => ("claude-code.state.permission", Tone::Attention),
        WaitingInput => ("claude-code.state.waiting", Tone::Question),
        Done => ("claude-code.state.done", Tone::Success),
        Error => ("claude-code.state.error", Tone::Error),
    };
    let label = crate::t!(key);
    Item {
        id: session.id.clone(),
        title: project_name(session.cwd.as_deref()),
        label,
        detail: session.tool.clone(),
        tone,
        dot: true,
        companion: session
            .cwd
            .as_ref()
            .and_then(|cwd| settings.companions.get(cwd))
            .map(|id| companion::Choice {
                id: id.clone(),
                size: settings.companion_size,
                color: session
                    .cwd
                    .as_ref()
                    .and_then(|cwd| settings.companion_colors.get(cwd))
                    // A color dropped from the palette: the state's again.
                    .filter(|color| companion::COLORS.contains(&color.as_str()))
                    .cloned(),
            }),
        ..Item::default()
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

    fn session(cwd: Option<&str>) -> Session {
        let event: sessions::HookEvent = serde_json::from_value(json!({
            "session_id": "s1",
            "hook_event_name": "UserPromptSubmit",
            "cwd": cwd,
        }))
        .unwrap();
        let mut store = SessionStore::default();
        store.apply(&event, Instant::now());
        store.list().remove(0)
    }

    #[test]
    fn a_project_s_companion_replaces_the_dot_of_its_sessions() {
        let settings = Settings {
            companions: BTreeMap::from([("/work/app".to_owned(), "bloop".to_owned())]),
            companion_size: companion::Size::Large,
            companion_colors: BTreeMap::from([("/work/app".to_owned(), "pink".to_owned())]),
            ..Settings::default()
        };
        let item_of = |cwd| item(&session(cwd), &settings).companion;
        assert_eq!(
            item_of(Some("/work/app")),
            Some(companion::Choice {
                id: "bloop".into(),
                size: companion::Size::Large,
                color: Some("pink".into()),
            })
        );
        assert_eq!(item_of(Some("/work/other")), None);
        assert_eq!(item_of(None), None);
    }

    #[test]
    fn the_companion_menu_ticks_the_current_choice() {
        let ticked = |current| {
            companion_menu(current)
                .choices
                .into_iter()
                .filter(|c| c.checked)
                .map(|c| c.id)
                .collect::<Vec<_>>()
        };
        assert_eq!(ticked(None), [DOT]);
        assert_eq!(ticked(Some("bloop")), ["companion:bloop"]);
        let ticked_color = |current| {
            color_menu(current)
                .choices
                .into_iter()
                .filter(|c| c.checked)
                .map(|c| c.id)
                .collect::<Vec<_>>()
        };
        assert_eq!(ticked_color(None), [STATE_COLOR]);
        assert_eq!(ticked_color(Some("teal")), ["color:teal"]);
    }
}
