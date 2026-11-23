//! Awaiting an answer is a derived flag, not a run state: it follows the open questions a task
//! refers to, for any assignee, lane, and run state, and never changes the stored run state.

use agent_project::domain::{Actor, Assignee, ChangeContext, Column, NewTask, ProjectId, RunState};
use agent_project::file_store::FileStore;
use chrono::{TimeZone, Utc};

#[path = "support/qa.rs"]
mod qa;

fn now() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, 4, 0, 0, 0).unwrap()
}

fn ctx(actor: Actor) -> ChangeContext {
    ChangeContext {
        actor,
        reason: "test".into(),
    }
}

struct Fx {
    root: tempfile::TempDir,
    store: FileStore,
    project: ProjectId,
}

fn fx() -> Fx {
    let root = tempfile::tempdir().unwrap();
    let store = FileStore::open(&root.path().join("shared"));
    let project = store.create_project("Alpha", now()).unwrap().id;
    Fx {
        root,
        store,
        project,
    }
}

impl Fx {
    fn task(&self, assignee: Assignee) -> i64 {
        self.store
            .create_task(
                &self.project,
                NewTask {
                    title: "t".into(),
                    description: String::new(),
                    assignee,
                    actor: Actor::Human,
                    draft: None,
                },
                now(),
            )
            .unwrap()
            .id
    }

    fn mv(&self, id: i64, to: Column) {
        self.store
            .move_work_task_for_user(&self.project, id, to, ctx(Actor::Human), now())
            .unwrap();
    }

    fn run(&self, id: i64) {
        self.mv(id, Column::Doing);
        self.store
            .set_agent_status(&self.project, id, RunState::Running, ctx(Actor::Ai), now())
            .unwrap();
    }

    fn ask(&self, id: i64) {
        qa::ask(
            &self.store,
            &self.project,
            &[id],
            "Which way?",
            &[("a", "A"), ("b", "B")],
        );
    }

    fn view(&self, id: i64) -> agent_project::file_store::model::TaskView {
        self.store.task(&self.project, id).unwrap()
    }
}

#[test]
fn the_flag_covers_any_assignee_and_lane_and_leaves_the_run_state_alone() {
    let fx = fx();
    let human = fx.task(Assignee::Human);
    let ai_todo = fx.task(Assignee::Ai);
    let ai_done = fx.task(Assignee::Ai);
    let backlog = fx.task(Assignee::Ai);
    for id in [human, ai_todo, ai_done, backlog] {
        assert!(!fx.view(id).awaiting_answer);
        fx.ask(id);
    }
    fx.mv(ai_done, Column::Done);
    fx.mv(backlog, Column::Backlog);
    for (id, lane) in [
        (human, Column::Todo),
        (ai_todo, Column::Todo),
        (ai_done, Column::Done),
        (backlog, Column::Backlog),
    ] {
        let view = fx.view(id);
        assert!(view.awaiting_answer, "#{id} in {lane:?}");
        assert_eq!(view.task_status, lane);
        assert_eq!(view.agent_status, RunState::Idle, "#{id}");
        assert_eq!(view.pending_reason.as_deref(), Some("Which way?"));
        assert_eq!(view.pending_choices.len(), 2);
    }
}

#[test]
fn an_archived_task_does_not_wait() {
    let fx = fx();
    let id = fx.task(Assignee::Ai);
    fx.ask(id);
    fx.mv(id, Column::Done);
    assert!(fx.view(id).awaiting_answer);
    fx.store.archive_task(&fx.project, id, now()).unwrap();
    let view = fx.view(id);
    assert!(view.archived_at.is_some());
    assert!(!view.awaiting_answer);
    assert!(view.pending_request_id.is_none());
}

#[test]
fn answering_clears_the_flag_and_the_json_names_it() {
    let fx = fx();
    let id = fx.task(Assignee::Human);
    fx.ask(id);
    let json = serde_json::to_value(fx.view(id)).unwrap();
    assert_eq!(json["awaitingAnswer"], true);
    assert_eq!(json["agentStatus"], "idle");
    qa::answer(&fx.store, &fx.project, "Q-1", Some("a"), None);
    let json = serde_json::to_value(fx.view(id)).unwrap();
    assert_eq!(json["awaitingAnswer"], false);
}

