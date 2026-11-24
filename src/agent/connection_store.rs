//! Where a session keeps the secret of its connection.
//!
//! `agpj connect` exchanges a connection code for a secret and stores it here, next to the
//! session's key, in a file only the user can read. Every later `agpj` command of the same session
//! finds it again. A session the board launched gets its secret in `AGPJ_CONNECTION_SECRET`
//! instead. The secret is never printed and never written to a shared file.

use std::io::Write;
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

use super::session_identity::SessionIdentity;
use crate::domain::ProjectId;
use crate::error::AgpjError;

/// The environment variable that carries the secret of a session the board launched.
pub(crate) const SECRET_ENV: &str = "AGPJ_CONNECTION_SECRET";

fn secret_file(data_dir: &Path, project: &ProjectId, key: &str) -> PathBuf {
    let name: String = key
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    data_dir
        .join("connections")
        .join(project.as_str())
        .join(format!("{name}.secret"))
}

/// The secret of this process's session: the environment first, then the stored file.
pub(crate) fn load_secret(data_dir: &Path, project: &ProjectId) -> Option<String> {
    if let Some(secret) = std::env::var(SECRET_ENV)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    {
        return Some(secret);
    }
    load_secret_for_key(data_dir, project, &SessionIdentity::from_env().key)
}

/// [`load_secret`] that tells a missing secret from one that cannot be read: only a missing
/// file or an empty one means "no secret"; any other read error is returned, so that `connect`
/// never replaces a secret it could not look at.
pub(crate) fn try_load_secret(
    data_dir: &Path,
    project: &ProjectId,
) -> Result<Option<String>, AgpjError> {
    if let Some(secret) = std::env::var(SECRET_ENV)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    {
        return Ok(Some(secret));
    }
    let path = secret_file(data_dir, project, &SessionIdentity::from_env().key);
    match std::fs::read_to_string(&path) {
        Ok(text) => {
            let text = text.trim();
            Ok((!text.is_empty()).then(|| text.to_string()))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(AgpjError::FileError {
            action: crate::error::FileAction::Read,
            file: "connections".into(),
            reason: format!("The stored connection secret could not be read: {e}."),
        }),
    }
}

/// The stored secret of the session with this key.
pub(crate) fn load_secret_for_key(
    data_dir: &Path,
    project: &ProjectId,
    key: &str,
) -> Option<String> {
    let text = std::fs::read_to_string(secret_file(data_dir, project, key)).ok()?;
    let text = text.trim();
    (!text.is_empty()).then(|| text.to_string())
}

/// Every project that holds a stored secret for the session with this key, with that secret.
/// A session that connected with `--project` from an unbound folder has no binding to find it by.
pub(crate) fn stored_secrets_for_key(data_dir: &Path, key: &str) -> Vec<(ProjectId, String)> {
    let Ok(entries) = std::fs::read_dir(data_dir.join("connections")) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter_map(|entry| {
            let project = entry.file_name().to_str()?.parse::<ProjectId>().ok()?;
            let secret = load_secret_for_key(data_dir, &project, key)?;
            Some((project, secret))
        })
        .collect()
}

/// Store the secret for this process's session, readable by the user only.
pub(crate) fn save_secret(
    data_dir: &Path,
    project: &ProjectId,
    secret: &str,
) -> Result<(), AgpjError> {
    let path = secret_file(data_dir, project, &SessionIdentity::from_env().key);
    let fail = |e: std::io::Error| AgpjError::FileError {
        action: crate::error::FileAction::Write,
        file: "connections".into(),
        reason: format!("The connection secret could not be stored: {e}."),
    };
    if let Some(dir) = path.parent() {
        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(dir)
            .map_err(fail)?;
    }
    // Written to a temporary file and renamed, so a failed write keeps the earlier secret.
    let temporary = path.with_extension("secret.tmp");
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(&temporary)
        .map_err(fail)?;
    let written = file
        .write_all(secret.as_bytes())
        .and_then(|()| file.sync_all());
    drop(file);
    if let Err(e) = written.and_then(|()| std::fs::rename(&temporary, &path)) {
        let _ = std::fs::remove_file(&temporary);
        return Err(fail(e));
    }
    Ok(())
}

/// Forget this process's stored secret for the project, but only when it is `secret`: a
/// `disconnect` that revoked another connection (named by `AGPJ_CONNECTION_SECRET`) must keep
/// this session's own stored secret.
pub(crate) fn delete_secret_if(data_dir: &Path, project: &ProjectId, secret: &str) {
    let key = SessionIdentity::from_env().key;
    if load_secret_for_key(data_dir, project, &key).as_deref() == Some(secret) {
        let _ = std::fs::remove_file(secret_file(data_dir, project, &key));
    }
}

/// An exclusive lock on this session's connection, held while it connects: the check that the
/// session holds no connection, the code exchange, and the secret save must not interleave with
/// another `connect` of the same session. Released when dropped.
pub(crate) struct SessionLock(#[allow(dead_code)] std::fs::File);

pub(crate) fn lock_session(data_dir: &Path, project: &ProjectId) -> Result<SessionLock, AgpjError> {
    use std::os::fd::AsRawFd;
    let path = secret_file(data_dir, project, &SessionIdentity::from_env().key)
        .with_extension("secret.lock");
    let fail = |e: std::io::Error| AgpjError::FileError {
        action: crate::error::FileAction::Write,
        file: "connections".into(),
        reason: format!("The session's connection lock could not be taken: {e}."),
    };
    if let Some(dir) = path.parent() {
        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(dir)
            .map_err(fail)?;
    }
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .open(&path)
        .map_err(fail)?;
    // SAFETY: flock on a descriptor this function owns; it blocks until the holder lets go.
    if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX) } != 0 {
        return Err(fail(std::io::Error::last_os_error()));
    }
    Ok(SessionLock(file))
}
