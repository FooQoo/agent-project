//! Accepting what a person wrote: a request, or the answer to a question. The acceptance is the
//! agent's receipt; it is written once.

use chrono::{DateTime, Utc};

use super::super::{HistoryKind, Transition, lock_project, save_task, stamp, transition_event};
use super::message_change;
use crate::domain::{Actor, Assignee, Column, ProjectId, RunState};
use crate::error::AgpjError;
use crate::file_store::FileStore;
use crate::file_store::history::{ParseMode, parse_task};
use crate::file_store::qa::{
    Message, MessageState, QaIndex, QuestionView, RequestView, read_message,
};
use crate::file_store::reader;
use crate::file_store::transaction::save_batch_locked;

/// A message after an agent accepted it.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::large_enum_variant)] // a question view is larger than a request view; boxing would change every caller
pub enum Accepted {
    Request(RequestView),
    Question(QuestionView),
}

impl FileStore {
    /// An agent accepts a request or the answer of a question. The first acceptance is final:
    /// a second one is `request_accepted` or `answer_accepted`. A withdrawn request takes none
    /// (`request_withdrawn`), and neither does a question without an answer (`question_open`,
    /// `question_withdrawn`).
    pub fn accept_message(
        &self,
        project: &ProjectId,
        id: &str,
        actor: Actor,
        now: DateTime<Utc>,
    ) -> Result<Accepted, AgpjError> {
        if actor != Actor::Ai {
            return Err(AgpjError::AiOnly {
                action: "accepting a message".into(),
            });
        }
        let shared = self.shared_dir();
        let (project_dir, _lock) = lock_project(shared, project)?;
        let message = match read_message(shared, &project_dir, id)? {
            Message::Request(mut request) => {
                match request.state() {
                    MessageState::Withdrawn => {
                        return Err(AgpjError::RequestWithdrawn { id: id.to_owned() });
                    }
                    MessageState::Closed => {
                        return Err(AgpjError::RequestAccepted { id: id.to_owned() });
                    }
                    MessageState::Open => {}
                }
                request.accepted_at = Some(stamp(now));
                Message::Request(request)
            }
            Message::Question(mut question) => {
                match question.state() {
                    MessageState::Withdrawn => {
                        return Err(AgpjError::QuestionWithdrawn {
                            ids: vec![id.to_owned()],
                        });
                    }
                    MessageState::Open => {
                        return Err(AgpjError::QuestionOpen { id: id.to_owned() });
                    }
                    MessageState::Closed => {}
                }
                let reply = question.reply.as_mut().expect("an answered question");
                if reply.accepted_at.is_some() {
                    // A retry after a failed resume: finish the resume (a no-op when it
                    // already happened), then report the answer as taken in.
                    self.resume_after_answers(project, &project_dir, &[question], now)?;
                    return Err(AgpjError::AnswerAccepted { id: id.to_owned() });
                }
                reply.accepted_at = Some(stamp(now));
                Message::Question(question)
            }
        };
        let change = message_change(&project_dir, &message, false)?;
        save_batch_locked(&project_dir, vec![change])?;
        if let Message::Question(question) = &message {
            self.resume_after_answers(project, &project_dir, std::slice::from_ref(question), now)?;
        }
        Ok(match message {
            Message::Request(request) => Accepted::Request(RequestView::of(&request)),
            Message::Question(question) => {
                Accepted::Question(QuestionView::of(&question, Vec::new()))
            }
        })
    }

    /// Accept every answer among the questions `ids` that no agent took in yet: the wait that
    /// returned them has handed them over. Open, withdrawn and accepted questions are left as
    /// they are. Returns the IDs written.
    pub fn accept_answers(
        &self,
        project: &ProjectId,
        ids: &[String],
        now: DateTime<Utc>,
    ) -> Result<Vec<String>, AgpjError> {
        let shared = self.shared_dir();
        let (project_dir, _lock) = lock_project(shared, project)?;
        let at = stamp(now);
        let mut changes = Vec::new();
        let mut written = Vec::new();
        let mut answered = Vec::new();
        let mut retried = Vec::new();
        for id in ids {
            let Message::Question(mut question) = read_message(shared, &project_dir, id)? else {
                continue;
            };
            if question.state() != MessageState::Closed {
                continue;
            }
            let Some(reply) = question.reply.as_mut() else {
                continue;
            };
            if reply.accepted_at.is_some() {
                // Taken in before: only finish a resume that an earlier failure left undone.
                retried.push(question);
                continue;
            }
            reply.accepted_at = Some(at);
            changes.push(message_change(
                &project_dir,
                &Message::Question(question.clone()),
                false,
            )?);
            written.push(id.clone());
            answered.push(question);
        }
        if !changes.is_empty() {
            save_batch_locked(&project_dir, changes)?;
        }
        answered.append(&mut retried);
        if !answered.is_empty() {
            self.resume_after_answers(project, &project_dir, &answered, now)?;
        }
        Ok(written)
    }

    /// After an agent took in answers: each AI task in Doing that is paused, refers to one of
    /// these questions, and now refers to no open question goes back to running, with a
    /// history entry. Runs under the project lock the caller holds.
    fn resume_after_answers(
        &self,
        project: &ProjectId,
        project_dir: &std::path::Path,
        answered: &[crate::file_store::qa::Question],
        now: DateTime<Utc>,
    ) -> Result<(), AgpjError> {
        let shared = self.shared_dir();
        let index = QaIndex::load(shared, project_dir)?;
        let mut by_task: Vec<(i64, Vec<&str>)> = Vec::new();
        for question in answered {
            for task in &question.tasks {
                match by_task.iter_mut().find(|(id, _)| id == task) {
                    Some((_, ids)) => ids.push(question.id.as_str()),
                    None => by_task.push((*task, vec![question.id.as_str()])),
                }
            }
        }
        by_task.sort_by_key(|(id, _)| *id);
        let at = stamp(now);
        for (id, questions) in by_task {
            let Ok(mut task) = reader::read_task_strict(shared, project, id) else {
                continue;
            };
            if task.archived_at.is_some()
                || task.draft.is_some()
                || task.assignee != Assignee::Ai
                || task.task_status != Column::Doing
                || task.agent_status != RunState::Paused
            {
                continue;
            }
            let Ok(text) = reader::read_body(shared, project, id) else {
                continue;
            };
            let parsed = parse_task(&text, ParseMode::Stored);
            if index.is_waiting(&parsed.questions, !parsed.questions_defects.is_empty()) {
                continue;
            }
            let language = self.history_language(project, &task)?;
            let listed = questions.join(", ");
            let english = format!("Resumed after the answers to {listed}.");
            let japanese = format!("{listed} の回答を受け取ったので再開しました。");
            task.history.push(transition_event(Transition {
                language,
                kind: HistoryKind::Agent,
                reason: Some(language.text(&english, &japanese)),
                from_task: Some(task.task_status),
                to_task: task.task_status,
                from_agent: Some(RunState::Paused),
                to_agent: RunState::Running,
                at,
            })?);
            task.agent_status = RunState::Running;
            task.agent_status_changed_at = at;
            task.updated_at = at;
            save_task(shared, project_dir, &task)?;
        }
        Ok(())
    }
}
