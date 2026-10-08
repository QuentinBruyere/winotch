//! Claude Code session tracking, see docs/fonctionnel/DF-0001-etats-du-notch.md.
//! Pure logic, no I/O: hook events in, session list out.

use std::collections::HashMap;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionState {
    Idle,
    Working,
    NeedsPermission,
    WaitingInput,
    Done,
    Error,
}

/// The subset of the Claude Code hook payload winotch cares about.
#[derive(Debug, Deserialize)]
pub struct HookEvent {
    pub session_id: String,
    pub hook_event_name: String,
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default)]
    pub tool_name: Option<String>,
    #[serde(default)]
    pub notification_type: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub id: String,
    /// The project folder: the first one Claude Code reported.
    pub cwd: Option<String>,
    pub state: SessionState,
    pub tool: Option<String>,
    pub started_at_ms: u64,
    #[serde(skip)]
    last_seen: Instant,
}

enum Transition {
    Set(SessionState),
    Remove,
    Ignore,
}

fn transition(event: &HookEvent) -> Transition {
    use SessionState::*;
    match event.hook_event_name.as_str() {
        "SessionStart" => Transition::Set(Idle),
        "UserPromptSubmit" | "PreToolUse" | "PostToolUse" | "PostToolUseFailure"
        | "PermissionDenied" => Transition::Set(Working),
        "PermissionRequest" => Transition::Set(NeedsPermission),
        "Notification" => match event.notification_type.as_deref() {
            Some("permission_prompt") => Transition::Set(NeedsPermission),
            Some("agent_needs_input" | "elicitation_dialog" | "elicitation_url_dialog") => {
                Transition::Set(WaitingInput)
            }
            // `idle_prompt` fires a while after `Stop`: keeping `done` is more useful.
            _ => Transition::Ignore,
        },
        "Stop" => Transition::Set(Done),
        "StopFailure" => Transition::Set(Error),
        "SessionEnd" => Transition::Remove,
        _ => Transition::Ignore,
    }
}

#[derive(Default)]
pub struct SessionStore {
    sessions: HashMap<String, Session>,
}

impl SessionStore {
    /// Applies a hook event. Returns true if the visible session list changed.
    pub fn apply(&mut self, event: &HookEvent, now: Instant) -> bool {
        match transition(event) {
            Transition::Ignore => {
                if let Some(s) = self.sessions.get_mut(&event.session_id) {
                    s.last_seen = now;
                }
                false
            }
            Transition::Remove => self.sessions.remove(&event.session_id).is_some(),
            Transition::Set(state) => {
                let tool = match event.hook_event_name.as_str() {
                    "PreToolUse" | "PermissionRequest" => event.tool_name.clone(),
                    _ => None,
                };
                // winotch may start after Claude Code: any event creates the session.
                let created = !self.sessions.contains_key(&event.session_id);
                let session = self
                    .sessions
                    .entry(event.session_id.clone())
                    .or_insert_with(|| Session {
                        id: event.session_id.clone(),
                        cwd: None,
                        state,
                        tool: None,
                        started_at_ms: unix_ms(),
                        last_seen: now,
                    });
                let before = (session.state, session.tool.clone(), session.cwd.clone());
                session.state = state;
                session.tool = tool;
                // The project: the first folder known. Claude Code reports the
                // current folder, which moves when a command changes it, and the
                // project (its name, its companion) must not follow.
                if session.cwd.is_none() {
                    session.cwd = event.cwd.clone();
                }
                session.last_seen = now;
                created || before != (session.state, session.tool.clone(), session.cwd.clone())
            }
        }
    }

    /// The user has seen the result: `done` and `error` go back to `idle`.
    pub fn acknowledge(&mut self) -> bool {
        let mut changed = false;
        for s in self.sessions.values_mut() {
            if matches!(s.state, SessionState::Done | SessionState::Error) {
                s.state = SessionState::Idle;
                changed = true;
            }
        }
        changed
    }

    /// The user has seen one session's result: back to `idle` if `done` or
    /// `error`.
    pub fn acknowledge_one(&mut self, id: &str) -> bool {
        match self.sessions.get_mut(id) {
            Some(s) if matches!(s.state, SessionState::Done | SessionState::Error) => {
                s.state = SessionState::Idle;
                true
            }
            _ => false,
        }
    }

