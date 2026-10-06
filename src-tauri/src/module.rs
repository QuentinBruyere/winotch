//! Module contract, see docs/adr/0009-systeme-de-modules.md. The core only
//! draws the notch: everything it shows comes from modules, as generic items.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager};

use crate::config::PinSide;
use crate::{AppState, config};

/// How an item looks and how urgent it is: decides its colour, which item
/// the compact notch shows first, and the sound when an item enters it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tone {
    #[default]
    Neutral,
    /// Something is running: the dot pulses.
    Active,
    /// The user must act (e.g. a permission).
    Attention,
    /// The user is asked a question.
    Question,
    Success,
    Error,
}

/// Icons the open notch knows for action buttons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionIcon {
    Play,
    Pause,
    Reset,
}

/// A button at the end of an item's row in the open notch (e.g. start a
/// timer). Clicking it calls `Module::item_action`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Action {
    /// Passed back to `Module::item_action`.
    pub id: String,
    /// Tooltip of an icon button, text of a button without icon ("+1 min").
    pub label: String,
    pub icon: Option<ActionIcon>,
}

impl Action {
    pub fn icon(id: &str, icon: ActionIcon, label: &str) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon: Some(icon),
        }
    }

    pub fn text(id: &str, label: &str) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon: None,
        }
    }
}

/// One line of the notch: a Claude Code session, a song, the weather…
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    /// Unique within its module; the core prefixes it with the module id.
    pub id: String,
    /// Main text, e.g. the project name.
    pub title: String,
    /// Short state, e.g. "Au travail".
    pub label: String,
    /// Extra detail, e.g. the running tool.
    pub detail: Option<String>,
    pub tone: Tone,
    /// Shows a dot in the tone's colour (e.g. a session's state). Items that
    /// only display information, like the time, go without.
    pub dot: bool,
    /// Buttons shown in the open notch, in order.
    pub actions: Vec<Action>,
    /// Nothing worth showing yet (e.g. a stopwatch at zero): the compact
    /// views (closed notch, pin) show the module's icon instead of the label.
    pub quiet: bool,
    /// Rings (e.g. a timer whose time is up): an alarm repeats and the shape
    /// showing it pulses until the user acknowledges it.
    pub ringing: bool,
}

/// A feature shown in the notch. Modules are compiled in and handed to
/// `crate::run`; only enabled modules are started.
pub trait Module: Send + Sync + 'static {
    /// Stable identifier, also the key of the module's settings in config.json.
    fn id(&self) -> &'static str;
    /// Name shown to the user.
    fn name(&self) -> &'static str;
    /// One sentence shown under the name in the settings' module list.
    fn description(&self) -> &'static str {
        ""
    }
    /// Whether the module runs until the user turns it off or on.
    fn enabled_by_default(&self) -> bool {
        true
    }
    /// Called at launch when enabled, and when the user enables the module.
    fn start(&self, host: &Host);
    /// Called when the user disables the module (not when winotch quits).
    fn stop(&self);
    /// What the notch shows for this module, in display order.
    fn items(&self) -> Vec<Item>;
    /// Shown in the open notch when the module has no item, e.g. its
    /// connection state. `None` = nothing to say.
    fn placeholder(&self) -> Option<String> {
        None
    }
    /// The user clicked the notch: they have seen what it shows.
    fn acknowledge(&self) {}
    /// The user clicked one of an item's buttons (`Item::actions`).
    fn item_action(&self, _item: &str, _action: &str) {}
    /// Data for the module's section of the settings window.
    fn settings(&self) -> Value {
        Value::Null
    }
    /// Action requested by the module's section of the settings window.
    fn call(&self, action: &str, _args: Value) -> Result<Value, String> {
        Err(format!("unknown action {action}"))
    }
}

/// What a module may ask of the core. Cheap to clone.
///
/// Never call `refresh` or `settings_changed` while holding one of the
/// module's own locks: they call back into `Module::items` / `settings`.
#[derive(Clone)]
pub struct Host {
    app: AppHandle,
    id: &'static str,
}

impl Host {
    pub(crate) fn new(app: AppHandle, id: &'static str) -> Self {
        Self { app, id }
    }

    /// The module's items changed: redraws the notch.
    pub fn refresh(&self) {
        crate::emit_content(&self.app);
    }

    /// The module's settings data changed: updates the settings window.
    pub fn settings_changed(&self) {
        crate::settings::changed(&self.app);
    }

    /// Short message shown in the notch for a few seconds.
    pub fn notice(&self, message: &str) {
        let _ = self.app.emit("notice", message);
    }

