//! The interaction mode of a project: how the agent works with the user.
//!
//! The default is sparring (組手): the agent listens with `agpj wait` and handles requests and
//! answers. In sleep mode (スリープ) questions to the user are not accepted, and a subagent
//! decides in the user's place. In brainstorm mode (壁打ち) the agent asks the user questions that
//! deepen one epic or task, the target saved with the mode. A mode is saved per project in the app's internal area as
//! `<data_dir>/.internal/modes.json`, not in the shared project files, so it is the user's
//! setting and does not travel with the project. A project with no entry is in sparring.
//!
//! The mode is independent of who the task is assigned to. The entry has a `kind` and, for
//! brainstorm, a `target`; an unknown `kind`, or brainstorm with no valid target, reads as
//! sparring. A manual mode holds until the next sleep window starts, so a window ends a brainstorm
//! and sleep takes over.

use std::path::{Path, PathBuf};

use chrono::{DateTime, Local, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

pub use crate::mode_schedule::{Effective, Schedule, Source};
pub use brainstorm::start_brainstorm;
pub use held::{held_count, held_ids, release_held};
pub use progress_fields::ProgressMode;
pub use target::{Target, TargetKind, check as check_target};

use store::{read_projects, update, update_with};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModeKind {
    /// The default: the agent listens and handles requests and answers.
    #[default]
    Sparring,
    /// Questions to the user are not accepted; a subagent decides.
    Sleep,
    /// The agent asks questions that deepen one epic or task (the target).
    Brainstorm,
}

impl ModeKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ModeKind::Sparring => "sparring",
            ModeKind::Sleep => "sleep",
            ModeKind::Brainstorm => "brainstorm",
        }
    }

    pub fn parse(text: &str) -> Option<ModeKind> {
        match text {
            "sparring" => Some(ModeKind::Sparring),
            "sleep" => Some(ModeKind::Sleep),
            "brainstorm" => Some(ModeKind::Brainstorm),
            _ => None,
        }
    }
}

/// A project's saved mode and when it was set (`None` for the default that was never set).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
pub struct ProjectMode {
    pub kind: ModeKind,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "serialize_since"
    )]
    pub since: Option<DateTime<Utc>>,
}

fn serialize_since<S: serde::Serializer>(
    since: &Option<DateTime<Utc>>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    match since {
        Some(time) => serializer.serialize_str(&crate::clock::format_utc(*time)),
        None => serializer.serialize_none(),
    }
}

