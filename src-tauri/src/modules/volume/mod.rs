//! Volume module (DF-0013): the volume of the default audio output, shown
//! and set from the notch. Windows only for now: Core Audio tells it about
//! every change (keys, Windows mixer, other apps), so it never polls.

#[cfg(windows)]
mod endpoint;

use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex, OnceLock};

use crate::module::{Action, Host, Icon, Item, Module, Slider};
use crate::t;

/// The output's volume, as shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Level {
    /// 0 to 100.
    pub percent: u8,
    pub muted: bool,
}

/// What the notch asks the audio thread to do.
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) enum Command {
    Set(u8),
    ToggleMute,
    /// The default output changed: follow the new one.
    DeviceChanged,
    Stop,
}

#[derive(Default)]
pub struct Volume {
    inner: Arc<Inner>,
}

#[derive(Default)]
struct Inner {
    host: OnceLock<Host>,
    /// `None`: no audio output (or not read yet).
    level: Mutex<Option<Level>>,
    /// To the audio thread, while the module runs.
    commands: Mutex<Option<Sender<Command>>>,
}

impl Inner {
    /// A new level from the audio thread; `external`: not set from the notch,
    /// which then shows it briefly.
    fn changed(&self, level: Option<Level>, external: bool) {
        *self.level.lock().unwrap() = level;
        if let Some(host) = self.host.get() {
            host.refresh();
            if external && level.is_some() {
                host.spotlight();
            }
        }
    }

    fn send(&self, command: Command) {
        if let Some(commands) = self.commands.lock().unwrap().as_ref() {
            let _ = commands.send(command);
        }
    }
}

impl Module for Volume {
    fn id(&self) -> &'static str {
        "volume"
    }

    fn name(&self) -> String {
        crate::t!("volume.name")
    }

    fn description(&self) -> String {
        crate::t!("volume.description")
    }

    fn enabled_by_default(&self) -> bool {
        false
    }

    fn start(&self, host: &Host) {
        let _ = self.inner.host.set(host.clone());
        #[cfg(windows)]
        {
            let inner = Arc::clone(&self.inner);
            let commands = endpoint::spawn(move |level, external| inner.changed(level, external));
            *self.inner.commands.lock().unwrap() = Some(commands);
        }
        host.refresh();
    }

    fn stop(&self) {
        self.inner.send(Command::Stop);
        *self.inner.commands.lock().unwrap() = None;
        *self.inner.level.lock().unwrap() = None;
    }

    fn items(&self) -> Vec<Item> {
        let level = *self.inner.level.lock().unwrap();
        level.map(item).into_iter().collect()
    }

    fn placeholder(&self) -> Option<String> {
        Some(if cfg!(windows) {
            t!("volume.no_output")
        } else {
            t!("volume.windows_only")
        })
    }

    /// `mute` toggles the sound, `set:<0-100>` comes from the slider.
    fn item_action(&self, _item: &str, action: &str) {
        match action.split_once(':') {
            None if action == "mute" => self.inner.send(Command::ToggleMute),
            Some(("set", value)) => {
                if let Ok(value) = value.parse::<f64>() {
                    self.inner
                        .send(Command::Set(value.round().clamp(0.0, 100.0) as u8));
                }
            }
            _ => {}
        }
    }
}

/// The speaker matching the level, like Windows: crossed out when silent,
/// more waves as it gets louder.
fn level_icon(level: Level) -> Icon {
    match level.percent {
        _ if level.muted => Icon::Muted,
        0 => Icon::Muted,
        1..=33 => Icon::VolumeLow,
        34..=66 => Icon::VolumeMedium,
        _ => Icon::VolumeHigh,
    }
}

fn item(level: Level) -> Item {
    let icon = level_icon(level);
    let tooltip = if level.muted {
        t!("volume.unmute")
    } else {
        t!("volume.mute")
    };
    Item {
        id: "level".into(),
        title: t!("volume.name"),
        label: if level.muted {
            t!("volume.muted")
        } else {
            t!("volume.percent", percent = level.percent)
        },
        slider: Some(Slider {
            value: f64::from(level.percent),
            min: 0.0,
            max: 100.0,
            step: 1.0,
        }),
        actions: vec![Action::icon("mute", icon, &tooltip)],
        // The compact views show the icon, and the value only for a moment
        // after a change (`Host::spotlight`), like the stopwatch at rest.
        quiet: true,
        icon: Some(icon),
        ..Item::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_item_shows_the_percentage_or_muted() {
        let on = item(Level {
            percent: 45,
            muted: false,
        });
        assert_eq!(on.label, "45%");
        assert_eq!(on.slider.as_ref().map(|s| s.value), Some(45.0));
        assert_eq!(on.actions[0].icon, Some(Icon::VolumeMedium));
        assert_eq!(on.icon, Some(Icon::VolumeMedium));
        assert!(on.quiet);
        let off = item(Level {
            percent: 45,
            muted: true,
        });
        assert_eq!(off.label, "Muted");
        assert_eq!(off.actions[0].icon, Some(Icon::Muted));
    }

    #[test]
    fn the_icon_follows_the_level() {
        let at = |percent, muted| level_icon(Level { percent, muted });
        assert_eq!(at(0, false), Icon::Muted);
        assert_eq!(at(80, true), Icon::Muted);
        assert_eq!(at(1, false), Icon::VolumeLow);
        assert_eq!(at(33, false), Icon::VolumeLow);
        assert_eq!(at(34, false), Icon::VolumeMedium);
        assert_eq!(at(66, false), Icon::VolumeMedium);
        assert_eq!(at(67, false), Icon::VolumeHigh);
        assert_eq!(at(100, false), Icon::VolumeHigh);
    }
}
