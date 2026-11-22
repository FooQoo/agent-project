//! The project's versions: the file stems of `*.md` files directly inside `docs/release/`.

use std::fs::{self, OpenOptions};
use std::io::ErrorKind;
use std::path::Path;

use super::{io, layout};
use crate::domain::{MAX_VERSIONS, compare_versions, validate_version_name};
use crate::error::AgboardError;

/// The listed versions in natural order, newest first (`v1.10.0` before `v1.9.0`). A file
/// counts when it is a plain `.md` file directly in `docs/release/`, is not hidden, and its stem
/// is a valid version name (`TEMPLATE` is not one). Subfolders, other extensions and other names are ignored. A missing
/// or unreadable folder lists nothing.
pub fn listed_versions(project_dir: &Path) -> Vec<String> {
    let Ok(entries) = fs::read_dir(layout::release_dir(project_dir)) else {
        return Vec::new();
    };
    let mut versions: Vec<String> = entries
        .filter_map(Result::ok)
        .filter(|entry| entry.path().is_file())
        .filter_map(|entry| {
            let name = entry.file_name().to_str()?.to_owned();
            let stem = name.strip_suffix(".md")?;
            validate_version_name(stem).ok()
        })
        .collect();
    versions.sort_by(|a, b| compare_versions(b, a));
    versions.dedup();
    versions
}

/// The stored current version when it is still listed, else `None`. The project file keeps the
/// stored name, so listing the file again brings the current version back.
pub fn effective_current(listed: &[String], stored: Option<&str>) -> Option<String> {
    stored
        .filter(|name| listed.iter().any(|v| v == name))
        .map(str::to_owned)
}

/// List `name` by creating an empty `docs/release/<name>.md`, and the folders above it when
/// they are missing. An existing file is never touched: `Ok(false)` says it was already listed.
pub fn create_release_file(
    shared: &Path,
    project_dir: &Path,
    name: &str,
) -> Result<bool, AgboardError> {
    let listed = listed_versions(project_dir);
    if listed.iter().any(|v| v == name) {
        return Ok(false);
    }
    if listed.len() >= MAX_VERSIONS {
        return Err(AgboardError::InvalidArgument {
            message: "A project lists at most 200 versions.".into(),
            command: None,
        });
    }
    let dir = layout::release_dir(project_dir);
    let path = dir.join(format!("{name}.md"));
    let label = layout::shared_label(shared, &path);
    fs::create_dir_all(&dir).map_err(|e| io::write_error(&label, e))?;
    // `create_new` makes the file in one step and fails if it exists, so an empty file appears
    // whole or not at all and an existing file is never overwritten.
    match OpenOptions::new().write(true).create_new(true).open(&path) {
        Ok(file) => {
            file.sync_all().map_err(|e| io::write_error(&label, e))?;
            io::sync_dir(&dir).map_err(|e| io::write_error(&label, e))?;
            Ok(true)
        }
        Err(error) if error.kind() == ErrorKind::AlreadyExists => {
            // A file system that ignores case can hold the name under another spelling.
            if listed_versions(project_dir).iter().any(|v| v == name) {
                Ok(false)
            } else {
                Err(AgboardError::InvalidArgument {
                    message: format!(
                        "A file named {name}.md already exists in docs/release/ but does not list this version."
                    ),
                    command: None,
                })
            }
        }
        Err(error) => Err(io::write_error(&label, error)),
    }
}
