//! Registering, renaming, and deleting agents.

use super::*;

impl FileStore {
    /// Whether the project has a live agent. Connections are required once a person has
    /// registered the first agent; a project without agents has nobody to be confused with.
    pub fn has_agents(&self, project: &ProjectId) -> Result<bool, AgpjError> {
        Ok(self
            .list_agents(project)?
            .iter()
            .any(|agent| agent.archived_at.is_none()))
    }

    /// The project's agents, oldest first, archived ones last.
    pub fn list_agents(&self, project: &ProjectId) -> Result<Vec<AgentView>, AgpjError> {
        let shared = self.shared_dir();
        let (dir, _lock) = lock_project(shared, project)?;
        let agents: AgentsFile = load(shared, &agents_path(&dir))?;
        let connections: ConnectionsFile = load(shared, &connections_path(&dir))?;
        let mut views: Vec<AgentView> = agents
            .agents
            .iter()
            .map(|agent| view_of(agent, &connections))
            .collect();
        views.sort_by_key(|view| view.archived_at.is_some());
        Ok(views)
    }

    /// Register an agent: a name that no other live agent has, and a one-line role.
    pub fn register_agent(
        &self,
        project: &ProjectId,
        name: &str,
        role: &str,
        now: DateTime<Utc>,
    ) -> Result<AgentView, AgpjError> {
        let name = check_text("name", name, MAX_AGENT_NAME_CHARS)?;
        let role = check_text("role", role, MAX_AGENT_ROLE_CHARS)?;
        let shared = self.shared_dir();
        let (dir, _lock) = lock_project(shared, project)?;
        let mut file: AgentsFile = load(shared, &agents_path(&dir))?;
        ensure_name_free(&file, &name, None)?;
        let record = AgentRecord {
            id: format!("A-{}", file.next_agent_id),
            name,
            role,
            created_at: stamp(now),
            archived_at: None,
        };
        file.next_agent_id += 1;
        file.agents.push(record.clone());
        save(shared, &agents_path(&dir), &file)?;
        Ok(view_of(&record, &ConnectionsFile::default()))
    }

    /// Rename an agent or change its role. The ID, and so every card's agent, stays.
    pub fn update_agent(
        &self,
        project: &ProjectId,
        id: &str,
        name: Option<&str>,
        role: Option<&str>,
    ) -> Result<AgentView, AgpjError> {
        let name = name
            .map(|n| check_text("name", n, MAX_AGENT_NAME_CHARS))
            .transpose()?;
        let role = role
            .map(|r| check_text("role", r, MAX_AGENT_ROLE_CHARS))
            .transpose()?;
        let shared = self.shared_dir();
        let (dir, _lock) = lock_project(shared, project)?;
        let mut file: AgentsFile = load(shared, &agents_path(&dir))?;
        find_agent(&file, id)?;
        if let Some(name) = &name {
            ensure_name_free(&file, name, Some(id))?;
        }
        let record = file
            .agents
            .iter_mut()
            .find(|agent| agent.id == id)
            .ok_or_else(|| AgpjError::AgentNotFound { agent: id.into() })?;
        if let Some(name) = name {
            record.name = name;
        }
        if let Some(role) = role {
            record.role = role;
        }
        let record = record.clone();
        save(shared, &agents_path(&dir), &file)?;
        let connections: ConnectionsFile = load(shared, &connections_path(&dir))?;
        Ok(view_of(&record, &connections))
    }

    /// Delete (archive) an agent. It cannot have a Doing card or an active connection.
    pub fn archive_agent(
        &self,
        project: &ProjectId,
        id: &str,
        now: DateTime<Utc>,
    ) -> Result<AgentView, AgpjError> {
        let shared = self.shared_dir();
        let (dir, _lock) = lock_project(shared, project)?;
        let mut file: AgentsFile = load(shared, &agents_path(&dir))?;
        find_agent(&file, id)?;
        let connections: ConnectionsFile = load(shared, &connections_path(&dir))?;
        let busy = connections
            .connections
            .iter()
            .any(|c| c.agent_id == id && is_active(c));
        // A task that cannot be read might be this agent's Doing card, so it stops the deletion.
        let mut holds_work = false;
        for task_id in reader::task_ids(&dir)? {
            let task = reader::read_task_strict(shared, project, task_id)?;
            holds_work |= task.agent_id.as_deref() == Some(id)
                && task.task_status == Column::Doing
                && task.archived_at.is_none();
        }
        if busy || holds_work {
            return Err(AgpjError::AgentInUse { agent: id.into() });
        }
        let record = file
            .agents
            .iter_mut()
            .find(|agent| agent.id == id)
            .ok_or_else(|| AgpjError::AgentNotFound { agent: id.into() })?;
        if record.archived_at.is_none() {
            record.archived_at = Some(stamp(now));
        }
        let record = record.clone();
        save(shared, &agents_path(&dir), &file)?;
        Ok(view_of(&record, &connections))
    }
}
