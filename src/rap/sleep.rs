use rusqlite::{Row, params};
use serde::{Deserialize, Serialize};

use super::collaboration::{fetch_question, read_question};
use super::completion::read_evidence;
use super::{DecisionId, Evidence, QuestionId, RapError, RapQuestion, RapStore, WorkId};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionRisk {
    Routine,
    PrivilegeEscalation,
    Irreversible,
}

impl DecisionRisk {
    fn as_str(self) -> &'static str {
        match self {
            Self::Routine => "routine",
            Self::PrivilegeEscalation => "privilege_escalation",
            Self::Irreversible => "irreversible",
        }
    }

    fn parse(value: &str) -> rusqlite::Result<Self> {
        match value {
            "routine" => Ok(Self::Routine),
            "privilege_escalation" => Ok(Self::PrivilegeEscalation),
            "irreversible" => Ok(Self::Irreversible),
            _ => Err(rusqlite::Error::InvalidColumnType(
                5,
                "risk_level".to_owned(),
                rusqlite::types::Type::Text,
            )),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DelegationStatus {
    Delegated,
    HeldForHuman,
}

impl DelegationStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Delegated => "delegated",
            Self::HeldForHuman => "held_for_human",
        }
    }

    fn parse(value: &str) -> rusqlite::Result<Self> {
        match value {
            "delegated" => Ok(Self::Delegated),
            "held_for_human" => Ok(Self::HeldForHuman),
            _ => Err(rusqlite::Error::InvalidColumnType(
                6,
                "status".to_owned(),
                rusqlite::types::Type::Text,
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DelegatedDecision {
    pub id: DecisionId,
    pub question_id: QuestionId,
    pub work_id: WorkId,
    pub delegate_actor: String,
    pub rationale: String,
    pub risk_level: DecisionRisk,
    pub status: DelegationStatus,
    pub decided_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WakeupReport {
    pub sleep_started_at: Option<i64>,
    pub sleep_ended_at: i64,
    pub delegated_decisions: Vec<DelegatedDecision>,
    pub unanswered_questions: Vec<RapQuestion>,
    pub recorded_evidence: Vec<Evidence>,
}

impl RapStore {
    pub fn enter_sleep(&self, now: i64) -> Result<(), RapError> {
        self.write(|tx| {
            tx.execute(
                "UPDATE sleep_mode SET active = 1, started_at = ?1, ended_at = NULL WHERE id = 1",
                [now],
            )?;
            Ok(())
        })
    }

    pub fn is_sleeping(&self) -> Result<bool, RapError> {
        self.read(|connection| {
            Ok(
                connection.query_row("SELECT active FROM sleep_mode WHERE id = 1", [], |row| {
                    row.get(0)
                })?,
            )
        })
    }

    /// Evaluates a question under Sleep delegation. Routine decisions record the
    /// delegate and rationale, answer the question, and deliver it to the session.
    /// Privilege escalation and irreversible operations are held for human approval.
    pub fn delegate_question_during_sleep(
        &self,
        question: QuestionId,
        delegate_actor: &str,
        answer_body: &str,
        rationale: &str,
        risk_level: DecisionRisk,
        now: i64,
    ) -> Result<DelegatedDecision, RapError> {
        if delegate_actor.trim().is_empty() || rationale.trim().is_empty() {
            return Err(RapError::InvalidInput(
                "delegate actor and rationale are required",
            ));
        }
        if risk_level == DecisionRisk::Routine && answer_body.trim().is_empty() {
            return Err(RapError::InvalidInput(
                "answer body is required for routine delegation",
            ));
        }

        let (decision, session_id) = self.write(|tx| {
            let active: bool =
                tx.query_row("SELECT active FROM sleep_mode WHERE id = 1", [], |r| r.get(0))?;
            if !active {
                return Err(RapError::Conflict("delegation requires Sleep mode"));
            }
            let q = fetch_question(tx, question)?;
            if q.answer_body.is_some() {
                return Err(RapError::Conflict("question already answered"));
            }
            let status = match risk_level {
                DecisionRisk::Routine => DelegationStatus::Delegated,
                DecisionRisk::PrivilegeEscalation | DecisionRisk::Irreversible => {
                    DelegationStatus::HeldForHuman
                }
            };
            let id: i64 = tx.query_row(
                "INSERT INTO delegated_decisions
                    (question_id, work_id, delegate_actor, rationale, risk_level, status, decided_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7) RETURNING id",
                params![
                    q.id.0,
                    q.work_id.0,
                    delegate_actor,
                    rationale,
                    risk_level.as_str(),
                    status.as_str(),
                    now,
                ],
                |row| row.get(0),
            )?;
            let decision = DelegatedDecision {
                id: DecisionId(id),
                question_id: q.id,
                work_id: q.work_id,
                delegate_actor: delegate_actor.to_owned(),
                rationale: rationale.to_owned(),
                risk_level,
                status,
                decided_at: now,
            };
            Ok((decision, q.session_id))
        })?;

        if decision.status == DelegationStatus::Delegated {
            self.answer_question(question, answer_body, delegate_actor, now)?;
            self.deliver_answer(question, session_id, now)?;
        }
        Ok(decision)
    }

    /// Exits Sleep mode and summarizes delegated decisions, unanswered questions,
    /// and evidence recorded while the human was away.
    pub fn wake_from_sleep(&self, now: i64) -> Result<WakeupReport, RapError> {
        self.write(|tx| {
            let started_at: Option<i64> = tx.query_row(
                "SELECT started_at FROM sleep_mode WHERE id = 1",
                [],
                |row| row.get(0),
            )?;
            tx.execute(
                "UPDATE sleep_mode SET active = 0, ended_at = ?1 WHERE id = 1",
                [now],
            )?;
            let since = started_at.unwrap_or(0);
            let delegated_decisions = {
                let mut stmt = tx.prepare(
                    "SELECT id, question_id, work_id, delegate_actor, rationale,
                            risk_level, status, decided_at
                     FROM delegated_decisions WHERE decided_at >= ?1 ORDER BY id",
                )?;
                let rows = stmt.query_map([since], read_decision)?;
                rows.collect::<rusqlite::Result<Vec<_>>>()?
            };
            let unanswered_questions = {
                let mut stmt = tx.prepare(
                    "SELECT id, work_id, session_id, prompt, asked_at, answer_body, answered_by,
                            answered_at, delivery_state, delivered_at, delivery_attempts
                     FROM rap_questions WHERE delivery_state != 'delivered' ORDER BY id",
                )?;
                let rows = stmt.query_map([], read_question)?;
                rows.collect::<rusqlite::Result<Vec<_>>>()?
            };
            let recorded_evidence = {
                let mut stmt = tx.prepare(
                    "SELECT id, work_id, contract_id, kind, summary, document_hash, recorded_at
                     FROM completion_evidence WHERE recorded_at >= ?1 ORDER BY id",
                )?;
                let rows = stmt.query_map([since], read_evidence)?;
                rows.collect::<rusqlite::Result<Vec<_>>>()?
            };
            Ok(WakeupReport {
                sleep_started_at: started_at,
                sleep_ended_at: now,
                delegated_decisions,
                unanswered_questions,
                recorded_evidence,
            })
        })
    }
}

fn read_decision(row: &Row<'_>) -> rusqlite::Result<DelegatedDecision> {
    let raw_risk: String = row.get(5)?;
    let raw_status: String = row.get(6)?;
    Ok(DelegatedDecision {
        id: DecisionId(row.get(0)?),
        question_id: QuestionId(row.get(1)?),
        work_id: WorkId(row.get(2)?),
        delegate_actor: row.get(3)?,
        rationale: row.get(4)?,
        risk_level: DecisionRisk::parse(&raw_risk)?,
        status: DelegationStatus::parse(&raw_status)?,
        decided_at: row.get(7)?,
    })
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::num::NonZeroU32;

    use super::*;
    use crate::rap::scheduler::{Provider, SchedulerSettings, SchedulerStore, StoppedOutcome};
    use crate::rap::{DeliveryState, EvidenceKind, NewWork, WorkKind};

    #[test]
    fn delegates_routine_holds_risky_avoids_slot1_deadlock_and_builds_wakeup_report() {
        let root = tempfile::tempdir().unwrap();
        let ws_a = root.path().join("ws-a");
        let ws_b = root.path().join("ws-b");
        fs::create_dir(&ws_a).unwrap();
        fs::create_dir(&ws_b).unwrap();
        let store = RapStore::open(&root.path().join("rap.db")).unwrap();
        let scheduler = SchedulerStore::new(store.clone());
        // Slot limit = 1 to verify no deadlock while awaiting delegated judgment.
        scheduler
            .set_settings(SchedulerSettings {
                auto_run: true,
                max_sessions: NonZeroU32::new(1).unwrap(),
            })
            .unwrap();

        let wa = store
            .create_work(NewWork {
                project_id: "demo",
                workspace: &ws_a,
                kind: WorkKind::Task,
                parent_epic_id: None,
                title: "Routine task",
                rank: 1,
            })
            .unwrap()
            .id;
        let wb = store
            .create_work(NewWork {
                project_id: "demo",
                workspace: &ws_b,
                kind: WorkKind::Task,
                parent_epic_id: None,
                title: "Risky task",
                rank: 2,
            })
            .unwrap()
            .id;
        for w in [wa, wb] {
            scheduler.configure_work(w, Provider::Codex, false).unwrap();
        }
        let ca = store
            .authorize_direct_execution(wa, b"routine brief", "human", 10)
            .unwrap();
        store
            .authorize_direct_execution(wb, b"risky brief", "human", 10)
            .unwrap();

        store.enter_sleep(100).unwrap();
        assert!(store.is_sleeping().unwrap());

        // Work A claims slot 1, asks a routine question, and pauses -> frees slot 1.
        let claim_a = scheduler
            .claim_next(101, &[Provider::Codex])
            .unwrap()
            .unwrap();
        assert!(scheduler.begin_launch(&claim_a).unwrap());
        let qa = store
            .ask_question(wa, claim_a.session_id, "Format with rustfmt?", 102)
            .unwrap();
        scheduler
            .confirm_stopped(&claim_a, StoppedOutcome::AwaitingInput)
            .unwrap();

        // Slot 1 is NOT deadlocked: independent Work B claims slot 1 while A waits.
        let claim_b = scheduler
            .claim_next(103, &[Provider::Codex])
            .unwrap()
            .unwrap();
        assert_eq!(claim_b.work_id, wb);
        assert!(scheduler.begin_launch(&claim_b).unwrap());
        let qb = store
            .ask_question(wb, claim_b.session_id, "Drop production table?", 104)
            .unwrap();
        scheduler
            .confirm_stopped(&claim_b, StoppedOutcome::AwaitingInput)
            .unwrap();

        // Routine decision is delegated with actor + rationale, answered and delivered.
        let dec_a = store
            .delegate_question_during_sleep(
                qa.id,
                "sleep-delegate",
                "Yes, run cargo fmt.",
                "Formatting is reversible and required by repo policy.",
                DecisionRisk::Routine,
                105,
            )
            .unwrap();
        assert_eq!(dec_a.status, DelegationStatus::Delegated);
        assert_eq!(
            store.question(qa.id).unwrap().delivery_state,
            DeliveryState::Delivered
        );
        assert_eq!(scheduler.waiting_reason(wa).unwrap(), None);

        // Irreversible operation is held for human approval and NOT answered.
        let dec_b = store
            .delegate_question_during_sleep(
                qb.id,
                "sleep-delegate",
                "",
                "Dropping production data is irreversible; requires human approval.",
                DecisionRisk::Irreversible,
                106,
            )
            .unwrap();
        assert_eq!(dec_b.status, DelegationStatus::HeldForHuman);
        assert_eq!(
            store.question(qb.id).unwrap().delivery_state,
            DeliveryState::Open
        );
        assert_eq!(
            scheduler.waiting_reason(wb).unwrap(),
            Some(("awaiting_input".to_owned(), None))
        );

        // Work A produces final evidence during Sleep.
        store
            .record_evidence(
                wa,
                ca.id,
                EvidenceKind::FinalResult,
                "Formatted cleanly",
                None,
                110,
            )
            .unwrap();

        // Wakeup report shows decisions, unanswered risky question, and recorded evidence.
        let report = store.wake_from_sleep(200).unwrap();
        assert!(!store.is_sleeping().unwrap());
        assert_eq!(report.sleep_started_at, Some(100));
        assert_eq!(report.delegated_decisions.len(), 2);
        assert_eq!(report.unanswered_questions.len(), 1);
        assert_eq!(report.unanswered_questions[0].id, qb.id);
        assert_eq!(report.recorded_evidence.len(), 1);
        assert_eq!(report.recorded_evidence[0].work_id, wa);
    }
}
