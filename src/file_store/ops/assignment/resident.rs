//! The connection reserved for a session the board launches.

use super::*;
use crate::file_store::history_language::HistoryLanguage;

impl FileStore {
    /// Reserve the default agent, or reuse the board's own saved connection. The project lock
    /// serializes this with code exchange, so an external connection is never replaced.
    /// The boolean reports whether this launch created a connection.
    pub(crate) fn connect_resident(
        &self,
        project: &ProjectId,
        saved_secret: Option<&str>,
        tool: &str,
        now: DateTime<Utc>,
        persist: impl FnOnce(&str) -> Result<(), AgpjError>,
    ) -> Result<(String, bool), AgpjError> {
        let shared = self.shared_dir();
        let (dir, _lock) = lock_project(shared, project)?;
        let now = stamp(now);
        let mut connections: ConnectionsFile = load(shared, &connections_path(&dir))?;
        let project_name = reader::read_project_file(shared, project)?.name;
        let language = HistoryLanguage::infer("", &project_name);
        let agent_id = default_agent_id_locked(shared, &dir, language, now)?;

        if let Some(record) = connections
            .connections
            .iter_mut()
            .find(|record| record.agent_id == agent_id && is_active(record))
        {
            let Some(secret) = saved_secret.filter(|secret| {
                record.origin == ConnectionOrigin::Board
                    && record.secret_digest == digest_of(secret)
            }) else {
                return Err(AgpjError::AgentBusy { agent: agent_id });
            };
            record.tool = tool.into();
            record.last_activity_at = now;
            save(shared, &connections_path(&dir), &connections)?;
            return Ok((secret.to_owned(), false));
        }

        let secret = format!("{SECRET_PREFIX}{}", hex(&random_bytes::<24>()?));
        connections.connections.push(ConnectionRecord {
            id: format!("C-{}", hex(&random_bytes::<6>()?)),
            agent_id,
            secret_digest: digest_of(&secret),
            origin: ConnectionOrigin::Board,
            tool: tool.into(),
            session_label: "Resident agent".into(),
            issued_at: now,
            last_activity_at: now,
            revoked_at: None,
            replaced_by: None,
        });
        // Save the local secret first. If the registry save fails, retrying with the unregistered
        // secret creates a fresh connection instead of leaving a connection nobody can resume.
        persist(&secret)?;
        save(shared, &connections_path(&dir), &connections)?;
        Ok((secret, true))
    }
}
