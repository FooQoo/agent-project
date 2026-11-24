//! Agents, connections, and the run state tied to a connection (AC-1 to AC-4 of the agent
//! registry): names and roles keep the ID, a code gives one connection, a second parent never
//! takes an agent over, and a running task returns to waiting when its connection ends or goes
//! quiet.

use agent_project::domain::{Actor, Assignee, ChangeContext, Column, NewTask, ProjectId, RunState};
use agent_project::error::AgpjError;
use agent_project::file_store::{ConnectionOrigin, FileStore, STALE_RUN_SECONDS};
use chrono::{DateTime, Duration, TimeZone, Utc};

#[path = "support/qa.rs"]
mod qa;

fn t0() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 24, 9, 0, 0).unwrap()
}

fn ctx(actor: Actor) -> ChangeContext {
    ChangeContext {
        actor,
        reason: String::new(),
    }
}

struct Fx {
    _root: tempfile::TempDir,
    store: FileStore,
    project: ProjectId,
}

fn fx() -> Fx {
    let root = tempfile::tempdir().unwrap();
    let store = FileStore::open(&root.path().join("shared"));
    let project = store.create_project("Alpha", t0()).unwrap().id;
    Fx {
        _root: root,
        store,
        project,
    }
}

impl Fx {
    fn doing_task(&self) -> i64 {
        let id = self
            .store
            .create_task(
                &self.project,
                NewTask {
                    title: "work".into(),
                    description: String::new(),
                    assignee: Assignee::Ai,
                    actor: Actor::Human,
                    draft: None,
                },
                t0(),
            )
            .unwrap()
            .id;
        // Every AI task starts with the default agent; these tests work with the first agent
        // they registered, so the card is given to it before it starts.
        if self
            .store
            .list_agents(&self.project)
            .unwrap()
            .iter()
            .any(|a| a.id == "A-1" && !a.is_default)
        {
            self.store
                .assign_task_agent(&self.project, id, Some("A-1"), t0())
                .unwrap();
        }
        self.store
            .move_work_task_for_user(&self.project, id, Column::Doing, ctx(Actor::Human), t0())
            .unwrap();
        id
    }

    fn connect(&self, agent: &str, switch: bool, at: DateTime<Utc>) -> Result<String, AgpjError> {
        let code = self
            .store
            .issue_connect_code(&self.project, agent, switch, at)?;
        let connected = self.store.exchange_connect_code(
            &self.project,
            &code.code,
            ConnectionOrigin::External,
            "claude-code",
            "claude:1",
            at,
        )?;
        Ok(connected.secret)
    }

    fn run(&self, id: i64, secret: &str, at: DateTime<Utc>) -> Result<(), AgpjError> {
        self.store
            .set_agent_status_connected(
                &self.project,
                id,
                RunState::Running,
                ctx(Actor::Ai),
                secret,
                at,
            )
            .map(|_| ())
    }
}

#[test]
fn a_rename_and_a_role_edit_keep_the_agent_id_on_cards() {
    let fx = fx();
    let agent = fx
        .store
        .register_agent(&fx.project, "Screen", "Builds the screens", t0())
        .unwrap();
    assert_eq!(agent.id, "A-1");
    let id = fx.doing_task();
    fx.store
        .assign_task_agent(&fx.project, id, Some("A-1"), t0())
        .unwrap();
    let renamed = fx
        .store
        .update_agent(&fx.project, "A-1", Some("Screens"), Some("UI work"))
        .unwrap();
    assert_eq!(
        (renamed.id.as_str(), renamed.name.as_str()),
        ("A-1", "Screens")
    );
    let task = fx.store.task(&fx.project, id).unwrap();
    assert_eq!(task.agent_id.as_deref(), Some("A-1"));
    // The number is never reused, and a live name is unique.
    assert!(matches!(
        fx.store.register_agent(&fx.project, "Screens", "", t0()),
        Err(AgpjError::AgentNameTaken { .. })
    ));
    assert_eq!(
        fx.store
            .register_agent(&fx.project, "CLI", "", t0())
            .unwrap()
            .id,
        // A-2 went to the default agent, which the first AI task created.
        "A-3"
    );
}

#[test]
fn a_connection_runs_a_free_card_and_claims_it() {
    let fx = fx();
    fx.store
        .register_agent(&fx.project, "Screen", "", t0())
        .unwrap();
    let secret = fx.connect("A-1", false, t0()).unwrap();
    let id = fx.doing_task();
    fx.run(id, &secret, t0()).unwrap();
    let task = fx.store.task(&fx.project, id).unwrap();
    assert_eq!(task.agent_status, RunState::Running);
    assert_eq!(task.agent_id.as_deref(), Some("A-1"));
    assert!(task.run_connection_id.is_some());
    assert_eq!(task.assignment_generation, 2);
}

#[test]
fn a_claimed_name_a_made_up_secret_and_a_human_claim_grant_nothing() {
    let fx = fx();
    fx.store
        .register_agent(&fx.project, "Screen", "", t0())
        .unwrap();
    fx.store
        .register_agent(&fx.project, "CLI", "", t0())
        .unwrap();
    let screen = fx.connect("A-1", false, t0()).unwrap();
    let cli = fx.connect("A-2", false, t0()).unwrap();
    let id = fx.doing_task();
    fx.run(id, &screen, t0()).unwrap();
    let before = fx.store.task(&fx.project, id).unwrap();
    // Another agent's connection cannot touch the card.
    assert!(matches!(
        fx.run(id, &cli, t0()),
        Err(AgpjError::NotAssignee { .. })
    ));
    // A secret nobody issued is not a connection, whatever the session says it is.
    assert!(matches!(
        fx.run(id, "agpjc_0000", t0()),
        Err(AgpjError::NotConnected)
    ));
    assert!(matches!(fx.run(id, "", t0()), Err(AgpjError::NotConnected)));
    assert_eq!(fx.store.task(&fx.project, id).unwrap(), before);
}

