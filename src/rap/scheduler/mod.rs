//! Transactional scheduler for authorized work in local workspaces.

mod claim;
mod lifecycle;
pub mod runner;
mod transitions;
mod workspace;

pub use lifecycle::{InFlight, ProcessIdentity};
pub use runner::{
    AdapterError, LaunchHandle, LaunchRequest, ProbeState, ProviderAdapter, RapRunner,
    ReconcileReport, parse_rate_limit_until,
};
pub use transitions::StoppedOutcome;

use std::num::NonZeroU32;
use std::path::PathBuf;

use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};

use super::{AttemptId, RapError, RapStore, SessionId, SessionPhase, WorkId};

pub(crate) const MIGRATION: &str = include_str!("schema.sql");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Provider {
    Codex,
    ClaudeCode,
}

impl Provider {
    fn as_str(self) -> &'static str {
        match self {
            Self::Codex => "codex",
            Self::ClaudeCode => "claude_code",
        }
    }

    fn parse(value: &str) -> Result<Self, RapError> {
        match value {
            "codex" => Ok(Self::Codex),
            "claude_code" => Ok(Self::ClaudeCode),
            _ => Err(RapError::InvalidInput("unrecognized scheduler provider")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchedulerSettings {
    pub auto_run: bool,
    pub max_sessions: NonZeroU32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Claim {
    pub work_id: WorkId,
    pub session_id: SessionId,
    pub attempt_id: AttemptId,
    pub epoch: i64,
    pub phase: SessionPhase,
    pub provider: Provider,
    pub workspace: PathBuf,
    pub contract_id: Option<i64>,
    pub native_id: Option<String>,
}

/// Scheduler settings and execution leases, stored in the RAP database.
#[derive(Debug, Clone)]
pub struct SchedulerStore {
    store: RapStore,
}

impl SchedulerStore {
    pub fn new(store: RapStore) -> Self {
        Self { store }
    }

    pub fn settings(&self) -> Result<SchedulerSettings, RapError> {
        self.store.read(|connection| {
            let settings = connection.query_row(
                "SELECT auto_run, max_sessions FROM scheduler_settings WHERE id = 1",
                [],
                |row| Ok((row.get(0)?, row.get::<_, u32>(1)?)),
            )?;
            Ok(SchedulerSettings {
                auto_run: settings.0,
                max_sessions: NonZeroU32::new(settings.1)
                    .ok_or(RapError::InvalidInput("execution limit must be positive"))?,
            })
        })
    }

    pub fn set_settings(&self, settings: SchedulerSettings) -> Result<(), RapError> {
        self.store.write(|tx| {
            tx.execute(
                "UPDATE scheduler_settings SET auto_run = ?1, max_sessions = ?2 WHERE id = 1",
                params![settings.auto_run, settings.max_sessions.get()],
            )?;
            Ok(())
        })
    }

    /// Returns the active wait reason and optional retry timestamp for a work item.
    pub fn waiting_reason(
        &self,
        work_id: WorkId,
    ) -> Result<Option<(String, Option<i64>)>, RapError> {
        self.store.read(|connection| {
            let row = connection
                .query_row(
                    "SELECT waiting_reason, retry_at FROM scheduler_work WHERE work_id = ?1",
                    [work_id.0],
                    |row| {
                        Ok((
                            row.get::<_, Option<String>>(0)?,
                            row.get::<_, Option<i64>>(1)?,
                        ))
                    },
                )
                .optional()?;
            Ok(row.and_then(|(reason, retry_at)| reason.map(|r| (r, retry_at))))
        })
    }

    /// Sets the provider and grants or withdraws permission to run a planning
    /// session. Execution has its own authorization in the current contract.
    pub fn configure_work(
        &self,
        work_id: WorkId,
        provider: Provider,
        planning_authorized: bool,
    ) -> Result<(), RapError> {
        self.store.write(|tx| {
            let parent = tx
                .query_row(
                    "SELECT parent_epic_id FROM work_items WHERE id = ?1",
                    [work_id.0],
                    |row| row.get::<_, Option<i64>>(0),
                )
                .optional()?
                .ok_or(RapError::NotFound("work item"))?;
            if parent.is_some() {
                return Err(RapError::InvalidInput(
                    "configure the parent epic to schedule a child task",
                ));
            }
            let original_provider = tx
                .query_row(
                    "SELECT provider FROM sessions WHERE work_id = ?1 ORDER BY id LIMIT 1",
                    [work_id.0],
                    |row| row.get::<_, String>(0),
                )
                .optional()?;
            if original_provider.is_some_and(|original| original != provider.as_str()) {
                return Err(RapError::Conflict("changing the provider of started work"));
            }
            tx.execute(
                "INSERT INTO scheduler_work (work_id, provider, planning_authorized)
                 VALUES (?1, ?2, ?3)
                 ON CONFLICT(work_id) DO UPDATE
                 SET provider = excluded.provider, planning_authorized = excluded.planning_authorized",
                params![work_id.0, provider.as_str(), planning_authorized],
            )?;
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::num::NonZeroU32;
    use std::path::Path;

    use super::*;
    use crate::rap::{BoardLane, ContractId, EvidenceKind, NewWork, WorkKind, WorkPhase};

    fn create_item(
        store: &RapStore,
        workspace: &Path,
        kind: WorkKind,
        parent_epic_id: Option<WorkId>,
        title: &str,
        rank: i64,
    ) -> WorkId {
        store
            .create_work(NewWork {
                project_id: "demo",
                workspace,
                kind,
                parent_epic_id,
                title,
                rank,
            })
            .unwrap()
            .id
    }

    /// Minimal fake adapter that runs a claimed attempt through launch -> process -> stop.
    fn run_fake_execution(
        store: &RapStore,
        scheduler: &SchedulerStore,
        claim: &Claim,
        pid: u32,
        now: i64,
    ) {
        assert!(scheduler.begin_launch(claim).unwrap());
        let proc = ProcessIdentity {
            pid,
            fingerprint: format!("fake-{pid}"),
        };
        scheduler
            .record_process(claim, &proc, Some(&format!("native-{pid}")))
            .unwrap();
        scheduler
            .confirm_stopped(claim, StoppedOutcome::Verify)
            .unwrap();
        let contract = ContractId(claim.contract_id.expect("execution contract"));
        store
            .record_evidence(
                claim.work_id,
                contract,
                EvidenceKind::Verification,
                "fake adapter tests passed",
                None,
                now,
            )
            .unwrap();
        store
            .record_evidence(
                claim.work_id,
                contract,
                EvidenceKind::FinalResult,
                "fake adapter final result",
                None,
                now,
            )
            .unwrap();
        let done = store.complete_work(claim.work_id).unwrap();
        assert_eq!(done.lane, BoardLane::Done);
    }

    #[test]
    fn fake_adapter_runs_standalone_tasks_in_order_and_scopes_children_to_epic() {
        let root = tempfile::tempdir().unwrap();
        let ws = root.path().join("repo");
        fs::create_dir(&ws).unwrap();
        let store = RapStore::open(&root.path().join("rap.db")).unwrap();
        let scheduler = SchedulerStore::new(store.clone());

        // Default settings: auto_run = false, max_sessions = 1.
        let defaults = scheduler.settings().unwrap();
        assert!(!defaults.auto_run);
        assert_eq!(defaults.max_sessions.get(), 1);

        let t1 = create_item(&store, &ws, WorkKind::Task, None, "First", 1);
        let t2 = create_item(&store, &ws, WorkKind::Task, None, "Second", 2);
        let epic = create_item(&store, &ws, WorkKind::Epic, None, "Epic", 3);
        let child = create_item(&store, &ws, WorkKind::Task, Some(epic), "Child", 0);

        // Child tasks cannot be scheduled directly; they belong to the parent epic.
        assert!(matches!(
            scheduler.configure_work(child, Provider::Codex, true),
            Err(RapError::InvalidInput(_))
        ));
        scheduler
            .configure_work(t1, Provider::Codex, false)
            .unwrap();
        scheduler
            .configure_work(t2, Provider::Codex, false)
            .unwrap();
        scheduler
            .configure_work(epic, Provider::Codex, false)
            .unwrap();

        // Unauthorized work is never claimed even when auto_run is on.
        scheduler
            .set_settings(SchedulerSettings {
                auto_run: true,
                max_sessions: NonZeroU32::new(1).unwrap(),
            })
            .unwrap();
        assert_eq!(scheduler.claim_next(10, &[Provider::Codex]).unwrap(), None);

        // Authorize t1, t2, and epic.
        store
            .authorize_direct_execution(t1, b"do t1", "human", 10)
            .unwrap();
        store
            .authorize_direct_execution(t2, b"do t2", "human", 10)
            .unwrap();
        store
            .authorize_direct_execution(epic, b"do epic", "human", 10)
            .unwrap();

        // Claims proceed in rank order: t1 -> t2 -> epic (never child).
        let c1 = scheduler
            .claim_next(11, &[Provider::Codex])
            .unwrap()
            .unwrap();
        assert_eq!(c1.work_id, t1);
        // Same workspace cannot be claimed concurrently.
        assert_eq!(scheduler.claim_next(11, &[Provider::Codex]).unwrap(), None);
        run_fake_execution(&store, &scheduler, &c1, 1001, 12);

        let c2 = scheduler
            .claim_next(13, &[Provider::Codex])
            .unwrap()
            .unwrap();
        assert_eq!(c2.work_id, t2);
        run_fake_execution(&store, &scheduler, &c2, 1002, 14);

        let c3 = scheduler
            .claim_next(15, &[Provider::Codex])
            .unwrap()
            .unwrap();
        assert_eq!(c3.work_id, epic);
        run_fake_execution(&store, &scheduler, &c3, 1003, 16);

        assert_eq!(scheduler.claim_next(17, &[Provider::Codex]).unwrap(), None);
    }

    #[test]
    fn enforces_slot_limits_workspace_exclusion_unknown_hold_and_awaiting_input_pause() {
        let root = tempfile::tempdir().unwrap();
        let ws_a = root.path().join("ws-a");
        let ws_b = root.path().join("ws-b");
        fs::create_dir(&ws_a).unwrap();
        fs::create_dir(&ws_b).unwrap();
        let store = RapStore::open(&root.path().join("rap.db")).unwrap();
        let scheduler = SchedulerStore::new(store.clone());
        scheduler
            .set_settings(SchedulerSettings {
                auto_run: true,
                max_sessions: NonZeroU32::new(2).unwrap(),
            })
            .unwrap();

        let a1 = create_item(&store, &ws_a, WorkKind::Task, None, "A1", 1);
        let a2 = create_item(&store, &ws_a, WorkKind::Task, None, "A2", 2);
        let b1 = create_item(&store, &ws_b, WorkKind::Task, None, "B1", 3);
        for id in [a1, a2, b1] {
            scheduler
                .configure_work(id, Provider::Codex, false)
                .unwrap();
            store
                .authorize_direct_execution(id, b"brief", "human", 10)
                .unwrap();
        }

        // Independent workspaces A and B run concurrently up to max_sessions = 2.
        let ca1 = scheduler
            .claim_next(11, &[Provider::Codex])
            .unwrap()
            .unwrap();
        assert_eq!(ca1.work_id, a1);
        // Same workspace A2 is skipped; independent workspace B1 is claimed.
        let cb1 = scheduler
            .claim_next(11, &[Provider::Codex])
            .unwrap()
            .unwrap();
        assert_eq!(cb1.work_id, b1);
        assert_eq!(scheduler.in_flight().unwrap().len(), 2);

        // Start both attempts.
        assert!(scheduler.begin_launch(&ca1).unwrap());
        assert!(scheduler.begin_launch(&cb1).unwrap());
        scheduler
            .record_process(
                &ca1,
                &ProcessIdentity {
                    pid: 2001,
                    fingerprint: "a1".into(),
                },
                Some("sess-a1"),
            )
            .unwrap();
        scheduler
            .record_process(
                &cb1,
                &ProcessIdentity {
                    pid: 2002,
                    fingerprint: "b1".into(),
                },
                Some("sess-b1"),
            )
            .unwrap();

        // Unknown stop state on B1 still holds its session slot.
        scheduler.mark_unknown(&cb1).unwrap();
        assert_eq!(scheduler.in_flight().unwrap().len(), 2);

        // Safe question pause (AwaitingInput) on A1 frees the session slot
        // while retaining ws-a ownership, so A2 still cannot run.
        scheduler
            .confirm_stopped(&ca1, StoppedOutcome::AwaitingInput)
            .unwrap();
        assert_eq!(scheduler.in_flight().unwrap().len(), 1);
        assert_eq!(store.work(a1).unwrap().phase, WorkPhase::Ready);
        assert_eq!(scheduler.claim_next(12, &[Provider::Codex]).unwrap(), None);

        // Another independent workspace C can use the freed slot while A1 waits!
        let ws_c = root.path().join("ws-c");
        fs::create_dir(&ws_c).unwrap();
        let c1 = create_item(&store, &ws_c, WorkKind::Task, None, "C1", 4);
        scheduler
            .configure_work(c1, Provider::Codex, false)
            .unwrap();
        store
            .authorize_direct_execution(c1, b"brief c", "human", 12)
            .unwrap();
        let cc1 = scheduler
            .claim_next(13, &[Provider::Codex])
            .unwrap()
            .unwrap();
        assert_eq!(cc1.work_id, c1);

        // Resuming A1 after freeing C1 reuses A1's session and native_id.
        assert!(scheduler.begin_launch(&cc1).unwrap());
        scheduler
            .confirm_stopped(&cc1, StoppedOutcome::Verify)
            .unwrap();
        scheduler.resume_work(a1).unwrap();
        let resumed = scheduler
            .claim_next(14, &[Provider::Codex])
            .unwrap()
            .unwrap();
        assert_eq!(resumed.work_id, a1);
        assert_eq!(resumed.session_id, ca1.session_id);
        assert_eq!(resumed.epoch, 2);
        assert_eq!(resumed.native_id.as_deref(), Some("sess-a1"));
    }
}
