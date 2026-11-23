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

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

const FORMAT_VERSION: u32 = 1;
const INTERNAL_DIR: &str = ".internal";
const FILE_NAME: &str = "modes.json";

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

fn parse_entry(entry: &Value) -> Option<ProjectMode> {
    let kind = entry
        .get("kind")
        .and_then(Value::as_str)
        .and_then(ModeKind::parse)?;
    let since = entry
        .get("since")
        .and_then(Value::as_str)
        .and_then(|text| DateTime::parse_from_rfc3339(text).ok())
        .map(|time| time.with_timezone(&Utc));
    Some(ProjectMode { kind, since })
}

/// The saved mode of a project. A missing, invalid, or unknown entry is sparring.
pub fn load(data_dir: &Path, project: &str) -> ProjectMode {
    read_projects(data_dir)
        .get(project)
        .and_then(parse_entry)
        .unwrap_or_default()
}

/// Every project that has a saved, valid mode entry, for the board's change stream.
pub fn load_all(data_dir: &Path) -> std::collections::BTreeMap<String, ProjectMode> {
    read_projects(data_dir)
        .iter()
        .filter_map(|(id, entry)| parse_entry(entry).map(|mode| (id.clone(), mode)))
        .collect()
}

impl ProjectMode {
    /// The JSON the board's API and event stream carry: `{"mode": "sleep", "since": "…"}`.
    pub fn api_json(&self) -> Value {
        let mut body = json!({"mode": self.kind.as_str()});
        if let Some(since) = self.since {
            body["since"] = Value::String(crate::clock::format_utc(since));
        }
        body
    }
}

/// The saved kind of a project.
pub fn current(data_dir: &Path, project: &str) -> ModeKind {
    load(data_dir, project).kind
}

/// The project's saved mode when it is not sparring, for the `mode` of the progress table.
pub fn non_default(shared: &Path, project: &str) -> Option<ModeKind> {
    let kind = current(&data_dir_of(shared), project);
    (kind != ModeKind::Sparring).then_some(kind)
}

/// Save a project's mode. Sparring removes the entry. The file is replaced in one step.
pub fn set(
    data_dir: &Path,
    project: &str,
    kind: ModeKind,
    now: DateTime<Utc>,
) -> std::io::Result<ProjectMode> {
    let mut projects = read_projects(data_dir);
    let mode = if kind == ModeKind::Sparring {
        projects.remove(project);
        ProjectMode::default()
    } else {
        let since = now;
        projects.insert(
            project.to_string(),
            json!({"kind": kind.as_str(), "since": crate::clock::format_utc(since)}),
        );
        ProjectMode {
            kind,
            since: Some(since),
        }
    };
    let bytes = serde_json::to_vec_pretty(&json!({
        "formatVersion": FORMAT_VERSION,
        "projects": projects,
    }))
    .map_err(std::io::Error::other)?;
    let path = path_in(data_dir);
    let dir = path.parent().unwrap_or(data_dir);
    crate::server::preferences::create_private_dir(dir)?;
    let temporary = dir.join(format!("{FILE_NAME}.{}.tmp", std::process::id()));
    let result = crate::server::preferences::write_owner_only(&temporary, &bytes)
        .and_then(|()| std::fs::rename(&temporary, &path));
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    } else {
        crate::server::preferences::sync_dir(dir);
    }
    result.map(|()| mode)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn default_is_sparring_and_sleep_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let now = Utc.with_ymd_and_hms(2026, 11, 24, 1, 2, 3).unwrap();
        assert_eq!(current(dir.path(), "p1"), ModeKind::Sparring);
        let saved = set(dir.path(), "p1", ModeKind::Sleep, now).unwrap();
        assert_eq!(saved.since, Some(now));
        assert_eq!(load(dir.path(), "p1").kind, ModeKind::Sleep);
        assert_eq!(current(dir.path(), "p2"), ModeKind::Sparring);
        set(dir.path(), "p1", ModeKind::Sparring, now).unwrap();
        assert_eq!(current(dir.path(), "p1"), ModeKind::Sparring);
    }

    #[test]
    fn unknown_kind_and_broken_file_read_as_sparring() {
        let dir = tempfile::tempdir().unwrap();
        let internal = dir.path().join(INTERNAL_DIR);
        std::fs::create_dir_all(&internal).unwrap();
        std::fs::write(
            internal.join(FILE_NAME),
            r#"{"projects":{"p1":{"kind":"brainstorm"}}}"#,
        )
        .unwrap();
        assert_eq!(current(dir.path(), "p1"), ModeKind::Sparring);
        std::fs::write(internal.join(FILE_NAME), "not json").unwrap();
        assert_eq!(current(dir.path(), "p1"), ModeKind::Sparring);
    }
}
