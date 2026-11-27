use std::path::Path;

use rusqlite::{OptionalExtension, Transaction, params};

use super::workspace::WorkspaceIdentity;
use super::{
    AttemptId, Claim, Provider, RapError, SchedulerStore, SessionId, SessionPhase, WorkId,
};
use crate::rap::{AttemptState, BoardLane, WorkPhase};

/// Reads the top-level candidates directly from the board. A work item with an
/// outstanding reservation comes before fresh work. Rank and id stabilize ties.
const CANDIDATES: &str = "
    SELECT w.id, w.phase, w.workspace, sw.provider, c.id
    FROM work_items w JOIN scheduler_work sw ON sw.work_id = w.id
    LEFT JOIN execution_contracts c ON c.id = w.current_contract_id
        AND c.work_id = w.id AND c.revision_id IS w.current_revision_id
    LEFT JOIN scheduler_provider_wait gate ON gate.provider = sw.provider
    WHERE w.parent_epic_id IS NULL
        AND NOT EXISTS (
            SELECT 1 FROM scheduler_dispatches d WHERE d.work_id = w.id
                AND d.state != 'released'
        )
        AND (gate.retry_at IS NULL OR gate.retry_at <= ?1)
        AND (sw.waiting_reason IS NULL OR sw.retry_at <= ?1)
        AND (
            (w.lane = 'backlog' AND w.phase = 'requested' AND sw.planning_authorized = 1)
            OR (w.lane = 'doing' AND w.phase = 'revising' AND sw.planning_authorized = 1)
            OR (
                w.lane IN ('todo', 'doing') AND w.phase = 'ready' AND c.id IS NOT NULL
                AND NOT EXISTS (
                    SELECT 1 FROM change_requests r WHERE r.work_id = w.id AND r.state = 'pending'
                )
            )
        )
    ORDER BY (w.lane = 'doing') DESC, w.rank, w.id";

struct Candidate {
    work_id: WorkId,
    phase: WorkPhase,
    workspace: String,
    provider: String,
    contract_id: Option<i64>,
}

struct Reservation {
    work_id: WorkId,
    identity: WorkspaceIdentity,
}

impl SchedulerStore {
    /// Reserves one free slot and the workspace in the same write transaction
    /// that creates the attempt. The caller may only launch after this commits.
    pub fn claim_next(
        &self,
        now: i64,
        available_providers: &[Provider],
    ) -> Result<Option<Claim>, RapError> {
        self.store.write(|tx| {
            if !has_capacity(tx)? {
                return Ok(None);
            }

            let reservations = reservations(tx)?;
            let mut statement = tx.prepare(CANDIDATES)?;
            let candidates = statement.query_map([now], |row| {
                Ok(Candidate {
                    work_id: WorkId(row.get(0)?),
                    phase: row.get(1)?,
                    workspace: row.get(2)?,
                    provider: row.get(3)?,
                    contract_id: row.get(4)?,
                })
            })?;

            for candidate in candidates {
                let candidate = candidate?;
                let provider = Provider::parse(&candidate.provider)?;
                if !available_providers.contains(&provider) {
                    continue;
                }
                let Ok(identity) = WorkspaceIdentity::resolve(Path::new(&candidate.workspace))
                else {
                    continue;
                };
                if reservations.iter().any(|lease| {
                    lease.work_id != candidate.work_id && lease.identity.overlaps(&identity)
                }) {
                    continue;
                }
                if self
                    .store
                    .verify_work_documents_in(tx, candidate.work_id)
                    .is_err()
                {
                    continue;
                }

                let claim = reserve(tx, candidate, provider, identity)?;
                return Ok(Some(claim));
            }
            Ok(None)
        })
    }
}

fn has_capacity(tx: &Transaction<'_>) -> Result<bool, RapError> {
    Ok(tx.query_row(
        "SELECT auto_run AND max_sessions > (
            SELECT COUNT(*) FROM scheduler_dispatches WHERE state != 'released'
         ) FROM scheduler_settings WHERE id = 1",
        [],
        |row| row.get(0),
    )?)
}

