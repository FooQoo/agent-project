//! The write guard (AC-1 to AC-3 of the task protection): every AI task has an agent, and from
//! Doing onward only a session connected as that agent can change it.

use agent_project::domain::{
    Actor, Assignee, AttachmentInput, ChangeContext, Column, NewTask, ProjectId, ResultKind,
    RunState,
};
use agent_project::error::AgpjError;
use agent_project::file_store::{ConnectionOrigin, FileStore, ResultInput, enter_session_writes};
use chrono::{DateTime, TimeZone, Utc};

#[path = "common/mod.rs"]
mod common;

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
    root: tempfile::TempDir,
    store: FileStore,
    project: ProjectId,
}

fn fx() -> Fx {
    let root = tempfile::tempdir().unwrap();
    let store = FileStore::open(&root.path().join("shared"));
    let project = store.create_project("Alpha", t0()).unwrap().id;
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
                    title: "作業".into(),
                    description: String::new(),
                    assignee,
                    actor: Actor::Human,
                    draft: None,
                },
                t0(),
            )
            .unwrap()
            .id
    }

    fn connect(&self, agent: &str) -> String {
        let code = self
            .store
            .issue_connect_code(&self.project, agent, false, t0())
            .unwrap();
        self.store
            .exchange_connect_code(
                &self.project,
                &code.code,
                ConnectionOrigin::External,
                "claude-code",
                "claude:1",
                t0(),
            )
            .unwrap()
            .secret
    }

    fn history_len(&self, id: i64) -> usize {
        self.store.task(&self.project, id).unwrap().history.len()
    }

    fn image_files(&self) -> usize {
        fn count(dir: &std::path::Path) -> usize {
            std::fs::read_dir(dir)
                .map(|entries| {
                    entries
                        .flatten()
                        .map(|e| {
                            let p = e.path();
                            if p.is_dir() {
                                count(&p)
                            } else {
                                usize::from(p.extension().is_some_and(|x| x == "png"))
                            }
                        })
                        .sum()
                })
                .unwrap_or(0)
        }
        count(self.root.path())
    }
}

fn input() -> ResultInput {
    ResultInput {
        kind: ResultKind::Progress,
        title: Some("Done a part".into()),
        summary: "A part is done.".into(),
        link: None,
        images: Vec::new(),
        key: None,
        published: None,
        plan: None,
    }
}

#[test]
fn every_ai_task_gets_the_default_agent_and_a_person_task_gets_none() {
    let fx = fx();
    let ai = fx.task(Assignee::Ai);
    let human = fx.task(Assignee::Human);
    let ai2 = fx.task(Assignee::Ai);
    let agents = fx.store.list_agents(&fx.project).unwrap();
    assert_eq!(agents.len(), 1);
    assert!(agents[0].is_default);
    assert_eq!(agents[0].name, "エージェント");
    let id = agents[0].id.clone();
    for task in [ai, ai2] {
        let view = fx.store.task(&fx.project, task).unwrap();
        assert_eq!(view.agent_id.as_deref(), Some(id.as_str()));
    }
    assert_eq!(fx.store.task(&fx.project, human).unwrap().agent_id, None);
    // The default agent alone does not make connections mandatory, and it cannot be deleted
    // or taken away from a task.
    assert!(!fx.store.has_agents(&fx.project).unwrap());
    assert!(fx.store.archive_agent(&fx.project, &id, t0()).is_err());
    assert!(
        fx.store
            .assign_task_agent(&fx.project, ai, None, t0())
            .is_err()
    );
}

