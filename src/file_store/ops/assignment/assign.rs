//! Giving a task to an agent, and putting running tasks back to waiting.

use super::*;

impl FileStore {
    /// Give a task to an agent, or take its agent away. A person's action: the web UI calls it.
    /// The generation goes up, and a running task goes back to waiting.
    pub fn assign_task_agent(
        &self,
        project: &ProjectId,
        id: i64,
        agent_id: Option<&str>,
        now: DateTime<Utc>,
    ) -> Result<crate::file_store::model::TaskView, AgpjError> {
        let shared = self.shared_dir();
        let (dir, _lock) = lock_project(shared, project)?;
        let mut task = reader::read_task_strict(shared, project, id)?;
        if task.archived_at.is_some() {
            return Err(AgpjError::TaskArchived { id });
        }
        if task.assignee != Assignee::Ai {
            return Err(AgpjError::NotAssignee { id });
        }
        match agent_id {
            Some(agent_id) => {
                let agents: AgentsFile = load(shared, &agents_path(&dir))?;
                let agent = find_agent(&agents, agent_id)?;
                if agent.archived_at.is_some() {
                    return Err(AgpjError::AgentNotFound {
                        agent: agent_id.into(),
                    });
                }
            }
            None if task.task_status == Column::Doing => {
                return Err(AgpjError::InvalidValue {
                    field: "agentId".into(),
                    reason: "A Doing task needs an agent.".into(),
                });
            }
            None => {}
        }
        let at = stamp(now);
        if task.agent_id.as_deref() == agent_id {
            return reader::read_task_locked(shared, project, id);
        }
        task.agent_id = agent_id.map(str::to_string);
        task.assignment_generation += 1;
        if task.agent_status == RunState::Running {
            let event = transition_event(Transition {
                language: self.history_language(project, &task)?,
                kind: HistoryKind::Agent,
                reason: Some("The task was given to another agent; it is waiting."),
                from_task: Some(task.task_status),
                to_task: task.task_status,
                from_agent: Some(RunState::Running),
                to_agent: RunState::Paused,
                at,
            })?;
            task.agent_status = RunState::Paused;
            task.agent_status_changed_at = at;
            task.history.push(event);
        }
        task.run_connection_id = None;
        task.updated_at = at;
        save_task(shared, &dir, &task)?;
        reader::read_task_locked(shared, project, id)
    }

    /// Pause the running tasks that `select` picks, with one history entry each. The project
    /// lock is held by the caller. Returns the IDs.
    pub(super) fn pause_runs_locked(
        &self,
        project: &ProjectId,
        dir: &std::path::Path,
        select: impl Fn(&TaskFile) -> bool,
        reason: &str,
        now: DateTime<Utc>,
    ) -> Result<Vec<i64>, AgpjError> {
        let shared = self.shared_dir();
        let mut paused = Vec::new();
        for id in reader::task_ids(dir)? {
            let Ok(mut task) = reader::read_task_strict(shared, project, id) else {
                continue;
            };
            if task.agent_status != RunState::Running
                || task.run_connection_id.is_none()
                || task.archived_at.is_some()
                || !select(&task)
            {
                continue;
            }
            let event = transition_event(Transition {
                language: self.history_language(project, &task)?,
                kind: HistoryKind::Agent,
                reason: Some(reason),
                from_task: Some(task.task_status),
                to_task: task.task_status,
                from_agent: Some(RunState::Running),
                to_agent: RunState::Paused,
                at: now,
            })?;
            task.agent_status = RunState::Paused;
            task.agent_status_changed_at = now;
            task.run_connection_id = None;
            task.updated_at = now;
            task.history.push(event);
            save_task(shared, dir, &task)?;
            paused.push(id);
        }
        Ok(paused)
    }
}