    /// Drops sessions that sent nothing for `timeout` (terminal killed without `SessionEnd`).
    pub fn prune(&mut self, now: Instant, timeout: Duration) -> bool {
        let before = self.sessions.len();
        self.sessions
            .retain(|_, s| now.saturating_duration_since(s.last_seen) < timeout);
        before != self.sessions.len()
    }

    pub fn get(&self, id: &str) -> Option<&Session> {
        self.sessions.get(id)
    }

    pub fn list(&self) -> Vec<Session> {
        let mut list: Vec<Session> = self.sessions.values().cloned().collect();
        list.sort_by(|a, b| a.started_at_ms.cmp(&b.started_at_ms).then(a.id.cmp(&b.id)));
        list
    }
}

fn unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(name: &str) -> HookEvent {
        HookEvent {
            session_id: "s1".into(),
            hook_event_name: name.into(),
            cwd: Some("/work/project".into()),
            tool_name: None,
            notification_type: None,
        }
    }

    fn state(store: &SessionStore) -> SessionState {
        store.list()[0].state
    }

    #[test]
    fn full_turn_with_permission() {
        let mut store = SessionStore::default();
        let now = Instant::now();
        store.apply(&event("SessionStart"), now);
        assert_eq!(state(&store), SessionState::Idle);
        store.apply(&event("UserPromptSubmit"), now);
        assert_eq!(state(&store), SessionState::Working);
        let mut perm = event("PermissionRequest");
        perm.tool_name = Some("Bash".into());
        store.apply(&perm, now);
        assert_eq!(state(&store), SessionState::NeedsPermission);
        assert_eq!(store.list()[0].tool.as_deref(), Some("Bash"));
        store.apply(&event("PostToolUse"), now);
        assert_eq!(state(&store), SessionState::Working);
        store.apply(&event("Stop"), now);
        assert_eq!(state(&store), SessionState::Done);
        store.apply(&event("SessionEnd"), now);
        assert!(store.list().is_empty());
    }

    #[test]
    fn unknown_session_is_created_on_first_event() {
        let mut store = SessionStore::default();
        assert!(store.apply(&event("PreToolUse"), Instant::now()));
        assert_eq!(state(&store), SessionState::Working);
        assert_eq!(store.list()[0].cwd.as_deref(), Some("/work/project"));
    }

    #[test]
    fn the_project_folder_does_not_follow_folder_changes() {
        let mut store = SessionStore::default();
        let now = Instant::now();
        store.apply(&event("SessionStart"), now);
        let mut moved = event("PreToolUse");
        moved.cwd = Some("/work/project/src".into());
        store.apply(&moved, now);
        assert_eq!(store.list()[0].cwd.as_deref(), Some("/work/project"));
    }

    #[test]
    fn acknowledging_one_session_leaves_the_others_done() {
        let mut store = SessionStore::default();
        let now = Instant::now();
        let mut other = event("Stop");
        other.session_id = "s2".into();
        store.apply(&event("Stop"), now);
        store.apply(&other, now);
        assert!(store.acknowledge_one("s1"));
        assert_eq!(store.get("s1").unwrap().state, SessionState::Idle);
        assert_eq!(store.get("s2").unwrap().state, SessionState::Done);
        assert!(!store.acknowledge_one("s1"));
    }

    #[test]
    fn idle_prompt_keeps_done() {
        let mut store = SessionStore::default();
        let now = Instant::now();
        store.apply(&event("Stop"), now);
        let mut idle = event("Notification");
        idle.notification_type = Some("idle_prompt".into());
        assert!(!store.apply(&idle, now));
        assert_eq!(state(&store), SessionState::Done);
    }

    #[test]
    fn unknown_events_are_ignored() {
        let mut store = SessionStore::default();
        assert!(!store.apply(&event("PreCompact"), Instant::now()));
        assert!(store.list().is_empty());
    }

    #[test]
    fn acknowledge_resets_done_and_error() {
        let mut store = SessionStore::default();
        store.apply(&event("Stop"), Instant::now());
        assert!(store.acknowledge());
        assert_eq!(state(&store), SessionState::Idle);
        assert!(!store.acknowledge());
    }

    #[test]
    fn prune_drops_silent_sessions() {
        let mut store = SessionStore::default();
        let start = Instant::now();
        store.apply(&event("Stop"), start);
        let timeout = Duration::from_secs(60);
        assert!(!store.prune(start + Duration::from_secs(30), timeout));
        assert!(store.prune(start + Duration::from_secs(61), timeout));
        assert!(store.list().is_empty());
    }
}
