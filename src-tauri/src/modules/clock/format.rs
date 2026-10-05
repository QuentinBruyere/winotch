//! How the time and the date are written, from the module's settings.
//! Pure logic, tested.

use chrono::{Datelike, NaiveDateTime, Timelike};
use serde::{Deserialize, Serialize};

/// What the notch shows.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Show {
    #[default]
    TimeAndDate,
    Time,
    Date,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DateStyle {
    /// "lundi 5 octobre"
    #[default]
    Long,
    /// "lun. 5 oct."
    Short,
    /// "05/10/2026"
    Numeric,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub show: Show,
    /// "2:32 PM" instead of "14:32".
    pub hour12: bool,
    pub seconds: bool,
    pub date_style: DateStyle,
}

const WEEKDAYS: [&str; 7] = [
    "lundi", "mardi", "mercredi", "jeudi", "vendredi", "samedi", "dimanche",
];
const WEEKDAYS_SHORT: [&str; 7] = ["lun.", "mar.", "mer.", "jeu.", "ven.", "sam.", "dim."];
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
const MONTHS_SHORT: [&str; 12] = [
    "janv.", "févr.", "mars", "avr.", "mai", "juin", "juil.", "août", "sept.", "oct.", "nov.",
    "déc.",
];

/// The item's label (main text) and title (secondary text, may be empty).
pub fn format(now: NaiveDateTime, settings: &Settings) -> (String, String) {
    let time = time(now, settings);
    let date = date(now, settings.date_style);
    match settings.show {
        Show::TimeAndDate => (time, date),
        Show::Time => (time, String::new()),
        Show::Date => (date, String::new()),
    }
}

fn time(now: NaiveDateTime, settings: &Settings) -> String {
    let seconds = if settings.seconds {
        format!(":{:02}", now.second())
    } else {
        String::new()
    };
    if settings.hour12 {
        let (pm, hour) = now.hour12();
        let suffix = if pm { "PM" } else { "AM" };
        format!("{hour}:{:02}{seconds} {suffix}", now.minute())
    } else {
        format!("{:02}:{:02}{seconds}", now.hour(), now.minute())
    }
}

fn date(now: NaiveDateTime, style: DateStyle) -> String {
    let weekday = now.weekday().num_days_from_monday() as usize;
    let month = now.month0() as usize;
    let day = match now.day() {
        1 => "1er".to_owned(),
        d => d.to_string(),
    };
    match style {
        DateStyle::Long => format!("{} {day} {}", WEEKDAYS[weekday], MONTHS[month]),
        DateStyle::Short => format!("{} {day} {}", WEEKDAYS_SHORT[weekday], MONTHS_SHORT[month]),
        DateStyle::Numeric => format!("{:02}/{:02}/{}", now.day(), now.month(), now.year()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn at(y: i32, m: u32, d: u32, h: u32, min: u32, s: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(y, m, d)
            .unwrap()
            .and_hms_opt(h, min, s)
            .unwrap()
    }

    fn pair(a: &str, b: &str) -> (String, String) {
        (a.into(), b.into())
    }

    #[test]
    fn default_is_24h_time_and_long_date() {
        let settings = Settings::default();
        assert_eq!(
            format(at(2026, 10, 5, 14, 32, 7), &settings),
            pair("14:32", "lundi 5 octobre")
        );
        assert_eq!(
            format(at(2026, 2, 1, 9, 5, 0), &settings),
            pair("09:05", "dimanche 1er février")
        );
    }

    #[test]
    fn time_only_or_date_only() {
        let now = at(2026, 10, 5, 14, 32, 7);
        let time = Settings {
            show: Show::Time,
            ..Settings::default()
        };
        assert_eq!(format(now, &time), pair("14:32", ""));
        let date = Settings {
            show: Show::Date,
            ..Settings::default()
        };
        assert_eq!(format(now, &date), pair("lundi 5 octobre", ""));
    }

    #[test]
    fn twelve_hours_and_seconds() {
        let settings = Settings {
            show: Show::Time,
            hour12: true,
            seconds: true,
            ..Settings::default()
        };
        assert_eq!(
            format(at(2026, 10, 5, 14, 32, 7), &settings).0,
            "2:32:07 PM"
        );
        assert_eq!(format(at(2026, 10, 5, 0, 5, 0), &settings).0, "12:05:00 AM");
    }

    #[test]
    fn short_and_numeric_dates() {
        let now = at(2026, 10, 5, 14, 32, 0);
        let short = Settings {
            show: Show::Date,
            date_style: DateStyle::Short,
            ..Settings::default()
        };
        assert_eq!(format(now, &short).0, "lun. 5 oct.");
        let numeric = Settings {
            date_style: DateStyle::Numeric,
            ..short
        };
        assert_eq!(format(now, &numeric).0, "05/10/2026");
    }
}
