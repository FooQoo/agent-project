//! Connection codes, connections, and what a session ending does.

use super::*;

impl FileStore {
    /// Issue a one-time connection code for an agent. A plain code is refused while the agent
    /// has an active connection (`agent_busy`); a switch code replaces it when exchanged.
    pub fn issue_connect_code(
        &self,
        project: &ProjectId,
        agent_id: &str,
        switch: bool,
        now: DateTime<Utc>,
    ) -> Result<IssuedCode, AgpjError> {
        let shared = self.shared_dir();
        let (dir, _lock) = lock_project(shared, project)?;
        let agents: AgentsFile = load(shared, &agents_path(&dir))?;
        let agent = find_agent(&agents, agent_id)?;
        if agent.archived_at.is_some() {
            return Err(AgpjError::AgentNotFound {
                agent: agent_id.into(),
            });
        }
        let mut connections: ConnectionsFile = load(shared, &connections_path(&dir))?;
        let busy = connections
            .connections
            .iter()
            .any(|c| c.agent_id == agent_id && is_active(c));
        if busy && !switch {
            return Err(AgpjError::AgentBusy {
                agent: agent_id.into(),
            });
        }
        let now = stamp(now);
        connections
            .codes
            .retain(|c| c.expires_at > now && c.agent_id != agent_id);
        let code = new_code()?;
        let expires_at = now + Duration::minutes(CONNECT_CODE_MINUTES);
        connections.codes.push(ConnectCode {
            digest: digest_of(&code),
            agent_id: agent_id.into(),
            switch,
            expires_at,
        });
        save(shared, &connections_path(&dir), &connections)?;
        Ok(IssuedCode {
            agent_id: agent_id.into(),
            code,
            expires_at,
        })
    }

    /// Exchange a connection code for a connection. A switch code revokes the agent's earlier
    /// connection in the same save and puts its running tasks back to waiting; a plain code
    /// never takes over a live connection.
    pub fn exchange_connect_code(
        &self,
        project: &ProjectId,
        code: &str,
        origin: ConnectionOrigin,
        tool: &str,
        session_label: &str,
        now: DateTime<Utc>,
    ) -> Result<Connected, AgpjError> {
        let shared = self.shared_dir();
        let (dir, _lock) = lock_project(shared, project)?;
        let now = stamp(now);
        let mut connections: ConnectionsFile = load(shared, &connections_path(&dir))?;
        let digest = digest_of(&normalize_code(code));
        let Some(position) = connections.codes.iter().position(|c| c.digest == digest) else {
            return Err(AgpjError::ConnectCodeInvalid);
        };
        let issued = connections.codes.remove(position);
        if issued.expires_at <= now {
            save(shared, &connections_path(&dir), &connections)?;
            return Err(AgpjError::ConnectCodeInvalid);
        }
        let agents: AgentsFile = load(shared, &agents_path(&dir))?;
        let agent = find_agent(&agents, &issued.agent_id)?.clone();
        if agent.archived_at.is_some() {
            return Err(AgpjError::AgentNotFound { agent: agent.id });
        }
        let id = format!("C-{}", hex(&random_bytes::<6>()?));
        let secret = format!("{SECRET_PREFIX}{}", hex(&random_bytes::<24>()?));
        let active: Vec<usize> = connections
            .connections
            .iter()
            .enumerate()
            .filter(|(_, c)| c.agent_id == agent.id && is_active(c))
            .map(|(i, _)| i)
            .collect();
        if !active.is_empty() && !issued.switch {
            return Err(AgpjError::AgentBusy { agent: agent.id });
        }
        let replaced: Vec<String> = active
            .iter()
            .map(|&i| connections.connections[i].id.clone())
            .collect();
        for &i in &active {
            let old = &mut connections.connections[i];
            old.revoked_at = Some(now);
            old.replaced_by = Some(id.clone());
        }
        let record = ConnectionRecord {
            id,
            agent_id: agent.id.clone(),
            secret_digest: digest_of(&secret),
            origin,
            tool: tool.to_string(),
            session_label: session_label.to_string(),
            issued_at: now,
            last_activity_at: now,
            revoked_at: None,
            replaced_by: None,
        };
        connections.connections.push(record.clone());
        // The tasks go back to waiting first: if the save below fails, the old connection is
        // still the live one and nothing runs on a connection that was meant to be replaced.
        for old in &replaced {
            self.pause_runs_locked(
                project,
                &dir,
                |task| task.run_connection_id.as_deref() == Some(old.as_str()),
                "The agent was switched to another session; the task is waiting.",
                now,
            )?;
        }
        save(shared, &connections_path(&dir), &connections)?;
        Ok(Connected {
            agent: view_of(&agent, &connections),
            connection: ConnectionView::from(&record),
            secret,
        })
    }

    /// Check a secret and, at most every half minute, record the activity. Used by commands that
    /// only need to know which connection they are.
    pub fn verify_connection(
        &self,
        project: &ProjectId,
        secret: &str,
        now: DateTime<Utc>,
    ) -> Result<Verified, AgpjError> {
        let shared = self.shared_dir();
        let (dir, _lock) = lock_project(shared, project)?;
        verify_locked(shared, &dir, secret, stamp(now))
    }

    /// A session ended: its running tasks go back to waiting. The connection stays, so the same
    /// session can resume; the person revokes it by switching.
    pub fn end_connection_session(
        &self,
        project: &ProjectId,
        secret: &str,
        now: DateTime<Utc>,
    ) -> Result<Vec<i64>, AgpjError> {
        let shared = self.shared_dir();
        let (dir, _lock) = lock_project(shared, project)?;
        let now = stamp(now);
        let verified = verify_locked(shared, &dir, secret, now)?;
        self.pause_runs_locked(
            project,
            &dir,
            |task| task.run_connection_id.as_deref() == Some(verified.connection_id.as_str()),
            "The session ended; the task is waiting.",
            now,
        )
    }

    /// Put running tasks back to waiting when the connection that set them running is gone
    /// (revoked or replaced) or has shown no activity for [`STALE_RUN_SECONDS`].
    pub fn release_stale_runs(
        &self,
        project: &ProjectId,
        now: DateTime<Utc>,
    ) -> Result<Vec<i64>, AgpjError> {
        let shared = self.shared_dir();
        let (dir, _lock) = lock_project(shared, project)?;
        let now = stamp(now);
        let connections: ConnectionsFile = load(shared, &connections_path(&dir))?;
        let limit = Duration::seconds(STALE_RUN_SECONDS);
        self.pause_runs_locked(
            project,
            &dir,
            |task| {
                let Some(id) = task.run_connection_id.as_deref() else {
                    return false;
                };
                match connections.connections.iter().find(|c| c.id == id) {
                    Some(c) => !is_active(c) || now - c.last_activity_at > limit,
                    None => true,
                }
            },
            "The session shows no activity; the task is waiting.",
            now,
        )
    }
}
