//! Project versions (the files of `docs/release/`), the current version, and the planned version
//! of tasks (the `plannedVersion` key of TASK.md) with its history entries, on the shared files.

use std::fs;
use std::path::PathBuf;

use agent_board::domain::{Actor, Assignee, NewTask, ProjectId};
use agent_board::file_store::model::{NewEpic, TaskView};
use agent_board::file_store::validate::EpicId;
use agent_board::file_store::{FileStore, VersionChoice};
use chrono::{DateTime, TimeZone, Utc};
use serde_json::Value;

fn at(day: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, day, 3, 0, 0).unwrap()
}

struct Fixture {
    _root: tempfile::TempDir,
    shared: PathBuf,
    store: FileStore,
    project: ProjectId,
}

fn fixture() -> Fixture {
    let root = tempfile::tempdir().unwrap();
    let shared = root.path().join("shared");
    let store = FileStore::open(&shared);
    let project = store.create_project("Alpha", at(4)).unwrap().id;
    Fixture {
        _root: root,
        shared,
        store,
        project,
    }
}

const BODY: &str = "---\nacceptance:\n  - id: AC-1\n    text: The thing works.\n    done: false\n---\n\nDo the work.\n";

impl Fixture {
    fn dir(&self, project: &ProjectId) -> PathBuf {
        self.shared.join("projects").join(project.as_str())
    }

    fn task_json(&self, id: i64) -> Vec<u8> {
        fs::read(
            self.dir(&self.project)
                .join(format!("tasks/{id}/task.json")),
        )
        .unwrap()
    }

    fn task_md(&self, id: i64) -> Vec<u8> {
        fs::read(self.dir(&self.project).join(format!("tasks/{id}/TASK.md"))).unwrap()
    }

    fn project_json(&self, project: &ProjectId) -> Vec<u8> {
        fs::read(self.dir(project).join("project.json")).unwrap()
    }

    fn task(&self, title: &str, assignee: Assignee) -> TaskView {
        self.store
            .create_task(
                &self.project,
                NewTask {
                    title: title.into(),
                    description: BODY.into(),
                    assignee,
                    actor: Actor::Human,
                    draft: None,
                },
                at(4),
            )
            .unwrap()
    }

    fn epic(&self, id: &str) -> EpicId {
        self.store
            .create_epic(
                &self.project,
                NewEpic {
                    id: id.into(),
                    name: "Release".into(),
                    color: None,
                    draft: None,
                    document: None,
                },
                at(4),
            )
            .unwrap();
        EpicId::normalize(id).unwrap()
    }

    fn set(&self, id: i64, choice: VersionChoice, day: u32) -> (TaskView, bool) {
        let changed = self
            .store
            .set_task_planned_version(&self.project, id, &choice, Actor::Human, at(day))
            .unwrap();
        (changed.value, changed.changed)
    }
}

fn named(name: &str) -> VersionChoice {
    VersionChoice::Named(name.into())
}

#[test]
fn two_projects_keep_the_same_version_name_independently() {
    let f = fixture();
    let other = f.store.create_project("Beta", at(4)).unwrap().id;
    f.store.add_version(&f.project, "v1.8.0", at(5)).unwrap();
    assert!(
        f.store
            .project_versions(&other)
            .unwrap()
            .versions
            .is_empty()
    );
    f.store.add_version(&other, "v1.8.0", at(5)).unwrap();
    f.store
        .set_current_version(&other, Some("v1.8.0"), at(5))
        .unwrap();

    let alpha = f.store.project_versions(&f.project).unwrap();
    assert_eq!(alpha.versions, ["v1.8.0"]);
    assert_eq!(alpha.current, None);
    let beta = f.store.project_versions(&other).unwrap();
    assert_eq!(beta.current.as_deref(), Some("v1.8.0"));

    // Clearing in one project leaves the other.
    f.store.set_current_version(&other, None, at(5)).unwrap();
    assert_eq!(
        f.store.project_versions(&f.project).unwrap().versions,
        ["v1.8.0"]
    );
    assert_eq!(
        f.store.project_versions(&other).unwrap().versions,
        ["v1.8.0"]
    );
}