#[test]
fn a_waiting_task_is_never_a_next_task() {
    let fx = fx();
    let waiting = fx.task(Assignee::Ai);
    let free = fx.task(Assignee::Ai);
    let table = |fx: &Fx| {
        let snapshot = fx.store.board(&fx.project).unwrap();
        agent_project::progress::build(&snapshot, &fx.root.path().join("shared"), now())
    };
    let ids = |table: &agent_project::progress::Progress| -> Vec<i64> {
        table.next_tasks.iter().map(|t| t.id).collect()
    };
    assert!(ids(&table(&fx)).contains(&waiting));
    fx.ask(waiting);
    let after = table(&fx);
    assert!(!ids(&after).contains(&waiting));
    assert!(ids(&after).contains(&free));
}

#[test]
fn asking_pauses_a_running_doing_task_and_nothing_else() {
    let fx = fx();
    let running = fx.task(Assignee::Ai);
    fx.run(running);
    let paused = fx.task(Assignee::Ai);
    fx.run(paused);
    fx.store
        .set_agent_status(
            &fx.project,
            paused,
            RunState::Paused,
            ctx(Actor::Human),
            now(),
        )
        .unwrap();
    let idle_doing = fx.task(Assignee::Ai);
    fx.mv(idle_doing, Column::Doing);
    let todo = fx.task(Assignee::Ai);

    qa::ask(
        &fx.store,
        &fx.project,
        &[running, paused, idle_doing, todo],
        "Which way?",
        &[("a", "A"), ("b", "B")],
    );

    let view = fx.view(running);
    assert_eq!(view.agent_status, RunState::Paused);
    assert!(view.awaiting_answer);
    assert!(
        view.history
            .iter()
            .any(|e| e.reason == "Waiting for the answers to Q-1."),
        "{:?}",
        view.history
    );
    assert_eq!(fx.view(paused).agent_status, RunState::Paused);
    assert_eq!(fx.view(idle_doing).agent_status, RunState::Idle);
    assert_eq!(fx.view(todo).agent_status, RunState::Idle);
    for id in [paused, idle_doing, todo] {
        assert!(fx.view(id).awaiting_answer);
    }
}

#[test]
fn linking_an_open_question_pauses_a_running_task() {
    let fx = fx();
    let asker = fx.task(Assignee::Ai);
    let running = fx.task(Assignee::Ai);
    fx.run(running);
    fx.ask(asker);
    fx.store
        .link_questions(
            &fx.project,
            running,
            &["Q-1".to_string()],
            true,
            Actor::Ai,
            now(),
        )
        .unwrap();
    let view = fx.view(running);
    assert_eq!(view.agent_status, RunState::Paused);
    assert!(view.awaiting_answer);
}

#[test]
fn a_stored_ran_or_awaiting_answer_state_is_an_invalid_file() {
    let fx = fx();
    let id = fx.task(Assignee::Ai);
    let path = fx
        .root
        .path()
        .join("shared/projects")
        .join(fx.project.as_str())
        .join("tasks")
        .join(id.to_string())
        .join("task.json");
    let original = std::fs::read_to_string(&path).unwrap();
    for state in ["ran", "awaiting_answer"] {
        let broken = original.replace(
            "\"agentStatus\": \"idle\"",
            &format!("\"agentStatus\": \"{state}\""),
        );
        assert_ne!(broken, original);
        std::fs::write(&path, broken).unwrap();
        assert!(fx.store.task(&fx.project, id).is_err(), "{state}");
    }
}

#[test]
fn an_agent_cannot_pause_a_task_but_a_person_can() {
    let fx = fx();
    let id = fx.task(Assignee::Ai);
    fx.run(id);
    let err = fx
        .store
        .set_agent_status(&fx.project, id, RunState::Paused, ctx(Actor::Ai), now())
        .unwrap_err();
    assert!(
        matches!(err, agent_project::error::AgpjError::HumanOnly { .. }),
        "{err:?}"
    );
    assert_eq!(fx.view(id).agent_status, RunState::Running);
    fx.store
        .set_agent_status(&fx.project, id, RunState::Paused, ctx(Actor::Human), now())
        .unwrap();
    assert_eq!(fx.view(id).agent_status, RunState::Paused);
}

