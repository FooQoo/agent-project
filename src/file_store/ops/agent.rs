//! Agent operations: lane moves and run states.
//!
//! A lane move returns the task to idle; the questions a task refers to stay. A run-state
//! change keeps the lane. A completion report is an idle state in Doing, so
//! the task stays in Doing until the coordinator moves it.

use chrono::{DateTime, Utc};

use super::super::FileStore;
use super::super::model::TaskView;
use super::super::reader;
use super::acceptance_check::ensure_acceptance_met;
use super::send::{check_sender, open_questions};
use super::{
    HistoryKind, Transition, ensure_ai_task_not_done, lane_end, lock_project, save_task, stamp,
    task_waits, transition_event,
};
use crate::domain::{
    Actor, ChangeContext, Changed, Column, Draft, ProjectId, RunState, check_state_change,
    check_work_state,
};
use crate::error::AgpjError;

/// Who moves: the CLI (drafts stay), the CLI with the open questions confirmed (`--force`), a
/// person in the Web UI, or that person after confirming the open questions.
#[derive(Clone, Copy, PartialEq, Eq)]
enum MoveMode {
    Agent,
    AgentConfirmed,
    User,
    UserConfirmed,
}

impl FileStore {
    /// Move a task to the end of a lane. A draft is refused for agents.
    pub fn move_work_task(
        &self,
        project: &ProjectId,
        id: i64,
        to: Column,
        ctx: ChangeContext,
        now: DateTime<Utc>,
    ) -> Result<Changed<TaskView>, AgpjError> {
        self.move_task(project, id, to, None, ctx, now, MoveMode::Agent)
    }

    /// [`FileStore::move_work_task`] after a person confirmed the open questions of a card
    /// leaving the Backlog (`mv --force`).
    pub fn move_work_task_confirmed(
        &self,
        project: &ProjectId,
        id: i64,
        to: Column,
        ctx: ChangeContext,
        now: DateTime<Utc>,
    ) -> Result<Changed<TaskView>, AgpjError> {
        self.move_task(project, id, to, None, ctx, now, MoveMode::AgentConfirmed)
    }

    /// The same move for a user in the Web UI: drafts can be moved.
    pub fn move_work_task_for_user(
        &self,
        project: &ProjectId,
        id: i64,
        to: Column,
        ctx: ChangeContext,
        now: DateTime<Utc>,
    ) -> Result<Changed<TaskView>, AgpjError> {
        self.move_task(project, id, to, None, ctx, now, MoveMode::User)
    }

    /// [`FileStore::move_work_task_for_user`] for a move that names the status it started from:
    /// a task that is no longer there is `task_status_changed`, so the history never records a
    /// transition from a status the user did not see.
    pub fn move_work_task_for_user_from(
        &self,
        project: &ProjectId,
        id: i64,
        from: Column,
        to: Column,
        ctx: ChangeContext,
        now: DateTime<Utc>,
    ) -> Result<Changed<TaskView>, AgpjError> {
        self.move_task(project, id, to, Some(from), ctx, now, MoveMode::User)
    }

