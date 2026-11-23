//! `message ask --brainstorm`: the questions belong to the project's brainstorm target.

use crate::error::AgpjError;
use crate::mode::{self, ModeKind, Target};

/// Refuse in sleep mode (sleep wins over everything), then, with `--brainstorm`, return the
/// target of the brainstorm in effect, checked to still exist. Without the flag the question is an
/// ordinary one and `None` is returned.
pub(super) fn target_for_ask(
    shared: &std::path::Path,
    project: &crate::domain::ProjectId,
    brainstorm: bool,
) -> Result<Option<Target>, AgpjError> {
    let data_dir = mode::data_dir_of(shared);
    if mode::current(&data_dir, project.as_str()) == ModeKind::Sleep {
        return Err(AgpjError::SleepMode);
    }
    if !brainstorm {
        return Ok(None);
    }
    let target = mode::brainstorm_target(&data_dir, project.as_str())
        .ok_or(AgpjError::BrainstormNotActive)?;
    mode::check_target(shared, project, &target)?;
    Ok(Some(target))
}
