//! The interaction mode of a project: how the agent works with the user.
//!
//! The default is sparring (組手): the agent listens with `agpj wait` and handles requests and
//! answers. In sleep mode (スリープ) questions to the user are not accepted, and a subagent
//! decides in the user's place. A mode is saved per project in the app's internal area as
//! `<data_dir>/.internal/modes.json`, not in the shared project files, so it is the user's
//! setting and does not travel with the project. A project with no entry is in sparring.
//!
//! The mode is independent of who the task is assigned to. A later mode (brainstorm) adds a
//! `kind` and an optional target to the same entry; an unknown `kind` reads as sparring.

use std::path::{Path, PathBuf};

use chrono::{DateTime, Local, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

pub use crate::mode_schedule::{Effective, Schedule, Source};

const FORMAT_VERSION: u32 = 1;
const INTERNAL_DIR: &str = ".internal";
const FILE_NAME: &str = "modes.json";
const LOCK_NAME: &str = "modes.lock";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModeKind {
    /// The default: the agent listens and handles requests and answers.
    #[default]
    Sparring,
    /// Questions to the user are not accepted; a subagent decides.
    Sleep,
}

impl ModeKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ModeKind::Sparring => "sparring",
            ModeKind::Sleep => "sleep",
        }
    }

    pub fn parse(text: &str) -> Option<ModeKind> {
        match text {
            "sparring" => Some(ModeKind::Sparring),
            "sleep" => Some(ModeKind::Sleep),
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

fn path_in(data_dir: &Path) -> PathBuf {
    data_dir.join(INTERNAL_DIR).join(FILE_NAME)
}

fn read_projects(data_dir: &Path) -> Map<String, Value> {
    let Ok(bytes) = std::fs::read(path_in(data_dir)) else {
        return Map::new();
    };
    match serde_json::from_slice::<Value>(&bytes) {
        Ok(Value::Object(mut file)) => match file.remove("projects") {
            Some(Value::Object(projects)) => projects,
            _ => Map::new(),
        },
        _ => Map::new(),
    }
}

/// The saved entries for `set`. A missing file is an empty map; any other read or parse error is
/// returned, so `set` never overwrites saved modes it could not read.
fn read_projects_strict(data_dir: &Path) -> std::io::Result<Map<String, Value>> {
    let bytes = match std::fs::read(path_in(data_dir)) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Map::new()),
        Err(error) => return Err(error),
    };
    let invalid =
        |what: &str| std::io::Error::new(std::io::ErrorKind::InvalidData, what.to_string());
    match serde_json::from_slice::<Value>(&bytes).map_err(std::io::Error::other)? {
        Value::Object(mut file) => match file.remove("projects") {
            Some(Value::Object(projects)) => Ok(projects),
            None => Ok(Map::new()),
            Some(_) => Err(invalid("modes.json: projects is not an object")),
        },
        _ => Err(invalid("modes.json is not an object")),
    }
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
    let schedule = entry.get("schedule").and_then(Schedule::from_json);
    State { manual, schedule }
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

/// Fills the `mode`, `modeSource`, and `modeUntil` of the progress table while the project sleeps.
/// All three are left out for sparring.
pub fn fill_progress(table: &mut crate::progress::Progress, shared: &Path, project: &str) {
    let effective = effective(&data_dir_of(shared), project);
    if effective.kind == ModeKind::Sparring {
        return;
    }
    table.mode = Some(effective.kind);
    table.mode_source = Some(effective.source.as_str());
    table.mode_until = effective.until.map(crate::clock::format_utc);
}

/// Read the file, let `change` edit the project's entry, and replace the file in one step. An
/// entry with neither a manual mode nor a window is removed.
fn update(
    data_dir: &Path,
    project: &str,
    change: impl FnOnce(&mut Map<String, Value>),
) -> std::io::Result<()> {
    update_with(data_dir, project, |entry| {
        change(entry);
        Ok::<(), std::convert::Infallible>(())
    })
    .map(|_| ())
}

/// Like `update`, but `change` may refuse; a refusal saves nothing and is the inner error.
fn update_with<E>(
    data_dir: &Path,
    project: &str,
    change: impl FnOnce(&mut Map<String, Value>) -> Result<(), E>,
) -> std::io::Result<Result<(), E>> {
    let path = path_in(data_dir);
    let dir = path.parent().unwrap_or(data_dir);
    crate::server::preferences::create_private_dir(dir)?;
    // Exclusive from the read to the rename: the board and the CLI cannot overwrite each other.
    let _lock = crate::file_store::lock::WriteLock::acquire(&dir.join(LOCK_NAME))
        .map_err(std::io::Error::other)?;
    let mut projects = read_projects_strict(data_dir)?;
    let mut entry = projects
        .get(project)
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    if let Err(error) = change(&mut entry) {
        return Ok(Err(error));
    }
    if entry.contains_key("kind") || entry.contains_key("schedule") {
        projects.insert(project.to_string(), Value::Object(entry));
    } else {
        projects.remove(project);
    }
    let bytes = serde_json::to_vec_pretty(&json!({
        "formatVersion": FORMAT_VERSION,
        "projects": projects,
    }))
    .map_err(std::io::Error::other)?;
    let temporary = dir.join(format!("{FILE_NAME}.{}.tmp", std::process::id()));
    let result = crate::server::preferences::write_owner_only(&temporary, &bytes)
        .and_then(|()| std::fs::rename(&temporary, &path));
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    } else {
        crate::server::preferences::sync_dir(dir);
    }
    result.map(|()| Ok(()))
}

/// Set the manual mode, keeping the daily window. Sparring with no window removes the entry; with
/// a window it is kept, so the manual sparring wins over the schedule.
pub fn set(
    data_dir: &Path,
    project: &str,
    kind: ModeKind,
    now: DateTime<Utc>,
) -> std::io::Result<ProjectMode> {
    let keep = kind != ModeKind::Sparring || state(data_dir, project).schedule.is_some();
    update(data_dir, project, |entry| {
        if keep {
            entry.insert("kind".into(), json!(kind.as_str()));
            entry.insert("since".into(), json!(crate::clock::format_utc(now)));
        } else {
            entry.remove("kind");
            entry.remove("since");
        }
    })?;
    Ok(if keep {
        ProjectMode {
            kind,
            since: Some(now),
        }
    } else {
        ProjectMode::default()
    })
}

/// Forget the manual mode, so the daily window (if any) decides.
pub fn clear_manual(data_dir: &Path, project: &str) -> std::io::Result<()> {
    update(data_dir, project, |entry| {
        entry.remove("kind");
        entry.remove("since");
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

mod state;
#[cfg(test)]
mod tests;

pub use state::State;
