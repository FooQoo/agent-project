use std::path::{Path, PathBuf};

use rusqlite::ToSql;
use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ToSqlOutput, ValueRef};
use serde::{Deserialize, Serialize};

macro_rules! id_type {
    ($name:ident) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub i64);
    };
}

id_type!(WorkId);
id_type!(SessionId);
id_type!(AttemptId);
id_type!(RevisionId);
id_type!(ContractId);
id_type!(ChangeRequestId);
id_type!(EvidenceId);

macro_rules! state_type {
    ($name:ident { $($variant:ident => $value:literal),+ $(,)? }) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
        #[serde(rename_all = "snake_case")]
        pub enum $name {
            $($variant),+
        }

        impl ToSql for $name {
            fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
                Ok(match self {
                    $(Self::$variant => $value),+
                }.into())
            }
        }

        impl FromSql for $name {
            fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
                match value.as_str()? {
                    $($value => Ok(Self::$variant)),+,
                    _ => Err(FromSqlError::InvalidType),
                }
            }
        }
    };
}

state_type!(WorkKind { Epic => "epic", Task => "task" });
state_type!(BoardLane {
    Backlog => "backlog", Todo => "todo", Doing => "doing",
    Done => "done", Canceled => "canceled",
});
state_type!(WorkPhase {
    Requested => "requested", Planning => "planning", AwaitingApproval => "awaiting_approval",
    Ready => "ready", Executing => "executing", Revising => "revising", Verifying => "verifying",
    Completed => "completed", Canceled => "canceled",
});
state_type!(SessionPhase { Planning => "planning", Execution => "execution" });
state_type!(AttemptState {
    Queued => "queued", Starting => "starting", Running => "running", Stopping => "stopping",
    Suspended => "suspended", RetryWait => "retry_wait", Finished => "finished",
    Failed => "failed", Unknown => "unknown",
});

#[derive(Debug, Clone)]
pub struct NewWork<'a> {
    pub project_id: &'a str,
    pub workspace: &'a Path,
    pub kind: WorkKind,
    pub parent_epic_id: Option<WorkId>,
    pub title: &'a str,
    pub rank: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WorkItem {
    pub id: WorkId,
    pub project_id: String,
    pub workspace: PathBuf,
    pub kind: WorkKind,
    pub parent_epic_id: Option<WorkId>,
    pub title: String,
    pub rank: i64,
    pub lane: BoardLane,
    pub phase: WorkPhase,
    pub version: i64,
    pub current_revision_id: Option<RevisionId>,
    pub current_contract_id: Option<ContractId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Session {
    pub id: SessionId,
    pub work_id: WorkId,
    pub phase: SessionPhase,
    pub provider: String,
    pub native_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Attempt {
    pub id: AttemptId,
    pub session_id: SessionId,
    pub epoch: i64,
    pub state: AttemptState,
}