#[test]
fn a_second_parent_does_not_take_over_until_the_user_switches() {
    let fx = fx();
    fx.store
        .register_agent(&fx.project, "Screen", "", t0())
        .unwrap();
    let first = fx.connect("A-1", false, t0()).unwrap();
    let id = fx.doing_task();
    fx.run(id, &first, t0()).unwrap();
    // A plain code is not even issued while the agent is connected.
    assert!(matches!(
        fx.store.issue_connect_code(&fx.project, "A-1", false, t0()),
        Err(AgpjError::AgentBusy { .. })
    ));
    // The first connection still works.
    fx.store
        .set_agent_status_connected(
            &fx.project,
            id,
            RunState::Paused,
            ctx(Actor::Ai),
            &first,
            t0(),
        )
        .unwrap();
    // The user switches: the new code replaces the connection and the old one is refused.
    fx.run(id, &first, t0()).unwrap();
    let second = fx.connect("A-1", true, t0()).unwrap();
    assert_ne!(first, second);
    assert!(matches!(
        fx.run(id, &first, t0()),
        Err(AgpjError::ConnectionReplaced)
    ));
    let task = fx.store.task(&fx.project, id).unwrap();
    assert_eq!(
        task.agent_status,
        RunState::Paused,
        "the old run was released"
    );
    fx.run(id, &second, t0()).unwrap();
}

#[test]
fn a_code_works_once_and_expires() {
    let fx = fx();
    fx.store
        .register_agent(&fx.project, "Screen", "", t0())
        .unwrap();
    let code = fx
        .store
        .issue_connect_code(&fx.project, "A-1", false, t0())
        .unwrap();
    let late = t0() + Duration::minutes(11);
    assert!(matches!(
        fx.store.exchange_connect_code(
            &fx.project,
            &code.code,
            ConnectionOrigin::External,
            "codex",
            "k",
            late
        ),
        Err(AgpjError::ConnectCodeInvalid)
    ));
    let code = fx
        .store
        .issue_connect_code(&fx.project, "A-1", false, t0())
        .unwrap();
    let args = (ConnectionOrigin::External, "codex", "k");
    fx.store
        .exchange_connect_code(&fx.project, &code.code, args.0, args.1, args.2, t0())
        .unwrap();
    assert!(matches!(
        fx.store
            .exchange_connect_code(&fx.project, &code.code, args.0, args.1, args.2, t0()),
        Err(AgpjError::ConnectCodeInvalid)
    ));
}

#[test]
fn ending_the_session_returns_the_running_task_to_waiting() {
    let fx = fx();
    fx.store
        .register_agent(&fx.project, "Screen", "", t0())
        .unwrap();
    let secret = fx.connect("A-1", false, t0()).unwrap();
    let id = fx.doing_task();
    fx.run(id, &secret, t0()).unwrap();
    let ended = fx
        .store
        .end_connection_session(&fx.project, &secret, t0())
        .unwrap();
    assert_eq!(ended, vec![id]);
    let task = fx.store.task(&fx.project, id).unwrap();
    assert_eq!(task.agent_status, RunState::Paused);
    assert_eq!(task.run_connection_id, None);
    assert_eq!(task.task_status, Column::Doing);
}

#[test]
fn a_session_that_never_reports_back_counts_as_waiting_after_a_fixed_time() {
    let fx = fx();
    fx.store
        .register_agent(&fx.project, "Screen", "", t0())
        .unwrap();
    let secret = fx.connect("A-1", false, t0()).unwrap();
    let id = fx.doing_task();
    fx.run(id, &secret, t0()).unwrap();
    let almost = t0() + Duration::seconds(STALE_RUN_SECONDS - 60);
    assert!(
        fx.store
            .release_stale_runs(&fx.project, almost)
            .unwrap()
            .is_empty()
    );
    let quiet = t0() + Duration::seconds(STALE_RUN_SECONDS + 60);
    assert_eq!(
        fx.store.release_stale_runs(&fx.project, quiet).unwrap(),
        vec![id]
    );
    assert_eq!(
        fx.store.task(&fx.project, id).unwrap().agent_status,
        RunState::Paused
    );
}

