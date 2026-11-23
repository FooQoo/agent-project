//! The file behind the modes: reading `modes.json` and replacing it under a lock.

use std::path::{Path, PathBuf};

use serde_json::{Map, Value, json};

const FORMAT_VERSION: u32 = 1;
pub(super) const INTERNAL_DIR: &str = ".internal";
pub(super) const FILE_NAME: &str = "modes.json";
const LOCK_NAME: &str = "modes.lock";

fn path_in(data_dir: &Path) -> PathBuf {
    data_dir.join(INTERNAL_DIR).join(FILE_NAME)
}

pub(super) fn read_projects(data_dir: &Path) -> Map<String, Value> {
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

/// Read the file, let `change` edit the project's entry, and replace the file in one step. An
/// entry with neither a manual mode nor a window is removed.
pub(super) fn update(
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

/// Like `update`, but `change` may refuse (nothing is saved).
pub(super) fn update_with<E>(
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