    /// winotch's config dir, for files of the module's own.
    pub fn config_dir(&self) -> PathBuf {
        self.app.state::<AppState>().config_dir.clone()
    }

    pub fn home_dir(&self) -> Option<PathBuf> {
        self.app.path().home_dir().ok()
    }

    /// The module's settings, defaults for missing or invalid fields.
    pub fn load_settings<T: DeserializeOwned + Default>(&self) -> T {
        let state = self.app.state::<AppState>();
        let entry = state.config.lock().unwrap().modules.get(self.id).cloned();
        entry
            .and_then(|e| serde_json::from_value(Value::Object(e.settings)).ok())
            .unwrap_or_default()
    }

    pub fn save_settings<T: Serialize>(&self, settings: &T) {
        let Ok(Value::Object(map)) = serde_json::to_value(settings) else {
            log::error!("settings of module {} are not an object", self.id);
            return;
        };
        crate::settings::update_config(&self.app, |c| {
            c.modules.entry(self.id.to_owned()).or_default().settings = map;
        });
    }
}

/// What one enabled module shows: its own part of the notch (DF-0011).
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Section {
    /// Id of the module: the front picks its icon from it.
    pub module: String,
    pub items: Vec<Item>,
    /// The module's placeholder, when it has no item.
    pub note: Option<String>,
    /// Pinned: shown in its own mini-notch on this side (DF-0012).
    pub pin: Option<Side>,
}

/// Where a pinned module's mini-notch is, next to the notch. On the left and
/// right edges of the screen: above and below.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Side {
    Left,
    Right,
}

/// Everything the notch draws: one section per enabled module with
/// something to show, in the user's order (DF-0011).
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Content {
    pub sections: Vec<Section>,
    /// Shown in the open notch when no module is enabled.
    pub note: Option<String>,
}

/// A duration as a module would show it: "04:05", "1:02:03" past an hour.
pub fn format_duration(seconds: u64) -> String {
    let (h, m, s) = (seconds / 3600, seconds / 60 % 60, seconds % 60);
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m:02}:{s:02}")
    }
}

pub(crate) fn is_enabled(module: &dyn Module, config: &config::Config) -> bool {
    config.module_enabled(module.id(), module.enabled_by_default())
}

/// The modules in the user's order: those listed in `order` first, the
/// others (e.g. newly installed ones) after, in their declaration order.
pub(crate) fn ordered<'a>(modules: &'a [Box<dyn Module>], order: &[String]) -> Vec<&'a dyn Module> {
    let mut sorted: Vec<&dyn Module> = modules.iter().map(AsRef::as_ref).collect();
    // Stable sort: unlisted modules keep their relative order, at the end.
    sorted.sort_by_key(|m| {
        order
            .iter()
            .position(|id| id == m.id())
            .unwrap_or(usize::MAX)
    });
    sorted
}

/// Side of each module, `ids` being the enabled modules in display order
/// (DF-0012). The first one is the notch itself: never pinned. A forced side
/// is kept; each `Auto` pin goes, in order, to the side with fewer pins (the
/// right one when even).
pub(crate) fn pin_sides(ids: &[&str], pins: &BTreeMap<String, PinSide>) -> Vec<Option<Side>> {
    let wanted: Vec<Option<PinSide>> = ids
        .iter()
        .enumerate()
        .map(|(i, id)| if i == 0 { None } else { pins.get(*id).copied() })
        .collect();
    let forced = |side| wanted.iter().filter(|w| **w == Some(side)).count();
    let (mut left, mut right) = (forced(PinSide::Left), forced(PinSide::Right));
    wanted
        .into_iter()
        .map(|w| match w? {
            PinSide::Left => Some(Side::Left),
            PinSide::Right => Some(Side::Right),
            PinSide::Auto if left < right => {
                left += 1;
                Some(Side::Left)
            }
            PinSide::Auto => {
                right += 1;
                Some(Side::Right)
            }
        })
        .collect()
}

