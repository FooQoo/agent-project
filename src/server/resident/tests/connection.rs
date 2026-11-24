use chrono::Utc;

use super::health::{fast, no_projects};
use super::push::{board, turn_ends};
use super::*;
use crate::agent::connection_store::SECRET_ENV;
use crate::domain::{Actor, Assignee, ChangeContext, Column, RunState};
use crate::file_store::ConnectionOrigin;
use crate::server::resident::driver::mock::{Call, Calls, Step, factory_replying};

fn connection_secrets(calls: &Calls) -> Vec<String> {
    calls
        .lock()
        .unwrap()
        .iter()
        .filter_map(|call| match call {
            Call::Start { env, .. } => env.iter().find(|(name, _)| name == SECRET_ENV),
            _ => None,
        })
        .map(|(_, secret)| secret.clone())
        .collect()
}

#[test]
fn resident_reuses_its_connection_after_relaunch_and_server_restart_then_stop_revokes_it() {
    let _env = ENV.lock().unwrap_or_else(|e| e.into_inner());
    let board = board();
    let (factory, calls) = factory_replying(
        vec![
            Step::Sleep(Duration::from_millis(100)),
            Step::Event(driver::DriverEvent::Exited {
                code: Some(1),
                signal: None,
            }),
        ],
        Vec::new(),
    );
    let fx = fixture_with(fast(), factory);
    fx.launcher.attach_board(&board.shared);
    fx.launcher.start(no_projects);
    fx.launcher
        .launch(&board.id, "Demo", fx.folder.path(), Tool::ClaudeCode, None)
        .unwrap();
    wait_until("a second launch", || connection_secrets(&calls).len() >= 2);
    fx.launcher.shutdown();
    let secrets = connection_secrets(&calls);
    assert!(secrets.iter().all(|secret| secret == &secrets[0]));

    let (factory, calls) = factory_replying(turn_ends(), Vec::new());
    let restarted = Launcher::with_factory(
        fx.registry.clone(),
        fx.launcher.executables.clone(),
        &fx.launcher.data_dir,
        fx.launcher.stop_grace,
        fast(),
        factory,
    );
    restarted.attach_board(&board.shared);
    restarted
        .launch(&board.id, "Demo", fx.folder.path(), Tool::Codex, None)
        .unwrap();
    let secret = connection_secrets(&calls).remove(0);
    assert_eq!(secret, secrets[0]);

    let id = board.task("Run with the board connection", Assignee::Ai);
    let start = || ChangeContext {
        actor: Actor::Ai,
        reason: "Start".into(),
    };
    board
        .store
        .move_work_task(&board.id, id, Column::Doing, start(), Utc::now())
        .unwrap();
    board
        .store
        .set_agent_status_connected(
            &board.id,
            id,
            RunState::Running,
            start(),
            &secret,
            Utc::now(),
        )
        .unwrap();
    assert!(restarted.stop().unwrap());
    assert!(matches!(
        board
            .store
            .verify_connection(&board.id, &secret, Utc::now()),
        Err(AgpjError::ConnectionReplaced)
    ));
    let task = board.store.task(&board.id, id).unwrap();
    assert_eq!(task.agent_status, RunState::Paused);
    assert_eq!(task.run_connection_id, None);
    assert!(!restarted.stop().unwrap());
}

#[test]
fn a_resident_launch_does_not_replace_an_external_connection_to_the_default_agent() {
    let _env = ENV.lock().unwrap_or_else(|e| e.into_inner());
    let board = board();
    board.task("Work for the default agent", Assignee::Ai);
    let agent = board.store.list_agents(&board.id).unwrap().remove(0);
    let code = board
        .store
        .issue_connect_code(&board.id, &agent.id, false, Utc::now())
        .unwrap()
        .code;
    let external = board
        .store
        .exchange_connect_code(
            &board.id,
            &code,
            ConnectionOrigin::External,
            "codex",
            "External session",
            Utc::now(),
        )
        .unwrap();
    let (factory, calls) = factory_replying(turn_ends(), Vec::new());
    let fx = fixture_with(fast(), factory);
    fx.launcher.attach_board(&board.shared);
    assert!(matches!(
        fx.launcher
            .launch(&board.id, "Demo", fx.folder.path(), Tool::ClaudeCode, None),
        Err(AgpjError::AgentBusy { .. })
    ));
    assert!(calls.lock().unwrap().is_empty());
    assert!(!fx.launcher.stop().unwrap());
    let connection = board
        .store
        .verify_connection(&board.id, &external.secret, Utc::now())
        .unwrap();
    assert_eq!(connection.connection_id, external.connection.id);
}
