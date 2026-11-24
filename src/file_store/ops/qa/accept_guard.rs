use std::path::Path;

use crate::domain::ProjectId;
use crate::error::AgpjError;
use crate::file_store::FileStore;
use crate::file_store::history::{ParseMode, parse_task};
use crate::file_store::qa::Question;
use crate::file_store::reader;

impl FileStore {
    /// Check every current task reference before accepting an answer. The project lock keeps
    /// these references and their assignments stable until the receipt is saved.
    pub(super) fn ensure_answer_session_may_write(
        &self,
        project: &ProjectId,
        project_dir: &Path,
        question: &Question,
    ) -> Result<(), AgpjError> {
        let shared = self.shared_dir();
        // The question's task list is authoritative for ordinary links. Check it first and
        // fail closed if one of those task files is malformed.
        for id in &question.tasks {
            let task = reader::read_task_strict(shared, project, *id)?;
            super::super::assignment::ensure_session_may_write(shared, project_dir, &task)?;
        }
        for id in reader::task_ids(project_dir)? {
            if question.tasks.contains(&id) {
                continue;
            }
            // A broken unrelated task must not prevent an answer from being taken in. A
            // readable task that names this question still gets the full authority check.
            let Ok(task) = reader::read_task_strict(shared, project, id) else {
                continue;
            };
            let Ok(text) = reader::read_body(shared, project, id) else {
                continue;
            };
            let parsed = parse_task(&text, ParseMode::Stored);
            if parsed.questions.iter().any(|q| q == &question.id) {
                super::super::assignment::ensure_session_may_write(shared, project_dir, &task)?;
            }
        }
        Ok(())
    }
}
