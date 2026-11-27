use serde_json::Value;

use super::{LaunchRequest, Provider};
use crate::rap::SessionPhase;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderCommand {
    pub program: String,
    pub args: Vec<String>,
}

/// Builds the non-interactive CLI invocation for Codex or Claude Code from a
/// claimed RAP launch request, honoring planning vs. execution and resume.
pub fn build_provider_command(request: &LaunchRequest, prompt: &str) -> ProviderCommand {
    let ws = request.claim.workspace.to_string_lossy().into_owned();
    match request.claim.provider {
        Provider::Codex => {
            if let Some(native_id) = &request.claim.native_id {
                ProviderCommand {
                    program: "codex".into(),
                    args: vec![
                        "exec".into(),
                        "resume".into(),
                        native_id.clone(),
                        "--json".into(),
                        prompt.into(),
                    ],
                }
            } else {
                let sandbox = match request.claim.phase {
                    SessionPhase::Planning => "read-only",
                    SessionPhase::Execution => "workspace-write",
                };
                ProviderCommand {
                    program: "codex".into(),
                    args: vec![
                        "exec".into(),
                        "--json".into(),
                        "-s".into(),
                        sandbox.into(),
                        "-C".into(),
                        ws,
                        prompt.into(),
                    ],
                }
            }
        }
        Provider::ClaudeCode => {
            let mode = match request.claim.phase {
                SessionPhase::Planning => "plan",
                SessionPhase::Execution => "acceptEdits",
            };
            let mut args = vec!["-p".into()];
            if let Some(native_id) = &request.claim.native_id {
                args.push("--resume".into());
                args.push(native_id.clone());
            }
            args.extend([
                "--permission-mode".into(),
                mode.into(),
                "--output-format".into(),
                "stream-json".into(),
                "--verbose".into(),
                prompt.into(),
            ]);
            ProviderCommand {
                program: "claude".into(),
                args,
            }
        }
    }
}

