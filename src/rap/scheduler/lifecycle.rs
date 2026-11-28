use std::path::Path;

use rusqlite::{OptionalExtension, params};

use super::transitions::{
    StoppedOutcome, plan_exit, release_unstarted, set_attempt, set_rate_limit, set_retry, set_wait,
};
use super::workspace::WorkspaceIdentity;
use super::{Claim, RapError, SchedulerStore};
use crate::rap::{AttemptState, WorkId};

/// Evidence from the process supervisor, where a process still holding the
/// workspace can outlive the original CLI PID.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessIdentity {
    pub pid: u32,
    /// A process creation identity, not its pid alone.
    pub fingerprint: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InFlight {
    pub claim: Claim,
    pub started: bool,
    pub process: Option<ProcessIdentity>,
}

impl SchedulerStore {
    /// Durably records the point after which launch may have side effects.
    /// Callers must hold the runtime's singleton lease, start at most one
    /// child, and never retry a failed launch before reconciling.
    pub fn begin_launch(&self, claim: &Claim) -> Result<bool, RapError> {
        self.begin_launch_inner(claim, true)
    }

    pub fn begin_manual_launch(&self, claim: &Claim) -> Result<bool, RapError> {
        self.begin_launch_inner(claim, false)
    }

    fn begin_launch_inner(&self, claim: &Claim, require_auto_run: bool) -> Result<bool, RapError> {
        self.store.write(|tx| {
            let reservation = tx
                .query_row(
                    "SELECT r.path, r.fingerprint, (NOT ?5 OR cfg.auto_run) AND
                         ((s.phase = 'planning' AND sw.planning_authorized = 1
                           AND w.phase IN ('planning', 'revising')) OR
                          (s.phase = 'execution' AND w.phase = 'executing'
                           AND a.contract_id = w.current_contract_id
                           AND NOT EXISTS (
                               SELECT 1 FROM change_requests c
                               WHERE c.work_id = w.id AND c.state = 'pending'
                           )))
                     FROM scheduler_dispatches d
                     JOIN attempts a ON a.id = d.attempt_id
                     JOIN sessions s ON s.id = a.session_id
                     JOIN work_items w ON w.id = d.work_id
                     JOIN scheduler_workspaces r ON r.work_id = w.id
                     JOIN scheduler_work sw ON sw.work_id = w.id
                     JOIN scheduler_settings cfg ON cfg.id = 1
                     WHERE d.attempt_id = ?1 AND d.work_id = ?2 AND d.state = 'reserved'
                       AND s.id = ?3 AND a.epoch = ?4",
                    params![
                        claim.attempt_id.0,
                        claim.work_id.0,
                        claim.session_id.0,
                        claim.epoch,
                        require_auto_run
                    ],
                    |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, bool>(2)?,
                        ))
                    },
                )
                .optional()?
                .ok_or(RapError::Conflict("launching a stale reservation"))?;

