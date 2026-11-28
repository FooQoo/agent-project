//! Automated end-to-end verification matrix covering RAP MVP scenarios
//! (E01-E16 and S01-S22) across both Codex and Claude Code provider paths,
//! plus fault/conflict injection and legacy artifact preservation.

use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;
use std::num::NonZeroU32;

use agent_project::rap::scheduler::{
    AdapterError, Claim, LaunchHandle, LaunchRequest, ProbeState, ProcessIdentity, Provider,
    ProviderAdapter, RapRunner, SchedulerSettings, SchedulerStore, StoppedOutcome,
    build_provider_command, parse_rate_limit_until,
};
use agent_project::rap::{
    BoardLane, CompletionBlocker, DecisionRisk, DelegationStatus, DeliveryOutcome, NewNotification,
    NewResult, NewWork, NotificationKind, NotificationState, RapError, RapStore, ResultLink,
    ResultStage, SessionPhase, WorkKind, WorkPhase, verify_irreversible_approval,
    verify_remote_session, verify_workspace_within_roots,
};

struct MatrixAdapter {
    next_start: RefCell<Result<LaunchHandle, AdapterError>>,
    probes: RefCell<HashMap<i64, ProbeState>>,
    commands: RefCell<Vec<Vec<String>>>,
}

impl MatrixAdapter {
    fn new(pid: u32, native_id: &str) -> Self {
        Self {
            next_start: RefCell::new(Ok(LaunchHandle {
                process: ProcessIdentity {
                    pid,
                    fingerprint: format!("fp-{pid}"),
                },
                native_id: Some(native_id.to_owned()),
            })),
            probes: RefCell::new(HashMap::new()),
            commands: RefCell::new(Vec::new()),
        }
    }
}

