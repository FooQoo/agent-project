//! What a project has saved, and the mode in effect from it.

use chrono::{DateTime, TimeZone, Utc};
use serde_json::{Value, json};

use super::ProjectMode;
use crate::mode_schedule::{Effective, Schedule, Source, resolve};

/// What a project has saved: the manual mode and the daily window, either of which may be absent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct State {
    pub manual: Option<ProjectMode>,
    pub schedule: Option<Schedule>,
}

impl State {
    /// The mode in effect at `now` (see `mode_schedule` for the rules).
    pub fn effective_at<Tz: TimeZone>(&self, now: DateTime<Tz>) -> Effective {
        let epoch = DateTime::<Utc>::UNIX_EPOCH;
        let manual = self
            .manual
            .map(|manual| (manual.kind, manual.since.unwrap_or(epoch)));
        resolve(manual, self.schedule.as_ref(), now)
    }

    /// The JSON the board's API and event stream carry. `manual` is what was set by hand, so the
    /// board can offer to follow the schedule again.
    pub fn api_json<Tz: TimeZone>(&self, now: DateTime<Tz>) -> Value {
        let effective = self.effective_at(now);
        let mut body = json!({
            "mode": effective.kind.as_str(),
            "modeSource": effective.source.as_str(),
        });
        let format = |time: DateTime<Utc>| Value::String(crate::clock::format_utc(time));
        let since = effective
            .since
            .or_else(|| self.manual.and_then(|m| m.since));
        if let (Some(since), Source::Manual) = (since, effective.source) {
            body["since"] = format(since);
        }
        if let Some(until) = effective.until {
            body["until"] = format(until);
        }
        if let Some(manual) = self.manual {
            body["manual"] = json!(manual.kind.as_str());
        }
        if let Some(schedule) = self.schedule {
            body["schedule"] = schedule.to_json();
        }
        body
    }
}
