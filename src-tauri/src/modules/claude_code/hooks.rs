//! Installs / removes Minim Notch's HTTP hooks in the user's Claude Code settings,
//! see docs/adr/0002-integration-claude-code-via-hooks.md.
//! Existing settings are merged, never overwritten, and backed up first.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value, json};

/// URL path that identifies Minim Notch's hooks among the user's own hooks.
pub const HOOK_PATH: &str = "/minim-notch/hook";
/// The path of the hooks installed when the app was still called winotch:
/// still ours, replaced by the current one at the next install.
const LEGACY_HOOK_PATH: &str = "/winotch/hook";

const EVENTS: &[&str] = &[
    "SessionStart",
    "SessionEnd",
    "UserPromptSubmit",
    "PreToolUse",
    "PostToolUse",
    "PostToolUseFailure",
    "PermissionRequest",
    "PermissionDenied",
    "Notification",
    "Stop",
    "StopFailure",
];

/// Claude Code's config dir: `$CLAUDE_CONFIG_DIR`, else `~/.claude`.
pub fn settings_path(home: &Path) -> PathBuf {
    let dir = std::env::var_os("CLAUDE_CONFIG_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".claude"));
    dir.join("settings.json")
}

pub fn install(path: &Path, port: u16, token: &str) -> io::Result<()> {
    let settings = read(path)?;
    write(path, with_hooks(settings, port, token))
}

pub fn uninstall(path: &Path) -> io::Result<()> {
    if !path.exists() {
        return Ok(());
    }
    let settings = read(path)?;
    write(path, without_hooks(settings))
}

pub fn is_installed(path: &Path) -> bool {
    read(path).is_ok_and(|s| {
        s.get("hooks")
            .and_then(Value::as_object)
            .is_some_and(|hooks| hooks.values().any(contains_ours))
    })
}

/// Hooks installed under the old name (`LEGACY_HOOK_PATH`) are there.
pub fn has_legacy(path: &Path) -> bool {
    fs::read_to_string(path).is_ok_and(|text| text.contains(LEGACY_HOOK_PATH))
}

fn read(path: &Path) -> io::Result<Value> {
    match fs::read_to_string(path) {
        Ok(text) if text.trim().is_empty() => Ok(json!({})),
        // Refuse to touch a file we cannot parse rather than risk losing the user's settings.
        Ok(text) => serde_json::from_str(&text).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{} is not valid JSON: {e}", path.display()),
            )
        }),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(json!({})),
        Err(e) => Err(e),
    }
}

fn write(path: &Path, settings: Value) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    // One backup, the settings as they were before Minim Notch's last change.
    if path.exists() {
        fs::copy(path, path.with_extension("json.minim-notch-backup"))?;
        remove_old_backups(path);
    }
    let text = serde_json::to_string_pretty(&settings).map_err(io::Error::other)? + "\n";
    let tmp = path.with_extension("json.minim-notch-tmp");
    fs::write(&tmp, text)?;
    fs::rename(&tmp, path)
}

/// Removes the dated backups old versions left at each change, when the app
/// was still called winotch (`settings.json.winotch-backup-<seconds>`);
/// never anyone else's files.
fn remove_old_backups(path: &Path) {
    let (Some(dir), Some(name)) = (path.parent(), path.file_name()) else {
        return;
    };
    let prefix = format!("{}.winotch-backup-", name.to_string_lossy());
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let file = entry.file_name();
        let file = file.to_string_lossy();
        let dated = file
            .strip_prefix(&prefix)
            .is_some_and(|stamp| !stamp.is_empty() && stamp.bytes().all(|b| b.is_ascii_digit()));
        if dated && let Err(e) = fs::remove_file(entry.path()) {
            log::warn!("cannot remove the old backup {file}: {e}");
        }
    }
}

fn is_ours(handler: &Value) -> bool {
    handler
        .get("url")
        .and_then(Value::as_str)
        .is_some_and(|url| {
            url.starts_with("http://127.0.0.1:")
                && (url.ends_with(HOOK_PATH) || url.ends_with(LEGACY_HOOK_PATH))
        })
}

fn contains_ours(groups: &Value) -> bool {
    groups.as_array().is_some_and(|groups| {
        groups.iter().any(|g| {
            g.get("hooks")
                .and_then(Value::as_array)
                .is_some_and(|h| h.iter().any(is_ours))
        })
    })
}

/// Returns `settings` with Minim Notch's hooks (re)installed for the given port and token.
pub fn with_hooks(settings: Value, port: u16, token: &str) -> Value {
    let mut settings = without_hooks(settings);
    let root = as_object(&mut settings);
    let hooks = root
        .entry("hooks")
        .or_insert_with(|| Value::Object(Map::new()));
    let hooks = as_object(hooks);
    for event in EVENTS {
        let groups = hooks
            .entry(*event)
            .or_insert_with(|| Value::Array(Vec::new()));
        if !groups.is_array() {
            *groups = Value::Array(Vec::new());
        }
        groups.as_array_mut().unwrap().push(json!({
            "hooks": [{
                "type": "http",
                "url": format!("http://127.0.0.1:{port}{HOOK_PATH}"),
                "headers": { "Authorization": format!("Bearer {token}") },
                "timeout": 2
            }]
        }));
    }
    settings
}

