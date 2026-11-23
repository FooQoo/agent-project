//! Held questions: what the project's mode does to a question asked during sleep.
//!
//! In sleep mode `message ask` stores the questions as held. They are released, all at once,
//! when the mode in effect is no longer sleep. The end of a daily window is noticed lazily, so
//! every place that looks at the project (the CLI, the progress table, the board's stream) calls
//! `release_held`, and the release itself is exactly-once under the project lock.

use std::path::Path;

use chrono::Utc;

use super::{ModeKind, current, data_dir_of};
use crate::domain::ProjectId;
use crate::error::AgpjError;
use crate::file_store::FileStore;
use crate::file_store::qa::{MessageState, message_ids, read_message};

/// Release the project's held questions if it is not asleep now. Returns the IDs released, in
/// the order they were held; empty when asleep or when nothing is held.
pub fn release_held(shared: &Path, project: &ProjectId) -> Result<Vec<String>, AgpjError> {
    if current(&data_dir_of(shared), project.as_str()) == ModeKind::Sleep {
        return Ok(Vec::new());
    }
    if held_ids(shared, project).is_empty() {
        return Ok(Vec::new());
    }
    FileStore::open(shared).release_held_questions(project, Utc::now())
}

/// The IDs of the questions held now, lowest number first. Unreadable messages are skipped.
pub fn held_ids(shared: &Path, project: &ProjectId) -> Vec<String> {
    let project_dir = crate::file_store::layout::project_dir(shared, project);
    let Ok(ids) = message_ids(&project_dir) else {
        return Vec::new();
    };
    ids.into_iter()
        .filter(|id| {
            matches!(
                read_message(shared, &project_dir, id),
                Ok(crate::file_store::qa::Message::Question(q)) if q.state() == MessageState::Held
            )
        })
        .collect()
}

/// How many questions the project holds; the board shows it next to the mode.
pub fn held_count(shared: &Path, project: &str) -> usize {
    project
        .parse::<ProjectId>()
        .map_or(0, |project| held_ids(shared, &project).len())
}
