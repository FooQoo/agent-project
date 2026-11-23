//! The daily sleep window of a project and how it combines with the manual mode.
//!
//! One window per project: `start` and `end` as `HH:MM` in the machine's local time, and it may
//! cross midnight (23:00 to 08:00). The resolution is pure, takes the current time as an argument,
//! and is used by every reader (the refusal, `agpj mode`, the progress table, the board):
//!
//! - A manual mode wins while it is set.
//! - A manual change made while a window was active holds until that window ends; then the
//!   schedule takes over again.
//! - A manual change made outside a window holds until it is changed (`agpj mode auto` clears it).
//! - With no manual mode, the window means sleep and everything else means sparring.

use chrono::{DateTime, Duration, NaiveTime, TimeZone, Utc};
use serde_json::{Value, json};

use crate::mode::ModeKind;

/// The daily window. `enabled: false` keeps the times so a switch can turn it on again.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Schedule {
    pub enabled: bool,
    pub start: NaiveTime,
    pub end: NaiveTime,
}

/// Where the effective mode comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Manual,
    Schedule,
}

impl Source {
    pub fn as_str(self) -> &'static str {
        match self {
            Source::Manual => "manual",
            Source::Schedule => "schedule",
        }
    }
}

/// The mode in effect now. `until` is the end of the window while the schedule decides sleep.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Effective {
    pub kind: ModeKind,
    pub source: Source,
    pub since: Option<DateTime<Utc>>,
    pub until: Option<DateTime<Utc>>,
}

/// Reads `HH:MM` (24-hour, two digits each).
pub fn parse_time(text: &str) -> Option<NaiveTime> {
    let (hour, minute) = text.split_once(':')?;
    if hour.len() != 2 || minute.len() != 2 {
        return None;
    }
    NaiveTime::from_hms_opt(hour.parse().ok()?, minute.parse().ok()?, 0)
}

pub fn format_time(time: NaiveTime) -> String {
    time.format("%H:%M").to_string()
}

impl Schedule {
    /// A window needs two different times.
    pub fn is_valid(&self) -> bool {
        self.start != self.end
    }

    pub fn from_json(value: &Value) -> Option<Schedule> {
        let schedule = Schedule {
            enabled: value.get("enabled")?.as_bool()?,
            start: parse_time(value.get("start")?.as_str()?)?,
            end: parse_time(value.get("end")?.as_str()?)?,
        };
        schedule.is_valid().then_some(schedule)
    }

    pub fn to_json(&self) -> Value {
        json!({
            "enabled": self.enabled,
            "start": format_time(self.start),
            "end": format_time(self.end),
        })
    }

    /// The end of the window that contains `at`, or `None` outside a window or when disabled.
    pub fn window_end<Tz: TimeZone>(&self, at: &DateTime<Tz>) -> Option<DateTime<Utc>> {
        if !self.enabled || !self.is_valid() {
            return None;
        }
        let local = at.naive_local();
        let mut length = self.end - self.start;
        if length < Duration::zero() {
            length += Duration::days(1);
        }
        // A window that began yesterday may still run; one that begins today may have begun.
        for days_back in [1, 0] {
            let start = (local.date() - Duration::days(days_back)).and_time(self.start);
            let end = start + length;
            if start <= local && local < end {
                let tz = at.timezone();
                let instant = tz
                    .from_local_datetime(&end)
                    .earliest()
                    .or_else(|| tz.from_local_datetime(&end).latest())?;
                return Some(instant.with_timezone(&Utc));
            }
        }
        None
    }
}