#[test]
fn an_agent_cannot_move_doing_back_to_todo_but_a_person_can() {
    let fx = fx();
    let id = fx.task(Assignee::Ai);
    fx.run(id);
    let err = fx
        .store
        .move_work_task(&fx.project, id, Column::Todo, ctx(Actor::Ai), now())
        .unwrap_err();
    assert!(
        matches!(err, agent_project::error::AgpjError::HumanOnly { .. }),
        "{err:?}"
    );
    assert_eq!(fx.view(id).task_status, Column::Doing);
    fx.mv(id, Column::Todo);
    assert_eq!(fx.view(id).task_status, Column::Todo);
}

#[test]
fn accepting_the_last_answer_resumes_a_paused_doing_task() {
    let fx = fx();
    let id = fx.task(Assignee::Ai);
    fx.run(id);
    qa::ask(
        &fx.store,
        &fx.project,
        &[id],
        "First?",
        &[("a", "A"), ("b", "B")],
    );
    qa::ask(
        &fx.store,
        &fx.project,
        &[id],
        "Second?",
        &[("a", "A"), ("b", "B")],
    );
    assert_eq!(fx.view(id).agent_status, RunState::Paused);

    qa::answer(&fx.store, &fx.project, "Q-1", Some("a"), None);
    fx.store
        .accept_message(&fx.project, "Q-1", Actor::Ai, now())
        .unwrap();
    // Another open question keeps it paused.
    assert_eq!(fx.view(id).agent_status, RunState::Paused);

    qa::answer(&fx.store, &fx.project, "Q-2", Some("b"), None);
    fx.store
        .accept_message(&fx.project, "Q-2", Actor::Ai, now())
        .unwrap();
    let view = fx.view(id);
    assert_eq!(view.agent_status, RunState::Running);
    assert!(!view.awaiting_answer);
    assert!(
        view.history
            .iter()
            .any(|e| e.reason == "Resumed after the answers to Q-2."),
        "{:?}",
        view.history
    );
}

#[test]
fn a_wait_that_hands_over_the_answer_resumes_and_other_tasks_are_left_alone() {
    let fx = fx();
    let waiting = fx.task(Assignee::Ai);
    fx.run(waiting);
    let by_hand = fx.task(Assignee::Ai);
    fx.run(by_hand);
    fx.store
        .set_agent_status(
            &fx.project,
            by_hand,
            RunState::Paused,
            ctx(Actor::Human),
            now(),
        )
        .unwrap();
    qa::ask(
        &fx.store,
        &fx.project,
        &[waiting],
        "Which?",
        &[("a", "A"), ("b", "B")],
    );
    qa::answer(&fx.store, &fx.project, "Q-1", Some("a"), None);
    let written = fx
        .store
        .accept_answers(&fx.project, &["Q-1".to_string()], now())
        .unwrap();
    assert_eq!(written, vec!["Q-1".to_string()]);
    assert_eq!(fx.view(waiting).agent_status, RunState::Running);
    // A task that a person paused and that never asked anything stays paused.
    assert_eq!(fx.view(by_hand).agent_status, RunState::Paused);
}

#[test]
fn next_tasks_carry_the_run_state_and_list_a_stalled_task_first() {
    let fx = fx();
    let running = fx.task(Assignee::Ai);
    fx.run(running);
    let idle_doing = fx.task(Assignee::Ai);
    fx.mv(idle_doing, Column::Doing);
    let todo = fx.task(Assignee::Ai);
    let shared = fx.root.path().join("shared");
    let table = |minutes: i64| {
        let snapshot = fx.store.board(&fx.project).unwrap();
        agent_project::progress::build(
            &snapshot,
            &shared,
            now() + chrono::Duration::minutes(minutes),
        )
    };

    let fresh = table(5);
    let why = |t: &agent_project::progress::Progress, id: i64| {
        t.next_tasks
            .iter()
            .find(|n| n.id == id)
            .map(|n| (n.why, n.agent_status))
    };
    assert_eq!(why(&fresh, running), None);
    assert_eq!(
        why(&fresh, idle_doing),
        Some((agent_project::progress::NextReason::Resume, RunState::Idle))
    );
    assert_eq!(
        why(&fresh, todo),
        Some((
            agent_project::progress::NextReason::NextInEpic,
            RunState::Idle
        ))
    );

    let stale = table(45);
    let first = &stale.next_tasks[0];
    assert_eq!(first.id, running);
    assert_eq!(first.why, agent_project::progress::NextReason::Stalled);
    assert_eq!(first.agent_status, RunState::Running);
    let json = serde_json::to_value(first).unwrap();
    assert_eq!(json["why"], "stalled");
    assert_eq!(json["agentStatus"], "running");
}

