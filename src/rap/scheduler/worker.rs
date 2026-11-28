use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tokio::sync::watch;

use super::providers::{build_provider_command, extract_session_id};
use super::runner::{
    AdapterError, LaunchHandle, LaunchRequest, ProbeState, ProviderAdapter, RapRunner,
    parse_rate_limit_until,
};
use super::{Claim, ProcessIdentity, Provider, StoppedOutcome};
use crate::rap::{RapRuntime, SessionPhase};

struct LiveAttempt {
    child: Child,
    process: ProcessIdentity,
    native_id: Arc<Mutex<Option<String>>>,
    rate_limit_until: Arc<Mutex<Option<i64>>>,
    captured_lines: Arc<Mutex<Vec<String>>>,
}

/// Real subprocess adapter that spawns `codex` or `claude` per RAP attempt.
pub struct CliProviderAdapter {
    live: Mutex<HashMap<i64, LiveAttempt>>,
}

impl Default for CliProviderAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl CliProviderAdapter {
    pub fn new() -> Self {
        Self {
            live: Mutex::new(HashMap::new()),
        }
    }
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX))
}

impl ProviderAdapter for CliProviderAdapter {
    fn start_session(&self, request: &LaunchRequest) -> Result<LaunchHandle, AdapterError> {
        let prompt = match request.claim.phase {
            SessionPhase::Planning => {
                "Create a concise implementation plan for this task. Do not modify files."
                    .to_owned()
            }
            SessionPhase::Execution => request
                .contract_document
                .as_deref()
                .map(|bytes| String::from_utf8_lossy(bytes).into_owned())
                .unwrap_or_else(|| "Execute the approved task.".to_owned()),
        };
        let cmd = build_provider_command(request, &prompt);
        let mut child = Command::new(&cmd.program)
            .args(&cmd.args)
            .current_dir(&request.claim.workspace)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| AdapterError::Failed(e.to_string()))?;

        let pid = child.id();
        let process = ProcessIdentity {
            pid,
            fingerprint: format!("pid-{pid}"),
        };
        let native_id = Arc::new(Mutex::new(request.claim.native_id.clone()));
        let rate_limit_until = Arc::new(Mutex::new(None));
        let captured_lines = Arc::new(Mutex::new(Vec::new()));

        if let Some(stdout) = child.stdout.take() {
            let provider = request.claim.provider;
            let nid = native_id.clone();
            let rl = rate_limit_until.clone();
            let lines = captured_lines.clone();
            std::thread::spawn(move || {
                let reader = BufReader::new(stdout);
                for line in reader.lines().map_while(Result::ok) {
                    if let Some(found) = extract_session_id(provider, &line) {
                        *nid.lock().unwrap_or_else(|e| e.into_inner()) = Some(found);
                    }
                    if let Some(until) = parse_rate_limit_until(provider, &line, now_secs() + 300) {
                        *rl.lock().unwrap_or_else(|e| e.into_inner()) = Some(until);
                    }
                    let mut buf = lines.lock().unwrap_or_else(|e| e.into_inner());
                    if buf.len() < 200 {
                        buf.push(line);
                    }
                }
            });
        }

        self.live.lock().unwrap_or_else(|e| e.into_inner()).insert(
            request.claim.attempt_id.0,
            LiveAttempt {
                child,
                process: process.clone(),
                native_id,
                rate_limit_until,
                captured_lines,
            },
        );
        Ok(LaunchHandle {
            process,
            native_id: request.claim.native_id.clone(),
        })
    }

    fn probe_session(&self, claim: &Claim, _process: Option<&ProcessIdentity>) -> ProbeState {
        let mut live = self.live.lock().unwrap_or_else(|e| e.into_inner());
        let Some(entry) = live.get_mut(&claim.attempt_id.0) else {
            return ProbeState::Stopped {
                outcome: StoppedOutcome::Retry { now: now_secs() },
                plan_bytes: None,
            };
        };
        let native_id = entry
            .native_id
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        match entry.child.try_wait() {
            Ok(None) => ProbeState::Running {
                process: entry.process.clone(),
                native_id,
            },
            Ok(Some(status)) => {
                let rate_limit = *entry
                    .rate_limit_until
                    .lock()
                    .unwrap_or_else(|e| e.into_inner());
                let output = entry
                    .captured_lines
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .join("\n");
                live.remove(&claim.attempt_id.0);
                if let Some(until) = rate_limit {
                    return ProbeState::Stopped {
                        outcome: StoppedOutcome::RateLimited { until },
                        plan_bytes: None,
                    };
                }
                if status.success() {
                    match claim.phase {
                        SessionPhase::Planning => ProbeState::Stopped {
                            outcome: StoppedOutcome::PlanReady,
                            plan_bytes: Some(output.into_bytes()),
                        },
                        SessionPhase::Execution => ProbeState::Stopped {
                            outcome: StoppedOutcome::Verify,
                            plan_bytes: None,
                        },
                    }
                } else {
                    ProbeState::Stopped {
                        outcome: StoppedOutcome::Retry { now: now_secs() },
                        plan_bytes: None,
                    }
                }
            }
            Err(_) => ProbeState::Unknown,
        }
    }
}

/// Runs the background RAP scheduler loop inside `agpj serve`, reconciling in-flight
/// attempts and dispatching manual or auto-run work items as separate CLI sessions.
pub fn spawn_rap_worker(
    runtime: Arc<RapRuntime>,
    mut shutdown: watch::Receiver<bool>,
) -> tokio::task::JoinHandle<()> {
    let adapter = Arc::new(CliProviderAdapter::new());
    tokio::spawn(async move {
        let providers = [Provider::Codex, Provider::ClaudeCode];
        loop {
            let rt = runtime.clone();
            let ad = adapter.clone();
            let _ = tokio::task::spawn_blocking(move || {
                let runner = RapRunner::new(&rt.store, &rt.scheduler, ad.as_ref());
                let _ = runner.reconcile();
                let now = now_secs();
                while let Some(work_id) = rt.take_manual_run() {
                    let _ = runner.dispatch_work_now(work_id, now, &providers);
                }
                while let Ok(Some(_)) = runner.dispatch_next(now, &providers) {}
            })
            .await;

            tokio::select! {
                _ = runtime.woken() => {}
                _ = tokio::time::sleep(Duration::from_secs(2)) => {}
                _ = shutdown.changed() => {
                    if *shutdown.borrow() {
                        break;
                    }
                }
            }
        }
    })
}
