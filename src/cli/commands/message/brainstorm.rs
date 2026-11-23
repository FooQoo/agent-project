//! `message ask --brainstorm`: the questions belong to the project's brainstorm target.

use crate::error::AgpjError;
use crate::mode::{self, ModeKind, Target};

/// In sleep mode (sleep wins over everything) the question is held: `(None, true)`. Otherwise,
/// with `--brainstorm`, return the target of the brainstorm in effect, checked to still exist.
/// Without the flag the question is an ordinary one and the target is `None`.
pub(super) fn target_for_ask(
    shared: &std::path::Path,
    project: &crate::domain::ProjectId,
    brainstorm: bool,
) -> Result<(Option<Target>, bool), AgpjError> {
    let data_dir = mode::data_dir_of(shared);
    if mode::current(&data_dir, project.as_str()) == ModeKind::Sleep {
        return Ok((None, true));
    }
    if !brainstorm {
        return Ok((None, false));
    }
    let target = mode::brainstorm_target(&data_dir, project.as_str())
        .ok_or(AgpjError::BrainstormNotActive)?;
    mode::check_target(shared, project, &target)?;
    Ok((Some(target), false))
}
