use rusqlite::{Transaction, params};

use super::{Claim, RapError};
use crate::rap::{AttemptState, SessionPhase, WorkId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoppedOutcome {
    PlanReady,
    Verify,
    AwaitingInput,
    Retry { now: i64 },
    RateLimited { until: i64 },
    Canceled,
}

pub(super) fn release_unstarted(tx: &Transaction<'_>, claim: &Claim) -> Result<(), RapError> {
    let n = tx.execute(
        "UPDATE scheduler_dispatches SET state = 'released'
         WHERE attempt_id = ?1 AND work_id = ?2 AND state = 'reserved'",
        params![claim.attempt_id.0, claim.work_id.0],
    )?;
    if n != 1 {
        return Err(RapError::Conflict(
            "releasing a started or stale reservation",
        ));
    }
    set_attempt(tx, claim, AttemptState::RetryWait)?;
    restore_queue(tx, claim)?;
    Ok(())
}

pub(super) fn restore_queue(tx: &Transaction<'_>, claim: &Claim) -> Result<(), RapError> {
    let (previous, next) = match claim.phase {
        SessionPhase::Planning => ("planning", "requested"),
        SessionPhase::Execution => ("executing", "ready"),
    };
    tx.execute(
        "UPDATE work_items SET phase = ?1, version = version + 1 WHERE id = ?2 AND phase = ?3",
        params![next, claim.work_id.0, previous],
    )?;
    // A suspended or interrupted earlier execution may have partial writes.
    // Transfer the reservation to the next attempt rather than releasing it.
    tx.execute(
        "DELETE FROM scheduler_workspaces WHERE work_id = ?1 AND NOT EXISTS (
            SELECT 1 FROM scheduler_dispatches d JOIN attempts a ON a.id = d.attempt_id
            WHERE d.work_id = ?1 AND d.attempt_id <> ?2 AND a.state IN ('suspended', 'retry_wait')
         )",
        params![claim.work_id.0, claim.attempt_id.0],
    )?;
    Ok(())
}

pub(super) fn set_attempt(
    tx: &Transaction<'_>,
    claim: &Claim,
    state: AttemptState,
) -> Result<(), RapError> {
    let n = tx.execute(
        "UPDATE attempts SET state = ?1
         WHERE id = ?2 AND session_id = ?3 AND epoch = ?4
           AND epoch = (SELECT MAX(epoch) FROM attempts WHERE session_id = ?3)",
        params![state, claim.attempt_id.0, claim.session_id.0, claim.epoch],
    )?;
    if n != 1 {
        return Err(RapError::Conflict("updating a stale attempt"));
    }
    Ok(())
}

pub(super) type PhaseTransition = Option<(&'static str, &'static str)>;

pub(super) fn plan_exit(
    claim: &Claim,
    outcome: StoppedOutcome,
) -> Option<(PhaseTransition, AttemptState)> {
    let retry_phase = match claim.phase {
        SessionPhase::Planning => ("planning", "requested"),
        SessionPhase::Execution => ("executing", "ready"),
    };
    match outcome {
        StoppedOutcome::PlanReady if claim.phase == SessionPhase::Planning => {
            let transition = if claim.contract_id.is_some() {
                None
            } else {
                Some(("planning", "awaiting_approval"))
            };
            Some((transition, AttemptState::Finished))
        }
        StoppedOutcome::Verify if claim.phase == SessionPhase::Execution => {
            Some((Some(("executing", "verifying")), AttemptState::Finished))
        }
        StoppedOutcome::AwaitingInput => Some((Some(retry_phase), AttemptState::Suspended)),
        StoppedOutcome::Retry { .. } | StoppedOutcome::RateLimited { .. } => {
            Some((Some(retry_phase), AttemptState::RetryWait))
        }
        StoppedOutcome::Canceled => Some((None, AttemptState::Failed)),
        _ => None,
    }
}

pub(super) fn set_wait(
    tx: &Transaction<'_>,
    work_id: WorkId,
    reason: &str,
    retry_at: Option<i64>,
) -> Result<(), RapError> {
    tx.execute(
        "UPDATE scheduler_work SET waiting_reason = ?1, retry_at = ?2 WHERE work_id = ?3",
        params![reason, retry_at, work_id.0],
    )?;
    Ok(())
}

pub(super) fn set_retry(tx: &Transaction<'_>, work_id: WorkId, now: i64) -> Result<(), RapError> {
    const BACKOFF_SECS: [i64; 3] = [5, 30, 120];
    let failures: u32 = tx.query_row(
        "UPDATE scheduler_work SET retry_count = retry_count + 1 WHERE work_id = ?1
         RETURNING retry_count",
        [work_id.0],
        |row| row.get(0),
    )?;
    match BACKOFF_SECS.get((failures - 1) as usize) {
        Some(delay) => set_wait(tx, work_id, "retry_wait", Some(now.saturating_add(*delay))),
        None => set_wait(tx, work_id, "retry_exhausted", None),
    }
}

pub(super) fn set_rate_limit(
    tx: &Transaction<'_>,
    claim: &Claim,
    until: i64,
) -> Result<(), RapError> {
    tx.execute(
        "INSERT INTO scheduler_provider_wait (provider, retry_at) VALUES (?1, ?2)
         ON CONFLICT(provider) DO UPDATE SET retry_at = MAX(retry_at, excluded.retry_at)",
        params![claim.provider.as_str(), until],
    )?;
    set_wait(tx, claim.work_id, "rate_limited", Some(until))
}