/// The mode in effect at `now`, from the manual mode (with when it was set) and the schedule.
pub fn resolve<Tz: TimeZone>(
    manual: Option<(ModeKind, DateTime<Utc>)>,
    schedule: Option<&Schedule>,
    now: DateTime<Tz>,
) -> Effective {
    let in_window = |at: &DateTime<Tz>| schedule.and_then(|s| s.window_end(at));
    if let Some((kind, since)) = manual {
        let tz = now.timezone();
        let held = match in_window(&since.with_timezone(&tz)) {
            // Set during a window: it holds until that window ends.
            Some(end) => now.with_timezone(&Utc) < end,
            None => true,
        };
        if held {
            return Effective {
                kind,
                source: Source::Manual,
                since: Some(since),
                until: None,
            };
        }
    }
    match in_window(&now) {
        Some(end) => Effective {
            kind: ModeKind::Sleep,
            source: Source::Schedule,
            since: None,
            until: Some(end),
        },
        None => Effective {
            kind: ModeKind::Sparring,
            source: Source::Manual,
            since: None,
            until: None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::FixedOffset;

    fn tz() -> FixedOffset {
        FixedOffset::east_opt(9 * 3600).unwrap()
    }

    /// A local time of 2026-11-24 (plus `day` days) in UTC+9.
    fn at(day: i64, hour: u32, minute: u32) -> DateTime<FixedOffset> {
        tz().with_ymd_and_hms(2026, 11, 24, hour, minute, 0)
            .unwrap()
            + Duration::days(day)
    }

    fn night() -> Schedule {
        Schedule {
            enabled: true,
            start: parse_time("23:00").unwrap(),
            end: parse_time("08:00").unwrap(),
        }
    }

    #[test]
    fn a_window_that_crosses_midnight_covers_both_sides() {
        let s = night();
        assert!(s.window_end(&at(0, 22, 59)).is_none());
        assert_eq!(
            s.window_end(&at(0, 23, 0)),
            Some(at(1, 8, 0).with_timezone(&Utc))
        );
        assert_eq!(
            s.window_end(&at(1, 0, 30)),
            Some(at(1, 8, 0).with_timezone(&Utc))
        );
        assert_eq!(
            s.window_end(&at(1, 7, 59)),
            Some(at(1, 8, 0).with_timezone(&Utc))
        );
        assert!(s.window_end(&at(1, 8, 0)).is_none());
    }

    #[test]
    fn a_same_day_window_and_a_disabled_one() {
        let day = Schedule {
            enabled: true,
            start: parse_time("13:00").unwrap(),
            end: parse_time("15:30").unwrap(),
        };
        assert!(day.window_end(&at(0, 12, 59)).is_none());
        assert_eq!(
            day.window_end(&at(0, 14, 0)),
            Some(at(0, 15, 30).with_timezone(&Utc))
        );
        assert!(day.window_end(&at(0, 15, 30)).is_none());
        let off = Schedule {
            enabled: false,
            ..day
        };
        assert!(off.window_end(&at(0, 14, 0)).is_none());
    }

    #[test]
    fn with_no_manual_mode_the_window_decides() {
        let s = night();
        let inside = resolve(None, Some(&s), at(0, 23, 30));
        assert_eq!(
            (inside.kind, inside.source),
            (ModeKind::Sleep, Source::Schedule)
        );
        assert_eq!(inside.until, Some(at(1, 8, 0).with_timezone(&Utc)));
        let outside = resolve(None, Some(&s), at(1, 12, 0));
        assert_eq!(
            (outside.kind, outside.source),
            (ModeKind::Sparring, Source::Manual)
        );
        assert!(outside.until.is_none());
    }

    #[test]
    fn a_manual_change_during_a_window_holds_until_the_window_ends() {
        let s = night();
        // Woke at 02:00 and set sparring by hand.
        let since = at(1, 2, 0).with_timezone(&Utc);
        let manual = Some((ModeKind::Sparring, since));
        let held = resolve(manual, Some(&s), at(1, 5, 0));
        assert_eq!(
            (held.kind, held.source),
            (ModeKind::Sparring, Source::Manual)
        );
        assert_eq!(
            resolve(manual, Some(&s), at(1, 7, 59)).kind,
            ModeKind::Sparring
        );
        // The window is over at 08:00, and the next one (23:00) takes over again.
        assert_eq!(
            resolve(manual, Some(&s), at(1, 8, 0)).source,
            Source::Manual
        );
        assert_eq!(
            resolve(manual, Some(&s), at(1, 8, 0)).kind,
            ModeKind::Sparring
        );
        let next = resolve(manual, Some(&s), at(1, 23, 30));
        assert_eq!(
            (next.kind, next.source),
            (ModeKind::Sleep, Source::Schedule)
        );
    }

    #[test]
    fn a_manual_sleep_outside_a_window_holds_until_it_is_changed() {
        let s = night();
        let manual = Some((ModeKind::Sleep, at(0, 12, 0).with_timezone(&Utc)));
        for time in [at(0, 13, 0), at(0, 23, 30), at(1, 12, 0), at(5, 3, 0)] {
            let got = resolve(manual, Some(&s), time);
            assert_eq!(
                (got.kind, got.source),
                (ModeKind::Sleep, Source::Manual),
                "{time}"
            );
        }
    }

    #[test]
    fn a_manual_sleep_made_in_a_window_ends_with_it() {
        let s = night();
        let manual = Some((ModeKind::Sleep, at(0, 23, 10).with_timezone(&Utc)));
        assert_eq!(
            resolve(manual, Some(&s), at(1, 7, 0)).source,
            Source::Manual
        );
        let after = resolve(manual, Some(&s), at(1, 9, 0));
        assert_eq!(
            (after.kind, after.source),
            (ModeKind::Sparring, Source::Manual)
        );
    }

    #[test]
    fn no_schedule_means_the_manual_mode_alone() {
        let manual = Some((ModeKind::Sleep, at(0, 1, 0).with_timezone(&Utc)));
        assert_eq!(resolve(manual, None, at(3, 1, 0)).kind, ModeKind::Sleep);
        assert_eq!(resolve(None, None, at(3, 1, 0)).kind, ModeKind::Sparring);
    }

    #[test]
    fn times_are_two_digit_24_hour() {
        assert!(parse_time("08:05").is_some());
        for bad in ["8:05", "24:00", "08:60", "0805", "08:5", ""] {
            assert!(parse_time(bad).is_none(), "{bad}");
        }
        assert!(
            Schedule::from_json(&json!({"enabled": true, "start": "08:00", "end": "08:00"}))
                .is_none()
        );
    }
}
