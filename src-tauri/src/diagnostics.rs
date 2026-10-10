//! Log file and crash reports (ADR-0014): the log is written by installed
//! builds too, without the user's home folder in it; a panic leaves a marker
//! next to it, and the next start offers to open the log or copy a report.
//! Nothing is ever sent from here.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};
use tauri::plugin::TauriPlugin;
use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_log::{RotationStrategy, Target, TargetKind};

use crate::AppState;

/// The log file's name, without extension: development builds keep their
/// own, so they never report the installed app's crashes (or the reverse).
const LOG_NAME: &str = if cfg!(debug_assertions) {
    "minim-notch-dev"
} else {
    "minim-notch"
};
/// Above this size the log starts a new file; the previous ones are kept.
const MAX_LOG_BYTES: u128 = 1_000_000;
const KEPT_LOGS: usize = 2;
/// How much of the log a report carries.
const REPORT_LINES: usize = 200;

/// The last panic, as the marker file keeps it until the next start.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Crash {
    /// RFC 3339, local time.
    pub at: String,
    pub version: String,
    pub thread: String,
    /// `file:line` in the source code.
    pub location: String,
    pub message: String,
}

/// The logger: the console in development, and the log file, rotated.
pub fn log_plugin<R: Runtime>() -> TauriPlugin<R> {
    tauri_plugin_log::Builder::new()
        .clear_targets()
        .targets([
            Target::new(TargetKind::Stdout),
            Target::new(TargetKind::LogDir {
                file_name: Some(LOG_NAME.into()),
            }),
        ])
        .level(log::LevelFilter::Info)
        .max_file_size(MAX_LOG_BYTES)
        .rotation_strategy(RotationStrategy::KeepSome(KEPT_LOGS))
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{}][{}][{}] {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.target(),
                record.level(),
                redact(&message.to_string(), home())
            ))
        })
        .build()
}

/// The log file of this build.
pub fn log_file(log_dir: &Path) -> PathBuf {
    log_dir.join(format!("{LOG_NAME}.log"))
}

fn crash_file(log_dir: &Path) -> PathBuf {
    log_dir.join(format!("{LOG_NAME}.crash"))
}

/// Logs every panic, wherever it happens, and leaves the crash marker read
/// at the next start. Rust's own message still follows.
pub fn install_panic_hook(log_dir: PathBuf, version: String) {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let payload = info.payload();
        let message = payload
            .downcast_ref::<&str>()
            .map(|s| (*s).to_owned())
            .or_else(|| payload.downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "unknown panic".into());
        let crash = Crash {
            at: chrono::Local::now().to_rfc3339(),
            version: version.clone(),
            thread: std::thread::current()
                .name()
                .unwrap_or("unnamed")
                .to_owned(),
            location: info
                .location()
                .map(|l| redact(&format!("{}:{}", l.file(), l.line()), home()))
                .unwrap_or_default(),
            message: redact(&message, home()),
        };
        log::error!(
            "panic in thread {} at {}: {}",
            crash.thread,
            crash.location,
            crash.message
        );
        if let Ok(json) = serde_json::to_string(&crash) {
            let _ = fs::create_dir_all(&log_dir);
            let _ = fs::write(crash_file(&log_dir), json);
        }
        previous(info);
    }));
}

/// Development only: `MINIM_NOTCH_TEST_PANIC` set, a thread panics right away,
/// to try the crash report (the app itself keeps running).
pub fn test_panic() {
    if cfg!(debug_assertions) && std::env::var_os("MINIM_NOTCH_TEST_PANIC").is_some() {
        let _ = std::thread::Builder::new()
            .name("test-panic".into())
            .spawn(|| panic!("test panic asked by MINIM_NOTCH_TEST_PANIC"));
    }
}

/// The crash left by the previous run, if any; the marker is removed, so it
/// is reported once.
pub fn take_crash(log_dir: &Path) -> Option<Crash> {
    let path = crash_file(log_dir);
    let json = fs::read_to_string(&path).ok()?;
    if let Err(e) = fs::remove_file(&path) {
        log::warn!("cannot remove the crash marker: {e}");
    }
    serde_json::from_str(&json)
        .inspect_err(|e| log::warn!("unreadable crash marker: {e}"))
        .ok()
}

/// The user's home folder, replaced by `~` in everything logged.
fn home() -> Option<&'static str> {
    static HOME: OnceLock<Option<String>> = OnceLock::new();
    HOME.get_or_init(|| {
        let var = if cfg!(windows) { "USERPROFILE" } else { "HOME" };
        std::env::var(var).ok().filter(|h| h.len() > 1)
    })
    .as_deref()
}

/// `text` with the home folder replaced by `~`, written with either slash.
fn redact(text: &str, home: Option<&str>) -> String {
    let Some(home) = home else {
        return text.to_owned();
    };
    let home = home.trim_end_matches(['/', '\\']);
    let mut text = text.to_owned();
    for variant in [home.to_owned(), home.replace('\\', "/")] {
        text = text.replace(&variant, "~");
    }
    text
}

