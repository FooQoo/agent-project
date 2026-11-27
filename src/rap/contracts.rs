use rusqlite::{Connection, OptionalExtension, Row, params};
use serde::{Deserialize, Serialize};

use super::{
    BoardLane, ContractId, DocumentHash, RapError, RapStore, RevisionId, WorkId, WorkPhase,
    fetch_work,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContractKind {
    PlanApproval,
    DirectExecution,
}

impl ContractKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::PlanApproval => "plan_approval",
            Self::DirectExecution => "direct_execution",
        }
    }

    fn parse(value: &str) -> rusqlite::Result<Self> {
        match value {
            "plan_approval" => Ok(Self::PlanApproval),
            "direct_execution" => Ok(Self::DirectExecution),
            _ => Err(rusqlite::Error::InvalidColumnType(
                2,
                "kind".to_owned(),
                rusqlite::types::Type::Text,
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PlanRevision {
    pub id: RevisionId,
    pub work_id: WorkId,
    pub revision_number: i64,
    pub document_hash: DocumentHash,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExecutionContract {
    pub id: ContractId,
    pub work_id: WorkId,
    pub kind: ContractKind,
    pub revision_id: Option<RevisionId>,
    pub document_hash: DocumentHash,
    pub authorized_by: String,
    pub authorized_at: i64,
}

impl RapStore {
    /// Stores an immutable plan revision and makes it the current revision awaiting approval.
    pub fn record_plan_revision(
        &self,
        work: WorkId,
        summary: &str,
        plan_bytes: &[u8],
    ) -> Result<PlanRevision, RapError> {
        if summary.trim().is_empty() {
            return Err(RapError::InvalidInput("plan revision summary is required"));
        }
        self.write(|tx| {
            let item = fetch_work(tx, work)?;
            if matches!(item.lane, BoardLane::Done | BoardLane::Canceled) {
                return Err(RapError::Conflict("recording a plan for finished work"));
            }
            let hash = self.persist_document(tx, work, plan_bytes)?;
            let next_number: i64 = tx.query_row(
                "SELECT COALESCE(MAX(revision_number), 0) + 1 FROM plan_revisions WHERE work_id = ?1",
                [work.0],
                |row| row.get(0),
            )?;
            let id: i64 = tx.query_row(
                "INSERT INTO plan_revisions (work_id, revision_number, document_hash, summary)
                 VALUES (?1, ?2, ?3, ?4) RETURNING id",
                params![work.0, next_number, hash.as_str(), summary],
                |row| row.get(0),
            )?;
            let (lane, phase) = match item.lane {
                BoardLane::Backlog => (BoardLane::Backlog, WorkPhase::AwaitingApproval),
                BoardLane::Todo => (BoardLane::Backlog, WorkPhase::AwaitingApproval),
                BoardLane::Doing => (BoardLane::Doing, WorkPhase::Revising),
                BoardLane::Done | BoardLane::Canceled => unreachable!(),
            };
            tx.execute(
                "UPDATE work_items
                 SET lane = ?1, phase = ?2, current_revision_id = ?3,
                     current_contract_id = NULL, version = version + 1
                 WHERE id = ?4",
                params![lane, phase, id, work.0],
            )?;
            Ok(PlanRevision {
                id: RevisionId(id),
                work_id: work,
                revision_number: next_number,
                document_hash: hash,
                summary: summary.to_owned(),
            })
        })
    }

    /// Approves the current plan revision and freezes it as an immutable execution contract.
    pub fn approve_plan_revision(
        &self,
        work: WorkId,
        revision: RevisionId,
        authorized_by: &str,
        now: i64,
    ) -> Result<ExecutionContract, RapError> {
        if authorized_by.trim().is_empty() {
            return Err(RapError::InvalidInput("approver identity is required"));
        }
        self.write(|tx| {
            let item = fetch_work(tx, work)?;
            if matches!(item.lane, BoardLane::Done | BoardLane::Canceled) {
                return Err(RapError::Conflict("approving finished work"));
            }
            if item.current_revision_id != Some(revision) {
                return Err(RapError::Conflict(
                    "only the current plan revision can be approved",
                ));
            }
            let raw_hash: String = tx
                .query_row(
                    "SELECT document_hash FROM plan_revisions WHERE id = ?1 AND work_id = ?2",
                    params![revision.0, work.0],
                    |row| row.get(0),
                )
                .optional()?
                .ok_or(RapError::NotFound("plan revision"))?;
            self.verify_work_documents_in(tx, work)?;
            let id: i64 = tx.query_row(
                "INSERT INTO execution_contracts
                    (work_id, kind, revision_id, document_hash, authorized_by, authorized_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6) RETURNING id",
                params![
                    work.0,
                    ContractKind::PlanApproval.as_str(),
                    revision.0,
                    raw_hash,
                    authorized_by,
                    now,
                ],
                |row| row.get(0),
            )?;
            let next_lane = match item.lane {
                BoardLane::Backlog => BoardLane::Todo,
                other => other,
            };
            tx.execute(
                "UPDATE work_items
                 SET lane = ?1, phase = ?2, current_contract_id = ?3, version = version + 1
                 WHERE id = ?4",
                params![next_lane, WorkPhase::Ready, id, work.0],
            )?;
            fetch_contract(tx, ContractId(id))
        })
    }

    /// Explicitly authorizes direct execution without a plan revision.
    pub fn authorize_direct_execution(
        &self,
        work: WorkId,
        brief_bytes: &[u8],
        authorized_by: &str,
        now: i64,
    ) -> Result<ExecutionContract, RapError> {
        if authorized_by.trim().is_empty() {
            return Err(RapError::InvalidInput("approver identity is required"));
        }
        self.write(|tx| {
            let item = fetch_work(tx, work)?;
            if matches!(item.lane, BoardLane::Done | BoardLane::Canceled) {
                return Err(RapError::Conflict("authorizing finished work"));
            }
            let hash = self.persist_document(tx, work, brief_bytes)?;
            self.verify_work_documents_in(tx, work)?;
            let id: i64 = tx.query_row(
                "INSERT INTO execution_contracts
                    (work_id, kind, revision_id, document_hash, authorized_by, authorized_at)
                 VALUES (?1, ?2, NULL, ?3, ?4, ?5) RETURNING id",
                params![
                    work.0,
                    ContractKind::DirectExecution.as_str(),
                    hash.as_str(),
                    authorized_by,
                    now,
                ],
                |row| row.get(0),
            )?;
            let next_lane = match item.lane {
                BoardLane::Backlog => BoardLane::Todo,
                other => other,
            };
            tx.execute(
                "UPDATE work_items
                 SET lane = ?1, phase = ?2, current_revision_id = NULL,
                     current_contract_id = ?3, version = version + 1
                 WHERE id = ?4",
                params![next_lane, WorkPhase::Ready, id, work.0],
            )?;
            fetch_contract(tx, ContractId(id))
        })
    }

    pub fn current_contract(&self, work: WorkId) -> Result<Option<ExecutionContract>, RapError> {
        self.read(|connection| {
            let item = fetch_work(connection, work)?;
            match item.current_contract_id {
                Some(id) => fetch_contract(connection, id).map(Some),
                None => Ok(None),
            }
        })
    }
}

pub(crate) fn fetch_contract(
    connection: &Connection,
    id: ContractId,
) -> Result<ExecutionContract, RapError> {
    connection
        .query_row(
            "SELECT id, work_id, kind, revision_id, document_hash, authorized_by, authorized_at
             FROM execution_contracts WHERE id = ?1",
            [id.0],
            read_contract,
        )
        .optional()?
        .ok_or(RapError::NotFound("execution contract"))
}

fn read_contract(row: &Row<'_>) -> rusqlite::Result<ExecutionContract> {
    let raw_kind: String = row.get(2)?;
    let raw_hash: String = row.get(4)?;
    let document_hash = DocumentHash::try_from(raw_hash.as_str()).map_err(|_| {
        rusqlite::Error::InvalidColumnType(
            4,
            "document_hash".to_owned(),
            rusqlite::types::Type::Text,
        )
    })?;
    Ok(ExecutionContract {
        id: ContractId(row.get(0)?),
        work_id: WorkId(row.get(1)?),
        kind: ContractKind::parse(&raw_kind)?,
        revision_id: row.get::<_, Option<i64>>(3)?.map(RevisionId),
        document_hash,
        authorized_by: row.get(5)?,
        authorized_at: row.get(6)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rap::{NewWork, WorkKind};

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
    fn approves_current_revision_rejects_stale_and_forbids_overwrites() {
        let root = tempfile::tempdir().unwrap();
        let db = root.path().join("rap.db");
        let store = RapStore::open(&db).unwrap();
        let work = create_task(&store, root.path(), "Ship RAP");

        let rev1 = store
            .record_plan_revision(work, "v1", b"# Plan v1")
            .unwrap();
        let rev2 = store
            .record_plan_revision(work, "v2", b"# Plan v2")
            .unwrap();
        assert_eq!(rev1.revision_number, 1);
        assert_eq!(rev2.revision_number, 2);

        // Stale revision approval is rejected.
        assert!(matches!(
            store.approve_plan_revision(work, rev1.id, "human", 100),
            Err(RapError::Conflict(_))
        ));

        // Current revision approval freezes an immutable contract.
        let contract = store
            .approve_plan_revision(work, rev2.id, "human", 101)
            .unwrap();
        assert_eq!(contract.kind, ContractKind::PlanApproval);
        assert_eq!(contract.revision_id, Some(rev2.id));
        assert_eq!(contract.document_hash, rev2.document_hash);
        let item = store.work(work).unwrap();
        assert_eq!(item.lane, BoardLane::Todo);
        assert_eq!(item.phase, WorkPhase::Ready);
        assert_eq!(item.current_contract_id, Some(contract.id));

        // Approved revision and contract rows cannot be overwritten or deleted.
        let update_rev = store.write(|tx| {
            tx.execute(
                "UPDATE plan_revisions SET summary = 'tampered' WHERE id = ?1",
                [rev2.id.0],
            )?;
            Ok(())
        });
        assert!(matches!(update_rev, Err(RapError::Database(_))));
        let update_contract = store.write(|tx| {
            tx.execute(
                "UPDATE execution_contracts SET authorized_by = 'other' WHERE id = ?1",
                [contract.id.0],
            )?;
            Ok(())
        });
        assert!(matches!(update_contract, Err(RapError::Database(_))));
    }

    #[test]
    fn direct_execution_authorizes_without_plan_and_checks_integrity() {
        let root = tempfile::tempdir().unwrap();
        let db = root.path().join("rap.db");
        let store = RapStore::open(&db).unwrap();
        let work = create_task(&store, root.path(), "Quick fix");

        let contract = store
            .authorize_direct_execution(work, b"Fix typo directly", "human", 200)
            .unwrap();
        assert_eq!(contract.kind, ContractKind::DirectExecution);
        assert_eq!(contract.revision_id, None);
        assert_eq!(
            store.current_contract(work).unwrap(),
            Some(contract.clone())
        );

        // Tampering with the referenced object blocks subsequent authorization.
        let mut obj_dir = db.as_os_str().to_owned();
        obj_dir.push(".objects");
        std::fs::write(
            std::path::PathBuf::from(obj_dir).join(contract.document_hash.as_str()),
            b"corrupted",
        )
        .unwrap();
        let rev = store.record_plan_revision(work, "v1", b"# Replan").unwrap();
        assert!(matches!(
            store.approve_plan_revision(work, rev.id, "human", 201),
            Err(RapError::DocumentIntegrity(_))
        ));
    }
}
