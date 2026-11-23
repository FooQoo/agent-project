//! Releasing held questions: the questions asked during sleep mode reach the person at once.

use chrono::{DateTime, Utc};

use super::super::{HistoryKind, Transition, lock_project, stamp, transition_event};
use super::history_images::qualify_attachment_refs;
use super::{message_change, options_line, referrers};
use crate::domain::{ProjectId, RunState};
use crate::error::AgpjError;
use crate::file_store::FileStore;
use crate::file_store::model::HistoryRecord;
use crate::file_store::qa::{self, Message, MessageState, Question, message_ids, read_message};
use crate::file_store::transaction::save_batch_locked;

impl FileStore {
    /// Release every held question of the project, in the order they were held. Each becomes an
    /// ordinary open question: the tasks that refer to it get the "Asked a question" record, and
    /// a running task pauses to wait, as an ordinary ask does. Everything is written in one batch
    /// under the project lock, and what is already released is not held any more, so a second call
    /// finds nothing. Returns the IDs released.
    pub fn release_held_questions(
        &self,
        project: &ProjectId,
        now: DateTime<Utc>,
    ) -> Result<Vec<String>, AgpjError> {
        let shared = self.shared_dir();
        let (project_dir, _lock) = lock_project(shared, project)?;
        let mut held: Vec<Question> = Vec::new();
        for id in message_ids(&project_dir)? {
            // A malformed message is reported by the snapshot's problems; it must not keep the
            // valid held questions back.
            if let Ok(Message::Question(question)) = read_message(shared, &project_dir, &id)
                && question.state() == MessageState::Held
            {
                held.push(question);
            }
        }
        if held.is_empty() {
            return Ok(Vec::new());
        }
        let at = stamp(now);
        let ids: Vec<String> = held.iter().map(|q| q.id.clone()).collect();
        let mut changes = Vec::new();
        for question in &mut held {
            question.held_at = None;
            changes.push(message_change(
                &project_dir,
                &Message::Question(question.clone()),
                false,
            )?);
        }
        for mut referrer in referrers(shared, project, &project_dir, &ids)? {
            if referrer.task.archived_at.is_some() {
                continue;
            }
            let mine: Vec<&Question> = held
                .iter()
                .filter(|q| referrer.questions.contains(&q.id))
                .collect();
            let language = self.history_language(project, &referrer.task)?;
            for question in &mine {
                let options = options_line(&question.choices);
                let folder = qa::attachments_dir(&qa::message_dir(&project_dir, &question.id));
                let body = qualify_attachment_refs(&question.body, &question.id, |name| {
                    folder.join(name).is_file()
                });
                let english = format!(
                    "Asked a question ({}):\n\n{body}\n\nOptions: {options}",
                    question.id
                );
                let japanese = format!(
                    "質問しました（{}）:\n\n{body}\n\n選択肢: {options}",
                    question.id
                );
                referrer.task.history.push(HistoryRecord {
                    at,
                    reason: language.text(&english, &japanese).to_owned(),
                });
            }
            if referrer.task.agent_status == RunState::Running {
                let listed = mine
                    .iter()
                    .map(|q| q.id.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                let english = format!("Waiting for the answers to {listed}.");
                let japanese = format!("{listed} の回答を待ちます。");
                referrer.task.history.push(transition_event(Transition {
                    language,
                    kind: HistoryKind::Agent,
                    reason: Some(language.text(&english, &japanese)),
                    from_task: Some(referrer.task.task_status),
                    to_task: referrer.task.task_status,
                    from_agent: Some(RunState::Running),
                    to_agent: RunState::Paused,
                    at,
                })?);
                referrer.task.agent_status = RunState::Paused;
                referrer.task.agent_status_changed_at = at;
            }
            referrer.task.updated_at = at;
            changes.extend(super::super::save::task_pair_changes(
                shared,
                &project_dir,
                &referrer.task,
                Some(&referrer.text),
            )?);
        }
        save_batch_locked(&project_dir, changes)?;
        Ok(ids)
    }
}