/// What a report carries: version, system, the crash if any, the end of the
/// log. Shown to the user before they send it anywhere.
fn report(version: &str, system: &str, crash: Option<&Crash>, log: &str) -> String {
    let mut text = format!("Minim Notch {version}\nSystem: {system}\n");
    if let Some(crash) = crash {
        text.push_str(&format!(
            "Crash: {} (version {}), thread {} at {}\n{}\n",
            crash.at, crash.version, crash.thread, crash.location, crash.message
        ));
    }
    let lines: Vec<&str> = log.lines().collect();
    let tail = &lines[lines.len().saturating_sub(REPORT_LINES)..];
    text.push_str("\n--- Log ---\n");
    text.push_str(&tail.join("\n"));
    text
}

fn system() -> String {
    format!(
        "{} ({})",
        crate::platform::os_version(),
        std::env::consts::ARCH
    )
}

/// Opens the log file in the system's text viewer.
#[tauri::command]
pub fn open_log(app: AppHandle) -> Result<(), String> {
    let file = log_file(&app.state::<AppState>().log_dir);
    if !file.exists() {
        return Err(crate::t!("settings.log.missing"));
    }
    tauri_plugin_opener::open_path(&file, None::<&str>).map_err(|e| {
        log::warn!("cannot open the log: {e}");
        crate::t!("settings.log.open_failed")
    })
}

/// Shows the log file in its folder.
#[tauri::command]
pub fn reveal_log(app: AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    let file = log_file(&state.log_dir);
    let result = if file.exists() {
        tauri_plugin_opener::reveal_item_in_dir(&file)
    } else {
        tauri_plugin_opener::open_path(&state.log_dir, None::<&str>)
    };
    result.map_err(|e| {
        log::warn!("cannot show the log folder: {e}");
        crate::t!("settings.log.open_failed")
    })
}

/// The report the user may copy and send.
#[tauri::command]
pub fn crash_report(app: AppHandle) -> String {
    let state = app.state::<AppState>();
    let crash = state.crash.lock().unwrap().clone();
    let log = fs::read_to_string(log_file(&state.log_dir)).unwrap_or_default();
    report(
        &app.package_info().version.to_string(),
        &system(),
        crash.as_ref(),
        &log,
    )
}

/// The user has seen the crash: it is no longer shown.
#[tauri::command]
pub fn dismiss_crash(app: AppHandle) {
    *app.state::<AppState>().crash.lock().unwrap() = None;
    crate::settings::changed(&app);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_home_folder_is_hidden_whatever_the_slashes() {
        let home = Some("C:\\Users\\jane");
        assert_eq!(
            redact("cannot read C:\\Users\\jane\\.claude\\settings.json", home),
            "cannot read ~\\.claude\\settings.json"
        );
        assert_eq!(
            redact("at C:/Users/jane/.cargo/registry/x.rs:3", home),
            "at ~/.cargo/registry/x.rs:3"
        );
        assert_eq!(redact("nothing personal", home), "nothing personal");
        assert_eq!(redact("C:\\Users\\jane", None), "C:\\Users\\jane");
    }

    #[test]
    fn a_report_holds_the_crash_and_only_the_end_of_the_log() {
        let crash = Crash {
            at: "2026-10-09T12:00:00+02:00".into(),
            version: "0.5.0".into(),
            thread: "main".into(),
            location: "src\\notch.rs:42".into(),
            message: "boom".into(),
        };
        let log: String = (0..REPORT_LINES + 50)
            .map(|i| format!("line {i}\n"))
            .collect();
        let text = report("0.5.1", "Windows 11", Some(&crash), &log);
        assert!(text.starts_with("Minim Notch 0.5.1\nSystem: Windows 11\n"));
        assert!(text.contains("thread main at src\\notch.rs:42\nboom"));
        assert!(!text.contains("line 49\n"));
        assert!(text.contains("line 50\n"));
        assert!(text.ends_with(&format!("line {}", REPORT_LINES + 49)));
    }

    #[test]
    fn a_report_without_crash_still_carries_the_log() {
        let text = report("0.5.1", "Windows 10", None, "only line");
        assert!(!text.contains("Crash:"));
        assert!(text.ends_with("--- Log ---\nonly line"));
    }

    #[test]
    fn the_crash_marker_is_read_once() {
        let dir = std::env::temp_dir().join(format!("minim-notch-crash-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let crash = Crash {
            at: "now".into(),
            version: "1".into(),
            thread: "main".into(),
            location: "here".into(),
            message: "boom".into(),
        };
        fs::write(crash_file(&dir), serde_json::to_string(&crash).unwrap()).unwrap();
        assert_eq!(take_crash(&dir), Some(crash));
        assert_eq!(take_crash(&dir), None);
        let _ = fs::remove_dir_all(&dir);
    }
}