impl Fx {
    fn task_files(&self, id: i64) -> Vec<(std::path::PathBuf, Vec<u8>)> {
        let dir = self
            .root
            .path()
            .join("shared/projects")
            .join(self.project.as_str())
            .join("tasks")
            .join(id.to_string());
        std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p.is_file())
            .map(|p| {
                let bytes = std::fs::read(&p).unwrap();
                (p, bytes)
            })
            .collect()
    }

    fn resumes(&self, id: i64) -> usize {
        self.view(id)
            .history
            .iter()
            .filter(|e| e.reason == "Resumed after the answers to Q-1.")
            .count()
    }

    fn person_pauses(&self, id: i64) {
        self.store
            .set_agent_status(
                &self.project,
                id,
                RunState::Paused,
                ctx(Actor::Human),
                now(),
            )
            .unwrap();
    }
}

fn asked_and_answered(fx: &Fx) -> i64 {
    let id = fx.task(Assignee::Ai);
    fx.run(id);
    qa::ask(
        &fx.store,
        &fx.project,
        &[id],
        "Which?",
        &[("a", "A"), ("b", "B")],
    );
    qa::answer(&fx.store, &fx.project, "Q-1", Some("a"), None);
    id
}

#[test]
fn a_replayed_acceptance_leaves_a_pause_a_person_made_after_the_resume() {
    let fx = fx();
    let id = asked_and_answered(&fx);
    fx.store
        .accept_message(&fx.project, "Q-1", Actor::Ai, now())
        .unwrap();
    assert_eq!(fx.view(id).agent_status, RunState::Running);

    fx.person_pauses(id);
    let again = fx
        .store
        .accept_message(&fx.project, "Q-1", Actor::Ai, now());
    assert!(matches!(
        again,
        Err(agent_project::error::AgpjError::AnswerAccepted { .. })
    ));
    let written = fx
        .store
        .accept_answers(&fx.project, &["Q-1".to_string()], now())
        .unwrap();
    assert!(written.is_empty());
    assert_eq!(fx.view(id).agent_status, RunState::Paused);
    assert_eq!(fx.resumes(id), 1);
}

#[test]
fn a_replayed_acceptance_finishes_a_resume_that_never_happened() {
    for via_wait in [false, true] {
        let fx = fx();
        let id = asked_and_answered(&fx);
        // The acceptance is saved but the task save failed: restore the task as it was.
        let before = fx.task_files(id);
        fx.store
            .accept_message(&fx.project, "Q-1", Actor::Ai, now())
            .unwrap();
        for (path, bytes) in &before {
            std::fs::write(path, bytes).unwrap();
        }
        assert_eq!(fx.view(id).agent_status, RunState::Paused);
        assert_eq!(fx.resumes(id), 0);

        let replay = |fx: &Fx| {
            if via_wait {
                fx.store
                    .accept_answers(&fx.project, &["Q-1".to_string()], now())
                    .unwrap();
            } else {
                let _ = fx
                    .store
                    .accept_message(&fx.project, "Q-1", Actor::Ai, now());
            }
        };
        replay(&fx);
        assert_eq!(fx.view(id).agent_status, RunState::Running, "{via_wait}");
        assert_eq!(fx.resumes(id), 1);
        // A further replay adds nothing, even when a person paused the task in between.
        fx.person_pauses(id);
        replay(&fx);
        assert_eq!(fx.view(id).agent_status, RunState::Paused);
        assert_eq!(fx.resumes(id), 1);
    }
}
