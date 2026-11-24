//! Changing a task's run state, by a person or through a session's connection.

use chrono::{DateTime, Utc};

use super::super::FileStore;
use super::super::model::TaskView;
use super::super::reader;
use super::agent::draft_refusal;
use super::{
    HistoryKind, Transition, ensure_ai_task_not_done, lock_project, save_task, stamp, task_waits,
    transition_event,
};
use crate::domain::{
    Actor, ChangeContext, Changed, Column, ProjectId, RunState, check_state_change,
    check_work_state,
};
use crate::error::AgpjError;

impl FileStore {
    /// Change a task's run state. Only an AI-owned task in Doing can run or pause, and only
    /// idle to running, running to paused, and paused to running are allowed. An agent may
    /// pause its own running task (the stale-running reminder asks for it) but cannot
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
        self.set_agent_status_guarded(project, id, state, ctx, now, None, false)
    }

    /// [`FileStore::set_agent_status`] for a command line with no connection secret. The check
    /// that the project has no additional live agent runs under the same project lock as the save,
    /// so an agent registered meanwhile cannot be bypassed: an AI actor then gets `NotConnected`.
    /// A person's change (the web UI) is allowed either way.
    pub fn set_agent_status_unconnected(
        &self,
        project: &ProjectId,
        id: i64,
        state: RunState,
        ctx: ChangeContext,
        now: DateTime<Utc>,
    ) -> Result<Changed<TaskView>, AgpjError> {
        self.set_agent_status_guarded(project, id, state, ctx, now, None, true)
    }

    /// [`FileStore::set_agent_status`] for a session that acts through its connection. The secret
    /// is checked under the project lock before anything else, so a session without one, with a
    /// replaced one, or from another agent leaves no trace. A free card is taken by the first
    /// connection that sets it running; a card of another agent is refused.
    pub fn set_agent_status_connected(
        &self,
        project: &ProjectId,
        id: i64,
        state: RunState,
        ctx: ChangeContext,
        secret: &str,
        now: DateTime<Utc>,
    ) -> Result<Changed<TaskView>, AgpjError> {
        self.set_agent_status_guarded(project, id, state, ctx, now, Some(secret), false)
    }

    #[allow(clippy::too_many_arguments)]
    fn set_agent_status_guarded(
        &self,
        project: &ProjectId,
        id: i64,
        state: RunState,
        ctx: ChangeContext,
        now: DateTime<Utc>,
        secret: Option<&str>,
        refuse_when_agents: bool,
    ) -> Result<Changed<TaskView>, AgpjError> {
        let shared = self.shared_dir();
        let (project_dir, _lock) = lock_project(shared, project)?;
        if refuse_when_agents
            && secret.is_none()
            && super::assignment::has_other_live_agents_locked(shared, &project_dir)?
        {
            return Err(if ctx.actor == Actor::Ai {
                AgpjError::NotConnected
            } else {
                AgpjError::HumanOnly {
                    action: "Changing a run state from the command line as a person".into(),
                }
            });
        }
        let verified = secret
            .map(|secret| {
                super::assignment::verify_locked(shared, &project_dir, secret, stamp(now))
            })
            .transpose()?;
        let mut task = reader::read_task_strict(shared, project, id)?;
        if let Some(verified) = &verified {
            if task.assignee != crate::domain::Assignee::Ai {
                return Err(AgpjError::NotAssignee { id });
            }
            match task.agent_id.as_deref() {
                Some(owner) if owner != verified.agent_id => {
                    return Err(AgpjError::NotAssignee { id });
                }
                None if state != RunState::Running => {
                    return Err(AgpjError::NotAssignee { id });
                }
                _ => {}
            }
        }
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
        let run_connection = match (&verified, state) {
            (Some(verified), RunState::Running) => Some(verified.connection_id.clone()),
            _ => None,
        };
        let claims = verified
            .as_ref()
            .is_some_and(|verified| task.agent_id.as_deref() != Some(verified.agent_id.as_str()));
        // A request without a verified connection (the web UI) never changes the connection, so
        // the same state is a no-op whatever connection the card is on.
        if task.agent_status == state
            && !claims
            && (verified.is_none() || task.run_connection_id == run_connection)
        {
            return Ok(Changed {
                value: reader::read_task_locked(shared, project, id)?,
                changed: false,
            });
        }
        let at = stamp(now);
        if task.agent_status == state {
            // The same state on another connection (a switch) or a free card: only the owner
            // and the connection change, so there is no history line of a transition.
            if let Some(verified) = &verified {
                if claims {
                    task.agent_id = Some(verified.agent_id.clone());
                    task.assignment_generation += 1;
                }
                task.run_connection_id = run_connection;
                task.updated_at = at;
                save_task(shared, &project_dir, &task)?;
            }
            return Ok(Changed {
                value: reader::read_task_locked(shared, project, id)?,
                changed: true,
            });
        }
        if let Some(verified) = &verified
            && claims
        {
            task.agent_id = Some(verified.agent_id.clone());
            task.assignment_generation += 1;
        }
        task.run_connection_id = run_connection;
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