/// Extracts the native session/thread identifier from a provider JSONL event line.
pub fn extract_session_id(provider: Provider, line: &str) -> Option<String> {
    let value: Value = serde_json::from_str(line).ok()?;
    match provider {
        Provider::Codex => {
            if value.get("type").and_then(Value::as_str) == Some("thread.started") {
                return value
                    .get("thread_id")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned);
            }
            None
        }
        Provider::ClaudeCode => {
            if value.get("type").and_then(Value::as_str) == Some("system")
                && value.get("subtype").and_then(Value::as_str) == Some("init")
            {
                return value
                    .get("session_id")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned);
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
    use crate::rap::scheduler::{
        AdapterError, Claim, LaunchHandle, ProbeState, ProcessIdentity, ProviderAdapter, RapRunner,
        SchedulerSettings, SchedulerStore, StoppedOutcome,
    };
    use crate::rap::{BoardLane, ContractId, EvidenceKind, NewWork, RapStore, WorkKind, WorkPhase};

    struct ScriptedClaudeAdapter {
        commands: RefCell<Vec<ProviderCommand>>,
        next_handle: RefCell<LaunchHandle>,
        probes: RefCell<HashMap<i64, ProbeState>>,
    }

    impl ProviderAdapter for ScriptedClaudeAdapter {
        fn start_session(&self, request: &LaunchRequest) -> Result<LaunchHandle, AdapterError> {
            let prompt = request
                .contract_document
                .as_deref()
                .map(|b| String::from_utf8_lossy(b).into_owned())
                .unwrap_or_else(|| "plan".into());
            self.commands
                .borrow_mut()
                .push(build_provider_command(request, &prompt));
            Ok(self.next_handle.borrow().clone())
        }

        fn probe_session(&self, claim: &Claim, _process: Option<&ProcessIdentity>) -> ProbeState {
            self.probes
                .borrow()
                .get(&claim.attempt_id.0)
                .cloned()
                .unwrap_or(ProbeState::Unknown)
        }
    }

    #[test]
    fn builds_commands_and_parses_native_ids_for_codex_and_claude() {
        let codex_line =
            r#"{"type":"thread.started","thread_id":"01a21f18-4e06-7920-bbe0-005e3d4c8226"}"#;
        assert_eq!(
            extract_session_id(Provider::Codex, codex_line).as_deref(),
            Some("01a21f18-4e06-7920-bbe0-005e3d4c8226")
        );
        let claude_line = r#"{"type":"system","subtype":"init","session_id":"99c4ea71-9830-45ee-88fc-699d5d442895"}"#;
        assert_eq!(
            extract_session_id(Provider::ClaudeCode, claude_line).as_deref(),
            Some("99c4ea71-9830-45ee-88fc-699d5d442895")
        );
    }

    #[test]
    fn claude_code_runs_full_plan_handoff_qa_recovery_and_contract_completion() {
        let root = tempfile::tempdir().unwrap();
        let ws = root.path().join("repo");
        fs::create_dir(&ws).unwrap();
        let db = root.path().join("rap.db");
        let store = RapStore::open(&db).unwrap();
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
                title: "Claude integration",
                rank: 1,
            })
            .unwrap()
            .id;
        scheduler
            .configure_work(work, Provider::ClaudeCode, true)
            .unwrap();

        let adapter = ScriptedClaudeAdapter {
            commands: RefCell::new(Vec::new()),
            next_handle: RefCell::new(LaunchHandle {
                process: ProcessIdentity {
                    pid: 7001,
                    fingerprint: "cc-plan".into(),
                },
                native_id: Some("cc-sess-plan".into()),
            }),
            probes: RefCell::new(HashMap::new()),
        };
        let runner = RapRunner::new(&store, &scheduler, &adapter);

        // 1. Planning with `--permission-mode plan`.
        let plan_claim = runner
            .dispatch_next(10, &[Provider::ClaudeCode])
            .unwrap()
            .unwrap();
        assert!(adapter.commands.borrow()[0].args.contains(&"plan".into()));
        adapter.probes.borrow_mut().insert(
            plan_claim.attempt_id.0,
            ProbeState::Stopped {
                outcome: StoppedOutcome::PlanReady,
                plan_bytes: Some(b"# Claude Plan".to_vec()),
            },
        );
        runner.reconcile().unwrap();
        let rev_id = store.work(work).unwrap().current_revision_id.unwrap();
        let contract = store
            .approve_plan_revision(work, rev_id, "human", 15)
            .unwrap();

        // 2. Separate execution session with `--permission-mode acceptEdits`.
        *adapter.next_handle.borrow_mut() = LaunchHandle {
            process: ProcessIdentity {
                pid: 7002,
                fingerprint: "cc-exec".into(),
            },
            native_id: Some("cc-sess-exec".into()),
        };
        let exec_claim = runner
            .dispatch_next(16, &[Provider::ClaudeCode])
            .unwrap()
            .unwrap();
        assert_ne!(exec_claim.session_id, plan_claim.session_id);
        assert!(
            adapter.commands.borrow()[1]
                .args
                .contains(&"acceptEdits".into())
        );

        // 3. Question pause -> reopen DB (restart recovery) -> answer -> resume with `--resume`.
        let q = store
            .ask_question(work, exec_claim.session_id, "Proceed?", 17)
            .unwrap();
        scheduler
            .confirm_stopped(&exec_claim, StoppedOutcome::AwaitingInput)
            .unwrap();
        let reopened = RapStore::open(&db).unwrap();
        let reopened_sched = SchedulerStore::new(reopened.clone());
        reopened.answer_question(q.id, "Yes", "human", 18).unwrap();
        reopened
            .deliver_answer(q.id, exec_claim.session_id, 19)
            .unwrap();

        let runner2 = RapRunner::new(&reopened, &reopened_sched, &adapter);
        let resumed = runner2
            .dispatch_next(20, &[Provider::ClaudeCode])
            .unwrap()
            .unwrap();
        assert_eq!(resumed.session_id, exec_claim.session_id);
        assert_eq!(resumed.epoch, 2);
        assert!(
            adapter.commands.borrow()[2]
                .args
                .contains(&"--resume".into())
        );
        // Old attempt update is rejected.
        assert!(
            reopened_sched
                .confirm_stopped(&exec_claim, StoppedOutcome::Verify)
                .is_err()
        );

        // 4. Finish execution, record evidence for current contract, and complete.
        reopened_sched
            .confirm_stopped(&resumed, StoppedOutcome::Verify)
            .unwrap();
        assert_eq!(reopened.work(work).unwrap().phase, WorkPhase::Verifying);
        reopened
            .record_evidence(
                work,
                ContractId(contract.id.0),
                EvidenceKind::Verification,
                "tests passed",
                None,
                21,
            )
            .unwrap();
        reopened
            .record_evidence(
                work,
                ContractId(contract.id.0),
                EvidenceKind::FinalResult,
                "final",
                None,
                22,
            )
            .unwrap();
        assert_eq!(reopened.complete_work(work).unwrap().lane, BoardLane::Done);
    }
}