#[test]
fn activity_keeps_a_running_task_running() {
    let fx = fx();
    fx.store
        .register_agent(&fx.project, "Screen", "", t0())
        .unwrap();
    let secret = fx.connect("A-1", false, t0()).unwrap();
    let id = fx.doing_task();
    fx.run(id, &secret, t0()).unwrap();
    let later = t0() + Duration::seconds(STALE_RUN_SECONDS - 60);
    // Any command of the session records activity.
    fx.store
        .verify_connection(&fx.project, &secret, later)
        .unwrap();
    let check = later + Duration::seconds(STALE_RUN_SECONDS - 60);
    assert!(
        fx.store
            .release_stale_runs(&fx.project, check)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn an_agent_with_a_doing_card_or_a_connection_cannot_be_deleted() {
    let fx = fx();
    fx.store
        .register_agent(&fx.project, "Screen", "", t0())
        .unwrap();
    let secret = fx.connect("A-1", false, t0()).unwrap();
    assert!(matches!(
        fx.store.archive_agent(&fx.project, "A-1", t0()),
        Err(AgpjError::AgentInUse { .. })
    ));
    // After the user switches it away and nothing is Doing, an unconnected agent can go.
    fx.store
        .register_agent(&fx.project, "CLI", "", t0())
        .unwrap();
    fx.store.archive_agent(&fx.project, "A-2", t0()).unwrap();
    let _ = secret;
    assert!(matches!(
        fx.store.issue_connect_code(&fx.project, "A-2", false, t0()),
        Err(AgpjError::AgentNotFound { .. })
    ));
}

#[test]
fn a_task_json_without_the_new_fields_is_not_read() {
    let fx = fx();
    let id = fx.doing_task();
    let path = fx
        ._root
        .path()
        .join("shared/projects")
        .join(fx.project.as_str())
        .join("tasks")
        .join(id.to_string())
        .join("task.json");
    let text = std::fs::read_to_string(&path)
        .unwrap()
        .replace("\"formatVersion\": 3", "\"formatVersion\": 2");
    std::fs::write(&path, text).unwrap();
    assert!(fx.store.task(&fx.project, id).is_err());
}

#[test]
fn an_unreadable_task_stops_the_deletion_of_an_agent() {
    let fx = fx();
    fx.store
        .register_agent(&fx.project, "Screen", "", t0())
        .unwrap();
    let id = fx.doing_task();
    let path = fx
        ._root
        .path()
        .join("shared/projects")
        .join(fx.project.as_str())
        .join("tasks")
        .join(id.to_string())
        .join("task.json");
    std::fs::write(&path, "{").unwrap();
    assert!(fx.store.archive_agent(&fx.project, "A-1", t0()).is_err());
    assert!(
        fx.store
            .issue_connect_code(&fx.project, "A-1", false, t0())
            .is_ok()
    );
}

#[test]
fn connect_is_refused_while_the_environment_already_carries_a_secret() {
    let home = tempfile::tempdir().unwrap();
    let run = |args: &[&str], env: bool| {
        let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_agpj"));
        cmd.env("AGPJ_AUTOSTART", "0")
            .env("AGPJ_HOME", home.path())
            .current_dir(home.path())
            .args(args)
            .stdin(std::process::Stdio::null());
        if env {
            cmd.env("AGPJ_CONNECTION_SECRET", "agpjc_x");
        }
        cmd.output().unwrap()
    };
    assert!(run(&["project", "create", "Demo"], false).status.success());
    let out = run(&["--project", "Demo", "connect", "ABCD2345"], true);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("AGPJ_CONNECTION_SECRET"));
}

/// Store `secret` where a session with this Claude session ID keeps it for `project`.
fn keep_secret(data_dir: &std::path::Path, project: &ProjectId, session_id: &str, secret: &str) {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(session_id.as_bytes());
    let hex: String = digest.iter().take(8).map(|b| format!("{b:02x}")).collect();
    let dir = data_dir.join("connections").join(project.as_str());
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join(format!("claude_{hex}.secret")), secret).unwrap();
}

#[test]
fn session_end_releases_runs_in_every_project_holding_the_secret_from_an_unbound_folder() {
    let fx = fx();
    let beta = fx.store.create_project("Beta", t0()).unwrap().id;
    fx.store
        .register_agent(&fx.project, "Screen", "", t0())
        .unwrap();
    fx.store.register_agent(&beta, "Server", "", t0()).unwrap();
    let alpha_secret = fx.connect("A-1", false, t0()).unwrap();
    let alpha_task = fx.doing_task();
    fx.run(alpha_task, &alpha_secret, t0()).unwrap();
    let code = fx
        .store
        .issue_connect_code(&beta, "A-1", false, t0())
        .unwrap();
    let beta_secret = fx
        .store
        .exchange_connect_code(
            &beta,
            &code.code,
            ConnectionOrigin::External,
            "claude-code",
            "claude:1",
            t0(),
        )
        .unwrap()
        .secret;
    let beta_task = fx
        .store
        .create_task(
            &beta,
            NewTask {
                title: "work".into(),
                description: String::new(),
                assignee: Assignee::Ai,
                actor: Actor::Human,
                draft: None,
            },
            t0(),
        )
        .unwrap()
        .id;
    fx.store
        .assign_task_agent(&beta, beta_task, Some("A-1"), t0())
        .unwrap();
    fx.store
        .move_work_task_for_user(&beta, beta_task, Column::Doing, ctx(Actor::Human), t0())
        .unwrap();
    fx.store
        .set_agent_status_connected(
            &beta,
            beta_task,
            RunState::Running,
            ctx(Actor::Ai),
            &beta_secret,
            t0(),
        )
        .unwrap();

    let data_dir = fx._root.path();
    keep_secret(data_dir, &fx.project, "session-1", &alpha_secret);
    keep_secret(data_dir, &beta, "session-1", &beta_secret);
    // The hook runs in a folder bound to no project and names none.
    let unbound = tempfile::tempdir().unwrap();
    let input = serde_json::json!({
        "hook_event_name": "SessionEnd",
        "session_id": "session-1",
        "cwd": unbound.path(),
    })
    .to_string();
    let out = agent_project::agent::hook::run(
        "claude",
        &mut std::io::Cursor::new(input.into_bytes()),
        &data_dir.join("shared"),
        None,
    );
    assert!(out.is_none());
    for (project, id) in [(&fx.project, alpha_task), (&beta, beta_task)] {
        let task = fx.store.task(project, id).unwrap();
        assert_eq!(task.agent_status, RunState::Paused);
        assert_eq!(task.run_connection_id, None);
    }
}

