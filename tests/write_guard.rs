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
    // The default agent requires a connection and cannot be deleted or taken away.
    assert!(fx.store.has_agents(&fx.project).unwrap());
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
    {
        let _session = enter_session_writes(fx.root.path().to_path_buf());
        assert!(matches!(
            fx.store
                .move_work_task(&fx.project, id, Column::Doing, ctx(Actor::Ai), t0()),
            Err(AgpjError::NotConnected)
        ));
    }
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

    // Start and reassignment contend for the same project lock. Either the old owner starts
    // first, or reassignment wins and the old owner is refused; the final owner is always new.
    as_session(Some(&default_secret));
    drop(_session);
    for _ in 0..20 {
        let raced = fx.task(Assignee::Ai);
        let barrier = std::sync::Barrier::new(2);
        std::thread::scope(|scope| {
            let start = scope.spawn(|| {
                let _session = enter_session_writes(fx.root.path().to_path_buf());
                barrier.wait();
                fx.store
                    .move_work_task(&fx.project, raced, Column::Doing, ctx(Actor::Ai), t0())
            });
            barrier.wait();
            fx.store
                .assign_task_agent(&fx.project, raced, Some(&other), t0())
                .unwrap();
            let started = start.join().unwrap();
            assert!(started.is_ok() || matches!(started, Err(AgpjError::NotAssignee { .. })));
            let task = fx.store.task(&fx.project, raced).unwrap();
            assert_eq!(task.agent_id.as_deref(), Some(other.as_str()));
            assert_eq!(
                task.task_status,
                if started.is_ok() {
                    Column::Doing
                } else {
                    Column::Todo
                }
            );
        });
    }

    let _session = enter_session_writes(fx.root.path().to_path_buf());

    fx.store.edit_task(&fx.project, id, None, Some(
        "---\nacceptance:\n  - id: AC-1\n    text: Verified\n    done: false\n---\n\nPlan.\n"
    ), t0()).unwrap();

    // A Todo plan is co-editable by any agent; starting it is the assigned agent's alone.
    as_session(Some(&other_secret));
    fx.store
        .edit_task(&fx.project, id, None, Some("---\nacceptance:\n  - id: AC-1\n    text: Verified\n    done: false\n---\n\n## 概要\n\n計画\n"), t0())
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

    // The CLI checks the connection before changing the condition, including a retry.
    let check = |task_id: i64, secret: &str| {
        std::process::Command::new(env!("CARGO_BIN_EXE_agpj"))
            .args([
                "--project",
                "Alpha",
                "acceptance",
                "check",
                &task_id.to_string(),
                "AC-1",
                "--json",
            ])
            .env("AGPJ_HOME", fx.root.path())
            .env("AGPJ_CONNECTION_SECRET", secret)
            .env("AGPJ_AUTOSTART", "0")
            .env("AGPJ_OPEN_BROWSER", "0")
            .output()
            .unwrap()
    };
    assert!(!check(id, &other_secret).status.success());
    assert!(!fx.store.task(&fx.project, id).unwrap().acceptance[0].done);
    assert!(check(id, &default_secret).status.success());
    assert!(fx.store.task(&fx.project, id).unwrap().acceptance[0].done);
    assert!(!check(id, &other_secret).status.success());
    assert!(check(id, &default_secret).status.success());

    // Older cards can lack an explicit agentId; the default agent still owns them.
    let legacy = fx.task(Assignee::Ai);
    fx.store
        .edit_task(
            &fx.project,
            legacy,
            None,
            Some("---\nacceptance:\n  - id: AC-1\n    text: Verified\n    done: false\n---\n\nLegacy plan.\n"),
            t0(),
        )
        .unwrap();
    let metadata = fx
        .root
        .path()
        .join("shared/projects")
        .join(fx.project.as_str())
        .join(format!("tasks/{legacy}/task.json"));
    let mut value: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&metadata).unwrap()).unwrap();
    value["agentId"] = serde_json::Value::Null;
    std::fs::write(&metadata, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
    as_session(None);
    assert!(matches!(
        fx.store
            .move_work_task(&fx.project, legacy, Column::Doing, ctx(Actor::Ai), t0()),
        Err(AgpjError::NotConnected)
    ));
    as_session(Some(&default_secret));
    fx.store
        .move_work_task(&fx.project, legacy, Column::Doing, ctx(Actor::Ai), t0())
        .unwrap();
    assert!(check(legacy, &default_secret).status.success());
    assert!(fx.store.task(&fx.project, legacy).unwrap().acceptance[0].done);

    let question = qa::ask(&fx.store, &fx.project, &[id], "Which way?", &[]);
    qa::answer(
        &fx.store,
        &fx.project,
        &question.id,
        None,
        Some("This way."),
    );
    as_session(Some(&other_secret));
    assert!(matches!(
        fx.store
            .accept_message(&fx.project, &question.id, Actor::Ai, t0()),
        Err(AgpjError::NotAssignee { id: task_id }) if task_id == id
    ));
    as_session(Some(&default_secret));
    fx.store
        .accept_message(&fx.project, &question.id, Actor::Ai, t0())
        .unwrap();

    // The early `message ask`/`wait` path accepts answers in a batch. An unauthorized entry
    // stays pending while an independently authorized entry in the same batch is accepted.
    let other_question = qa::ask(&fx.store, &fx.project, &[id], "Another choice?", &[]);
    qa::answer(
        &fx.store,
        &fx.project,
        &other_question.id,
        None,
        Some("Yes."),
    );
    let second_task = fx.task(Assignee::Ai);
    fx.store
        .assign_task_agent(&fx.project, second_task, Some(&other), t0())
        .unwrap();
    as_session(Some(&other_secret));
    fx.store
        .move_work_task(
            &fx.project,
            second_task,
            Column::Doing,
            ctx(Actor::Ai),
            t0(),
        )
        .unwrap();
    let other_task_question = qa::ask(&fx.store, &fx.project, &[second_task], "Their choice?", &[]);
    qa::answer(
        &fx.store,
        &fx.project,
        &other_task_question.id,
        None,
        Some("No."),
    );
    let unrelated = fx.task(Assignee::Human);
    let project_dir =
        agent_project::file_store::layout::project_dir(fx.store.shared_dir(), &fx.project);
    std::fs::write(
        agent_project::file_store::layout::task_md(&agent_project::file_store::layout::task_dir(
            &project_dir,
            unrelated,
        )),
        "not valid task frontmatter",
    )
    .unwrap();
    as_session(Some(&default_secret));
    let accepted = fx
        .store
        .accept_answers(
            &fx.project,
            &[other_question.id.clone(), other_task_question.id.clone()],
            t0(),
        )
        .unwrap();
    assert_eq!(accepted, vec![other_question.id.clone()]);
    assert_eq!(
        fx.store
            .message(&fx.project, &other_question.id)
            .unwrap()
            .state(),
        "closed"
    );
    assert_eq!(
        fx.store
            .message(&fx.project, &other_task_question.id)
            .unwrap()
            .state(),
        "closed"
    );
    assert!(
        fx.store
            .accept_answers(&fx.project, &[other_task_question.id], t0())
            .unwrap()
            .is_empty()
    );

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
