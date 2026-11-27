use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::{OptionalExtension, params};

use super::{AttemptId, RapError, RapStore};

/// Validates that a remote handset session is neither revoked nor expired.
pub fn verify_remote_session(expires_at: i64, revoked: bool, now: i64) -> Result<(), RapError> {
    if revoked {
        return Err(RapError::Conflict("remote session has been revoked"));
    }
    if now >= expires_at {
        return Err(RapError::Conflict("remote session has expired"));
    }
    Ok(())
}

/// Ensures a requested workspace canonicalizes within at least one allowed project root.
pub fn verify_workspace_within_roots(
    workspace: &Path,
    allowed_roots: &[PathBuf],
) -> Result<PathBuf, RapError> {
    if allowed_roots.is_empty() {
        return Err(RapError::Conflict("no allowed workspace roots configured"));
    }
    let canonical = fs::canonicalize(workspace)?;
    if !canonical.is_dir() {
        return Err(RapError::InvalidInput("workspace must be a directory"));
    }
    for root in allowed_roots {
        if let Ok(allowed) = fs::canonicalize(root)
            && canonical.starts_with(&allowed)
        {
            return Ok(canonical);
        }
    }
    Err(RapError::Conflict(
        "workspace is outside allowed remote roots",
    ))
}

/// Prohibits irreversible operations unless explicitly approved by a human outside Sleep delegation.
pub fn verify_irreversible_approval(
    approved_by_human: bool,
    via_sleep_delegate: bool,
) -> Result<(), RapError> {
    if !approved_by_human || via_sleep_delegate {
        return Err(RapError::Conflict(
            "irreversible operations require explicit human approval",
        ));
    }
    Ok(())
}

impl RapStore {
    /// Verifies that an attempt update carries the matching `owner_token` of the latest epoch.
    pub fn verify_attempt_owner(
        &self,
        attempt: AttemptId,
        owner_token: &str,
    ) -> Result<(), RapError> {
        self.read(|connection| {
            let valid: Option<bool> = connection
                .query_row(
                    "SELECT a.owner_token = ?2
                         AND a.epoch = (SELECT MAX(epoch) FROM attempts WHERE session_id = a.session_id)
                     FROM attempts a WHERE a.id = ?1",
                    params![attempt.0, owner_token],
                    |row| row.get(0),
                )
                .optional()?;
            match valid {
                Some(true) => Ok(()),
                Some(false) => Err(RapError::Conflict("attempt owner token is invalid or stale")),
                None => Err(RapError::NotFound("attempt")),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rap::{NewWork, WorkKind};

    #[test]
    fn enforces_session_expiry_workspace_scope_attempt_owner_and_irreversible_guard() {
        // 1. Session revocation and expiry.
        verify_remote_session(200, false, 100).unwrap();
        assert!(verify_remote_session(200, true, 100).is_err());
        assert!(verify_remote_session(200, false, 200).is_err());

        // 2. Workspace scope (including symlink escape).
        let root = tempfile::tempdir().unwrap();
        let allowed = root.path().join("allowed");
        let sub = allowed.join("sub");
        let outside = root.path().join("outside");
        fs::create_dir_all(&sub).unwrap();
        fs::create_dir_all(&outside).unwrap();
        verify_workspace_within_roots(&sub, std::slice::from_ref(&allowed)).unwrap();
        assert!(verify_workspace_within_roots(&outside, std::slice::from_ref(&allowed)).is_err());

        #[cfg(unix)]
        {
            let escape = allowed.join("escape");
            std::os::unix::fs::symlink(&outside, &escape).unwrap();
            assert!(
                verify_workspace_within_roots(&escape, std::slice::from_ref(&allowed)).is_err()
            );
        }

        // 3. Irreversible approval guard.
        verify_irreversible_approval(true, false).unwrap();
        assert!(verify_irreversible_approval(false, false).is_err());
        assert!(verify_irreversible_approval(true, true).is_err());

        // 4. Attempt owner token + latest epoch check.
        let store = RapStore::open(&root.path().join("rap.db")).unwrap();
        let work = store
            .create_work(NewWork {
                project_id: "demo",
                workspace: &sub,
                kind: WorkKind::Task,
                parent_epic_id: None,
                title: "Guard test",
                rank: 1,
            })
            .unwrap()
            .id;
        let (a1, a2) = store
            .write(|tx| {
                tx.execute(
                    "INSERT INTO sessions (work_id, phase, provider) VALUES (?1, 'execution', 'codex')",
                    [work.0],
                )?;
                let sid = tx.last_insert_rowid();
                let a1: i64 = tx.query_row(
                    "INSERT INTO attempts (session_id, epoch, state, owner_token)
                     VALUES (?1, 1, 'failed', 'tok-1') RETURNING id",
                    [sid],
                    |r| r.get(0),
                )?;
                let a2: i64 = tx.query_row(
                    "INSERT INTO attempts (session_id, epoch, state, owner_token)
                     VALUES (?1, 2, 'running', 'tok-2') RETURNING id",
                    [sid],
                    |r| r.get(0),
                )?;
                Ok((AttemptId(a1), AttemptId(a2)))
            })
            .unwrap();
        store.verify_attempt_owner(a2, "tok-2").unwrap();
        assert!(store.verify_attempt_owner(a2, "wrong").is_err());
        assert!(store.verify_attempt_owner(a1, "tok-1").is_err());
    }
}