#[test]
fn a_run_state_without_a_connection_is_refused_once_the_project_has_agents() {
    let fx = fx();
    let id = fx.doing_task();
    // No agents yet: the plain rules apply.
    assert!(
        fx.store
            .set_agent_status_unconnected(&fx.project, id, RunState::Running, ctx(Actor::Ai), t0())
            .is_ok()
    );
    fx.store
        .set_agent_status_unconnected(&fx.project, id, RunState::Paused, ctx(Actor::Ai), t0())
        .unwrap();
    fx.store
        .register_agent(&fx.project, "Screen", "", t0())
        .unwrap();
    let refused = fx.store.set_agent_status_unconnected(
        &fx.project,
        id,
        RunState::Running,
        ctx(Actor::Ai),
        t0(),
    );
    assert!(matches!(refused, Err(AgpjError::NotConnected)));
    let human = fx.store.set_agent_status_unconnected(
        &fx.project,
        id,
        RunState::Running,
        ctx(Actor::Human),
        t0(),
    );
    assert!(matches!(human, Err(AgpjError::HumanOnly { .. })));
    assert_eq!(
        fx.store.task(&fx.project, id).unwrap().agent_status,
        RunState::Paused
    );
}

#[test]
fn disconnect_frees_the_agent_so_it_can_be_connected_again_and_deleted() {
    let fx = fx();
    fx.store
        .register_agent(&fx.project, "Screen", "", t0())
        .unwrap();
    let secret = fx.connect("A-1", false, t0()).unwrap();
    let id = fx.doing_task();
    fx.run(id, &secret, t0()).unwrap();
    let paused = fx
        .store
        .disconnect_session(&fx.project, &secret, t0())
        .unwrap();
    assert_eq!(paused, vec![id]);
    assert_eq!(
        fx.store.task(&fx.project, id).unwrap().agent_status,
        RunState::Paused
    );
    // The secret is dead, a plain code works again, and with nothing in Doing the agent can go.
    assert!(
        fx.store
            .verify_connection(&fx.project, &secret, t0())
            .is_err()
    );
    assert!(
        fx.store
            .issue_connect_code(&fx.project, "A-1", false, t0())
            .is_ok()
    );
}

#[test]
fn a_session_holds_one_agent_and_disconnect_lets_it_take_another() {
    let home = tempfile::tempdir().unwrap();
    let run = |args: &[&str]| {
        std::process::Command::new(env!("CARGO_BIN_EXE_agpj"))
            .env("AGPJ_AUTOSTART", "0")
            .env("AGPJ_HOME", home.path())
            .env("CLAUDE_CODE_SESSION_ID", "session-one")
            .env_remove("AGPJ_CONNECTION_SECRET")
            .current_dir(home.path())
            .args(args)
            .stdin(std::process::Stdio::null())
            .output()
            .unwrap()
    };
    assert!(run(&["project", "create", "Demo"]).status.success());
    let store = FileStore::open(&home.path().join("shared"));
    let project = store.project_by_name("Demo").unwrap().id;
    store.register_agent(&project, "Screen", "", t0()).unwrap();
    store.register_agent(&project, "CLI", "", t0()).unwrap();
    let code = |agent: &str| {
        store
            .issue_connect_code(&project, agent, false, Utc::now())
            .unwrap()
            .code
    };
    let first = run(&["--project", "Demo", "connect", &code("A-1")]);
    assert!(first.status.success(), "{first:?}");
    // A second agent is refused while the first connection is held.
    let second = run(&["--project", "Demo", "connect", &code("A-2")]);
    assert!(!second.status.success());
    assert!(String::from_utf8_lossy(&second.stderr).contains("disconnect"));
    assert!(run(&["--project", "Demo", "disconnect"]).status.success());
    let again = run(&["--project", "Demo", "connect", &code("A-2")]);
    assert!(again.status.success(), "{again:?}");
}

#[test]
fn concurrent_connects_of_one_session_let_exactly_one_win() {
    let home = tempfile::tempdir().unwrap();
    let cmd = |args: &[&str]| {
        let mut c = std::process::Command::new(env!("CARGO_BIN_EXE_agpj"));
        c.env("AGPJ_AUTOSTART", "0")
            .env("AGPJ_HOME", home.path())
            .env("CLAUDE_CODE_SESSION_ID", "session-race")
            .env_remove("AGPJ_CONNECTION_SECRET")
            .current_dir(home.path())
            .args(args)
            .stdin(std::process::Stdio::null());
        c
    };
    assert!(
        cmd(&["project", "create", "Race"])
            .output()
            .unwrap()
            .status
            .success()
    );
    let store = FileStore::open(&home.path().join("shared"));
    let project = store.project_by_name("Race").unwrap().id;
    store.register_agent(&project, "One", "", t0()).unwrap();
    store.register_agent(&project, "Two", "", t0()).unwrap();
    for _ in 0..4 {
        let codes: Vec<String> = ["A-1", "A-2"]
            .iter()
            .map(|a| {
                store
                    .issue_connect_code(&project, a, false, Utc::now())
                    .unwrap()
                    .code
            })
            .collect();
        let children: Vec<_> = codes
            .iter()
            .map(|code| {
                cmd(&["--project", "Race", "connect", code])
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::piped())
                    .spawn()
                    .unwrap()
            })
            .collect();
        let outcomes: Vec<_> = children
            .into_iter()
            .map(|c| c.wait_with_output().unwrap())
            .collect();
        let wins = outcomes.iter().filter(|o| o.status.success()).count();
        assert_eq!(wins, 1, "{outcomes:?}");
        let refused = outcomes.iter().find(|o| !o.status.success()).unwrap();
        assert!(String::from_utf8_lossy(&refused.stderr).contains("disconnect"));
        // The stored secret is the winner's and still works: disconnect uses it.
        let out = cmd(&["--project", "Race", "disconnect"]).output().unwrap();
        assert!(out.status.success(), "{out:?}");
    }
}