pub(crate) fn content(modules: &[Box<dyn Module>], config: &config::Config) -> Content {
    let mut content = Content::default();
    let enabled: Vec<&dyn Module> = ordered(modules, &config.module_order)
        .into_iter()
        .filter(|m| is_enabled(*m, config))
        .collect();
    let ids: Vec<&str> = enabled.iter().map(|m| m.id()).collect();
    let sides = pin_sides(&ids, &config.pins);
    let any_enabled = !enabled.is_empty();
    for (module, pin) in enabled.into_iter().zip(sides) {
        let items: Vec<Item> = module
            .items()
            .into_iter()
            .map(|item| Item {
                id: format!("{}:{}", module.id(), item.id),
                ..item
            })
            .collect();
        let note = if items.is_empty() {
            module.placeholder()
        } else {
            None
        };
        if !items.is_empty() || note.is_some() {
            content.sections.push(Section {
                module: module.id().into(),
                items,
                note,
                pin,
            });
        }
    }
    if !any_enabled {
        content.note = Some("Aucun module actif".into());
    }
    content
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fake {
        id: &'static str,
        items: Vec<Item>,
    }

    impl Module for Fake {
        fn id(&self) -> &'static str {
            self.id
        }
        fn name(&self) -> &'static str {
            "Fake"
        }
        fn start(&self, _: &Host) {}
        fn stop(&self) {}
        fn items(&self) -> Vec<Item> {
            self.items.clone()
        }
        fn placeholder(&self) -> Option<String> {
            Some(format!("{} : rien", self.id))
        }
    }

    fn item(id: &str) -> Item {
        Item {
            id: id.into(),
            title: "t".into(),
            label: "l".into(),
            detail: None,
            dot: true,
            ..Item::default()
        }
    }

    fn modules() -> Vec<Box<dyn Module>> {
        vec![
            Box::new(Fake {
                id: "a",
                items: vec![item("1")],
            }),
            Box::new(Fake {
                id: "b",
                items: vec![],
            }),
        ]
    }

    #[test]
    fn durations_get_hours_only_when_needed() {
        assert_eq!(format_duration(0), "00:00");
        assert_eq!(format_duration(245), "04:05");
        assert_eq!(format_duration(3723), "1:02:03");
    }

    #[test]
    fn items_are_prefixed_and_empty_modules_give_their_placeholder() {
        let content = content(&modules(), &config::Config::default());
        assert_eq!(content.sections.len(), 2);
        assert_eq!(content.sections[0].module, "a");
        assert_eq!(content.sections[0].items[0].id, "a:1");
        assert_eq!(content.sections[0].note, None);
        assert!(content.sections[1].items.is_empty());
        assert_eq!(content.sections[1].note.as_deref(), Some("b : rien"));
        assert_eq!(content.note, None);
    }

    #[test]
    fn sections_follow_the_user_order_unlisted_modules_last() {
        let mut modules = modules();
        modules.push(Box::new(Fake {
            id: "c",
            items: vec![item("1")],
        }));
        let config = config::Config {
            module_order: vec!["c".into(), "a".into()],
            ..config::Config::default()
        };
        let ids: Vec<_> = content(&modules, &config)
            .sections
            .into_iter()
            .map(|s| s.module)
            .collect();
        assert_eq!(ids, ["c", "a", "b"]);
    }

    #[test]
    fn disabled_modules_show_nothing() {
        let mut config = config::Config::default();
        config.modules.entry("a".into()).or_default().enabled = Some(false);
        let content = content(&modules(), &config);
        assert_eq!(content.sections.len(), 1);
        assert_eq!(content.sections[0].module, "b");
        assert_eq!(content.note, None);

        config.modules.entry("b".into()).or_default().enabled = Some(false);
        let content = super::content(&modules(), &config);
        assert!(content.sections.is_empty());
        assert_eq!(content.note.as_deref(), Some("Aucun module actif"));
    }

    #[test]
    fn auto_pins_balance_left_and_right_the_first_module_never_pinned() {
        let pins: BTreeMap<String, PinSide> = ["a", "b", "c", "d"]
            .into_iter()
            .map(|id| (id.to_string(), PinSide::Auto))
            .collect();
        assert_eq!(
            pin_sides(&["a", "b", "c", "d"], &pins),
            vec![None, Some(Side::Right), Some(Side::Left), Some(Side::Right)]
        );
        // Unpinned modules stay in the notch.
        assert_eq!(
            pin_sides(&["a", "x", "b"], &pins),
            vec![None, None, Some(Side::Right)]
        );
    }

    #[test]
    fn forced_sides_are_kept_and_counted_by_auto_pins() {
        let pins: BTreeMap<String, PinSide> = [
            ("b", PinSide::Auto),
            ("c", PinSide::Right),
            ("d", PinSide::Auto),
        ]
        .into_iter()
        .map(|(id, side)| (id.to_string(), side))
        .collect();
        // The forced right pin counts first: both auto pins go left, then right.
        assert_eq!(
            pin_sides(&["a", "b", "c", "d"], &pins),
            vec![None, Some(Side::Left), Some(Side::Right), Some(Side::Right)]
        );
    }
}