#[test]
fn files_without_the_fields_load_and_a_task_never_gets_a_version_by_itself() {
    let f = fixture();
    let task = f.task("Plain", Assignee::Ai);
    let text = String::from_utf8(f.project_json(&f.project)).unwrap();
    assert!(!text.contains("versions") && !text.contains("currentVersion"));
    let json_before = f.task_json(task.id);
    let md_before = f.task_md(task.id);
    assert!(
        !String::from_utf8(json_before.clone())
            .unwrap()
            .contains("plannedVersion")
    );

    // Unset on a task with no plan, and clearing a missing current version, change nothing.
    let (view, changed) = f.set(task.id, VersionChoice::Unset, 5);
    assert!(!changed);
    assert_eq!(view.planned_version, None);
    assert!(
        !f.store
            .set_current_version(&f.project, None, at(5))
            .unwrap()
            .changed
    );
    assert_eq!(f.task_json(task.id), json_before);
    assert_eq!(f.task_md(task.id), md_before);

    // A current version on the project does not give the task one, and a later write that
    // rewrites the task keeps it free of the fields.
    f.store
        .set_current_version(&f.project, Some("v1.8.0"), at(5))
        .unwrap();
    f.store
        .add_history(&f.project, task.id, Some("note"), at(5))
        .unwrap();
    assert_eq!(
        f.store.task(&f.project, task.id).unwrap().planned_version,
        None
    );
    let text = String::from_utf8(f.task_json(task.id)).unwrap();
    assert!(!text.contains("plannedVersion"));
    assert!(f.store.snapshot(&f.project).unwrap().problems.is_empty());

    // Clearing the current version again returns project.json to its legacy shape.
    f.store
        .set_current_version(&f.project, None, at(5))
        .unwrap();
    let text = String::from_utf8(f.project_json(&f.project)).unwrap();
    assert!(!text.contains("currentVersion"));
}

#[test]
fn the_current_version_is_stored_as_a_concrete_name_and_later_switches_do_not_move_tasks() {
    let f = fixture();
    let task = f.task("Plan", Assignee::Ai);
    assert!(
        f.store
            .set_task_planned_version(
                &f.project,
                task.id,
                &VersionChoice::Current,
                Actor::Human,
                at(5)
            )
            .is_err(),
        "there is no current version yet"
    );
    f.store
        .set_current_version(&f.project, Some("v1.8.0"), at(5))
        .unwrap();
    let (view, changed) = f.set(task.id, VersionChoice::Current, 5);
    assert!(changed);
    assert_eq!(view.planned_version.as_deref(), Some("v1.8.0"));
    let md = String::from_utf8(f.task_md(task.id)).unwrap();
    assert!(md.contains("plannedVersion: v1.8.0"), "{md}");
    let stored: Value = serde_json::from_slice(&f.task_json(task.id)).unwrap();
    assert!(stored.get("plannedVersion").is_none());

    f.store
        .set_current_version(&f.project, Some("v1.9.0"), at(5))
        .unwrap();
    f.store
        .set_current_version(&f.project, None, at(5))
        .unwrap();
    assert_eq!(
        f.store
            .task(&f.project, task.id)
            .unwrap()
            .planned_version
            .as_deref(),
        Some("v1.8.0")
    );
    // The versions stay listed after the current one is cleared.
    assert_eq!(
        f.store.project_versions(&f.project).unwrap().versions,
        ["v1.9.0", "v1.8.0"]
    );

    f.store
        .set_current_version(&f.project, Some("v1.9.0"), at(5))
        .unwrap();
    let (view, changed) = f.set(task.id, VersionChoice::Current, 6);
    assert!(changed);
    assert_eq!(view.planned_version.as_deref(), Some("v1.9.0"));
}

#[test]
fn a_version_must_be_listed_and_named_validly() {
    let f = fixture();
    let task = f.task("Plan", Assignee::Ai);
    assert!(
        f.store
            .set_task_planned_version(&f.project, task.id, &named("v9"), Actor::Human, at(5))
            .is_err()
    );
    for bad in ["", "has space", "tab\t", &"x".repeat(65)] {
        assert!(
            f.store.add_version(&f.project, bad, at(5)).is_err(),
            "{bad:?}"
        );
        assert!(
            f.store
                .set_current_version(&f.project, Some(bad), at(5))
                .is_err()
        );
    }
    assert!(
        f.store
            .add_version(&f.project, "v1.8.0", at(5))
            .unwrap()
            .changed
    );
    assert!(
        !f.store
            .add_version(&f.project, "v1.8.0", at(5))
            .unwrap()
            .changed
    );
    // Names are compared exactly.
    assert!(
        f.store
            .add_version(&f.project, "1.8.0", at(5))
            .unwrap()
            .changed
    );
    assert_eq!(
        f.store.project_versions(&f.project).unwrap().versions,
        ["v1.8.0", "1.8.0"]
    );
}

