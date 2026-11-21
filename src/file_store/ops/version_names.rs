//! Checking version names against a project's list.

use crate::domain::{MAX_VERSIONS, ProjectId, validate_version_name};
use crate::error::AgboardError;
use crate::file_store::reader;

use super::version::VersionChoice;

fn invalid(message: impl Into<String>) -> AgboardError {
    AgboardError::InvalidArgument {
        message: message.into(),
        command: None,
    }
}

/// The concrete version a choice stands for, checked against the project's list.
pub(super) fn resolve(
    shared: &std::path::Path,
    project: &ProjectId,
    choice: &VersionChoice,
) -> Result<Option<String>, AgboardError> {
    let file = reader::read_project_file(shared, project)?;
    match choice {
        VersionChoice::Unset => Ok(None),
        VersionChoice::Current => file
            .current_version
            .map(Some)
            .ok_or(AgboardError::NoCurrentVersion),
        VersionChoice::Named(name) => {
            let name = validate_version_name(name)?;
            if file.versions.contains(&name) {
                Ok(Some(name))
            } else {
                Err(invalid(format!(
                    "The project has no version {name}. Add it first."
                )))
            }
        }
    }
}

pub(super) fn add_listed(versions: &mut Vec<String>, name: &str) -> Result<bool, AgboardError> {
    if versions.iter().any(|v| v == name) {
        return Ok(false);
    }
    if versions.len() >= MAX_VERSIONS {
        return Err(invalid("A project lists at most 200 versions."));
    }
    versions.push(name.to_owned());
    Ok(true)
}
