use rusqlite::{Connection, OptionalExtension, Row, params};
use serde::{Deserialize, Serialize};

use super::{
    ChangeRequest, ContractId, DocumentHash, EvidenceKind, RapError, RapStore, ResultId,
    RevisionId, WorkId,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResultStage {
    Interim,
    Final,
}

impl ResultStage {
    fn as_str(self) -> &'static str {
        match self {
            Self::Interim => "interim",
            Self::Final => "final",
        }
    }

    fn parse(value: &str) -> rusqlite::Result<Self> {
        match value {
            "interim" => Ok(Self::Interim),
            "final" => Ok(Self::Final),
            _ => Err(rusqlite::Error::InvalidColumnType(
                4,
                "stage".to_owned(),
                rusqlite::types::Type::Text,
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResultLink {
    pub label: String,
    pub url: String,
}

#[derive(Debug, Clone)]
pub struct NewResult<'a> {
    pub work_id: WorkId,
    pub contract_id: ContractId,
    pub stage: ResultStage,
    pub title: &'a str,
    pub summary: &'a str,
    pub links: &'a [ResultLink],
    pub artifact: Option<(&'a str, &'a [u8])>,
    pub now: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RapResult {
    pub id: ResultId,
    pub work_id: WorkId,
    pub contract_id: ContractId,
    pub revision_id: Option<RevisionId>,
    pub stage: ResultStage,
    pub title: String,
    pub summary: String,
    pub links: Vec<ResultLink>,
    pub document_hash: Option<DocumentHash>,
    pub media_type: Option<String>,
    pub created_at: i64,
}

impl RapStore {
    /// Records an interim or final result with optional document/image artifact and links.
    /// Final results also record `EvidenceKind::FinalResult` in the same transaction so
    /// completion verification sees the published final artifact atomically.
    pub fn record_result(&self, input: NewResult<'_>) -> Result<RapResult, RapError> {
        if input.title.trim().is_empty() || input.summary.trim().is_empty() {
            return Err(RapError::InvalidInput(
                "result title and summary are required",
            ));
        }
        let links_json = serde_json::to_string(input.links)
            .map_err(|_| RapError::InvalidInput("invalid result links"))?;
        self.write(|tx| {
            let revision_id: Option<i64> = tx
                .query_row(
                    "SELECT revision_id FROM execution_contracts WHERE id = ?1 AND work_id = ?2",
                    params![input.contract_id.0, input.work_id.0],
                    |row| row.get(0),
                )
                .optional()?
                .ok_or(RapError::NotFound("execution contract"))?;

            let (hash, media_type) = match input.artifact {
                Some((mime, bytes)) => {
                    if mime.trim().is_empty() {
                        return Err(RapError::InvalidInput("artifact media type is required"));
                    }
                    let hash = self.persist_document(tx, input.work_id, bytes)?;
                    (Some(hash), Some(mime.to_owned()))
                }
                None => (None, None),
            };

            let id: i64 = tx.query_row(
                "INSERT INTO rap_results
                    (work_id, contract_id, revision_id, stage, title, summary, links_json,
                     document_hash, media_type, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10) RETURNING id",
                params![
                    input.work_id.0,
                    input.contract_id.0,
                    revision_id,
                    input.stage.as_str(),
                    input.title,
                    input.summary,
                    links_json,
                    hash.as_ref().map(DocumentHash::as_str),
                    media_type.as_deref(),
                    input.now,
                ],
                |row| row.get(0),
            )?;

            if input.stage == ResultStage::Final {
                tx.execute(
                    "INSERT INTO completion_evidence
                        (work_id, contract_id, kind, summary, document_hash, recorded_at)
                     VALUES (?1, ?2, 'final_result', ?3, ?4, ?5)",
                    params![
                        input.work_id.0,
                        input.contract_id.0,
                        input.summary,
                        hash.as_ref().map(DocumentHash::as_str),
                        input.now,
                    ],
                )?;
            }

            fetch_result(tx, ResultId(id))
        })
    }

    /// Lists all results for a work item across every revision (oldest to newest).
    pub fn list_results(&self, work: WorkId) -> Result<Vec<RapResult>, RapError> {
        self.read(|connection| {
            let mut stmt = connection.prepare(
                "SELECT id, work_id, contract_id, revision_id, stage, title, summary,
                        links_json, document_hash, media_type, created_at
                 FROM rap_results WHERE work_id = ?1 ORDER BY id",
            )?;
            let rows = stmt.query_map([work.0], read_result)?;
            Ok(rows.collect::<rusqlite::Result<_>>()?)
        })
    }

    /// Files a revision/change request directly from a published result against the same WorkItem.
    pub fn request_revision_from_result(
        &self,
        result_id: ResultId,
        body: &str,
        now: i64,
    ) -> Result<ChangeRequest, RapError> {
        let result = self.read(|connection| fetch_result(connection, result_id))?;
        let annotated = format!("[result #{}] {}", result.id.0, body);
        self.receive_change_request(result.work_id, &annotated, now)
    }

    /// Lists verification and final-result evidence recorded for a work item.
    pub fn list_evidence(&self, work: WorkId) -> Result<Vec<super::Evidence>, RapError> {
        self.read(|connection| {
            let mut stmt = connection.prepare(
                "SELECT id, work_id, contract_id, kind, summary, document_hash, recorded_at
                 FROM completion_evidence WHERE work_id = ?1 ORDER BY id",
            )?;
            let rows = stmt.query_map([work.0], super::completion::read_evidence)?;
            Ok(rows.collect::<rusqlite::Result<_>>()?)
        })
    }

    /// Helper used by tests and API to record verification evidence alongside results.
    pub fn record_verification_result(
        &self,
        work: WorkId,
        contract: ContractId,
        summary: &str,
        now: i64,
    ) -> Result<super::Evidence, RapError> {
        self.record_evidence(
            work,
            contract,
            EvidenceKind::Verification,
            summary,
            None,
            now,
        )
    }
}

fn fetch_result(connection: &Connection, id: ResultId) -> Result<RapResult, RapError> {
    connection
        .query_row(
            "SELECT id, work_id, contract_id, revision_id, stage, title, summary,
                    links_json, document_hash, media_type, created_at
             FROM rap_results WHERE id = ?1",
            [id.0],
            read_result,
        )
        .optional()?
        .ok_or(RapError::NotFound("result"))
}

fn read_result(row: &Row<'_>) -> rusqlite::Result<RapResult> {
    let raw_stage: String = row.get(4)?;
    let links_json: String = row.get(7)?;
    let links: Vec<ResultLink> = serde_json::from_str(&links_json).map_err(|_| {
        rusqlite::Error::InvalidColumnType(7, "links_json".to_owned(), rusqlite::types::Type::Text)
    })?;
    let document_hash = match row.get::<_, Option<String>>(8)? {
        Some(raw) => Some(DocumentHash::try_from(raw.as_str()).map_err(|_| {
            rusqlite::Error::InvalidColumnType(
                8,
                "document_hash".to_owned(),
                rusqlite::types::Type::Text,
            )
        })?),
        None => None,
    };
    Ok(RapResult {
        id: ResultId(row.get(0)?),
        work_id: WorkId(row.get(1)?),
        contract_id: ContractId(row.get(2)?),
        revision_id: row.get::<_, Option<i64>>(3)?.map(RevisionId),
        stage: ResultStage::parse(&raw_stage)?,
        title: row.get(5)?,
        summary: row.get(6)?,
        links,
        document_hash,
        media_type: row.get(9)?,
        created_at: row.get(10)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rap::{BoardLane, NewWork, WorkKind, WorkPhase};

    #[test]
    fn preserves_old_revision_results_couples_final_result_to_completion_and_requests_revision() {
        let root = tempfile::tempdir().unwrap();
        let store = RapStore::open(&root.path().join("rap.db")).unwrap();
        let work = store
            .create_work(NewWork {
                project_id: "demo",
                workspace: root.path(),
                kind: WorkKind::Task,
                parent_epic_id: None,
                title: "UI polish",
                rank: 1,
            })
            .unwrap()
            .id;

        let v1 = store
            .record_plan_revision(work, "v1", b"# Plan v1")
            .unwrap();
        let c1 = store
            .approve_plan_revision(work, v1.id, "human", 10)
            .unwrap();

        // Interim image/link result + final result + verification -> completes work!
        let interim = store
            .record_result(NewResult {
                work_id: work,
                contract_id: c1.id,
                stage: ResultStage::Interim,
                title: "Screenshot v1",
                summary: "Phone layout draft",
                links: &[ResultLink {
                    label: "Preview".into(),
                    url: "https://example.test/v1".into(),
                }],
                artifact: Some(("image/png", b"\x89PNG-fake")),
                now: 11,
            })
            .unwrap();
        assert_eq!(interim.media_type.as_deref(), Some("image/png"));
        assert!(interim.document_hash.is_some());

        let final_v1 = store
            .record_result(NewResult {
                work_id: work,
                contract_id: c1.id,
                stage: ResultStage::Final,
                title: "Final v1",
                summary: "Implemented and verified",
                links: &[],
                artifact: Some(("text/markdown", b"# Report v1")),
                now: 12,
            })
            .unwrap();
        store
            .record_verification_result(work, c1.id, "cargo nextest passed", 12)
            .unwrap();
        let done = store.complete_work(work).unwrap();
        assert_eq!(done.lane, BoardLane::Done);

        // Request revision directly from final_v1 -> reopens same WorkItem and preserves v1 results.
        let req = store
            .request_revision_from_result(final_v1.id, "Increase contrast on dark mode", 20)
            .unwrap();
        assert_eq!(req.work_id, work);
        assert_eq!(store.work(work).unwrap().phase, WorkPhase::Revising);

        let v2 = store
            .record_plan_revision(work, "v2", b"# Plan v2")
            .unwrap();
        let c2 = store
            .approve_plan_revision(work, v2.id, "human", 21)
            .unwrap();
        store
            .record_result(NewResult {
                work_id: work,
                contract_id: c2.id,
                stage: ResultStage::Final,
                title: "Final v2",
                summary: "Contrast updated",
                links: &[],
                artifact: None,
                now: 22,
            })
            .unwrap();
        let all = store.list_results(work).unwrap();
        assert_eq!(all.len(), 3);
        assert_eq!(all[0].revision_id, Some(v1.id));
        assert_eq!(all[1].revision_id, Some(v1.id));
        assert_eq!(all[2].revision_id, Some(v2.id));
    }
}