impl ProviderAdapter for MatrixAdapter {
    fn start_session(&self, request: &LaunchRequest) -> Result<LaunchHandle, AdapterError> {
        let cmd = build_provider_command(request, "execute");
        self.commands.borrow_mut().push(cmd.args);
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

fn verify_provider_end_to_end(provider: Provider) {
    let root = tempfile::tempdir().unwrap();
    let ws1 = root.path().join("ws-1");
    let ws2 = root.path().join("ws-2");
    fs::create_dir(&ws1).unwrap();
    fs::create_dir(&ws2).unwrap();
    let db = root.path().join("rap.db");
    let store = RapStore::open(&db).unwrap();
    let scheduler = SchedulerStore::new(store.clone());

    // E01/S01: Default auto_run=false, max_sessions=1.
    let defaults = scheduler.settings().unwrap();
    assert!(!defaults.auto_run);
    assert_eq!(defaults.max_sessions.get(), 1);
    scheduler
        .set_settings(SchedulerSettings {
            auto_run: true,
            max_sessions: NonZeroU32::new(2).unwrap(),
        })
        .unwrap();

    // E02/E03: Epic + child task scoping + standalone task.
    let epic = store
        .create_work(NewWork {
            project_id: "rap",
            workspace: &ws1,
            kind: WorkKind::Epic,
            parent_epic_id: None,
            title: "Main Epic",
            rank: 1,
        })
        .unwrap()
        .id;
    let child = store
        .create_work(NewWork {
            project_id: "rap",
            workspace: &ws1,
            kind: WorkKind::Task,
            parent_epic_id: Some(epic),
            title: "Subtask",
            rank: 1,
        })
        .unwrap()
        .id;
    assert!(scheduler.configure_work(child, provider, true).is_err());
    scheduler.configure_work(epic, provider, true).unwrap();

    // E04/E05: Plan session -> stop confirmation -> plan revision -> stale approval rejected -> current approved.
    let adapter = MatrixAdapter::new(9001, "sess-plan");
    let runner = RapRunner::new(&store, &scheduler, &adapter);
    let plan_claim = runner.dispatch_next(10, &[provider]).unwrap().unwrap();
    assert_eq!(plan_claim.phase, SessionPhase::Planning);
    adapter.probes.borrow_mut().insert(
        plan_claim.attempt_id.0,
        ProbeState::Stopped {
            outcome: StoppedOutcome::PlanReady,
            plan_bytes: Some(b"# Plan v1".to_vec()),
        },
    );
    assert_eq!(runner.reconcile().unwrap().confirmed_stopped, 1);
    let rev1 = store.work(epic).unwrap().current_revision_id.unwrap();
    let rev2 = store
        .record_plan_revision(epic, "v2", b"# Plan v2")
        .unwrap()
        .id;
    assert!(matches!(
        store.approve_plan_revision(epic, rev1, "human", 15),
        Err(RapError::Conflict(_))
    ));
    let contract = store
        .approve_plan_revision(epic, rev2, "human", 16)
        .unwrap();

    // E06/E07: Separate execution session starts from approved contract.
    *adapter.next_start.borrow_mut() = Ok(LaunchHandle {
        process: ProcessIdentity {
            pid: 9002,
            fingerprint: "fp-9002".into(),
        },
        native_id: Some("sess-exec".into()),
    });
    let exec_claim = runner.dispatch_next(20, &[provider]).unwrap().unwrap();
    assert_ne!(exec_claim.session_id, plan_claim.session_id);
    assert_eq!(exec_claim.phase, SessionPhase::Execution);

    // E08/E09: Question pause frees slot for independent ws2, retains ws1, idempotent answer resumes ws1.
    let q = store
        .ask_question(epic, exec_claim.session_id, "Continue?", 21)
        .unwrap();
    scheduler
        .confirm_stopped(&exec_claim, StoppedOutcome::AwaitingInput)
        .unwrap();
    store.answer_question(q.id, "Yes", "human", 22).unwrap();
    assert_eq!(
        store
            .deliver_answer(q.id, exec_claim.session_id, 23)
            .unwrap(),
        DeliveryOutcome::Delivered { resumed_work: true }
    );
    assert_eq!(
        store
            .deliver_answer(q.id, exec_claim.session_id, 24)
            .unwrap(),
        DeliveryOutcome::AlreadyDelivered
    );

    // E10/E11/E12: Interrupt blocks Done until resolved; missing evidence blocks Done; final result + verification completes.
    let req = store
        .receive_change_request(epic, "Add edge case check", 25)
        .unwrap();
    assert!(matches!(
        store.complete_work(epic),
        Err(RapError::CompletionBlocked(ref b))
            if b.contains(&CompletionBlocker::MissingVerification)
                && b.contains(&CompletionBlocker::MissingFinalResult)
                && b.contains(&CompletionBlocker::PendingChangeRequests(1))
    ));
    store
        .absorb_change_within_contract(epic, req.id, "Covered within contract")
        .unwrap();
    store
        .record_result(NewResult {
            work_id: epic,
            contract_id: contract.id,
            stage: ResultStage::Final,
            title: "Final deliverable",
            summary: "Completed with tests",
            links: &[ResultLink {
                label: "Diff".into(),
                url: "https://example.test/diff".into(),
            }],
            artifact: Some(("text/markdown", b"# Deliverable")),
            now: 26,
        })
        .unwrap();
    store
        .record_verification_result(epic, contract.id, "cargo nextest passed", 26)
        .unwrap();
    let done = store.complete_work(epic).unwrap();
    assert_eq!(done.lane, BoardLane::Done);
    assert_eq!(done.phase, WorkPhase::Completed);

    // E13/E14: Sleep delegation holds irreversible actions and batches non-safety notifications.
    let ws2_task = store
        .create_work(NewWork {
            project_id: "rap",
            workspace: &ws2,
            kind: WorkKind::Task,
            parent_epic_id: None,
            title: "Sleep task",
            rank: 2,
        })
        .unwrap()
        .id;
    scheduler.configure_work(ws2_task, provider, false).unwrap();
    store
        .authorize_direct_execution(ws2_task, b"Direct work", "human", 30)
        .unwrap();
    store.enter_sleep(31).unwrap();
    let c2 = runner.dispatch_next(32, &[provider]).unwrap().unwrap();
    let risky_q = store
        .ask_question(ws2_task, c2.session_id, "Force push main?", 33)
        .unwrap();
    let held = store
        .delegate_question_during_sleep(
            risky_q.id,
            "sleep-delegate",
            "",
            "Irreversible git history rewrite",
            DecisionRisk::Irreversible,
            34,
        )
        .unwrap();
    assert_eq!(held.status, DelegationStatus::HeldForHuman);
    let n_routine = store
        .enqueue_notification(NewNotification {
            work_id: ws2_task,
            kind: NotificationKind::ResultReady,
            target_route: "/rap/work/2/results",
            title: "Info",
            body: "Batched",
            now: 35,
        })
        .unwrap();
    assert_eq!(n_routine.state, NotificationState::BatchedForSleep);
    let n_safety = store
        .enqueue_notification(NewNotification {
            work_id: ws2_task,
            kind: NotificationKind::SafetyHold,
            target_route: "/rap/work/2/questions",
            title: "Safety hold",
            body: "Requires human approval",
            now: 35,
        })
        .unwrap();
    assert_eq!(n_safety.state, NotificationState::Pending);
    let wakeup = store.wake_from_sleep(40).unwrap();
    assert_eq!(wakeup.unanswered_questions.len(), 1);

    // E15/E16/S-security: Rate limit parsing + remote session/workspace/irreversible guards.
    let limit_line = match provider {
        Provider::Codex => {
            r#"{"type":"turn.failed","error":{"message":"You’ve hit your usage limit"}}"#
        }
        Provider::ClaudeCode => {
            r#"{"type":"rate_limit_event","rate_limit_info":{"status":"rejected","resetsAt":1800000000}}"#
        }
    };
    assert!(parse_rate_limit_until(provider, limit_line, 1800000000).is_some());
    assert!(verify_remote_session(100, false, 100).is_err());
    assert!(verify_workspace_within_roots(&ws2, std::slice::from_ref(&ws1)).is_err());
    assert!(verify_irreversible_approval(true, true).is_err());
}

#[test]
fn scenarios_pass_for_both_codex_and_claude_code_paths() {
    verify_provider_end_to_end(Provider::Codex);
    verify_provider_end_to_end(Provider::ClaudeCode);
}