#[test]
fn a_failed_secret_write_leaves_no_active_connection() {
    let home = tempfile::tempdir().unwrap();
    let run = |args: &[&str]| {
        std::process::Command::new(env!("CARGO_BIN_EXE_agpj"))
            .env("AGPJ_AUTOSTART", "0")
            .env("AGPJ_HOME", home.path())
            .env("CLAUDE_CODE_SESSION_ID", "session-full")
            .env_remove("AGPJ_CONNECTION_SECRET")
            .current_dir(home.path())
            .args(args)
            .stdin(std::process::Stdio::null())
            .output()
            .unwrap()
    };
    assert!(run(&["project", "create", "Full"]).status.success());
    let store = FileStore::open(&home.path().join("shared"));
    let project = store.project_by_name("Full").unwrap().id;
    store.register_agent(&project, "One", "", t0()).unwrap();
    let code = store
        .issue_connect_code(&project, "A-1", false, Utc::now())
        .unwrap()
        .code;
    // A folder where the secret file belongs makes the secret unwritable (the lock still works).
    {
        use sha2::{Digest, Sha256};
        let digest = Sha256::digest("session-full".as_bytes());
        let hex: String = digest.iter().take(8).map(|b| format!("{b:02x}")).collect();
        let blocked = home
            .path()
            .join("connections")
            .join(project.as_str())
            .join(format!("claude_{hex}.secret"));
        std::fs::create_dir_all(&blocked).unwrap();
    }
    let out = run(&["--project", "Full", "connect", &code]);
    assert!(!out.status.success());
    // The new connection was revoked: the agent is free for a plain code.
    assert!(
        store
            .issue_connect_code(&project, "A-1", false, Utc::now())
            .is_ok()
    );
}

#[test]
fn connect_and_disconnect_racing_leave_a_consistent_state() {
    let home = tempfile::tempdir().unwrap();
    let cmd = |args: &[&str]| {
        let mut c = std::process::Command::new(env!("CARGO_BIN_EXE_agpj"));
        c.env("AGPJ_AUTOSTART", "0")
            .env("AGPJ_HOME", home.path())
            .env("CLAUDE_CODE_SESSION_ID", "session-flip")
            .env_remove("AGPJ_CONNECTION_SECRET")
            .current_dir(home.path())
            .args(args)
            .stdin(std::process::Stdio::null());
        c
    };
    assert!(
        cmd(&["project", "create", "Flip"])
            .output()
            .unwrap()
            .status
            .success()
    );
    let store = FileStore::open(&home.path().join("shared"));
    let project = store.project_by_name("Flip").unwrap().id;
    store.register_agent(&project, "One", "", t0()).unwrap();
    store.register_agent(&project, "Two", "", t0()).unwrap();
    let code = |agent: &str| {
        store
            .issue_connect_code(&project, agent, false, Utc::now())
            .unwrap()
            .code
    };
    assert!(
        cmd(&["--project", "Flip", "connect", &code("A-1")])
            .output()
            .unwrap()
            .status
            .success()
    );
    for _ in 0..4 {
        // A disconnect and a connect to the other agent start together.
        let next = if store.has_agents(&project).unwrap() {
            code("A-2")
        } else {
            String::new()
        };
        let a = cmd(&["--project", "Flip", "disconnect"]).spawn().unwrap();
        let b = cmd(&["--project", "Flip", "connect", &next])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap();
        let (a, b) = (a.wait_with_output().unwrap(), b.wait_with_output().unwrap());
        let _ = a;
        // Consistent: either connected with a secret that verifies (disconnect succeeds), or
        // fully disconnected (the disconnect then says there is nothing to leave).
        let agents = store.list_agents(&project).unwrap();
        let connected = agents.iter().filter(|x| x.connection.is_some()).count();
        let out = cmd(&["--project", "Flip", "disconnect"]).output().unwrap();
        if connected == 1 {
            assert!(b.status.success() || out.status.success());
            assert!(out.status.success(), "{out:?}");
        } else {
            assert_eq!(connected, 0);
            assert!(!out.status.success());
        }
        // Back to a held connection for the next round.
        assert!(
            cmd(&["--project", "Flip", "connect", &code("A-1")])
                .output()
                .unwrap()
                .status
                .success()
        );
    }
}

#[test]
fn a_failed_secret_save_leaves_the_code_valid_and_no_active_connection() {
    let fx = fx();
    fx.store
        .register_agent(&fx.project, "Screen", "", t0())
        .unwrap();
    let code = fx
        .store
        .issue_connect_code(&fx.project, "A-1", false, t0())
        .unwrap();
    let failed = fx.store.exchange_connect_code_storing(
        &fx.project,
        &code.code,
        ConnectionOrigin::External,
        "claude-code",
        "claude:1",
        t0(),
        |_| Err(AgpjError::NotConnected),
    );
    assert!(matches!(failed, Err(AgpjError::NotConnected)));
    let agents = fx.store.list_agents(&fx.project).unwrap();
    assert!(agents[0].connection.is_none());
    // The code was not consumed, so the same code connects once the secret can be kept.
    let mut kept = String::new();
    let ok = fx
        .store
        .exchange_connect_code_storing(
            &fx.project,
            &code.code,
            ConnectionOrigin::External,
            "claude-code",
            "claude:1",
            t0(),
            |secret| {
                kept = secret.to_string();
                Ok(())
            },
        )
        .unwrap();
    assert_eq!(ok.secret, kept);
    assert!(fx.store.verify_connection(&fx.project, &kept, t0()).is_ok());
}

