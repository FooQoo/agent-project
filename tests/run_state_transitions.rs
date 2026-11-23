//! The run state of an AI task: the changes an agent may ask for in Doing, what a lane move
//! makes of the state, that a task that has run never goes back to idle, and that a person's
//! task stays idle.

use agent_project::domain::{
    Actor, Assignee, ChangeContext, Column, NewTask, ProjectId, RunState, check_state_change,
};
use agent_project::error::AgpjError;
use agent_project::file_store::FileStore;
use chrono::{TimeZone, Utc};

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
    _root: tempfile::TempDir,
    store: FileStore,
    project: ProjectId,
}

fn fx() -> Fx {
    let root = tempfile::tempdir().unwrap();
    let store = FileStore::open(&root.path().join("shared"));
    let project = store.create_project("Alpha", now()).unwrap().id;
    Fx {
        _root: root,
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

    fn mv(&self, id: i64, to: Column) -> RunState {
        self.store
            .move_work_task_for_user(&self.project, id, to, ctx(Actor::Human), now())
            .unwrap()
            .value
            .agent_status
    }

    fn pause(&self, id: i64) -> RunState {
        self.store
            .set_agent_status(
                &self.project,
                id,
                RunState::Paused,
                ctx(Actor::Human),
                now(),
            )
            .unwrap()
            .value
            .agent_status
    }

    fn state(&self, id: i64, state: RunState) -> Result<RunState, AgpjError> {
        self.store
            .set_agent_status(&self.project, id, state, ctx(Actor::Ai), now())
            .map(|c| c.value.agent_status)
    }
}

#[test]
fn only_the_three_changes_in_doing_are_allowed() {
    use RunState::*;
    let states = [Idle, Running, Paused];
    for from in states {
        for to in states {
            let allowed = matches!(
                (from, to),
                (Idle, Running) | (Running, Paused) | (Paused, Running)
            );
            assert_eq!(
                check_state_change(from, to).is_ok(),
                allowed,
                "{from:?} -> {to:?}"
            );
        }
    }
}

#[test]
fn an_agent_runs_pauses_and_resumes_in_doing() {
    let fx = fx();
    let id = fx.task(Assignee::Ai);
    assert_eq!(fx.mv(id, Column::Doing), RunState::Idle);
    assert_eq!(fx.state(id, RunState::Running).unwrap(), RunState::Running);
    assert_eq!(fx.pause(id), RunState::Paused);
    assert_eq!(fx.state(id, RunState::Running).unwrap(), RunState::Running);
}

#[test]
fn an_agent_pauses_its_own_running_task() {
    let fx = fx();
    let id = fx.task(Assignee::Ai);
    fx.mv(id, Column::Doing);
    fx.state(id, RunState::Running).unwrap();
    assert_eq!(fx.state(id, RunState::Paused).unwrap(), RunState::Paused);
    // Pausing again is not a change, and the agent can resume.
    assert_eq!(fx.state(id, RunState::Paused).unwrap(), RunState::Paused);
    assert_eq!(fx.state(id, RunState::Running).unwrap(), RunState::Running);
}

#[test]
fn an_agent_cannot_pause_a_person_task_or_one_outside_doing() {
    let fx = fx();
    let person = fx.task(Assignee::Human);
    fx.mv(person, Column::Doing);
    assert!(fx.state(person, RunState::Paused).is_err());
    let todo = fx.task(Assignee::Ai);
    assert!(fx.state(todo, RunState::Paused).is_err());
}

#[test]
fn refused_changes_leave_the_state_as_it_was() {
    let fx = fx();
    let id = fx.task(Assignee::Ai);
    fx.mv(id, Column::Doing);
    // An idle task cannot be paused, by an agent or by anyone.
    let err = fx.state(id, RunState::Paused).unwrap_err();
    assert!(
        matches!(err, AgpjError::InvalidStateChange { .. }),
        "{err:?}"
    );
    fx.state(id, RunState::Running).unwrap();
    // running -> idle: a task that has run never goes back to idle.
    let err = fx.state(id, RunState::Idle).unwrap_err();
    assert!(
        matches!(err, AgpjError::InvalidStateChange { .. }),
        "{err:?}"
    );
    fx.pause(id);
    let err = fx.state(id, RunState::Idle).unwrap_err();
    assert!(
        matches!(err, AgpjError::InvalidStateChange { .. }),
        "{err:?}"
    );
    // `ran` and `awaiting_answer` are no run states: they are unknown values.
    for name in ["ran", "awaiting_answer"] {
        assert!(name.parse::<RunState>().is_err(), "{name}");
    }
    assert_eq!(fx.pause(id), RunState::Paused);
}

#[test]
fn a_task_that_never_ran_stays_idle_when_it_moves() {
    let fx = fx();
    let id = fx.task(Assignee::Ai);
    assert_eq!(fx.mv(id, Column::Doing), RunState::Idle);
    assert_eq!(fx.mv(id, Column::Todo), RunState::Idle);
    assert_eq!(fx.mv(id, Column::Done), RunState::Idle);
}

#[test]
fn entering_or_leaving_doing_resets_the_state_to_idle() {
    let fx = fx();
    let id = fx.task(Assignee::Ai);
    fx.mv(id, Column::Doing);
    fx.state(id, RunState::Running).unwrap();
    assert_eq!(fx.mv(id, Column::Todo), RunState::Idle);
    assert_eq!(fx.mv(id, Column::Doing), RunState::Idle);
    fx.state(id, RunState::Running).unwrap();
    fx.pause(id);
    assert_eq!(fx.mv(id, Column::Done), RunState::Idle);
    assert_eq!(fx.mv(id, Column::Doing), RunState::Idle);
}

#[test]
fn a_done_task_cannot_be_given_a_run_state() {
    let fx = fx();
    let id = fx.task(Assignee::Ai);
    fx.mv(id, Column::Doing);
    fx.state(id, RunState::Running).unwrap();
    fx.mv(id, Column::Done);
    assert!(fx.state(id, RunState::Idle).is_err());
    assert!(fx.state(id, RunState::Running).is_err());
}

#[test]
fn a_person_task_stays_idle() {
    let fx = fx();
    let id = fx.task(Assignee::Human);
    assert_eq!(fx.mv(id, Column::Doing), RunState::Idle);
    let err = fx.state(id, RunState::Running).unwrap_err();
    assert!(matches!(err, AgpjError::InvalidStateForAssignee), "{err:?}");
    let err = fx
        .store
        .set_agent_status(&fx.project, id, RunState::Paused, ctx(Actor::Human), now())
        .unwrap_err();
    assert!(matches!(err, AgpjError::InvalidStateForAssignee), "{err:?}");
    assert_eq!(fx.mv(id, Column::Done), RunState::Idle);
}

#[test]
fn archiving_and_restoring_leave_the_task_idle() {
    let fx = fx();
    let id = fx.task(Assignee::Ai);
    fx.mv(id, Column::Doing);
    fx.state(id, RunState::Running).unwrap();
    fx.mv(id, Column::Done);
    let archived = fx.store.archive_task(&fx.project, id, now()).unwrap().value;
    assert_eq!(archived.agent_status, RunState::Idle);
    let restored = fx.store.restore_task(&fx.project, id, now()).unwrap().value;
    assert_eq!(restored.agent_status, RunState::Idle);
}
