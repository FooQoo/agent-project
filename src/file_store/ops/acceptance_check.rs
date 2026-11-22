//! The acceptance check an agent's move to Done passes first.

use super::super::frontmatter;
use super::super::layout;
use super::super::reader;
use crate::domain::ProjectId;
use crate::error::AgpjError;

/// An agent may move a task to Done only when its frontmatter reads and every
/// condition is done. A person's move is never checked here.
pub(super) fn ensure_acceptance_met(
    shared: &std::path::Path,
    project: &ProjectId,
    id: i64,
) -> Result<(), AgpjError> {
    let text = reader::read_body(shared, project, id)?;
    let parsed = frontmatter::parse(&text);
    if let Some(defect) = parsed.defects.into_iter().next() {
        let path = layout::task_md(&layout::task_dir(&layout::project_dir(shared, project), id));
        return Err(AgpjError::FileInvalid {
            file: layout::shared_label(shared, &path),
            field: defect.field,
            reason: defect.reason,
        });
    }
    let unmet: Vec<String> = parsed
        .acceptance
        .iter()
        .filter(|item| !item.done)
        .map(|item| item.id.clone())
        .collect();
    if unmet.is_empty() {
        Ok(())
    } else {
        Err(AgpjError::AcceptanceUnmet { id, unmet })
    }
}