fn reservations(tx: &Transaction<'_>) -> Result<Vec<Reservation>, RapError> {
    let mut statement =
        tx.prepare("SELECT work_id, path, fingerprint FROM scheduler_workspaces")?;
    let rows = statement.query_map([], |row| {
        Ok((
            WorkId(row.get(0)?),
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
        ))
    })?;
    rows.map(|row| {
        let (work_id, path, expected) = row?;
        // If a reserved directory moved or was replaced, a child may still
        // write through its old cwd. Hold all new claims until reconciled.
        let identity = WorkspaceIdentity::resolve(Path::new(&path))
            .map_err(|_| RapError::Conflict("workspace reservation could not be reconciled"))?;
        if identity.fingerprint() != expected {
            return Err(RapError::Conflict("workspace reservation identity changed"));
        }
        Ok(Reservation { work_id, identity })
    })
    .collect()
}

fn reserve(
    tx: &Transaction<'_>,
    candidate: Candidate,
    provider: Provider,
    identity: WorkspaceIdentity,
) -> Result<Claim, RapError> {
    let work_id = candidate.work_id;
    let phase = if candidate.phase == WorkPhase::Ready {
        SessionPhase::Execution
    } else {
        SessionPhase::Planning
    };
    let contract_id = if phase == SessionPhase::Execution {
        candidate.contract_id
    } else {
        None
    };

    let (session_id, native_id) = reserve_session(tx, work_id, phase, provider, contract_id)?;
    let epoch: i64 = tx.query_row(
        "SELECT COALESCE(MAX(epoch), 0) + 1 FROM attempts WHERE session_id = ?1",
        [session_id.0],
        |row| row.get(0),
    )?;
    let (attempt_id,): (i64,) = tx.query_row(
        "INSERT INTO attempts (session_id, epoch, state, owner_token, contract_id)
         VALUES (?1, ?2, ?3, lower(hex(randomblob(32))), ?4) RETURNING id",
        params![session_id.0, epoch, AttemptState::Starting, contract_id],
        |row| Ok((row.get(0)?,)),
    )?;
    tx.execute(
        "INSERT INTO scheduler_dispatches (attempt_id, work_id, state)
         VALUES (?1, ?2, 'reserved')",
        params![attempt_id, work_id.0],
    )?;
    tx.execute(
        "INSERT INTO scheduler_workspaces (work_id, path, fingerprint)
         VALUES (?1, ?2, ?3) ON CONFLICT(work_id) DO NOTHING",
        params![work_id.0, candidate.workspace, identity.fingerprint()],
    )?;
    match phase {
        SessionPhase::Execution => {
            tx.execute(
                "UPDATE work_items SET lane = ?1, phase = ?2, version = version + 1 WHERE id = ?3",
                params![BoardLane::Doing, WorkPhase::Executing, work_id.0],
            )?;
        }
        SessionPhase::Planning if candidate.phase == WorkPhase::Requested => {
            tx.execute(
                "UPDATE work_items SET phase = ?1, version = version + 1 WHERE id = ?2",
                params![WorkPhase::Planning, work_id.0],
            )?;
        }
        SessionPhase::Planning => {}
    }
    Ok(Claim {
        work_id,
        session_id,
        attempt_id: AttemptId(attempt_id),
        epoch,
        phase,
        provider,
        workspace: identity.path().to_owned(),
        contract_id,
        native_id,
    })
}

fn reserve_session(
    tx: &Transaction<'_>,
    work_id: WorkId,
    phase: SessionPhase,
    provider: Provider,
    contract_id: Option<i64>,
) -> Result<(SessionId, Option<String>), RapError> {
    let reusable = tx
        .query_row(
            "SELECT s.id, s.native_id FROM sessions s JOIN attempts a ON a.session_id = s.id
             WHERE s.work_id = ?1 AND s.phase = ?2 AND s.provider = ?3
                AND a.contract_id IS ?4 AND a.state IN ('suspended', 'retry_wait')
                AND a.epoch = (SELECT MAX(epoch) FROM attempts WHERE session_id = s.id)
             ORDER BY s.id DESC LIMIT 1",
            params![work_id.0, phase, provider.as_str(), contract_id],
            |row| Ok((SessionId(row.get(0)?), row.get(1)?)),
        )
        .optional()?;
    match reusable {
        Some(session) => Ok(session),
        None => {
            let id = tx.query_row(
                "INSERT INTO sessions (work_id, phase, provider) VALUES (?1, ?2, ?3)
                 RETURNING id",
                params![work_id.0, phase, provider.as_str()],
                |row| row.get(0),
            )?;
            Ok((SessionId(id), None))
        }
    }
}