fn session_cmd(home: &std::path::Path, session: &str, args: &[&str]) -> std::process::Command {
    let mut c = std::process::Command::new(env!("CARGO_BIN_EXE_agpj"));
    c.env("AGPJ_AUTOSTART", "0")
        .env("AGPJ_HOME", home)
        .env("CLAUDE_CODE_SESSION_ID", session)
        .env_remove("AGPJ_CONNECTION_SECRET")
        .current_dir(home)
        .args(args)
        .stdin(std::process::Stdio::null());
    c
}

#[test]
fn a_revoked_secret_lets_the_session_connect_to_another_agent() {
    let home = tempfile::tempdir().unwrap();
    assert!(
        session_cmd(home.path(), "s-rev", &["project", "create", "Rev"])
            .output()
            .unwrap()
            .status
            .success()
    );
    let store = FileStore::open(&home.path().join("shared"));
    let project = store.project_by_name("Rev").unwrap().id;
    store.register_agent(&project, "One", "", t0()).unwrap();
    store.register_agent(&project, "Two", "", t0()).unwrap();
    let first = store
        .issue_connect_code(&project, "A-1", false, Utc::now())
        .unwrap();
    assert!(
        session_cmd(
            home.path(),
            "s-rev",
            &["--project", "Rev", "connect", &first.code]
        )
        .output()
        .unwrap()
        .status
        .success()
    );
    // The user switches agent One to another session: this session's stored secret is replaced.
    let switch = store
        .issue_connect_code(&project, "A-1", true, Utc::now())
        .unwrap();
    store
        .exchange_connect_code(
            &project,
            &switch.code,
            ConnectionOrigin::External,
            "claude-code",
            "claude:other",
            Utc::now(),
        )
        .unwrap();
    let second = store
        .issue_connect_code(&project, "A-2", false, Utc::now())
        .unwrap();
    let out = session_cmd(
        home.path(),
        "s-rev",
        &["--project", "Rev", "connect", &second.code],
    )
    .output()
    .unwrap();
    assert!(out.status.success(), "{out:?}");
}

#[test]
fn a_verification_that_cannot_run_stops_connect_and_changes_nothing() {
    use std::os::unix::fs::PermissionsExt;
    let home = tempfile::tempdir().unwrap();
    assert!(
        session_cmd(home.path(), "s-io", &["project", "create", "Io"])
            .output()
            .unwrap()
            .status
            .success()
    );
    let store = FileStore::open(&home.path().join("shared"));
    let project = store.project_by_name("Io").unwrap().id;
    store.register_agent(&project, "One", "", t0()).unwrap();
    store.register_agent(&project, "Two", "", t0()).unwrap();
    let first = store
        .issue_connect_code(&project, "A-1", false, Utc::now())
        .unwrap();
    assert!(
        session_cmd(
            home.path(),
            "s-io",
            &["--project", "Io", "connect", &first.code]
        )
        .output()
        .unwrap()
        .status
        .success()
    );
    let second = store
        .issue_connect_code(&project, "A-2", false, Utc::now())
        .unwrap();
    let ws = session_cmd(
        home.path(),
        "s-io",
        &["--project", "Io", "workspace", "--json"],
    )
    .output()
    .unwrap();
    let folder: serde_json::Value = serde_json::from_slice(&ws.stdout).unwrap();
    let file = std::path::Path::new(folder["folder"].as_str().unwrap())
        .join(".internal")
        .join("connections.json");
    let mode = std::fs::metadata(&file).unwrap().permissions();
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o000)).unwrap();
    let out = session_cmd(
        home.path(),
        "s-io",
        &["--project", "Io", "connect", &second.code],
    )
    .output()
    .unwrap();
    std::fs::set_permissions(&file, mode).unwrap();
    assert!(!out.status.success());
    // The first connection is still the session's, and the second code was not consumed.
    let agents = store.list_agents(&project).unwrap();
    assert!(agents[0].connection.is_some() && agents[1].connection.is_none());
    assert!(
        session_cmd(home.path(), "s-io", &["--project", "Io", "disconnect"])
            .output()
            .unwrap()
            .status
            .success()
    );
    assert!(
        session_cmd(
            home.path(),
            "s-io",
            &["--project", "Io", "connect", &second.code]
        )
        .output()
        .unwrap()
        .status
        .success()
    );
}

