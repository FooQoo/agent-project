use rusqlite::{Connection, OptionalExtension, Row, params};
use serde::{Deserialize, Serialize};

use super::{
    ChangeRequest, ChangeRequestId, ChangeResolution, DocumentHash, PlanRevision, QuestionId,
    RapError, RapStore, RevisionId, SessionId, WorkId, fetch_work,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeliveryState {
    Open,
    PendingDelivery,
    Delivered,
}

impl DeliveryState {
    fn parse(value: &str) -> rusqlite::Result<Self> {
        match value {
            "open" => Ok(Self::Open),
            "pending_delivery" => Ok(Self::PendingDelivery),
            "delivered" => Ok(Self::Delivered),
            _ => Err(rusqlite::Error::InvalidColumnType(
                8,
                "delivery_state".to_owned(),
                rusqlite::types::Type::Text,
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RapQuestion {
    pub id: QuestionId,
    pub work_id: WorkId,
    pub session_id: SessionId,
    pub prompt: String,
    pub asked_at: i64,
    pub answer_body: Option<String>,
    pub answered_by: Option<String>,
    pub answered_at: Option<i64>,
    pub delivery_state: DeliveryState,
    pub delivered_at: Option<i64>,
    pub delivery_attempts: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DeliveryOutcome {
    Delivered { resumed_work: bool },
    AlreadyDelivered,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReplanDiff {
    pub previous_revision_id: Option<RevisionId>,
    pub new_revision: PlanRevision,
    pub previous_plan: Option<Vec<u8>>,
    pub new_plan: Vec<u8>,
}

impl RapStore {
    /// Persists a question raised by one specific session of a work item.
    pub fn ask_question(
        &self,
        work: WorkId,
        session: SessionId,
        prompt: &str,
        now: i64,
    ) -> Result<RapQuestion, RapError> {
        if prompt.trim().is_empty() {
            return Err(RapError::InvalidInput("question prompt is required"));
        }
        self.write(|tx| {
            let valid: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM sessions WHERE id = ?1 AND work_id = ?2)",
                params![session.0, work.0],
                |row| row.get(0),
            )?;
            if !valid {
                return Err(RapError::NotFound("session"));
            }
            let id: i64 = tx.query_row(
                "INSERT INTO rap_questions (work_id, session_id, prompt, asked_at)
                 VALUES (?1, ?2, ?3, ?4) RETURNING id",
                params![work.0, session.0, prompt, now],
                |row| row.get(0),
            )?;
            fetch_question(tx, QuestionId(id))
        })
    }

    /// Durably records a human answer. Re-submitting the identical answer is idempotent.
    pub fn answer_question(
        &self,
        question: QuestionId,
        answer_body: &str,
        answered_by: &str,
        now: i64,
    ) -> Result<RapQuestion, RapError> {
        if answer_body.trim().is_empty() || answered_by.trim().is_empty() {
            return Err(RapError::InvalidInput(
                "answer body and responder are required",
            ));
        }
        self.write(|tx| {
            let current = fetch_question(tx, question)?;
            if let Some(existing) = &current.answer_body {
                if existing == answer_body {
                    return Ok(current);
                }
                return Err(RapError::Conflict("question already answered differently"));
            }
            tx.execute(
                "UPDATE rap_questions
                 SET answer_body = ?1, answered_by = ?2, answered_at = ?3,
                     delivery_state = 'pending_delivery'
                 WHERE id = ?4",
                params![answer_body, answered_by, now, question.0],
            )?;
            fetch_question(tx, question)
        })
    }

    /// Delivers an answer exactly once to its target session and resumes only that work item
    /// when no other undelivered questions remain.
    pub fn deliver_answer(
        &self,
        question: QuestionId,
        target_session: SessionId,
        now: i64,
    ) -> Result<DeliveryOutcome, RapError> {
        self.write(|tx| {
            let current = fetch_question(tx, question)?;
            if current.session_id != target_session {
                return Err(RapError::Conflict("answer targets a different session"));
            }
            if current.delivery_state == DeliveryState::Open {
                return Err(RapError::Conflict("question has not been answered yet"));
            }
            if current.delivery_state == DeliveryState::Delivered {
                return Ok(DeliveryOutcome::AlreadyDelivered);
            }
            tx.execute(
                "UPDATE rap_questions
                 SET delivery_state = 'delivered', delivered_at = ?1,
                     delivery_attempts = delivery_attempts + 1
                 WHERE id = ?2",
                params![now, question.0],
            )?;
            let remaining: i64 = tx.query_row(
                "SELECT COUNT(*) FROM rap_questions
                 WHERE work_id = ?1 AND delivery_state != 'delivered'",
                [current.work_id.0],
                |row| row.get(0),
            )?;
            let resumed_work = if remaining == 0 {
                tx.execute(
                    "UPDATE scheduler_work
                     SET waiting_reason = NULL, retry_at = NULL, retry_count = 0
                     WHERE work_id = ?1 AND waiting_reason = 'awaiting_input'",
                    [current.work_id.0],
                )? > 0
            } else {
                false
            };
            Ok(DeliveryOutcome::Delivered { resumed_work })
        })
    }

    pub fn question(&self, id: QuestionId) -> Result<RapQuestion, RapError> {
        self.read(|connection| fetch_question(connection, id))
    }

    /// Absorbs an in-scope change request while recording the reason and keeping the contract.
    pub fn absorb_change_within_contract(
        &self,
        work: WorkId,
        request: ChangeRequestId,
        reason: &str,
    ) -> Result<ChangeRequest, RapError> {
        self.resolve_change_request(work, request, ChangeResolution::Absorbed { reason })
    }

    /// Replans a work item after proving no old execution dispatch is active, returning
    /// the revision diff for human re-approval.
    pub fn replan_from_change_request(
        &self,
        work: WorkId,
        request: ChangeRequestId,
        summary: &str,
        new_plan_bytes: &[u8],
        reason: &str,
    ) -> Result<ReplanDiff, RapError> {
        if reason.trim().is_empty() {
            return Err(RapError::InvalidInput("replan reason is required"));
        }
        let (previous_revision_id, previous_plan) = self.read(|connection| {
            let has_active: bool = connection.query_row(
                "SELECT EXISTS(
                    SELECT 1 FROM scheduler_dispatches
                    WHERE work_id = ?1 AND state != 'released'
                 )",
                [work.0],
                |row| row.get(0),
            )?;
            if has_active {
                return Err(RapError::Conflict(
                    "old execution must be confirmed stopped before replanning",
                ));
            }
            let item = fetch_work(connection, work)?;
            let previous_plan = match item.current_revision_id {
                Some(rev_id) => {
                    let raw_hash: String = connection.query_row(
                        "SELECT document_hash FROM plan_revisions WHERE id = ?1",
                        [rev_id.0],
                        |row| row.get(0),
                    )?;
                    let hash = DocumentHash::try_from(raw_hash.as_str())?;
                    Some(self.read_work_document(connection, work, &hash)?)
                }
                None => None,
            };
            Ok((item.current_revision_id, previous_plan))
        })?;

        let new_revision = self.record_plan_revision(work, summary, new_plan_bytes)?;
        self.resolve_change_request(
            work,
            request,
            ChangeResolution::Replanned {
                revision_id: new_revision.id,
                reason,
            },
        )?;
        Ok(ReplanDiff {
            previous_revision_id,
            new_revision,
            previous_plan,
            new_plan: new_plan_bytes.to_vec(),
        })
    }
}

pub(crate) fn fetch_question(
    connection: &Connection,
    id: QuestionId,
) -> Result<RapQuestion, RapError> {
    connection
        .query_row(
            "SELECT id, work_id, session_id, prompt, asked_at, answer_body, answered_by,
                    answered_at, delivery_state, delivered_at, delivery_attempts
             FROM rap_questions WHERE id = ?1",
            [id.0],
            read_question,
        )
        .optional()?
        .ok_or(RapError::NotFound("question"))
}

pub(crate) fn read_question(row: &Row<'_>) -> rusqlite::Result<RapQuestion> {
    let raw_state: String = row.get(8)?;
    Ok(RapQuestion {
        id: QuestionId(row.get(0)?),
        work_id: WorkId(row.get(1)?),
        session_id: SessionId(row.get(2)?),
        prompt: row.get(3)?,
        asked_at: row.get(4)?,
        answer_body: row.get(5)?,
        answered_by: row.get(6)?,
        answered_at: row.get(7)?,
        delivery_state: DeliveryState::parse(&raw_state)?,
        delivered_at: row.get(9)?,
        delivery_attempts: row.get(10)?,
    })
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::num::NonZeroU32;

    use super::*;
    use crate::rap::scheduler::{
        ProcessIdentity, Provider, SchedulerSettings, SchedulerStore, StoppedOutcome,
    };
    use crate::rap::{ChangeRequestState, NewWork, WorkKind};

    #[test]
    fn persists_answers_delivers_once_and_resumes_only_target_work() {
        let root = tempfile::tempdir().unwrap();
        let ws1 = root.path().join("ws1");
        let ws2 = root.path().join("ws2");
        fs::create_dir(&ws1).unwrap();
        fs::create_dir(&ws2).unwrap();
        let db = root.path().join("rap.db");
        let store = RapStore::open(&db).unwrap();
        let scheduler = SchedulerStore::new(store.clone());
        scheduler
            .set_settings(SchedulerSettings {
                auto_run: true,
                max_sessions: NonZeroU32::new(2).unwrap(),
            })
            .unwrap();

        let w1 = store
            .create_work(NewWork {
                project_id: "demo",
                workspace: &ws1,
                kind: WorkKind::Task,
                parent_epic_id: None,
                title: "Work 1",
                rank: 1,
            })
            .unwrap()
            .id;
        let w2 = store
            .create_work(NewWork {
                project_id: "demo",
                workspace: &ws2,
                kind: WorkKind::Task,
                parent_epic_id: None,
                title: "Work 2",
                rank: 2,
            })
            .unwrap()
            .id;
        for w in [w1, w2] {
            scheduler.configure_work(w, Provider::Codex, false).unwrap();
            store
                .authorize_direct_execution(w, b"brief", "human", 1)
                .unwrap();
        }
        let c1 = scheduler
            .claim_next(2, &[Provider::Codex])
            .unwrap()
            .unwrap();
        let c2 = scheduler
            .claim_next(2, &[Provider::Codex])
            .unwrap()
            .unwrap();
        assert!(scheduler.begin_launch(&c1).unwrap());
        assert!(scheduler.begin_launch(&c2).unwrap());
        let q1 = store
            .ask_question(w1, c1.session_id, "Use SQLite?", 3)
            .unwrap();
        scheduler
            .confirm_stopped(&c1, StoppedOutcome::AwaitingInput)
            .unwrap();
        scheduler
            .confirm_stopped(&c2, StoppedOutcome::AwaitingInput)
            .unwrap();

        // Reopen DB after restart and answer/deliver.
        let reopened = RapStore::open(&db).unwrap();
        let reopened_sched = SchedulerStore::new(reopened.clone());
        let answered = reopened
            .answer_question(q1.id, "Yes, use SQLite.", "human", 10)
            .unwrap();
        assert_eq!(answered.delivery_state, DeliveryState::PendingDelivery);
        // Wrong session is rejected.
        assert!(matches!(
            reopened.deliver_answer(q1.id, c2.session_id, 11),
            Err(RapError::Conflict(_))
        ));
        // Delivering to target session succeeds and resumes ONLY w1.
        assert_eq!(
            reopened.deliver_answer(q1.id, c1.session_id, 11).unwrap(),
            DeliveryOutcome::Delivered { resumed_work: true }
        );
        assert_eq!(reopened_sched.waiting_reason(w1).unwrap(), None);
        assert_eq!(
            reopened_sched.waiting_reason(w2).unwrap(),
            Some(("awaiting_input".to_owned(), None))
        );
        // Re-delivering the same answer is a no-op and persists delivery count = 1.
        assert_eq!(
            reopened.deliver_answer(q1.id, c1.session_id, 12).unwrap(),
            DeliveryOutcome::AlreadyDelivered
        );
        let persisted = reopened.question(q1.id).unwrap();
        assert_eq!(persisted.delivery_state, DeliveryState::Delivered);
        assert_eq!(persisted.delivery_attempts, 1);
    }

    #[test]
    fn absorbs_in_scope_changes_and_requires_stop_and_diff_for_replan() {
        let root = tempfile::tempdir().unwrap();
        let ws = root.path().join("ws");
        fs::create_dir(&ws).unwrap();
        let store = RapStore::open(&root.path().join("rap.db")).unwrap();
        let scheduler = SchedulerStore::new(store.clone());
        scheduler
            .set_settings(SchedulerSettings {
                auto_run: true,
                max_sessions: NonZeroU32::new(1).unwrap(),
            })
            .unwrap();

        let work = store
            .create_work(NewWork {
                project_id: "demo",
                workspace: &ws,
                kind: WorkKind::Task,
                parent_epic_id: None,
                title: "Auth flow",
                rank: 1,
            })
            .unwrap()
            .id;
        scheduler
            .configure_work(work, Provider::Codex, true)
            .unwrap();
        let v1 = store
            .record_plan_revision(work, "v1", b"# Plan v1\n- Login")
            .unwrap();
        store
            .approve_plan_revision(work, v1.id, "human", 10)
            .unwrap();

        // In-scope adjustment records reason without breaking contract.
        let minor = store
            .receive_change_request(work, "Use clearer button copy", 11)
            .unwrap();
        let absorbed = store
            .absorb_change_within_contract(work, minor.id, "Within approved copy scope")
            .unwrap();
        assert_eq!(absorbed.state, ChangeRequestState::Absorbed);
        assert_eq!(
            absorbed.resolution_reason.as_deref(),
            Some("Within approved copy scope")
        );
        assert!(store.current_contract(work).unwrap().is_some());

        // Start execution; scope change cannot replan until old execution is confirmed stopped.
        let claim = scheduler
            .claim_next(12, &[Provider::Codex])
            .unwrap()
            .unwrap();
        assert!(scheduler.begin_launch(&claim).unwrap());
        scheduler
            .record_process(
                &claim,
                &ProcessIdentity {
                    pid: 5000,
                    fingerprint: "fp-5000".into(),
                },
                None,
            )
            .unwrap();
        let scope = store
            .receive_change_request(work, "Add OAuth2 support", 13)
            .unwrap();
        assert!(matches!(
            store.replan_from_change_request(
                work,
                scope.id,
                "v2",
                b"# Plan v2\n- Login\n- OAuth2",
                "Scope expanded to OAuth2",
            ),
            Err(RapError::Conflict(_))
        ));

        // After confirming old execution stopped, replan returns diff and awaits re-approval.
        scheduler
            .confirm_stopped(&claim, StoppedOutcome::AwaitingInput)
            .unwrap();
        let diff = store
            .replan_from_change_request(
                work,
                scope.id,
                "v2",
                b"# Plan v2\n- Login\n- OAuth2",
                "Scope expanded to OAuth2",
            )
            .unwrap();
        assert_eq!(diff.previous_revision_id, Some(v1.id));
        assert_eq!(
            diff.previous_plan.as_deref(),
            Some(&b"# Plan v1\n- Login"[..])
        );
        assert_eq!(diff.new_plan, b"# Plan v2\n- Login\n- OAuth2");
        assert!(store.current_contract(work).unwrap().is_none());

        // Re-approval freezes the new v2 contract.
        let c2 = store
            .approve_plan_revision(work, diff.new_revision.id, "human", 20)
            .unwrap();
        assert_eq!(c2.revision_id, Some(diff.new_revision.id));
    }
}