/// The data directory that holds `shared`.
pub fn data_dir_of(shared: &Path) -> PathBuf {
    shared
        .parent()
        .filter(|dir| !dir.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf()
}

fn parse_entry(entry: &Value) -> State {
    let manual = entry
        .get("kind")
        .and_then(Value::as_str)
        .and_then(ModeKind::parse)
        .map(|kind| ProjectMode {
            kind,
            since: entry
                .get("since")
                .and_then(Value::as_str)
                .and_then(|text| DateTime::parse_from_rfc3339(text).ok())
                .map(|time| time.with_timezone(&Utc)),
        });
    let target = entry
        .get("target")
        .and_then(|target| serde_json::from_value::<Target>(target.clone()).ok());
    // Brainstorm needs a target; without a valid one the entry reads as sparring.
    let (manual, target) = match manual {
        Some(ProjectMode {
            kind: ModeKind::Brainstorm,
            ..
        }) if target.is_none() => (None, None),
        Some(ProjectMode {
            kind: ModeKind::Brainstorm,
            ..
        }) => (manual, target),
        other => (other, None),
    };
    let schedule = entry.get("schedule").and_then(Schedule::from_json);
    State {
        manual,
        schedule,
        target,
    }
}

/// The saved state of a project. A missing, invalid, or unknown entry is sparring with no window.
pub fn state(data_dir: &Path, project: &str) -> State {
    read_projects(data_dir)
        .get(project)
        .map(parse_entry)
        .unwrap_or_default()
}

/// The saved manual mode of a project (sparring when none is set).
pub fn load(data_dir: &Path, project: &str) -> ProjectMode {
    state(data_dir, project).manual.unwrap_or_default()
}

/// The JSON of every project that has an entry, as of `now`, for the board's change stream.
pub fn api_json_all<Tz: TimeZone>(
    data_dir: &Path,
    now: &DateTime<Tz>,
) -> std::collections::BTreeMap<String, Value> {
    read_projects(data_dir)
        .iter()
        .map(|(id, entry)| (id.clone(), parse_entry(entry).api_json(now.clone())))
        .collect()
}

/// The mode in effect for a project now, on this machine's clock.
pub fn effective(data_dir: &Path, project: &str) -> Effective {
    state(data_dir, project).effective_at(Local::now())
}

/// The kind in effect now: what the refusal of `message ask` checks.
pub fn current(data_dir: &Path, project: &str) -> ModeKind {
    effective(data_dir, project).kind
}

/// The brainstorm target, only while the mode in effect is brainstorm.
pub fn brainstorm_target(data_dir: &Path, project: &str) -> Option<Target> {
    state(data_dir, project).target_at(Local::now())
}

/// Fills the `mode`, `modeSource`, and `modeUntil` of the progress table while the project is not
/// in sparring, and `modeTarget` in brainstorm. All are left out for sparring.
pub fn fill_progress(table: &mut crate::progress::Progress, shared: &Path, project: &str) {
    let state = state(&data_dir_of(shared), project);
    let now = Local::now();
    let effective = state.effective_at(now);
    if effective.kind == ModeKind::Sparring {
        return;
    }
    table.mode = ProgressMode {
        kind: Some(effective.kind),
        source: Some(effective.source.as_str()),
        until: effective.until.map(crate::clock::format_utc),
        target: state.target_at(now),
    };
}

/// Set the manual mode, keeping the window. Sparring is kept only if the locked entry has one.
pub fn set(
    data_dir: &Path,
    project: &str,
    kind: ModeKind,
    now: DateTime<Utc>,
) -> std::io::Result<ProjectMode> {
    let mut saved = ProjectMode::default();
    update(data_dir, project, |entry| {
        if kind != ModeKind::Sparring
            || entry
                .get("schedule")
                .is_some_and(|s| Schedule::from_json(s).is_some())
        {
            entry.insert("kind".into(), json!(kind.as_str()));
            entry.insert("since".into(), json!(crate::clock::format_utc(now)));
            entry.remove("target");
            saved = ProjectMode {
                kind,
                since: Some(now),
            };
        } else {
            entry.remove("kind");
            entry.remove("since");
            entry.remove("target");
        }
    })?;
    Ok(saved)
}

/// Set the manual mode to brainstorm on `target`, keeping the window.
pub fn set_brainstorm(
    data_dir: &Path,
    project: &str,
    target: &Target,
    now: DateTime<Utc>,
) -> std::io::Result<()> {
    update(data_dir, project, |entry| {
        entry.insert("kind".into(), json!(ModeKind::Brainstorm.as_str()));
        entry.insert("since".into(), json!(crate::clock::format_utc(now)));
        entry.insert("target".into(), json!(target));
    })
}

/// Forget the manual mode, so the daily window (if any) decides.
pub fn clear_manual(data_dir: &Path, project: &str) -> std::io::Result<()> {
    update(data_dir, project, |entry| {
        entry.remove("kind");
        entry.remove("since");
        entry.remove("target");
    })
}

/// Save the daily window (enabled or not), keeping the manual mode.
pub fn set_schedule(data_dir: &Path, project: &str, schedule: Schedule) -> std::io::Result<()> {
    update(data_dir, project, |entry| {
        entry.insert("schedule".into(), schedule.to_json());
    })
}

/// Compute the new window from the saved one and save it under the lock; `None` leaves it alone.
pub fn change_schedule<E>(
    data_dir: &Path,
    project: &str,
    change: impl FnOnce(Option<Schedule>) -> Result<Option<Schedule>, E>,
) -> std::io::Result<Result<(), E>> {
    update_with(data_dir, project, |entry| {
        let saved = entry.get("schedule").and_then(Schedule::from_json);
        if let Some(schedule) = change(saved)? {
            entry.insert("schedule".into(), schedule.to_json());
        }
        Ok(())
    })
}

mod brainstorm;
mod held;
mod progress_fields;
mod state;
mod store;
mod target;
#[cfg(test)]
mod tests;

pub use state::State;
