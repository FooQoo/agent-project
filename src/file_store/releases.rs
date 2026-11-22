//! The project's versions: the file stems of `*.md` files directly inside `docs/release/`.

use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, Write};
use std::path::Path;

use super::{io, layout};
use crate::domain::{MAX_VERSIONS, compare_versions, validate_version_name};
use crate::error::AgpjError;

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

/// The file name of the release-note template inside `docs/release/`. It never lists a version.
pub const TEMPLATE_FILE: &str = "TEMPLATE.md";

/// The standard release-note template, written to `docs/release/TEMPLATE.md` when the project
/// has none. It follows the shape of the repository's release notes.
pub const STANDARD_TEMPLATE: &str = "\
One sentence that says what this release is.

## Changes

None.

## Fixes

None.

## Breaking changes

None.

## Required update steps

None.
";

/// Create `dir/file_name` with `bytes` unless it exists. The bytes go to a hidden temporary file
/// that is hard-linked into place, so the file appears whole or not at all and an existing file
/// is never overwritten. `Ok(false)` says the file already existed.
fn create_file_whole(
    shared: &Path,
    dir: &Path,
    file_name: &str,
    bytes: &[u8],
) -> Result<bool, AgpjError> {
    let path = dir.join(file_name);
    let label = layout::shared_label(shared, &path);
    let temp = dir.join(format!(
        ".{file_name}.tmp-{}",
        io::random_hex(8).map_err(|e| io::write_error(&label, e))?
    ));
    let result = (|| -> std::io::Result<bool> {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        match fs::hard_link(&temp, &path) {
            Ok(()) => {
                io::sync_dir(dir)?;
                Ok(true)
            }
            Err(error) if error.kind() == ErrorKind::AlreadyExists => Ok(false),
            Err(error) => Err(error),
        }
    })();
    let _ = fs::remove_file(&temp);
    result.map_err(|e| io::write_error(&label, e))
}

/// The text of the release-note template, writing the standard one first when the project has
/// none. An existing template is read as it is and never rewritten.
fn template_text(shared: &Path, dir: &Path) -> Result<String, AgpjError> {
    create_file_whole(shared, dir, TEMPLATE_FILE, STANDARD_TEMPLATE.as_bytes())?;
    let path = dir.join(TEMPLATE_FILE);
    io::read_text(&path, &layout::shared_label(shared, &path))
}

/// List `name` by creating `docs/release/<name>.md` as a copy of the release-note template, and
/// the folders above it when they are missing. A missing template is written first. An existing
/// file is never touched: `Ok(false)` says the version was already listed.
pub fn create_release_file(
    shared: &Path,
    project_dir: &Path,
    name: &str,
) -> Result<bool, AgpjError> {
    let listed = listed_versions(project_dir);
    if listed.iter().any(|v| v == name) {
        return Ok(false);
    }
    if listed.len() >= MAX_VERSIONS {
        return Err(AgpjError::InvalidArgument {
            message: "A project lists at most 200 versions.".into(),
            command: None,
        });
    }
    let dir = layout::release_dir(project_dir);
    let path = dir.join(format!("{name}.md"));
    let label = layout::shared_label(shared, &path);
    fs::create_dir_all(&dir).map_err(|e| io::write_error(&label, e))?;
    let text = template_text(shared, &dir)?;
    if create_file_whole(shared, &dir, &format!("{name}.md"), text.as_bytes())? {
        return Ok(true);
    }
    // A file system that ignores case can hold the name under another spelling.
    if listed_versions(project_dir).iter().any(|v| v == name) {
        Ok(false)
    } else {
        Err(AgpjError::InvalidArgument {
            message: format!(
                "A file named {name}.md already exists in docs/release/ but does not list this version."
            ),
            command: None,
        })
    }
}