#[test]
fn moves_between_versions_are_traced_in_the_history_with_before_after_and_time() {
    let f = fixture();
    let task = f.task("Plan", Assignee::Ai);
    f.store.add_version(&f.project, "v1.8.0", at(5)).unwrap();
    f.store.add_version(&f.project, "v1.9.0", at(5)).unwrap();

    f.set(task.id, named("v1.8.0"), 5);
    f.set(task.id, named("v1.9.0"), 6);
    let (view, _) = f.set(task.id, VersionChoice::Unset, 7);

    assert_eq!(view.planned_version, None);
    // The history holds one readable entry per change after the creation entry.
    let reasons: Vec<&str> = view.history.iter().map(|h| h.reason.as_str()).collect();
    assert_eq!(reasons.len(), 4);
    assert_eq!(reasons[1], "Changed planned version from none to v1.8.0.");
    assert_eq!(reasons[2], "Changed planned version from v1.8.0 to v1.9.0.");
    assert_eq!(reasons[3], "Changed planned version from v1.9.0 to none.");
    assert_eq!(view.history[3].at, at(7));
    // Neither file keeps a structured trace: the history entries are the record, and no plan is
    // left in TASK.md or task.json after it is cleared.
    let stored: Value = serde_json::from_slice(&f.task_json(task.id)).unwrap();
    assert!(stored.get("plannedVersion").is_none());
    assert!(stored.get("plannedVersionChanges").is_none());
    assert!(
        !String::from_utf8(f.task_md(task.id))
            .unwrap()
            .contains("plannedVersion")
    );
    assert!(f.store.snapshot(&f.project).unwrap().problems.is_empty());
}

#[test]
fn a_japanese_task_gets_a_japanese_history_reason() {
    let f = fixture();
    let task = f
        .store
        .create_task(
            &f.project,
            NewTask {
                title: "版を決める".into(),
                description: "---\nacceptance: []\n---\n\n日本語の本文です。\n".into(),
                assignee: Assignee::Ai,
                actor: Actor::Human,
                draft: None,
            },
            at(4),
        )
        .unwrap();
    f.store.add_version(&f.project, "v1.8.0", at(5)).unwrap();
    let (view, _) = f.set(task.id, named("v1.8.0"), 5);
    assert_eq!(
        view.history.last().unwrap().reason,
        "リリース予定の版をなしからv1.8.0に変更しました。"
    );
}

#[test]
fn a_retried_set_is_a_no_op_without_a_second_history_entry() {
    let f = fixture();
    let task = f.task("Plan", Assignee::Ai);
    f.store.add_version(&f.project, "v1.8.0", at(5)).unwrap();
    let (first, changed) = f.set(task.id, named("v1.8.0"), 5);
    assert!(changed);
    let json = f.task_json(task.id);
    let md = f.task_md(task.id);
    let (again, changed) = f.set(task.id, named("v1.8.0"), 9);
    assert!(!changed);
    assert_eq!(again.history, first.history);
    assert_eq!(f.task_json(task.id), json);
    assert_eq!(f.task_md(task.id), md);
}

#[test]
fn concurrent_sets_of_the_same_version_record_one_change() {
    let f = fixture();
    let task = f.task("Plan", Assignee::Ai);
    f.store.add_version(&f.project, "v1.8.0", at(5)).unwrap();
    let shared = f.shared.clone();
    let project = f.project.clone();
    let id = task.id;
    let threads: Vec<_> = (0..8)
        .map(|n| {
            let shared = shared.clone();
            let project = project.clone();
            std::thread::spawn(move || {
                FileStore::open(&shared)
                    .set_task_planned_version(
                        &project,
                        id,
                        &named("v1.8.0"),
                        Actor::Human,
                        at(5 + n),
                    )
                    .unwrap()
                    .changed
            })
        })
        .collect();
    let changed = threads
        .into_iter()
        .map(|t| t.join().unwrap())
        .filter(|c| *c)
        .count();
    assert_eq!(changed, 1);
    let view = f.store.task(&f.project, id).unwrap();
    assert_eq!(view.history.len(), 2);
}

#[test]
fn a_set_keeps_assignee_lane_position_and_acceptance() {
    let f = fixture();
    let ai = f.task("AI task", Assignee::Ai);
    let human = f.task("Human task", Assignee::Human);
    f.store.add_version(&f.project, "v1.8.0", at(5)).unwrap();
    for before in [ai, human] {
        let (after, _) = f.set(before.id, named("v1.8.0"), 5);
        assert_eq!(after.assignee, before.assignee);
        assert_eq!(after.task_status, before.task_status);
        assert_eq!(after.agent_status, before.agent_status);
        assert_eq!(after.position, before.position);
        assert_eq!(after.acceptance, before.acceptance);
        assert_eq!(after.acceptance.len(), 1);
        assert_eq!(after.body, before.body);
        assert_eq!(after.title, before.title);
        assert_eq!(after.epic_id, before.epic_id);
    }
}