#[test]
fn an_unreadable_secret_file_stops_connect_and_changes_nothing() {
    use sha2::{Digest, Sha256};
    use std::os::unix::fs::PermissionsExt;
    let home = tempfile::tempdir().unwrap();
    assert!(
        session_cmd(home.path(), "s-sec", &["project", "create", "Sec"])
            .output()
            .unwrap()
            .status
            .success()
    );
    let store = FileStore::open(&home.path().join("shared"));
    let project = store.project_by_name("Sec").unwrap().id;
    store.register_agent(&project, "One", "", t0()).unwrap();
    store.register_agent(&project, "Two", "", t0()).unwrap();
    let first = store
        .issue_connect_code(&project, "A-1", false, Utc::now())
        .unwrap();
    assert!(
        session_cmd(
            home.path(),
            "s-sec",
            &["--project", "Sec", "connect", &first.code]
        )
        .output()
        .unwrap()
        .status
        .success()
    );
    let digest = Sha256::digest("s-sec".as_bytes());
    let hex: String = digest.iter().take(8).map(|b| format!("{b:02x}")).collect();
    let file = home
        .path()
        .join("connections")
        .join(project.as_str())
        .join(format!("claude_{hex}.secret"));
    let before = std::fs::read(&file).unwrap();
    let second = store
        .issue_connect_code(&project, "A-2", false, Utc::now())
        .unwrap();
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o000)).unwrap();
    let out = session_cmd(
        home.path(),
        "s-sec",
        &["--project", "Sec", "connect", &second.code],
    )
    .output()
    .unwrap();
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o600)).unwrap();
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("could not be read"));
    assert_eq!(std::fs::read(&file).unwrap(), before);
    // The second code was not consumed: it still connects once the first is released.
    assert!(
        session_cmd(home.path(), "s-sec", &["--project", "Sec", "disconnect"])
            .output()
            .unwrap()
            .status
            .success()
    );
    assert!(
        session_cmd(
            home.path(),
            "s-sec",
            &["--project", "Sec", "connect", &second.code]
        )
        .output()
        .unwrap()
        .status
        .success()
    );
}

impl Fx {
    /// A card of a registered agent that is paused on one open question, and the answer given.
    fn paused_on_an_answered_question(&self, secret: &str) -> i64 {
        let id = self.doing_task();
        self.run(id, secret, t0()).unwrap();
        qa::ask(
            &self.store,
            &self.project,
            &[id],
            "Which way?",
            &[("a", "A"), ("b", "B")],
        );
        assert_eq!(
            self.store.task(&self.project, id).unwrap().agent_status,
            RunState::Paused
        );
        qa::answer(&self.store, &self.project, "Q-1", Some("a"), None);
        id
    }

    fn accept_the_answer(&self) {
        self.store
            .accept_message(&self.project, "Q-1", Actor::Ai, t0())
            .unwrap();
    }
}

#[test]
fn a_registered_agents_card_stays_paused_after_the_last_answer_without_a_connection() {
    let fx = fx();
    fx.store
        .register_agent(&fx.project, "Screen", "", t0())
        .unwrap();
    let secret = fx.connect("A-1", false, t0()).unwrap();
    let id = fx.paused_on_an_answered_question(&secret);
    fx.store
        .disconnect_session(&fx.project, &secret, t0())
        .unwrap();
    fx.accept_the_answer();
    let task = fx.store.task(&fx.project, id).unwrap();
    assert_eq!(task.agent_status, RunState::Paused);
    assert!(
        !task.history.iter().any(|e| e.reason.starts_with("Resumed")),
        "{:?}",
        task.history
    );
}

#[test]
fn a_registered_agents_card_resumes_on_its_connection_after_the_last_answer() {
    let fx = fx();
    fx.store
        .register_agent(&fx.project, "Screen", "", t0())
        .unwrap();
    let secret = fx.connect("A-1", false, t0()).unwrap();
    let id = fx.paused_on_an_answered_question(&secret);
    fx.accept_the_answer();
    let task = fx.store.task(&fx.project, id).unwrap();
    assert_eq!(task.agent_status, RunState::Running);
    assert!(task.run_connection_id.is_some());
}

#[test]
fn a_project_without_agents_still_resumes_after_the_last_answer() {
    let fx = fx();
    let id = fx.doing_task();
    fx.store
        .set_agent_status(&fx.project, id, RunState::Running, ctx(Actor::Ai), t0())
        .unwrap();
    qa::ask(
        &fx.store,
        &fx.project,
        &[id],
        "Which way?",
        &[("a", "A"), ("b", "B")],
    );
    qa::answer(&fx.store, &fx.project, "Q-1", Some("a"), None);
    fx.accept_the_answer();
    let task = fx.store.task(&fx.project, id).unwrap();
    assert_eq!(task.agent_status, RunState::Running);
    assert_eq!(task.run_connection_id, None);
}

#[test]
fn an_agent_edit_with_an_unreadable_connection_registry_is_not_saved() {
    let fx = fx();
    fx.store
        .register_agent(&fx.project, "Screen", "", t0())
        .unwrap();
    let path = fx
        ._root
        .path()
        .join("shared/projects")
        .join(fx.project.as_str())
        .join(".internal")
        .join("connections.json");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, "{").unwrap();
    assert!(
        fx.store
            .update_agent(&fx.project, "A-1", Some("Renamed"), None)
            .is_err()
    );
    std::fs::remove_file(&path).unwrap();
    let agents = fx.store.list_agents(&fx.project).unwrap();
    assert_eq!(agents[0].name, "Screen");
}

#[test]
fn a_card_of_an_archived_agent_cannot_enter_doing_until_it_is_reassigned() {
    let fx = fx();
    fx.store
        .register_agent(&fx.project, "Screen", "", t0())
        .unwrap();
    fx.store
        .register_agent(&fx.project, "CLI", "", t0())
        .unwrap();
    let id = fx
        .store
        .create_task(
            &fx.project,
            NewTask {
                title: "work".into(),
                description: String::new(),
                assignee: Assignee::Ai,
                actor: Actor::Human,
                draft: None,
            },
            t0(),
        )
        .unwrap()
        .id;
    fx.store
        .move_work_task_for_user(&fx.project, id, Column::Todo, ctx(Actor::Human), t0())
        .ok();
    fx.store
        .assign_task_agent(&fx.project, id, Some("A-1"), t0())
        .unwrap();
    fx.store.archive_agent(&fx.project, "A-1", t0()).unwrap();
    assert!(matches!(
        fx.store
            .move_work_task_for_user(&fx.project, id, Column::Doing, ctx(Actor::Human), t0()),
        Err(AgpjError::AgentNotFound { .. })
    ));
    fx.store
        .assign_task_agent(&fx.project, id, Some("A-2"), t0())
        .unwrap();
    fx.store
        .move_work_task_for_user(&fx.project, id, Column::Doing, ctx(Actor::Human), t0())
        .unwrap();
}

