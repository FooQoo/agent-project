//! The board keeps its connection across child and server restarts. Only the local data area
//! holds the credential; the project registry has its digest.

use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::Launcher;
use crate::clock::{Clock, SystemClock};
use crate::domain::ProjectId;
use crate::error::AgpjError;
use crate::file_store::FileStore;
use crate::server::preferences::{create_private_dir, sync_dir, write_owner_only};
use crate::server::sessions::Tool;

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SavedConnection {
    project_id: ProjectId,
    secret: String,
}

fn connection_error(error: impl std::fmt::Display) -> AgpjError {
    AgpjError::SessionLaunchFailed {
        reason: format!("The resident connection could not be stored or read: {error}."),
    }
}

impl Launcher {
    fn connection_path(&self) -> PathBuf {
        self.data_dir.join(".internal/resident-connection.json")
    }

    fn saved_connection(&self) -> Result<Option<SavedConnection>, AgpjError> {
        match fs::read(self.connection_path()) {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map(Some)
                .map_err(|_| connection_error("invalid connection file")),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(connection_error(error)),
        }
    }

    fn save_connection(&self, project: &ProjectId, secret: &str) -> Result<(), AgpjError> {
        let path = self.connection_path();
        let dir = path.parent().expect("resident connection parent");
        let bytes = serde_json::to_vec(&SavedConnection {
            project_id: project.clone(),
            secret: secret.into(),
        })
        .map_err(connection_error)?;
        create_private_dir(dir).map_err(connection_error)?;
        let temporary = path.with_extension("tmp");
        if let Err(error) =
            write_owner_only(&temporary, &bytes).and_then(|()| fs::rename(&temporary, &path))
        {
            let _ = fs::remove_file(&temporary);
            return Err(connection_error(error));
        }
        sync_dir(dir);
        Ok(())
    }

    /// Called with the lifecycle gate held, before a child gets any work.
    pub(super) fn connect_child(
        &self,
        project: &ProjectId,
        tool: Tool,
    ) -> Result<Option<(String, bool)>, AgpjError> {
        let Some(shared) = self.board_dir() else {
            return Ok(None);
        };
        let mut saved = self.saved_connection()?;
        if let Some(other) = saved.as_ref().filter(|saved| saved.project_id != *project) {
            // There is only one board-launched session. A new project releases its predecessor
            // when no child is running, so the old project's default agent becomes available.
            self.release_connection(&shared, other)?;
            saved = None;
        }
        FileStore::open(&shared)
            .connect_resident(
                project,
                saved.as_ref().map(|saved| saved.secret.as_str()),
                tool.as_str(),
                SystemClock.now(),
                |secret| self.save_connection(project, secret),
            )
            .map(Some)
    }

    /// Called with the lifecycle gate held on a person's Stop or to undo a failed first launch.
    /// An automatic restart and a board shutdown keep the saved connection.
    pub(super) fn disconnect_child(&self) -> Result<bool, AgpjError> {
        let (Some(shared), Some(saved)) = (self.board_dir(), self.saved_connection()?) else {
            return Ok(false);
        };
        self.release_connection(&shared, &saved)?;
        Ok(true)
    }

    fn release_connection(
        &self,
        shared: &std::path::Path,
        saved: &SavedConnection,
    ) -> Result<(), AgpjError> {
        match FileStore::open(shared).disconnect_session(
            &saved.project_id,
            &saved.secret,
            SystemClock.now(),
        ) {
            Ok(_)
            | Err(AgpjError::NotConnected | AgpjError::ConnectionReplaced)
            | Err(AgpjError::ProjectNotFound { .. }) => {}
            Err(error) => return Err(error),
        }
        let path = self.connection_path();
        match fs::remove_file(&path) {
            Ok(()) => sync_dir(path.parent().expect("resident connection parent")),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(connection_error(error)),
        }
        Ok(())
    }
}