#[test]
fn an_epic_can_hold_mixed_versions_and_a_bulk_set_reports_what_changed() {
    let f = fixture();
    let epic = f.epic("REL");
    let a = f.task("A", Assignee::Ai);
    let b = f.task("B", Assignee::Ai);
    let c = f.task("C", Assignee::Human);
    let outside = f.task("Outside", Assignee::Ai);
    for id in [a.id, b.id, c.id] {
        f.store
            .set_task_epic(&f.project, id, Some(&epic), Actor::Human, at(4))
            .unwrap();
    }
    f.store.add_version(&f.project, "v1.8.0", at(5)).unwrap();
    f.store.add_version(&f.project, "v1.9.0", at(5)).unwrap();
    f.set(b.id, named("v1.9.0"), 5);

    let run = |choice: &VersionChoice, day: u32| {
        f.store
            .set_epic_planned_version(&f.project, &epic, choice, Actor::Human, at(day))
            .unwrap()
    };
    let first = run(&named("v1.8.0"), 6);
    assert_eq!(first.changed, [a.id, b.id, c.id]);
    assert!(first.unchanged.is_empty());
    // The retry changes nothing and adds no history.
    let history_len = f.store.task(&f.project, b.id).unwrap().history.len();
    let retry = run(&named("v1.8.0"), 7);
    assert!(retry.changed.is_empty());
    assert_eq!(retry.unchanged, [a.id, b.id, c.id]);
    assert_eq!(
        f.store.task(&f.project, b.id).unwrap().history.len(),
        history_len
    );
    // B moved from the other version, and the task outside the epic was not touched.
    let b_view = f.store.task(&f.project, b.id).unwrap();
    // Creation, joining the epic, then two planned-version changes.
    assert_eq!(b_view.history.len(), 4);
    assert_eq!(
        f.store
            .task(&f.project, outside.id)
            .unwrap()
            .planned_version,
        None
    );

    // One task can move on its own: the epic may mix versions.
    f.set(c.id, named("v1.9.0"), 8);
    let mixed: Vec<Option<String>> = [a.id, b.id, c.id]
        .iter()
        .map(|id| f.store.task(&f.project, *id).unwrap().planned_version)
        .collect();
    assert_eq!(
        mixed,
        [
            Some("v1.8.0".into()),
            Some("v1.8.0".into()),
            Some("v1.9.0".into())
        ]
    );
    // Setting the current version for the whole epic resolves it once.
    f.store
        .set_current_version(&f.project, Some("v1.9.0"), at(5))
        .unwrap();
    let all = run(&VersionChoice::Current, 9);
    assert_eq!(all.changed, [a.id, b.id]);
    assert_eq!(all.unchanged, [c.id]);
    let cleared = run(&VersionChoice::Unset, 10);
    assert_eq!(cleared.changed, [a.id, b.id, c.id]);
    // Assignee and acceptance survive every bulk write.
    let c_view = f.store.task(&f.project, c.id).unwrap();
    assert_eq!(c_view.assignee, Assignee::Human);
    assert_eq!(c_view.acceptance, c.acceptance);
    assert_eq!(c_view.history.len(), 5);
}

#[test]
fn archived_tasks_are_refused_alone_and_skipped_in_bulk() {
    let f = fixture();
    let epic = f.epic("REL");
    let a = f.task("A", Assignee::Ai);
    let b = f.task("B", Assignee::Ai);
    for id in [a.id, b.id] {
        f.store
            .set_task_epic(&f.project, id, Some(&epic), Actor::Human, at(4))
            .unwrap();
    }
    f.store.add_version(&f.project, "v1.8.0", at(5)).unwrap();
    f.store.archive_task(&f.project, b.id, at(5)).unwrap();
    assert!(
        f.store
            .set_task_planned_version(&f.project, b.id, &named("v1.8.0"), Actor::Human, at(6))
            .is_err()
    );
    let result = f
        .store
        .set_epic_planned_version(&f.project, &epic, &named("v1.8.0"), Actor::Human, at(6))
        .unwrap();
    assert_eq!(result.changed, [a.id]);
    assert_eq!(result.skipped, [b.id]);
}
