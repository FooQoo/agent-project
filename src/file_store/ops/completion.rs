//! Automatic completion when every valid acceptance condition is checked.

use chrono::{DateTime, Utc};

use super::super::FileStore;
use super::super::frontmatter;
use super::super::qa::QaIndex;
use super::super::reader;
use super::HistoryKind;
use super::{Transition, lane_end, lock_project, stamp, transition_event};
use crate::domain::{Column, ProjectId, RunState};
use crate::error::AgpjError;

const AUTO_DONE_REASON: &str = "All acceptance conditions are complete.";

impl FileStore {
    /// Complete every eligible task in every project. Used before the serve
    /// publishes its first snapshot, so existing work is reconciled on startup.
    pub(crate) fn auto_complete_all(&self, now: DateTime<Utc>) -> Result<bool, AgpjError> {
        let mut changed = false;
        for project in self.projects()? {
            changed |= self.auto_complete_project(&project.id, now)?;
        }
        Ok(changed)
    }

    /// Recheck the latest task files while holding the project write lock.
    /// Invalid or ineligible tasks are left alone; moving a task to Done makes
    /// subsequent watch notifications idempotent.
    pub(crate) fn auto_complete_project(
        &self,
        project: &ProjectId,
        now: DateTime<Utc>,
    ) -> Result<bool, AgpjError> {
        let shared = self.shared_dir();
        let (project_dir, _lock) = lock_project(shared, project)?;
        let mut changed = false;
        // A task that waits for answers is not completed. The index is read once under the lock.
        let index = QaIndex::load(shared, &project_dir)?;

        for id in reader::task_ids(&project_dir)? {
            // Reconcile direct edits before considering automatic completion.
            let dir = super::super::layout::task_dir(&project_dir, id);
            let meta_path = super::super::layout::task_json(&dir);
            let md_path = super::super::layout::task_md(&dir);
            let Some(mut metadata) = std::fs::read(&meta_path).ok().and_then(|bytes| {
                serde_json::from_slice::<super::super::model::TaskMetadata>(&bytes).ok()
            }) else {
                continue;
            };
            let Ok(markdown) = std::fs::read(&md_path) else {
                continue;
            };
            let mismatched = metadata
                .content_digest
                .as_ref()
                .is_some_and(|digest| *digest != super::save::content_digest(&markdown));
            if metadata.content_digest.is_none() {
                if reader::read_task_strict(shared, project, id).is_err() {
                    continue;
                }
                // Bootstrapping a legacy task only changes task.json. Rewriting TASK.md here
                // would change its modification time and make an unchanged task look edited.
                metadata.content_digest = Some(super::save::content_digest(&markdown));
                let mut json = serde_json::to_vec_pretty(&metadata)
                    .map_err(|error| super::super::io::write_error("task.json", error))?;
                json.push(b'\n');
                let label = super::super::layout::shared_label(shared, &meta_path);
                super::super::io::write_atomic(&meta_path, &json, &label)?;
                changed = true;
            } else if mismatched {
                let Ok(task) = reader::read_task_strict(shared, project, id) else {
                    continue;
                };
                super::save_task_planning(shared, &project_dir, &task)?;
                changed = true;
            }
            if metadata.external_edit
                || (mismatched && matches!(metadata.task_status, Column::Doing | Column::Done))
            {
                continue;
            }
            let Ok(loaded) = reader::read_task_lenient(shared, project, id) else {
                continue;
            };
            let task = loaded.view;
            if task.assignee != crate::domain::Assignee::Ai
                || task.draft.is_some()
                || task.archived_at.is_some()
                || !matches!(task.task_status, Column::Todo | Column::Doing)
                || index.is_waiting(&task.question_ids, task.questions_error)
                || task.acceptance_error
                || task.acceptance.is_empty()
                || task.acceptance.iter().any(|item| !item.done)
            {
                continue;
            }

            // Re-read under the lock rather than relying on a prior snapshot.
            let Ok(mut task) = reader::read_task_strict(shared, project, id) else {
                continue;
            };
            let text = match reader::read_body(shared, project, id) {
                Ok(text) => text,
                Err(_) => continue,
            };
            let parsed = frontmatter::parse(&text);
            if !parsed.defects.is_empty()
                || parsed.acceptance.is_empty()
                || parsed.acceptance.iter().any(|item| !item.done)
                || task.assignee != crate::domain::Assignee::Ai
                || task.draft.is_some()
                || task.archived_at.is_some()
                || !matches!(task.task_status, Column::Todo | Column::Doing)
            {
                continue;
            }

            let at = stamp(now);
            let position = lane_end(shared, project, Column::Done)?;
            let next_state = RunState::Idle;
            let event = transition_event(Transition {
                language: self.history_language(project, &task)?,
                kind: HistoryKind::Lane,
                reason: Some(
                    self.history_language(project, &task)?
                        .text(AUTO_DONE_REASON, "すべての完了条件を達成しました。"),
                ),
                from_task: Some(task.task_status),
                to_task: Column::Done,
                from_agent: Some(task.agent_status),
                to_agent: next_state,
                at,
            })?;
            if task.agent_status != next_state {
                task.agent_status_changed_at = at;
            }
            task.task_status = Column::Done;
            task.task_status_changed_at = at;
            task.agent_status = next_state;
            task.position = position;
            task.updated_at = at;
            task.history.push(event);
            self.save_done_task(project, &project_dir, &mut task, now)?;
            changed = true;
        }
        Ok(changed)
    }
}

#[cfg(test)]
mod release_tests;
#[cfg(test)]
mod tests;
