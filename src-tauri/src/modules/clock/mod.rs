//! Date and time module: one item with the local time and date. Off by
//! default, the notch stays empty until the user turns it on (ADR-0009).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use chrono::{Datelike, Local, NaiveDateTime, Timelike};

use crate::module::{Host, Item, Module, Tone};

/// Longest wait between two checks of the minute: a sleeping computer may
/// overrun a single sleep until the next minute.
const MAX_WAIT: Duration = Duration::from_secs(5);

const WEEKDAYS: [&str; 7] = [
    "lundi", "mardi", "mercredi", "jeudi", "vendredi", "samedi", "dimanche",
];
const MONTHS: [&str; 12] = [
    "janvier",
    "février",
    "mars",
    "avril",
    "mai",
    "juin",
    "juillet",
    "août",
    "septembre",
    "octobre",
    "novembre",
    "décembre",
];

#[derive(Default)]
pub struct Clock {
    inner: Arc<Inner>,
}

#[derive(Default)]
struct Inner {
    host: OnceLock<Host>,
    running: AtomicBool,
    ticker_started: AtomicBool,
}

impl Module for Clock {
    fn id(&self) -> &'static str {
        "clock"
    }

    fn name(&self) -> &'static str {
        "Date et heure"
    }

    fn enabled_by_default(&self) -> bool {
        false
    }

    fn start(&self, host: &Host) {
        let inner = &self.inner;
        let _ = inner.host.set(host.clone());
        inner.running.store(true, Ordering::Relaxed);
        if !inner.ticker_started.swap(true, Ordering::Relaxed) {
            spawn_ticker(Arc::clone(inner));
        }
        host.refresh();
    }

    fn stop(&self) {
        self.inner.running.store(false, Ordering::Relaxed);
        if let Some(host) = self.inner.host.get() {
            host.refresh();
        }
    }

    fn items(&self) -> Vec<Item> {
        if !self.inner.running.load(Ordering::Relaxed) {
            return Vec::new();
        }
        let (time, date) = format(Local::now().naive_local());
        vec![Item {
            id: "now".into(),
            title: date,
            label: time,
            detail: None,
            tone: Tone::Neutral,
        }]
    }
}

/// "14:32" and "lundi 5 octobre".
fn format(now: NaiveDateTime) -> (String, String) {
    let time = format!("{:02}:{:02}", now.hour(), now.minute());
    let weekday = WEEKDAYS[now.weekday().num_days_from_monday() as usize];
    let month = MONTHS[now.month0() as usize];
    let day = match now.day() {
        1 => "1er".to_owned(),
        d => d.to_string(),
    };
    (time, format!("{weekday} {day} {month}"))
}

/// Redraws the notch when the minute changes, only while the module runs.
fn spawn_ticker(inner: Arc<Inner>) {
    std::thread::spawn(move || {
        let mut shown = None;
        loop {
            let now = Local::now();
            let minute = (now.date_naive(), now.hour(), now.minute());
            if inner.running.load(Ordering::Relaxed) && shown != Some(minute) {
                shown = Some(minute);
                if let Some(host) = inner.host.get() {
                    host.refresh();
                }
            }
            let to_next_minute = Duration::from_secs(u64::from(60 - now.second()));
            std::thread::sleep(to_next_minute.min(MAX_WAIT));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn at(y: i32, m: u32, d: u32, h: u32, min: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(y, m, d)
            .unwrap()
            .and_hms_opt(h, min, 0)
            .unwrap()
    }

    #[test]
    fn formats_time_and_french_date() {
        assert_eq!(
            format(at(2026, 10, 5, 14, 32)),
            ("14:32".into(), "lundi 5 octobre".into())
        );
        assert_eq!(
            format(at(2026, 2, 1, 9, 5)),
            ("09:05".into(), "dimanche 1er février".into())
        );
    }
}