            let still_same =
                WorkspaceIdentity::resolve(Path::new(&reservation.0)).is_ok_and(|identity| {
                    identity.path() == claim.workspace && identity.fingerprint() == reservation.1
                });
            if !reservation.2 || !still_same {
                release_unstarted(tx, claim)?;
                return Ok(false);
            }
            tx.execute(
                "UPDATE scheduler_dispatches SET state = 'starting' WHERE attempt_id = ?1",
                [claim.attempt_id.0],
            )?;
            Ok(true)
        })
    }

    pub fn record_process(
        &self,
        claim: &Claim,
        process: &ProcessIdentity,
        native_id: Option<&str>,
    ) -> Result<(), RapError> {
        self.store.write(|tx| {
            let n = tx.execute(
                "UPDATE scheduler_dispatches SET state = 'running', pid = ?1, process_fingerprint = ?2
                 WHERE attempt_id = ?3 AND work_id = ?4
                   AND state IN ('starting', 'running', 'unknown')
                   AND EXISTS (
                       SELECT 1 FROM attempts a WHERE a.id = attempt_id
                           AND a.session_id = ?5 AND a.epoch = ?6
                   )",
                params![
                    process.pid,
                    process.fingerprint,
                    claim.attempt_id.0,
                    claim.work_id.0,
                    claim.session_id.0,
                    claim.epoch,
                ],
            )?;
            if n != 1 {
                return Err(RapError::Conflict("recording a stale running attempt"));
            }
            set_attempt(tx, claim, AttemptState::Running)?;
            if let Some(native_id) = native_id {
                tx.execute(
                    "UPDATE sessions SET native_id = ?1 WHERE id = ?2",
                    params![native_id, claim.session_id.0],
                )?;
            }
            Ok(())
        })
    }

    pub fn mark_unknown(&self, claim: &Claim) -> Result<(), RapError> {
        self.store.write(|tx| {
            let n = tx.execute(
                "UPDATE scheduler_dispatches SET state = 'unknown'
                 WHERE attempt_id = ?1 AND work_id = ?2 AND state IN ('starting', 'running', 'stopping', 'unknown')",
                params![claim.attempt_id.0, claim.work_id.0],
            )?;
            if n != 1 {
                return Err(RapError::Conflict("marking a stale attempt unknown"));
            }
            set_attempt(tx, claim, AttemptState::Unknown)
        })
    }

    /// Return a reservation from which no start was ever attempted.
    pub fn release_unstarted(&self, claim: &Claim) -> Result<(), RapError> {
        self.store.write(|tx| release_unstarted(tx, claim))
    }

    /// Release the session slot after the supervisor proves the process and
    /// its writers have stopped. Awaiting input retains workspace ownership.
    pub fn confirm_stopped(&self, claim: &Claim, outcome: StoppedOutcome) -> Result<(), RapError> {
        self.store.write(|tx| {
            let n = tx.execute(
                "UPDATE scheduler_dispatches SET state = 'released'
                 WHERE attempt_id = ?1 AND work_id = ?2 AND state IN ('starting', 'running', 'stopping', 'unknown')",
                params![claim.attempt_id.0, claim.work_id.0],
            )?;
            if n != 1 {
                return Err(RapError::Conflict("releasing a stale attempt"));
            }
            let Some((phase, state)) = plan_exit(claim, outcome) else {
                return Err(RapError::InvalidInput("the stop outcome does not match the session"));
            };
            set_attempt(tx, claim, state)?;
            match outcome {
                StoppedOutcome::AwaitingInput => set_wait(tx, claim.work_id, "awaiting_input", None)?,
                StoppedOutcome::Retry { now } => set_retry(tx, claim.work_id, now)?,
                StoppedOutcome::RateLimited { until } => set_rate_limit(tx, claim, until)?,
                StoppedOutcome::PlanReady | StoppedOutcome::Verify | StoppedOutcome::Canceled => {
                    tx.execute(
                        "DELETE FROM scheduler_workspaces WHERE work_id = ?1",
                        [claim.work_id.0],
                    )?;
                    tx.execute(
                        "UPDATE scheduler_work SET waiting_reason = NULL, retry_at = NULL, retry_count = 0
                         WHERE work_id = ?1",
                        [claim.work_id.0],
                    )?;
                }
            }
            if outcome == StoppedOutcome::Canceled {
                tx.execute(
                    "UPDATE work_items SET lane = 'canceled', phase = 'canceled', version = version + 1
                     WHERE id = ?1 AND phase NOT IN ('completed', 'canceled')",
                    [claim.work_id.0],
                )?;
            } else if let Some((previous, next)) = phase {
                // A newer request may already have moved the work into
                // replanning. An exit from the old execution must not undo it.
                tx.execute(
                    "UPDATE work_items SET phase = ?1, version = version + 1
                     WHERE id = ?2 AND phase = ?3",
                    params![next, claim.work_id.0, previous],
                )?;
            }
            Ok(())
        })
    }

    /// Clear a scheduler wait after the relevant input or explicit retry
    /// was authorized. The contract and other claim checks still apply.
    pub fn resume_work(&self, work_id: WorkId) -> Result<(), RapError> {
        self.store.write(|tx| {
            tx.execute(
                "UPDATE scheduler_work SET waiting_reason = NULL, retry_at = NULL, retry_count = 0
                 WHERE work_id = ?1 AND waiting_reason IN ('awaiting_input', 'retry_exhausted')",
                [work_id.0],
            )?;
            Ok(())
        })
    }

    pub fn in_flight(&self) -> Result<Vec<InFlight>, RapError> {
        self.store.read(|connection| {
            let mut statement = connection.prepare(
                "SELECT w.id, s.id, a.id, a.epoch, s.phase, s.provider, r.path,
                    a.contract_id, s.native_id, d.state != 'reserved', d.pid, d.process_fingerprint
                 FROM scheduler_dispatches d
                 JOIN attempts a ON a.id = d.attempt_id
                 JOIN sessions s ON s.id = a.session_id
                 JOIN work_items w ON w.id = d.work_id
                 JOIN scheduler_workspaces r ON r.work_id = w.id
                 WHERE d.state != 'released' ORDER BY a.id",
            )?;
            let rows = statement.query_map([], |row| {
                Ok((
                    (
                        row.get::<_, i64>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get::<_, String>(5)?,
                        row.get::<_, String>(6)?,
                        row.get(7)?,
                        row.get(8)?,
                    ),
                    row.get(9)?,
                    row.get::<_, Option<u32>>(10)?,
                    row.get::<_, Option<String>>(11)?,
                ))
            })?;
            rows.map(|row| {
                let (
                    (
                        work,
                        session,
                        attempt,
                        epoch,
                        phase,
                        provider,
                        workspace,
                        contract_id,
                        native_id,
                    ),
                    started,
                    pid,
                    fingerprint,
                ) = row?;
                Ok(InFlight {
                    claim: Claim {
                        work_id: WorkId(work),
                        session_id: super::SessionId(session),
                        attempt_id: super::AttemptId(attempt),
                        epoch,
                        phase,
                        provider: super::Provider::parse(&provider)?,
                        workspace: workspace.into(),
                        contract_id,
                        native_id,
                    },
                    started,
                    process: pid
                        .zip(fingerprint)
                        .map(|(pid, fingerprint)| ProcessIdentity { pid, fingerprint }),
                })
            })
            .collect()
        })
    }
}
