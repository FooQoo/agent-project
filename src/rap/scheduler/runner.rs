use serde_json::Value;

use super::{Claim, ProcessIdentity, Provider, SchedulerStore, StoppedOutcome};
use crate::rap::{ContractId, ContractKind, RapError, RapStore, SessionPhase};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchRequest {
    pub claim: Claim,
    pub contract_document: Option<Vec<u8>>,
    pub is_direct_execution: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchHandle {
    pub process: ProcessIdentity,
    pub native_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdapterError {
    RateLimited { until: i64 },
    LaunchUncertain,
    Failed(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProbeState {
    Running {
        process: ProcessIdentity,
        native_id: Option<String>,
    },
    Stopped {
        outcome: StoppedOutcome,
        plan_bytes: Option<Vec<u8>>,
    },
    Unknown,
}

pub trait ProviderAdapter {
    fn start_session(&self, request: &LaunchRequest) -> Result<LaunchHandle, AdapterError>;
    fn probe_session(&self, claim: &Claim, process: Option<&ProcessIdentity>) -> ProbeState;
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ReconcileReport {
    pub released_unstarted: usize,
    pub still_running: usize,
    pub confirmed_stopped: usize,
    pub marked_unknown: usize,
}

/// Coordinates atomic claims, provider launches, and crash/event-loss recovery.
pub struct RapRunner<'a> {
    store: &'a RapStore,
    scheduler: &'a SchedulerStore,
    adapter: &'a dyn ProviderAdapter,
}

impl<'a> RapRunner<'a> {
    pub fn new(
        store: &'a RapStore,
        scheduler: &'a SchedulerStore,
        adapter: &'a dyn ProviderAdapter,
    ) -> Self {
        Self {
            store,
            scheduler,
            adapter,
        }
    }

    /// Claims one ready work item, verifies its contract document if executing,
    /// and launches a fresh planning or execution session through the adapter.
    pub fn dispatch_next(
        &self,
        now: i64,
        available_providers: &[Provider],
    ) -> Result<Option<Claim>, RapError> {
        let Some(claim) = self.scheduler.claim_next(now, available_providers)? else {
            return Ok(None);
        };

        let (contract_document, is_direct_execution) = match claim.phase {
            SessionPhase::Planning => (None, false),
            SessionPhase::Execution => {
                let contract_id = claim
                    .contract_id
                    .map(ContractId)
                    .ok_or(RapError::Conflict("execution claim lacks contract"))?;
                let contract = self
                    .store
                    .current_contract(claim.work_id)?
                    .filter(|c| c.id == contract_id)
                    .ok_or(RapError::Conflict(
                        "execution contract is no longer current",
                    ))?;
                let bytes = self
                    .store
                    .read_document(claim.work_id, &contract.document_hash)?;
                (Some(bytes), contract.kind == ContractKind::DirectExecution)
            }
        };

        if !self.scheduler.begin_launch(&claim)? {
            return Ok(None);
        }

        let request = LaunchRequest {
            claim: claim.clone(),
            contract_document,
            is_direct_execution,
        };
        match self.adapter.start_session(&request) {
            Ok(handle) => {
                self.scheduler.record_process(
                    &claim,
                    &handle.process,
                    handle.native_id.as_deref(),
                )?;
                Ok(Some(claim))
            }
            Err(AdapterError::RateLimited { until }) => {
                self.scheduler
                    .confirm_stopped(&claim, StoppedOutcome::RateLimited { until })?;
                Ok(None)
            }
            Err(AdapterError::LaunchUncertain) => {
                self.scheduler.mark_unknown(&claim)?;
                Ok(None)
            }
            Err(AdapterError::Failed(_)) => {
                self.scheduler
                    .confirm_stopped(&claim, StoppedOutcome::Retry { now })?;
                Ok(None)
            }
        }
    }

    /// Reconciles every in-flight dispatch against the adapter to recover from
    /// crashes before/after launch and from missed exit events.
    pub fn reconcile(&self) -> Result<ReconcileReport, RapError> {
        let mut report = ReconcileReport::default();
        for entry in self.scheduler.in_flight()? {
            if !entry.started {
                self.scheduler.release_unstarted(&entry.claim)?;
                report.released_unstarted += 1;
                continue;
            }
            match self
                .adapter
                .probe_session(&entry.claim, entry.process.as_ref())
            {
                ProbeState::Running { process, native_id } => {
                    self.scheduler
                        .record_process(&entry.claim, &process, native_id.as_deref())?;
                    report.still_running += 1;
                }
                ProbeState::Stopped {
                    outcome,
                    plan_bytes,
                } => {
                    self.scheduler.confirm_stopped(&entry.claim, outcome)?;
                    if outcome == StoppedOutcome::PlanReady
                        && let Some(bytes) = plan_bytes
                    {
                        self.store.record_plan_revision(
                            entry.claim.work_id,
                            "captured plan revision",
                            &bytes,
                        )?;
                    }
                    report.confirmed_stopped += 1;
                }
                ProbeState::Unknown => {
                    self.scheduler.mark_unknown(&entry.claim)?;
                    report.marked_unknown += 1;
                }
            }
        }
        Ok(report)
    }
}

/// Extracts a rate-limit reset epoch (seconds) from a provider JSONL event line.
pub fn parse_rate_limit_until(provider: Provider, line: &str, fallback_until: i64) -> Option<i64> {
    let value: Value = serde_json::from_str(line).ok()?;
    match provider {
        Provider::ClaudeCode => {
            if value.get("type").and_then(Value::as_str) == Some("rate_limit_event") {
                let info = value.get("rate_limit_info")?;
                if info.get("status").and_then(Value::as_str) == Some("rejected") {
                    return info
                        .get("resetsAt")
                        .and_then(Value::as_i64)
                        .or(Some(fallback_until));
                }
            }
            None
        }
        Provider::Codex => {
            if value.get("type").and_then(Value::as_str) == Some("turn.failed") {
                let message = value
                    .get("error")
                    .and_then(|e| e.get("message"))
                    .and_then(Value::as_str)?;
                if message.to_ascii_lowercase().contains("usage limit") {
                    return Some(fallback_until);
                }
            }
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::fs;
    use std::num::NonZeroU32;

    use super::*;
    use crate::rap::scheduler::SchedulerSettings;
    use crate::rap::{NewWork, WorkId, WorkKind, WorkPhase};

    struct FakeAdapter {
        launches: RefCell<Vec<LaunchRequest>>,
        next_start: RefCell<Result<LaunchHandle, AdapterError>>,
        probes: RefCell<HashMap<i64, ProbeState>>,
    }

    impl FakeAdapter {
        fn new() -> Self {
            Self {
                launches: RefCell::new(Vec::new()),
                next_start: RefCell::new(Ok(LaunchHandle {
                    process: ProcessIdentity {
                        pid: 4000,
                        fingerprint: "fp-4000".into(),
                    },
                    native_id: Some("native-1".into()),
                })),
                probes: RefCell::new(HashMap::new()),
            }
        }
    }

    impl ProviderAdapter for FakeAdapter {
        fn start_session(&self, request: &LaunchRequest) -> Result<LaunchHandle, AdapterError> {
            self.launches.borrow_mut().push(request.clone());
            self.next_start.borrow().clone()
        }

        fn probe_session(&self, claim: &Claim, _process: Option<&ProcessIdentity>) -> ProbeState {
            self.probes
                .borrow()
                .get(&claim.attempt_id.0)
                .cloned()
                .unwrap_or(ProbeState::Unknown)
        }
    }

    fn setup() -> (tempfile::TempDir, RapStore, SchedulerStore, WorkId) {
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
                title: "Build feature",
                rank: 1,
            })
            .unwrap()
            .id;
        (root, store, scheduler, work)
    }

    #[test]
    fn hands_off_from_stopped_planning_session_to_separate_execution_session() {
        let (_root, store, scheduler, work) = setup();
        scheduler
            .configure_work(work, Provider::Codex, true)
            .unwrap();
        let adapter = FakeAdapter::new();
        let runner = RapRunner::new(&store, &scheduler, &adapter);

        // 1. Dispatch planning session.
        let plan_claim = runner
            .dispatch_next(10, &[Provider::Codex])
            .unwrap()
            .unwrap();
        assert_eq!(plan_claim.phase, SessionPhase::Planning);

        // 2. Planning stops cleanly and produces plan text -> reconcile records revision.
        adapter.probes.borrow_mut().insert(
            plan_claim.attempt_id.0,
            ProbeState::Stopped {
                outcome: StoppedOutcome::PlanReady,
                plan_bytes: Some(b"# Approved Plan".to_vec()),
            },
        );
        let report = runner.reconcile().unwrap();
        assert_eq!(report.confirmed_stopped, 1);
        let item = store.work(work).unwrap();
        assert_eq!(item.phase, WorkPhase::AwaitingApproval);
        let rev_id = item.current_revision_id.expect("plan revision recorded");

        // 3. Human approves the revision -> separate execution session starts with contract bytes.
        store
            .approve_plan_revision(work, rev_id, "human", 20)
            .unwrap();
        *adapter.next_start.borrow_mut() = Ok(LaunchHandle {
            process: ProcessIdentity {
                pid: 4001,
                fingerprint: "fp-4001".into(),
            },
            native_id: Some("native-exec".into()),
        });
        let exec_claim = runner
            .dispatch_next(21, &[Provider::Codex])
            .unwrap()
            .unwrap();
        assert_eq!(exec_claim.phase, SessionPhase::Execution);
        assert_ne!(exec_claim.session_id, plan_claim.session_id);
        let launches = adapter.launches.borrow();
        assert_eq!(
            launches[1].contract_document.as_deref(),
            Some(&b"# Approved Plan"[..])
        );
        assert!(!launches[1].is_direct_execution);
    }

    #[test]
    fn direct_execution_skips_planning_session() {
        let (_root, store, scheduler, work) = setup();
        scheduler
            .configure_work(work, Provider::ClaudeCode, false)
            .unwrap();
        store
            .authorize_direct_execution(work, b"Direct brief", "human", 10)
            .unwrap();
        let adapter = FakeAdapter::new();
        let runner = RapRunner::new(&store, &scheduler, &adapter);

        let claim = runner
            .dispatch_next(11, &[Provider::ClaudeCode])
            .unwrap()
            .unwrap();
        assert_eq!(claim.phase, SessionPhase::Execution);
        assert_eq!(store.sessions(work).unwrap().len(), 1);
        assert!(adapter.launches.borrow()[0].is_direct_execution);
    }

    #[test]
    fn recovers_from_pre_and_post_launch_crashes_rate_limits_and_missed_events() {
        let (_root, store, scheduler, work) = setup();
        scheduler
            .configure_work(work, Provider::ClaudeCode, false)
            .unwrap();
        store
            .authorize_direct_execution(work, b"Recoverable task", "human", 10)
            .unwrap();
        let adapter = FakeAdapter::new();
        let runner = RapRunner::new(&store, &scheduler, &adapter);

        // Crash before begin_launch (reserved) -> reconcile releases without double launch.
        let unstarted = scheduler
            .claim_next(11, &[Provider::ClaudeCode])
            .unwrap()
            .unwrap();
        assert_eq!(runner.reconcile().unwrap().released_unstarted, 1);
        assert!(
            scheduler
                .confirm_stopped(&unstarted, StoppedOutcome::Verify)
                .is_err()
        );

        // Uncertain launch -> stays unknown and blocks second launch until probe confirms stopped.
        *adapter.next_start.borrow_mut() = Err(AdapterError::LaunchUncertain);
        assert_eq!(
            runner.dispatch_next(12, &[Provider::ClaudeCode]).unwrap(),
            None
        );
        assert_eq!(scheduler.in_flight().unwrap().len(), 1);
        assert_eq!(
            runner.dispatch_next(13, &[Provider::ClaudeCode]).unwrap(),
            None
        );
        let unknown_claim = scheduler.in_flight().unwrap()[0].claim.clone();
        assert_eq!(runner.reconcile().unwrap().marked_unknown, 1);

        // Probe now confirms missed exit -> recovers to verifying!
        adapter.probes.borrow_mut().insert(
            unknown_claim.attempt_id.0,
            ProbeState::Stopped {
                outcome: StoppedOutcome::Verify,
                plan_bytes: None,
            },
        );
        assert_eq!(runner.reconcile().unwrap().confirmed_stopped, 1);
        assert_eq!(store.work(work).unwrap().phase, WorkPhase::Verifying);

        // Real Claude rate-limit event parsing and wait-reason tracking.
        let claude_line = r#"{"type":"rate_limit_event","rate_limit_info":{"status":"rejected","resetsAt":1795834800}}"#;
        let until = parse_rate_limit_until(Provider::ClaudeCode, claude_line, 999).unwrap();
        assert_eq!(until, 1795834800);
        let codex_line =
            r#"{"type":"turn.failed","error":{"message":"You’ve hit your usage limit."}}"#;
        assert_eq!(
            parse_rate_limit_until(Provider::Codex, codex_line, 5000),
            Some(5000)
        );

        // Rate-limited launch records wait reason and blocks claims until reset time.
        let work2 = store
            .create_work(NewWork {
                project_id: "demo",
                workspace: &_root.path().join("ws"),
                kind: WorkKind::Task,
                parent_epic_id: None,
                title: "Rate limited task",
                rank: 2,
            })
            .unwrap()
            .id;
        scheduler
            .configure_work(work2, Provider::ClaudeCode, false)
            .unwrap();
        store
            .authorize_direct_execution(work2, b"brief 2", "human", 20)
            .unwrap();
        *adapter.next_start.borrow_mut() = Err(AdapterError::RateLimited { until });
        assert_eq!(
            runner.dispatch_next(21, &[Provider::ClaudeCode]).unwrap(),
            None
        );
        assert_eq!(
            scheduler.waiting_reason(work2).unwrap(),
            Some(("rate_limited".to_owned(), Some(until)))
        );
        // Blocked before reset time, allowed at reset time.
        *adapter.next_start.borrow_mut() = Ok(LaunchHandle {
            process: ProcessIdentity {
                pid: 4002,
                fingerprint: "fp-4002".into(),
            },
            native_id: Some("native-after-limit".into()),
        });
        assert_eq!(
            runner
                .dispatch_next(until - 1, &[Provider::ClaudeCode])
                .unwrap(),
            None
        );
        let after = runner
            .dispatch_next(until, &[Provider::ClaudeCode])
            .unwrap()
            .unwrap();
        assert_eq!(after.work_id, work2);
    }
}
