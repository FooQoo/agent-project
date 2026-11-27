use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};

use super::{
    BoardLane, ContractId, DocumentHash, EvidenceId, RapError, RapStore, WorkId, WorkItem,
    WorkPhase, fetch_work,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    Verification,
    FinalResult,
}

impl EvidenceKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Verification => "verification",
            Self::FinalResult => "final_result",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Evidence {
    pub id: EvidenceId,
    pub work_id: WorkId,
    pub contract_id: ContractId,
    pub kind: EvidenceKind,
    pub summary: String,
    pub document_hash: Option<DocumentHash>,
    pub recorded_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CompletionBlocker {
    NoCurrentContract,
    StaleContract,
    DocumentIntegrity(String),
    MissingVerification,
    MissingFinalResult,
    PendingChangeRequests(i64),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CompletionCheck {
    pub ready: bool,
    pub blockers: Vec<CompletionBlocker>,
}

impl RapStore {
    /// Records verification or final-result evidence tied to a specific execution contract.
    pub fn record_evidence(
        &self,
        work: WorkId,
        contract: ContractId,
        kind: EvidenceKind,
        summary: &str,
        artifact: Option<&[u8]>,
        now: i64,
    ) -> Result<Evidence, RapError> {
        if summary.trim().is_empty() {
            return Err(RapError::InvalidInput("evidence summary is required"));
        }
        self.write(|tx| {
            let valid_contract: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM execution_contracts WHERE id = ?1 AND work_id = ?2)",
                params![contract.0, work.0],
                |row| row.get(0),
            )?;
            if !valid_contract {
                return Err(RapError::NotFound("execution contract"));
            }
            let hash = match artifact {
                Some(bytes) => Some(self.persist_document(tx, work, bytes)?),
                None => None,
            };
            let id: i64 = tx.query_row(
                "INSERT INTO completion_evidence
                    (work_id, contract_id, kind, summary, document_hash, recorded_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6) RETURNING id",
                params![
                    work.0,
                    contract.0,
                    kind.as_str(),
                    summary,
                    hash.as_ref().map(DocumentHash::as_str),
                    now,
                ],
                |row| row.get(0),
            )?;
            Ok(Evidence {
                id: EvidenceId(id),
                work_id: work,
                contract_id: contract,
                kind,
                summary: summary.to_owned(),
                document_hash: hash,
                recorded_at: now,
            })
        })
    }

    /// Evaluates whether a work item meets every completion condition.
    pub fn evaluate_completion(&self, work: WorkId) -> Result<CompletionCheck, RapError> {
        self.read(|connection| self.check_completion_in(connection, work))
    }

    /// Transitions a work item to Done only when contract, evidence, integrity, and
    /// pending change-request checks all pass in the same write transaction.
    pub fn complete_work(&self, work: WorkId) -> Result<WorkItem, RapError> {
        self.write(|tx| {
            let check = self.check_completion_in(tx, work)?;
            if !check.ready {
                return Err(RapError::CompletionBlocked(check.blockers));
            }
            tx.execute(
                "UPDATE work_items SET lane = ?1, phase = ?2, version = version + 1 WHERE id = ?3",
                params![BoardLane::Done, WorkPhase::Completed, work.0],
            )?;
            fetch_work(tx, work)
        })
    }

    fn check_completion_in(
        &self,
        connection: &Connection,
        work: WorkId,
    ) -> Result<CompletionCheck, RapError> {
        let item = fetch_work(connection, work)?;
        let mut blockers = Vec::new();
        match item.current_contract_id {
            None => blockers.push(CompletionBlocker::NoCurrentContract),
            Some(contract_id) => {
                let matches_revision: bool = connection.query_row(
                    "SELECT EXISTS(
                        SELECT 1 FROM execution_contracts c
                        JOIN work_items w ON w.id = c.work_id
                        WHERE c.id = ?1 AND c.work_id = ?2 AND c.revision_id IS w.current_revision_id
                     )",
                    params![contract_id.0, work.0],
                    |row| row.get(0),
                )?;
                if !matches_revision {
                    blockers.push(CompletionBlocker::StaleContract);
                }
                if !has_evidence(connection, work, contract_id, EvidenceKind::Verification)? {
                    blockers.push(CompletionBlocker::MissingVerification);
                }
                if !has_evidence(connection, work, contract_id, EvidenceKind::FinalResult)? {
                    blockers.push(CompletionBlocker::MissingFinalResult);
                }
            }
        }
        if let Err(RapError::DocumentIntegrity(hash)) =
            self.verify_work_documents_in(connection, work)
        {
            blockers.push(CompletionBlocker::DocumentIntegrity(hash));
        }
        let pending: i64 = connection.query_row(
            "SELECT COUNT(*) FROM change_requests WHERE work_id = ?1 AND state = 'pending'",
            [work.0],
            |row| row.get(0),
        )?;
        if pending > 0 {
            blockers.push(CompletionBlocker::PendingChangeRequests(pending));
        }
        Ok(CompletionCheck {
            ready: blockers.is_empty(),
            blockers,
        })
    }
}

