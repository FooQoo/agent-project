use rusqlite::{Connection, OptionalExtension, Row, params};
use serde::{Deserialize, Serialize};

use super::{
    BoardLane, ChangeRequestId, RapError, RapStore, RevisionId, WorkId, WorkPhase, fetch_work,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeRequestState {
    Pending,
    Absorbed,
    Replanned,
    Rejected,
}

impl ChangeRequestState {
    fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Absorbed => "absorbed",
            Self::Replanned => "replanned",
            Self::Rejected => "rejected",
        }
    }

    fn parse(value: &str) -> rusqlite::Result<Self> {
        match value {
            "pending" => Ok(Self::Pending),
            "absorbed" => Ok(Self::Absorbed),
            "replanned" => Ok(Self::Replanned),
            "rejected" => Ok(Self::Rejected),
            _ => Err(rusqlite::Error::InvalidColumnType(
                3,
                "state".to_owned(),
                rusqlite::types::Type::Text,
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ChangeRequest {
    pub id: ChangeRequestId,
    pub work_id: WorkId,
    pub body: String,
    pub state: ChangeRequestState,
    pub received_at: i64,
    pub resolution_reason: Option<String>,
    pub resolved_revision_id: Option<RevisionId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChangeResolution<'a> {
    Absorbed {
        reason: &'a str,
    },
    Replanned {
        revision_id: RevisionId,
        reason: &'a str,
    },
    Rejected {
        reason: &'a str,
    },
}

impl RapStore {
    /// Durably accepts an interrupt or change request without immediately applying it.
    /// A request arriving after completion reopens the work item for a new revision.
    pub fn receive_change_request(
        &self,
        work: WorkId,
        body: &str,
        now: i64,
    ) -> Result<ChangeRequest, RapError> {
        if body.trim().is_empty() {
            return Err(RapError::InvalidInput("change request body is required"));
        }
        self.write(|tx| {
            let item = fetch_work(tx, work)?;
            if item.lane == BoardLane::Canceled {
                return Err(RapError::Conflict("requesting a change on canceled work"));
            }
            let id: i64 = tx.query_row(
                "INSERT INTO change_requests (work_id, body, state, received_at)
                 VALUES (?1, ?2, 'pending', ?3) RETURNING id",
                params![work.0, body, now],
                |row| row.get(0),
            )?;
            if item.lane == BoardLane::Done {
                tx.execute(
                    "UPDATE work_items
                     SET lane = ?1, phase = ?2, current_contract_id = NULL, version = version + 1
                     WHERE id = ?3",
                    params![BoardLane::Doing, WorkPhase::Revising, work.0],
                )?;
            }
            fetch_change_request(tx, ChangeRequestId(id))
        })
    }

    /// Marks a pending change request as absorbed, replanned, or rejected.
    pub fn resolve_change_request(
        &self,
        work: WorkId,
        request: ChangeRequestId,
        resolution: ChangeResolution<'_>,
    ) -> Result<ChangeRequest, RapError> {
        let (state, reason, revision_id) = match resolution {
            ChangeResolution::Absorbed { reason } => (ChangeRequestState::Absorbed, reason, None),
            ChangeResolution::Replanned {
                revision_id,
                reason,
            } => (ChangeRequestState::Replanned, reason, Some(revision_id)),
            ChangeResolution::Rejected { reason } => (ChangeRequestState::Rejected, reason, None),
        };
        if reason.trim().is_empty() {
            return Err(RapError::InvalidInput("resolution reason is required"));
        }
        self.write(|tx| {
            let updated = tx.execute(
                "UPDATE change_requests
                 SET state = ?1, resolution_reason = ?2, resolved_revision_id = ?3
                 WHERE id = ?4 AND work_id = ?5 AND state = 'pending'",
                params![
                    state.as_str(),
                    reason,
                    revision_id.map(|id| id.0),
                    request.0,
                    work.0,
                ],
            )?;
            if updated != 1 {
                return Err(RapError::Conflict("change request is not pending"));
            }
            fetch_change_request(tx, request)
        })
    }

    pub fn pending_change_requests(&self, work: WorkId) -> Result<Vec<ChangeRequest>, RapError> {
        self.read(|connection| {
            let mut statement = connection.prepare(
                "SELECT id, work_id, body, state, received_at, resolution_reason, resolved_revision_id
                 FROM change_requests WHERE work_id = ?1 AND state = 'pending' ORDER BY id",
            )?;
            let rows = statement.query_map([work.0], read_change_request)?;
            Ok(rows.collect::<rusqlite::Result<_>>()?)
        })
    }
}

fn fetch_change_request(
    connection: &Connection,
    id: ChangeRequestId,
) -> Result<ChangeRequest, RapError> {
    connection
        .query_row(
            "SELECT id, work_id, body, state, received_at, resolution_reason, resolved_revision_id
             FROM change_requests WHERE id = ?1",
            [id.0],
            read_change_request,
        )
        .optional()?
        .ok_or(RapError::NotFound("change request"))
}

fn read_change_request(row: &Row<'_>) -> rusqlite::Result<ChangeRequest> {
    let raw_state: String = row.get(3)?;
    Ok(ChangeRequest {
        id: ChangeRequestId(row.get(0)?),
        work_id: WorkId(row.get(1)?),
        body: row.get(2)?,
        state: ChangeRequestState::parse(&raw_state)?,
        received_at: row.get(4)?,
        resolution_reason: row.get(5)?,
        resolved_revision_id: row.get::<_, Option<i64>>(6)?.map(RevisionId),
    })
}
