//! Starting a brainstorm: save the mode, then send the agent a request so a waiting agent wakes.

use std::path::Path;

use chrono::{DateTime, Utc};

use super::target::{self, Target};
use super::{data_dir_of, set_brainstorm};
use crate::domain::ProjectId;
use crate::error::{AgpjError, FileAction};
use crate::file_store::FileStore;
use crate::file_store::SendRequestInput;

fn request_text(target: &Target) -> String {
    format!(
        "Start brainstorming on {}. Read its body and what is already decided, then ask one deepening question with `agpj message ask --brainstorm`. Follow `agpj skills brainstorm`. If the mode is no longer brainstorm, just accept this request.",
        target.label()
    )
}

/// Check the target, save brainstorm as the manual mode, and send the request (human to agent)
/// that wakes an `agpj wait` agent. The CLI and the board's API both call this. The mode is saved
/// first, so an agent that wakes sees brainstorm. Returns the target in its canonical form.
pub fn start_brainstorm(
    shared: &Path,
    project: &ProjectId,
    target: Target,
    now: DateTime<Utc>,
) -> Result<Target, AgpjError> {
    let target = target.normalized();
    target::check(shared, project, &target)?;
    set_brainstorm(&data_dir_of(shared), project.as_str(), &target, now).map_err(|error| {
        AgpjError::FileError {
            action: FileAction::Write,
            file: "modes.json".to_string(),
            reason: error.to_string(),
        }
    })?;
    FileStore::open(shared).send_request(
        project,
        SendRequestInput {
            text: request_text(&target),
            images: Vec::new(),
        },
        now,
    )?;
    Ok(target)
}
