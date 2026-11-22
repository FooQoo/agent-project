//! The run state of an AI task: the changes an agent may ask for in Doing, what a lane move
//! makes of the state, that a task that has run never goes back to idle, and that a person's
//! task stays idle.

use agent_project::domain::{
    Actor, Assignee, ChangeContext, Column, NewTask, ProjectId, RunState, check_state_change,
    state_after_move,
};
use agent_project::error::AgboardError;
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

    fn state(&self, id: i64, state: RunState) -> Result<RunState, AgboardError> {
        self.store
            .set_agent_status(&self.project, id, state, ctx(Actor::Ai), now())
            .map(|c| c.value.agent_status)
    }
}

#[test]
fn only_the_three_changes_in_doing_are_allowed() {
    use RunState::*;
    let states = [Idle, Running, Paused, Ran, AwaitingAnswer];
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
fn a_lane_move_decides_the_state_of_a_task_that_has_run() {
    use RunState::*;
    for to in [Column::Todo, Column::Doing, Column::Done] {
        assert_eq!(state_after_move(Idle, to), Idle);
        assert_eq!(state_after_move(AwaitingAnswer, to), Idle);
    }
    for from in [Running, Paused, Ran] {
        assert_eq!(state_after_move(from, Column::Doing), Paused);
        assert_eq!(state_after_move(from, Column::Todo), Ran);
        assert_eq!(state_after_move(from, Column::Done), Ran);
    }
}

#[test]
fn an_agent_runs_pauses_and_resumes_in_doing() {
    let fx = fx();
    let id = fx.task(Assignee::Ai);
    assert_eq!(fx.mv(id, Column::Doing), RunState::Idle);
    assert_eq!(fx.state(id, RunState::Running).unwrap(), RunState::Running);
    assert_eq!(fx.state(id, RunState::Paused).unwrap(), RunState::Paused);
    assert_eq!(fx.state(id, RunState::Running).unwrap(), RunState::Running);
}

#[test]
fn refused_changes_leave_the_state_as_it_was() {
    let fx = fx();
    let id = fx.task(Assignee::Ai);
    fx.mv(id, Column::Doing);
    // idle -> paused is not a change an agent may ask for.
    let err = fx.state(id, RunState::Paused).unwrap_err();
    assert!(
        matches!(err, AgboardError::InvalidStateChange { .. }),
        "{err:?}"
    );
    fx.state(id, RunState::Running).unwrap();
    // running -> idle: a task that has run never goes back to idle.
    let err = fx.state(id, RunState::Idle).unwrap_err();
    assert!(
        matches!(err, AgboardError::InvalidStateChange { .. }),
        "{err:?}"
    );
    fx.state(id, RunState::Paused).unwrap();
    let err = fx.state(id, RunState::Idle).unwrap_err();
    assert!(
        matches!(err, AgboardError::InvalidStateChange { .. }),
        "{err:?}"
    );
    // `ran` and `awaiting_answer` are never set directly.
    for state in [RunState::Ran, RunState::AwaitingAnswer] {
        let err = fx.state(id, state).unwrap_err();
        assert!(matches!(err, AgboardError::InvalidValue { .. }), "{err:?}");
    }
    assert_eq!(fx.state(id, RunState::Paused).unwrap(), RunState::Paused);
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
fn a_task_that_has_run_is_paused_in_doing_and_has_run_in_todo_and_done() {
    let fx = fx();
    let id = fx.task(Assignee::Ai);
    fx.mv(id, Column::Doing);
    fx.state(id, RunState::Running).unwrap();
    assert_eq!(fx.mv(id, Column::Todo), RunState::Ran);
    assert_eq!(fx.mv(id, Column::Doing), RunState::Paused);
    assert_eq!(fx.state(id, RunState::Running).unwrap(), RunState::Running);
    assert_eq!(fx.mv(id, Column::Done), RunState::Ran);
    assert_eq!(fx.mv(id, Column::Todo), RunState::Ran);
    assert_eq!(fx.mv(id, Column::Doing), RunState::Paused);
    assert_eq!(fx.mv(id, Column::Done), RunState::Ran);
}

#[test]
fn ran_cannot_be_asked_for_outside_a_lane_move_and_cannot_return_to_idle() {
    let fx = fx();
    let id = fx.task(Assignee::Ai);
    fx.mv(id, Column::Doing);
    fx.state(id, RunState::Running).unwrap();
    fx.mv(id, Column::Done);
    // A Done task cannot be changed by an agent, and a state in Done is refused for any request.
    assert!(fx.state(id, RunState::Idle).is_err());
    assert!(fx.state(id, RunState::Running).is_err());
}

#[test]
fn a_person_task_stays_idle() {
    let fx = fx();
    let id = fx.task(Assignee::Human);
    assert_eq!(fx.mv(id, Column::Doing), RunState::Idle);
    for state in [RunState::Running, RunState::Paused] {
        let err = fx.state(id, state).unwrap_err();
        assert!(
            matches!(err, AgboardError::InvalidStateForAssignee),
            "{err:?}"
        );
    }
    assert_eq!(fx.mv(id, Column::Done), RunState::Idle);
}

#[test]
fn archiving_keeps_has_run_and_leaves_an_idle_task_idle() {
    let fx = fx();
    let ran = fx.task(Assignee::Ai);
    fx.mv(ran, Column::Doing);
    fx.state(ran, RunState::Running).unwrap();
    fx.mv(ran, Column::Done);
    let never = fx.task(Assignee::Ai);
    fx.mv(never, Column::Done);
    let ran_view = fx
        .store
        .archive_task(&fx.project, ran, now())
        .unwrap()
        .value;
    let never_view = fx
        .store
        .archive_task(&fx.project, never, now())
        .unwrap()
        .value;
    assert_eq!(ran_view.agent_status, RunState::Ran);
    assert_eq!(never_view.agent_status, RunState::Idle);
}

#[test]
fn restoring_a_task_that_has_run_in_doing_pauses_it() {
    let fx = fx();
    let id = fx.task(Assignee::Ai);
    fx.mv(id, Column::Doing);
    fx.state(id, RunState::Running).unwrap();
    fx.store.archive_task(&fx.project, id, now()).unwrap();
    let restored = fx.store.restore_task(&fx.project, id, now()).unwrap().value;
    assert_eq!(restored.agent_status, RunState::Paused);
}