// The connection secret comes from the process environment, so the session checks run in one
// test: they must not interleave with another test that sets it.
#[test]
fn only_the_assigned_agents_connection_may_write_from_doing_onward() {
    let fx = fx();
    let id = fx.task(Assignee::Ai);
    let default_agent = fx.store.list_agents(&fx.project).unwrap()[0].id.clone();
    let other = fx
        .store
        .register_agent(&fx.project, "Screen", "UI", t0())
        .unwrap()
        .id;
    let default_secret = fx.connect(&default_agent);
    let other_secret = fx.connect(&other);
    let _session = enter_session_writes(fx.root.path().to_path_buf());
    let as_session = |secret: Option<&str>| {
        // SAFETY: only this test sets the variable, and it runs on one thread.
        unsafe {
            match secret {
                Some(secret) => std::env::set_var("AGPJ_CONNECTION_SECRET", secret),
                None => std::env::remove_var("AGPJ_CONNECTION_SECRET"),
            }
        }
    };
    let to_doing = |fx: &Fx| {
        fx.store
            .move_work_task(&fx.project, id, Column::Doing, ctx(Actor::Ai), t0())
    };

    // A Todo plan is co-editable by any agent; starting it is the assigned agent's alone.
    as_session(Some(&other_secret));
    fx.store
        .edit_task(&fx.project, id, None, Some("## 概要\n\n計画\n"), t0())
        .unwrap();
    assert!(matches!(to_doing(&fx), Err(AgpjError::NotAssignee { .. })));
    as_session(None);
    assert!(matches!(to_doing(&fx), Err(AgpjError::NotConnected)));
    assert_eq!(
        fx.store.task(&fx.project, id).unwrap().task_status,
        Column::Todo
    );
    as_session(Some(&default_secret));
    to_doing(&fx).unwrap();

    // Another and an unconnected agent leave no trace in the task, its history, or its files.
    let before = fx.history_len(id);
    let images = fx.image_files();
    for secret in [Some(other_secret.as_str()), None] {
        as_session(secret);
        let refused = |r: Result<(), AgpjError>| {
            assert!(matches!(
                r,
                Err(AgpjError::NotAssignee { .. } | AgpjError::NotConnected)
            ));
        };
        refused(
            fx.store
                .edit_task(&fx.project, id, None, Some("## 概要\n\n横取り\n"), t0())
                .map(drop),
        );
        refused(
            fx.store
                .add_result(&fx.project, id, input(), t0())
                .map(drop),
        );
        refused(
            fx.store
                .set_agent_status_connected(
                    &fx.project,
                    id,
                    RunState::Running,
                    ctx(Actor::Ai),
                    secret.unwrap_or("none"),
                    t0(),
                )
                .map(drop),
        );
        refused(
            fx.store
                .add_attachment(
                    &fx.project,
                    &agent_project::file_store::model::AttachmentOwner::Task(id),
                    AttachmentInput {
                        id: "img-1".into(),
                        name: "a.png".into(),
                        bytes: common::png(),
                    },
                    t0(),
                )
                .map(drop),
        );
        refused(
            fx.store
                .move_work_task(&fx.project, id, Column::Done, ctx(Actor::Ai), t0())
                .map(drop),
        );
    }
    assert_eq!(fx.history_len(id), before);
    assert_eq!(fx.image_files(), images);
    assert_eq!(fx.store.task(&fx.project, id).unwrap().results.len(), 0);

    // The assigned agent can write.
    as_session(Some(&default_secret));
    fx.store.add_result(&fx.project, id, input(), t0()).unwrap();

    // A person hands the card over (the board is not a session): the former agent is refused
    // from then on, and the new one can write. Assignment and start race on one lock, so
    // only one agent's start is ever valid.
    drop(_session);
    fx.store
        .assign_task_agent(&fx.project, id, Some(&other), t0())
        .unwrap();
    let _session = enter_session_writes(fx.root.path().to_path_buf());
    as_session(Some(&default_secret));
    assert!(matches!(
        fx.store.add_result(&fx.project, id, input(), t0()),
        Err(AgpjError::NotAssignee { .. })
    ));
    as_session(Some(&other_secret));
    fx.store.add_result(&fx.project, id, input(), t0()).unwrap();
    as_session(None);
}