/// Returns `settings` without any Minim Notch hook, leaving the user's own hooks intact.
pub fn without_hooks(mut settings: Value) -> Value {
    let Some(hooks) = settings.get_mut("hooks").and_then(Value::as_object_mut) else {
        return settings;
    };
    for groups in hooks.values_mut() {
        let Some(list) = groups.as_array_mut() else {
            continue;
        };
        for group in list.iter_mut() {
            if let Some(handlers) = group.get_mut("hooks").and_then(Value::as_array_mut) {
                handlers.retain(|h| !is_ours(h));
            }
        }
        list.retain(|g| {
            g.get("hooks")
                .and_then(Value::as_array)
                .is_none_or(|h| !h.is_empty())
        });
    }
    hooks.retain(|_, groups| groups.as_array().is_none_or(|g| !g.is_empty()));
    if hooks.is_empty() {
        settings.as_object_mut().unwrap().remove("hooks");
    }
    settings
}

fn as_object(value: &mut Value) -> &mut Map<String, Value> {
    if !value.is_object() {
        *value = Value::Object(Map::new());
    }
    value.as_object_mut().unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn user_settings() -> Value {
        json!({
            "model": "opus",
            "hooks": {
                "PreToolUse": [
                    { "matcher": "Bash", "hooks": [{ "type": "command", "command": "my-check.sh" }] }
                ]
            }
        })
    }

    #[test]
    fn install_keeps_user_hooks_and_settings() {
        let s = with_hooks(user_settings(), 1234, "tok");
        assert_eq!(s["model"], "opus");
        let pre = s["hooks"]["PreToolUse"].as_array().unwrap();
        assert_eq!(pre.len(), 2);
        assert_eq!(pre[0]["hooks"][0]["command"], "my-check.sh");
        assert_eq!(
            pre[1]["hooks"][0]["url"],
            "http://127.0.0.1:1234/minim-notch/hook"
        );
        assert_eq!(pre[1]["hooks"][0]["headers"]["Authorization"], "Bearer tok");
        assert_eq!(s["hooks"].as_object().unwrap().len(), EVENTS.len());
    }

    #[test]
    fn reinstall_does_not_duplicate() {
        let once = with_hooks(user_settings(), 1234, "tok");
        let twice = with_hooks(once.clone(), 5678, "new");
        assert_eq!(twice["hooks"]["PreToolUse"].as_array().unwrap().len(), 2);
        assert_eq!(
            twice["hooks"]["Stop"][0]["hooks"][0]["url"],
            "http://127.0.0.1:5678/minim-notch/hook"
        );
    }

    #[test]
    fn hooks_installed_under_the_old_name_are_replaced() {
        let mut old = with_hooks(user_settings(), 1234, "tok");
        let text = serde_json::to_string(&old)
            .unwrap()
            .replace(HOOK_PATH, LEGACY_HOOK_PATH);
        old = serde_json::from_str(&text).unwrap();
        assert!(
            old["hooks"]
                .as_object()
                .unwrap()
                .values()
                .any(contains_ours)
        );
        let new = with_hooks(old, 1234, "tok");
        assert_eq!(new["hooks"]["PreToolUse"].as_array().unwrap().len(), 2);
        assert!(
            !serde_json::to_string(&new)
                .unwrap()
                .contains(LEGACY_HOOK_PATH)
        );
    }

    #[test]
    fn uninstall_restores_original() {
        let original = user_settings();
        let s = without_hooks(with_hooks(original.clone(), 1234, "tok"));
        assert_eq!(s, original);
    }

    #[test]
    fn uninstall_removes_empty_hooks_key() {
        let s = without_hooks(with_hooks(json!({ "model": "opus" }), 1234, "tok"));
        assert_eq!(s, json!({ "model": "opus" }));
    }

    #[test]
    fn install_and_uninstall_on_disk() {
        let dir = std::env::temp_dir().join(format!("minim-notch-test-{}", std::process::id()));
        let path = dir.join("settings.json");
        fs::create_dir_all(&dir).unwrap();
        fs::write(&path, serde_json::to_string(&user_settings()).unwrap()).unwrap();

        install(&path, 1234, "tok").unwrap();
        assert!(is_installed(&path));
        uninstall(&path).unwrap();
        assert!(!is_installed(&path));
        assert_eq!(read(&path).unwrap(), user_settings());

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn one_backup_is_kept_and_old_dated_ones_removed() {
        let dir =
            std::env::temp_dir().join(format!("minim-notch-test-backup-{}", std::process::id()));
        let path = dir.join("settings.json");
        fs::create_dir_all(&dir).unwrap();
        fs::write(&path, serde_json::to_string(&user_settings()).unwrap()).unwrap();
        for old in [
            "settings.json.winotch-backup-1790961255",
            "settings.json.winotch-backup-17",
        ] {
            fs::write(dir.join(old), "{}").unwrap();
        }
        // Not the old dated backups: left alone.
        for other in [
            "settings.json.bak.20260813",
            "settings.json.winotch-backup-notes",
        ] {
            fs::write(dir.join(other), "{}").unwrap();
        }

        install(&path, 1234, "tok").unwrap();
        install(&path, 1234, "tok").unwrap();

        let mut files: Vec<String> = fs::read_dir(&dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        files.sort();
        assert_eq!(
            files,
            [
                "settings.json",
                "settings.json.bak.20260813",
                "settings.json.minim-notch-backup",
                "settings.json.winotch-backup-notes",
            ]
        );

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn invalid_json_is_left_untouched() {
        let dir = std::env::temp_dir().join(format!("minim-notch-test-bad-{}", std::process::id()));
        let path = dir.join("settings.json");
        fs::create_dir_all(&dir).unwrap();
        fs::write(&path, "{ not json").unwrap();

        assert!(install(&path, 1234, "tok").is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), "{ not json");

        fs::remove_dir_all(&dir).unwrap();
    }
}