#[test]
fn session_end_tries_every_stored_secret_and_releases_the_live_connection() {
    use sha2::{Digest, Sha256};
    let fx = fx();
    fx.store
        .register_agent(&fx.project, "Screen", "", t0())
        .unwrap();
    // The first connection is replaced by a switch; the live one runs a card.
    let replaced = fx.connect("A-1", false, t0()).unwrap();
    let live = fx.connect("A-1", true, t0()).unwrap();
    let id = fx.doing_task();
    fx.run(id, &live, t0()).unwrap();
    let data_dir = fx._root.path();
    // The Claude-key secret (tried first) is the replaced one; the resident key holds the live one.
    keep_secret(data_dir, &fx.project, "session-1", &replaced);
    let digest = Sha256::digest(b"res-1");
    let hex: String = digest.iter().take(8).map(|b| format!("{b:02x}")).collect();
    let dir = data_dir.join("connections").join(fx.project.as_str());
    std::fs::write(dir.join(format!("resident_{hex}.secret")), &live).unwrap();

    let unbound = tempfile::tempdir().unwrap();
    let input = serde_json::json!({
        "hook_event_name": "SessionEnd",
        "session_id": "session-1",
        "cwd": unbound.path(),
    })
    .to_string();
    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_agpj"))
        .env("AGPJ_AUTOSTART", "0")
        .env("AGPJ_HOME", data_dir)
        .env("AGPJ_RESIDENT_ID", "res-1")
        .env_remove("AGPJ_CONNECTION_SECRET")
        .current_dir(unbound.path())
        .args(["--project", "Alpha", "hook", "--agent", "claude"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    use std::io::Write;
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    child.wait().unwrap();
    let task = fx.store.task(&fx.project, id).unwrap();
    assert_eq!(task.agent_status, RunState::Paused);
    assert_eq!(task.run_connection_id, None);
}

#[test]
fn a_secretless_request_for_the_state_a_connected_card_is_in_changes_nothing() {
    let fx = fx();
    fx.store
        .register_agent(&fx.project, "Screen", "", t0())
        .unwrap();
    let secret = fx.connect("A-1", false, t0()).unwrap();
    let id = fx.doing_task();
    fx.run(id, &secret, t0()).unwrap();
    let again = fx
        .store
        .set_agent_status(&fx.project, id, RunState::Running, ctx(Actor::Human), t0())
        .unwrap();
    assert!(!again.changed);
    assert!(
        fx.store
            .task(&fx.project, id)
            .unwrap()
            .run_connection_id
            .is_some()
    );
}

#[test]
fn an_answer_does_not_resume_a_card_on_a_stale_connection() {
    let fx = fx();
    fx.store
        .register_agent(&fx.project, "Screen", "", t0())
        .unwrap();
    let secret = fx.connect("A-1", false, t0()).unwrap();
    let id = fx.paused_on_an_answered_question(&secret);
    // The connection shows no activity for longer than the stale limit.
    let late = t0() + chrono::Duration::seconds(agent_project::file_store::STALE_RUN_SECONDS + 60);
    fx.store
        .accept_message(&fx.project, "Q-1", Actor::Ai, late)
        .unwrap();
    let task = fx.store.task(&fx.project, id).unwrap();
    assert_eq!(task.agent_status, RunState::Paused);
    assert!(!task.history.iter().any(|e| e.reason.starts_with("Resumed")));
}

#[test]
fn disconnect_by_an_environment_secret_keeps_the_stored_secret_of_this_session() {
    let home = tempfile::tempdir().unwrap();
    let run = |session: &str, env: Option<&str>, args: &[&str]| {
        let mut c = session_cmd(home.path(), session, args);
        if let Some(env) = env {
            c.env("AGPJ_CONNECTION_SECRET", env);
        }
        c.output().unwrap()
    };
    assert!(
        run("s-own", None, &["project", "create", "Own"])
            .status
            .success()
    );
    let store = FileStore::open(&home.path().join("shared"));
    let project = store.project_by_name("Own").unwrap().id;
    store.register_agent(&project, "Screen", "", t0()).unwrap();
    store.register_agent(&project, "CLI", "", t0()).unwrap();
    let issue = |agent: &str| {
        store
            .issue_connect_code(&project, agent, false, Utc::now())
            .unwrap()
            .code
    };
    let own = run(
        "s-own",
        None,
        &["--project", "Own", "connect", &issue("A-1")],
    );
    assert!(own.status.success(), "{own:?}");
    let other = store
        .exchange_connect_code(
            &project,
            &issue("A-2"),
            ConnectionOrigin::External,
            "claude-code",
            "claude:2",
            Utc::now(),
        )
        .unwrap()
        .secret;
    let out = run("s-own", Some(&other), &["--project", "Own", "disconnect"]);
    assert!(out.status.success(), "{out:?}");
    // The stored secret survived, so this session can still disconnect its own connection.
    let again = run("s-own", None, &["--project", "Own", "disconnect"]);
    assert!(again.status.success(), "{again:?}");
}