    /// [`FileStore::move_work_task_for_user_from`] after the person confirmed open questions.
    pub fn move_work_task_for_user_confirmed(
        &self,
        project: &ProjectId,
        id: i64,
        from: Column,
        to: Column,
        ctx: ChangeContext,
        now: DateTime<Utc>,
    ) -> Result<Changed<TaskView>, AgpjError> {
        self.move_task(
            project,
            id,
            to,
            Some(from),
            ctx,
            now,
            MoveMode::UserConfirmed,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn move_task(
        &self,
        project: &ProjectId,
        id: i64,
        to: Column,
        from: Option<Column>,
        ctx: ChangeContext,
        now: DateTime<Utc>,
        mode: MoveMode,
    ) -> Result<Changed<TaskView>, AgpjError> {
        let block_drafts = matches!(mode, MoveMode::Agent | MoveMode::AgentConfirmed);
        let ack = matches!(mode, MoveMode::UserConfirmed | MoveMode::AgentConfirmed);
        let shared = self.shared_dir();
        let (project_dir, _lock) = lock_project(shared, project)?;
        let mut task = reader::read_task_strict(shared, project, id)?;
        if task.archived_at.is_some() {
            return Err(AgpjError::TaskArchived { id });
        }
        if let Some(from) = from
            && task.task_status != from
        {
            return Err(AgpjError::TaskStatusChanged {
                id,
                current: task.task_status,
            });
        }
        ensure_ai_task_not_done(&task, ctx.actor)?;
        if block_drafts && let Some(draft) = task.draft {
            return Err(draft_refusal(id, draft));
        }
        if task.task_status == Column::Backlog && to != Column::Backlog {
            // Only a send leaves the Backlog. A move to Todo is the send: the rules of
            // `send_task_locked` (editing, open questions) apply, and it is the one
            // function the Web UI's send route calls too. Doing and Done follow the same rules,
            // then move like any card.
            if to == Column::Todo {
                return self.send_task_locked(
                    project,
                    &project_dir,
                    task,
                    ctx.actor,
                    ack,
                    &ctx.reason,
                    now,
                );
            }
            if ctx.actor != Actor::Human {
                return Err(AgpjError::TaskInBacklog { id });
            }
            if task.draft == Some(Draft::Human) {
                return Err(AgpjError::TaskIsDraft { id });
            }
            check_sender(Actor::Human, ack, open_questions(shared, project, &[id])?)?;
        }
        if to == Column::Backlog && task.task_status != Column::Backlog {
            // Taking a card back to the Backlog is a person's decision, and only
            // for a card nobody has started.
            if ctx.actor == Actor::Ai {
                return Err(AgpjError::HumanOnly {
                    action: "moving a card back to the Backlog".into(),
                });
            }
            if task.task_status != Column::Todo
                || task.agent_status != RunState::Idle
                || !task.results.is_empty()
            {
                return Err(AgpjError::InvalidValue {
                    field: "to".into(),
                    reason:
                        "Only a Todo card that has not been started can go back to the Backlog."
                            .into(),
                });
            }
        }
        if ctx.actor == Actor::Ai && to == Column::Done && task.task_status != Column::Done {
            ensure_acceptance_met(shared, project, id)?;
            if task_waits(shared, project, id)? {
                return Err(AgpjError::QuestionsUnanswered { id });
            }
        }
        if ctx.actor == Actor::Ai && task.task_status == Column::Doing && to == Column::Todo {
            return Err(AgpjError::HumanOnly {
                action: "moving a task from Doing back to Todo".into(),
            });
        }
        if task.task_status == to {
            return Ok(Changed {
                value: reader::read_task_locked(shared, project, id)?,
                changed: false,
            });
        }
        let at = stamp(now);
        let position = lane_end(shared, project, to)?;
        // Running and paused exist only in Doing, so a task that enters or leaves it is idle.
        let next_state = RunState::Idle;
        let event = transition_event(Transition {
            language: self.history_language(project, &task)?,
            kind: HistoryKind::Lane,
            reason: Some(&ctx.reason),
            from_task: Some(task.task_status),
            to_task: to,
            from_agent: Some(task.agent_status),
            to_agent: next_state,
            at,
        })?;
        if task.agent_status != next_state {
            task.agent_status_changed_at = at;
        }
        task.task_status = to;
        task.task_status_changed_at = at;
        task.position = position;
        task.agent_status = next_state;
        task.updated_at = at;
        task.history.push(event);
        save_task(shared, &project_dir, &task)?;
        Ok(Changed {
            value: reader::read_task_locked(shared, project, id)?,
            changed: true,
        })
    }

    /// Change a task's run state. Only an AI-owned task in Doing can run or pause, and only
    /// idle to running, running to paused, and paused to running are allowed. An agent cannot
    /// run a task that waits for answers. A state that is already set is not a change,
    /// whatever the reason. Waiting is never stored: it is read from the questions.
    pub fn set_agent_status(
        &self,
        project: &ProjectId,
        id: i64,
        state: RunState,
        ctx: ChangeContext,
        now: DateTime<Utc>,
    ) -> Result<Changed<TaskView>, AgpjError> {
        if ctx.actor == Actor::Ai && state == RunState::Paused {
            // An agent never pauses itself: asking a question pauses the task, and a person
            // may pause it.
            return Err(AgpjError::HumanOnly {
                action: "pausing a task".into(),
            });
        }
        let shared = self.shared_dir();
        let (project_dir, _lock) = lock_project(shared, project)?;
        let mut task = reader::read_task_strict(shared, project, id)?;
        if task.archived_at.is_some() {
            return Err(AgpjError::TaskArchived { id });
        }
        ensure_ai_task_not_done(&task, ctx.actor)?;
        if let Some(draft) = task.draft {
            return Err(draft_refusal(id, draft));
        }
        if task.task_status == Column::Backlog {
            return Err(AgpjError::TaskInBacklog { id });
        }
        check_work_state(task.assignee, task.task_status, state)?;
        if task.agent_status != state {
            check_state_change(task.agent_status, state)?;
        }
        if ctx.actor == Actor::Ai && state == RunState::Running && task_waits(shared, project, id)?
        {
            return Err(AgpjError::QuestionsUnanswered { id });
        }
        if task.agent_status == state {
            return Ok(Changed {
                value: reader::read_task_locked(shared, project, id)?,
                changed: false,
            });
        }
        let at = stamp(now);
        let event = transition_event(Transition {
            language: self.history_language(project, &task)?,
            kind: HistoryKind::Agent,
            reason: Some(&ctx.reason),
            from_task: Some(task.task_status),
            to_task: task.task_status,
            from_agent: Some(task.agent_status),
            to_agent: state,
            at,
        })?;
        task.agent_status = state;
        task.agent_status_changed_at = at;
        task.updated_at = at;
        task.history.push(event);
        save_task(shared, &project_dir, &task)?;
        Ok(Changed {
            value: reader::read_task_locked(shared, project, id)?,
            changed: true,
        })
    }
}

/// The refusal of a move or state change on a draft: a person's own draft waits for its
/// owner to finalize it.
fn draft_refusal(id: i64, draft: crate::domain::Draft) -> AgpjError {
    match draft {
        crate::domain::Draft::Human => AgpjError::TaskIsDraft { id },
    }
}