fn has_evidence(
    connection: &Connection,
    work: WorkId,
    contract: ContractId,
    kind: EvidenceKind,
) -> Result<bool, RapError> {
    Ok(connection.query_row(
        "SELECT EXISTS(
            SELECT 1 FROM completion_evidence
            WHERE work_id = ?1 AND contract_id = ?2 AND kind = ?3
         )",
        params![work.0, contract.0, kind.as_str()],
        |row| row.get(0),
    )?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rap::{ChangeRequestState, ChangeResolution, NewWork, WorkKind};

    fn create_task(store: &RapStore, root: &std::path::Path, title: &str) -> WorkId {
        store
            .create_work(NewWork {
                project_id: "demo",
                workspace: root,
                kind: WorkKind::Task,
                parent_epic_id: None,
                title,
                rank: 0,
            })
            .unwrap()
            .id
    }

    #[test]
    fn goal_finish_without_evidence_or_with_pending_change_does_not_complete() {
        let root = tempfile::tempdir().unwrap();
        let db = root.path().join("rap.db");
        let store = RapStore::open(&db).unwrap();
        let work = create_task(&store, root.path(), "Build verifier");

        let rev = store.record_plan_revision(work, "v1", b"# Plan").unwrap();
        let contract = store
            .approve_plan_revision(work, rev.id, "human", 10)
            .unwrap();

        // Simulate Goal ending (phase = verifying) without evidence.
        store
            .write(|tx| {
                tx.execute(
                    "UPDATE work_items SET lane = 'doing', phase = 'verifying' WHERE id = ?1",
                    [work.0],
                )?;
                Ok(())
            })
            .unwrap();

        let err = store.complete_work(work).unwrap_err();
        assert!(matches!(
            err,
            RapError::CompletionBlocked(ref b)
                if b.contains(&CompletionBlocker::MissingVerification)
                    && b.contains(&CompletionBlocker::MissingFinalResult)
        ));
        assert_eq!(store.work(work).unwrap().lane, BoardLane::Doing);

        // Add verification and final result, plus a pending change request before completion.
        store
            .record_evidence(
                work,
                contract.id,
                EvidenceKind::Verification,
                "cargo nextest passed",
                None,
                11,
            )
            .unwrap();
        store
            .record_evidence(
                work,
                contract.id,
                EvidenceKind::FinalResult,
                "PR summary",
                Some(b"final artifact"),
                12,
            )
            .unwrap();
        let req = store
            .receive_change_request(work, "Also handle empty input", 13)
            .unwrap();
        assert_eq!(req.state, ChangeRequestState::Pending);
        assert_eq!(store.pending_change_requests(work).unwrap().len(), 1);

        // Pending change request blocks Done.
        let blocked = store.complete_work(work).unwrap_err();
        assert!(matches!(
            blocked,
            RapError::CompletionBlocked(ref b)
                if b == &[CompletionBlocker::PendingChangeRequests(1)]
        ));

        // Resolving the change request lets completion succeed.
        store
            .resolve_change_request(
                work,
                req.id,
                ChangeResolution::Absorbed {
                    reason: "Covered by existing validation",
                },
            )
            .unwrap();
        let done = store.complete_work(work).unwrap();
        assert_eq!(done.lane, BoardLane::Done);
        assert_eq!(done.phase, WorkPhase::Completed);

        // A change request arriving AFTER completion starts a new revision flow.
        let post = store
            .receive_change_request(work, "Add follow-up option", 20)
            .unwrap();
        assert_eq!(post.state, ChangeRequestState::Pending);
        let reopened = store.work(work).unwrap();
        assert_eq!(reopened.lane, BoardLane::Doing);
        assert_eq!(reopened.phase, WorkPhase::Revising);
        assert_eq!(reopened.current_contract_id, None);
    }
}
