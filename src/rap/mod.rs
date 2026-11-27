//! Durable work and execution state for Remote Agent Project.

mod documents;
mod model;

pub use documents::DocumentHash;
pub use model::*;
use rusqlite::{Connection, OptionalExtension, Row, Transaction, TransactionBehavior, params};
use std::path::{Path, PathBuf};
use std::time::Duration;

const MIGRATIONS: &[&str] = &[include_str!("schema.sql"), include_str!("documents.sql")];
const APPLICATION_ID: i64 = 0x0052_4150;

#[derive(Debug, thiserror::Error)]
pub enum RapError {
    #[error(transparent)]
    Database(#[from] rusqlite::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("the file is not a RAP database")]
    ForeignDatabase,
    #[error("RAP database schema version {0} is newer than this binary")]
    NewerSchema(i64),
    #[error("{0}")]
    InvalidInput(&'static str),
    #[error("{0} conflicts with current RAP state")]
    Conflict(&'static str),
    #[error("RAP document {0} is missing, unreadable, or has changed")]
    DocumentIntegrity(String),
    #[error("RAP {0} was not found")]
    NotFound(&'static str),
}

/// A handle to one database shared by all RAP projects on this host.
#[derive(Clone, Debug)]
pub struct RapStore {
    database: PathBuf,
}

impl RapStore {
    /// Opens and migrates a RAP database at the supplied file path.
    ///
    /// The caller must supply a trusted, user-private data directory. This path is
    /// local runtime configuration, never a path taken from a remote request.
    pub fn open(database: &Path) -> Result<Self, RapError> {
        let store = Self {
            database: database.to_owned(),
        };
        let mut connection = store.connect()?;
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let application_id: i64 = tx.pragma_query_value(None, "application_id", |r| r.get(0))?;
        let version: i64 = tx.pragma_query_value(None, "user_version", |r| r.get(0))?;
        if application_id == 0 && version == 0 {
            let has_tables: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_schema WHERE name NOT LIKE 'sqlite_%')",
                [],
                |row| row.get(0),
            )?;
            if has_tables {
                return Err(RapError::ForeignDatabase);
            }
            tx.pragma_update(None, "application_id", APPLICATION_ID)?;
        } else if application_id != APPLICATION_ID {
            return Err(RapError::ForeignDatabase);
        }

        let version_index = usize::try_from(version)
            .ok()
            .filter(|index| *index <= MIGRATIONS.len())
            .ok_or(RapError::NewerSchema(version))?;
        for (migration, version) in MIGRATIONS.iter().zip(1_i64..).skip(version_index) {
            tx.execute_batch(migration)?;
            tx.pragma_update(None, "user_version", version)?;
        }
        store.recover_documents(&tx)?;
        tx.commit()?;
        Ok(store)
    }

    /// Registers an epic, a standalone task, or an epic's child task.
    pub fn create_work(&self, input: NewWork<'_>) -> Result<WorkItem, RapError> {
        let workspace = input.workspace.canonicalize()?;
        let workspace =
            workspace
                .to_str()
                .filter(|_| workspace.is_dir())
                .ok_or(RapError::InvalidInput(
                    "workspace must be a UTF-8 directory",
                ))?;
        if input.project_id.trim().is_empty() || input.title.trim().is_empty() {
            return Err(RapError::InvalidInput(
                "project and work title are required",
            ));
        }
        if input.kind == WorkKind::Epic && input.parent_epic_id.is_some() {
            return Err(RapError::InvalidInput("an epic cannot have a parent"));
        }
        self.write(|tx| {
            if let Some(parent_id) = input.parent_epic_id {
                let parent = fetch_work(tx, parent_id)?;
                if parent.kind != WorkKind::Epic
                    || parent.project_id != input.project_id
                    || parent.workspace != Path::new(workspace)
                {
                    return Err(RapError::InvalidInput(
                        "parent must be an epic in the same project and workspace",
                    ));
                }
            }
            tx.execute(
                "INSERT INTO work_items (project_id, workspace, kind, parent_epic_id, title, rank)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    input.project_id,
                    workspace,
                    input.kind,
                    input.parent_epic_id.map(|id| id.0),
                    input.title,
                    input.rank,
                ],
            )?;
            fetch_work(tx, WorkId(tx.last_insert_rowid()))
        })
    }

    pub fn work(&self, id: WorkId) -> Result<WorkItem, RapError> {
        self.read(|connection| fetch_work(connection, id))
    }

    pub fn project_work(&self, project_id: &str) -> Result<Vec<WorkItem>, RapError> {
        self.read(|connection| {
            let sql = format!("{WORK_SELECT} WHERE project_id = ?1 ORDER BY rank, id");
            let mut statement = connection.prepare(&sql)?;
            let rows = statement.query_map([project_id], read_work)?;
            Ok(rows.collect::<rusqlite::Result<_>>()?)
        })
    }

    pub fn sessions(&self, work_id: WorkId) -> Result<Vec<Session>, RapError> {
        self.read(|connection| {
            let mut statement = connection.prepare(
                "SELECT id, phase, provider, native_id
                 FROM sessions WHERE work_id = ?1 ORDER BY id",
            )?;
            let rows = statement.query_map([work_id.0], |row| {
                Ok(Session {
                    id: SessionId(row.get(0)?),
                    work_id,
                    phase: row.get(1)?,
                    provider: row.get(2)?,
                    native_id: row.get(3)?,
                })
            })?;
            Ok(rows.collect::<rusqlite::Result<_>>()?)
        })
    }

    pub fn attempts(&self, session_id: SessionId) -> Result<Vec<Attempt>, RapError> {
        self.read(|connection| {
            let mut statement = connection.prepare(
                "SELECT id, epoch, state FROM attempts WHERE session_id = ?1 ORDER BY epoch",
            )?;
            let rows = statement.query_map([session_id.0], |row| {
                Ok(Attempt {
                    id: AttemptId(row.get(0)?),
                    session_id,
                    epoch: row.get(1)?,
                    state: row.get(2)?,
                })
            })?;
            Ok(rows.collect::<rusqlite::Result<_>>()?)
        })
    }

    pub(crate) fn read<T>(
        &self,
        op: impl FnOnce(&Connection) -> Result<T, RapError>,
    ) -> Result<T, RapError> {
        op(&self.connect()?)
    }

    pub(crate) fn write<T>(
        &self,
        op: impl FnOnce(&Transaction<'_>) -> Result<T, RapError>,
    ) -> Result<T, RapError> {
        let mut connection = self.connect()?;
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let result = op(&tx)?;
        tx.commit()?;
        Ok(result)
    }

    fn connect(&self) -> Result<Connection, RapError> {
        let connection = Connection::open(&self.database)?;
        connection.busy_timeout(Duration::from_secs(5))?;
        connection.pragma_update(None, "foreign_keys", true)?;
        Ok(connection)
    }
}

const WORK_SELECT: &str = "SELECT id, project_id, workspace, kind, parent_epic_id, title,
                                  rank, lane, phase, version FROM work_items";

fn fetch_work(connection: &Connection, id: WorkId) -> Result<WorkItem, RapError> {
    let sql = format!("{WORK_SELECT} WHERE id = ?1");
    connection
        .query_row(&sql, [id.0], read_work)
        .optional()?
        .ok_or(RapError::NotFound("work item"))
}

fn read_work(row: &Row<'_>) -> rusqlite::Result<WorkItem> {
    Ok(WorkItem {
        id: WorkId(row.get(0)?),
        project_id: row.get(1)?,
        workspace: PathBuf::from(row.get::<_, String>(2)?),
        kind: row.get(3)?,
        parent_epic_id: row.get::<_, Option<i64>>(4)?.map(WorkId),
        title: row.get(5)?,
        rank: row.get(6)?,
        lane: row.get(7)?,
        phase: row.get(8)?,
        version: row.get(9)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn work_membership_and_separate_session_attempt_states_survive_reopen() {
        let root = tempfile::tempdir().unwrap();
        let db = root.path().join("rap.db");
        let (epic, child, standalone, session_id) = {
            let store = RapStore::open(&db).unwrap();
            let item = |kind, parent_epic_id, title, rank| {
                store
                    .create_work(NewWork {
                        project_id: "demo",
                        workspace: root.path(),
                        kind,
                        parent_epic_id,
                        title,
                        rank,
                    })
                    .unwrap()
            };
            let epic = item(WorkKind::Epic, None, "Release", 10);
            let child = item(WorkKind::Task, Some(epic.id), "Test", 12);
            let standalone = item(WorkKind::Task, None, "Triage", 1);

            // The scheduler owns these writes. Populate its state independently
            // from the board rows to verify persistence of all three dimensions.
            let session_id = store
                .write(|tx| {
                    tx.execute(
                        "UPDATE work_items SET phase = 'planning' WHERE id = ?1",
                        [epic.id.0],
                    )?;
                    tx.execute(
                        "INSERT INTO sessions (work_id, phase, provider, native_id)
                         VALUES (?1, 'planning', 'codex', 'native-plan')",
                        [epic.id.0],
                    )?;
                    let id = tx.last_insert_rowid();
                    tx.execute(
                        "INSERT INTO attempts (session_id, epoch, state, owner_token)
                         VALUES (?1, 1, 'failed', 'owner-1'), (?1, 2, 'running', 'owner-2')",
                        [id],
                    )?;
                    Ok(SessionId(id))
                })
                .unwrap();
            (epic, child, standalone, session_id)
        };

        let store = RapStore::open(&db).unwrap();
        let work = store.project_work("demo").unwrap();
        assert_eq!(
            work.iter().map(|item| item.id).collect::<Vec<_>>(),
            vec![standalone.id, epic.id, child.id]
        );
        assert_eq!(work[1].lane, BoardLane::Backlog);
        assert_eq!(work[1].phase, WorkPhase::Planning);
        assert_eq!(work[2].parent_epic_id, Some(epic.id));
        assert_eq!(work[0].parent_epic_id, None);

        let sessions = store.sessions(epic.id).unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].phase, SessionPhase::Planning);
        assert_eq!(sessions[0].native_id.as_deref(), Some("native-plan"));
        let attempts = store.attempts(session_id).unwrap();
        assert_eq!(attempts.len(), 2);
        assert_eq!(attempts[0].state, AttemptState::Failed);
        assert_eq!(attempts[1].state, AttemptState::Running);
        assert_eq!(attempts[1].epoch, 2);
    }

    #[test]
    fn foreign_databases_are_left_untouched() {
        let root = tempfile::tempdir().unwrap();
        let db = root.path().join("other.db");
        let connection = Connection::open(&db).unwrap();
        connection
            .execute("CREATE TABLE foreign_rows (value INTEGER)", [])
            .unwrap();
        assert!(matches!(
            RapStore::open(&db),
            Err(RapError::ForeignDatabase)
        ));
        let application_id: i64 = connection
            .pragma_query_value(None, "application_id", |row| row.get(0))
            .unwrap();
        assert_eq!(application_id, 0);
    }
}
