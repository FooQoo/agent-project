//! Checking version names against a project's list.

use crate::domain::{ProjectId, validate_version_name};
use crate::error::AgpjError;
use crate::file_store::releases::{effective_current, listed_versions};
use crate::file_store::{layout, reader};

use super::version::VersionChoice;

fn invalid(message: impl Into<String>) -> AgpjError {
    AgpjError::InvalidArgument {
        message: message.into(),
        command: None,
    }
}

/// The concrete version a choice stands for, checked against the project's list. A current
/// version whose file is missing counts as no current version.
pub(super) fn resolve(
    shared: &std::path::Path,
    project: &ProjectId,
    choice: &VersionChoice,
) -> Result<Option<String>, AgpjError> {
    let file = reader::read_project_file(shared, project)?;
    let listed = listed_versions(&layout::project_dir(shared, project));
    match choice {
        VersionChoice::Unset => Ok(None),
        VersionChoice::Current => effective_current(&listed, file.current_version.as_deref())
            .map(Some)
            .ok_or(AgpjError::NoCurrentVersion),
        VersionChoice::Named(name) => {
            let name = validate_version_name(name)?;
            if listed.contains(&name) {
                Ok(Some(name))
            } else {
                Err(invalid(format!(
                    "The project has no version {name}. Add it first."
                )))
            }
        }
    }
}
