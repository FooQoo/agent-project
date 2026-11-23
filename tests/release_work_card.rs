//! The release-work card that `version add` issues: its title and to-do, and the release version
//! moving to the next listed version when the card reaches Done. Store, CLI and web paths.

mod common;

use std::fs;
use std::path::PathBuf;

use agent_project::domain::{Actor, Assignee, ChangeContext, Column, ProjectId};
use agent_project::file_store::FileStore;
use agent_project::file_store::history_language::HistoryLanguage;
use chrono::{DateTime, TimeZone, Utc};
use common::{Running, agpj, json_ok};

fn at(day: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, day, 3, 0, 0).unwrap()
}

fn human(reason: &str) -> ChangeContext {
    ChangeContext {
        actor: Actor::Human,
        reason: reason.into(),
    }
}

struct Fixture {
    _root: tempfile::TempDir,
    store: FileStore,
    project: ProjectId,
}

fn fixture(language: Option<HistoryLanguage>) -> Fixture {
    let root = tempfile::tempdir().unwrap();
    let store = FileStore::open(&root.path().join("shared")).with_history_language(language);
    let project = store.create_project("Alpha", at(4)).unwrap().id;
    Fixture {
        _root: root,
        store,
        project,
    }
}

impl Fixture {
    fn release(&self) -> Option<String> {
        self.store.project_versions(&self.project).unwrap().release
    }

    fn card(&self, version: &str) -> i64 {
        let snapshot = self.store.snapshot(&self.project).unwrap();
        snapshot
            .tasks
            .iter()
            .find(|t| t.title.contains(version))
            .unwrap()
            .id
    }

    fn done(&self, id: i64) {
        self.store
            .move_work_task_for_user(&self.project, id, Column::Done, human("Released."), at(9))
            .unwrap();
    }

    fn reasons(&self, id: i64) -> Vec<String> {
        self.store
            .task(&self.project, id)
            .unwrap()
            .history
            .into_iter()
            .map(|h| h.reason)
            .collect()
    }
}

#[test]
fn ac1_the_card_is_a_release_work_card_in_english() {
    let f = fixture(None);
    f.store.add_version(&f.project, "v1.8.0", at(5)).unwrap();
    let task = f.store.task(&f.project, f.card("v1.8.0")).unwrap();
    assert_eq!(task.title, "Release v1.8.0");
    assert_eq!(task.assignee, Assignee::Ai);
    assert_eq!(task.task_status, Column::Backlog);
    assert!(task.body.contains("docs/release/v1.8.0.md"));
    assert!(task.body.contains("docs/release/TEMPLATE.md"));
    assert!(task.body.contains("publish the release"));
    assert!(task.body.contains("sends it to Todo"));
}

#[test]
fn ac1_the_card_is_a_release_work_card_in_japanese() {
    let f = fixture(Some(HistoryLanguage::Japanese));
    f.store.add_version(&f.project, "v1.8.0", at(5)).unwrap();
    let task = f.store.task(&f.project, f.card("v1.8.0")).unwrap();
    assert_eq!(task.title, "v1.8.0 をリリースする");
    assert!(task.body.contains("docs/release/v1.8.0.md"));
    assert!(task.body.contains("リリースを公開し"));
    assert!(task.body.contains("Todo に送ってから"));
}

#[test]
fn ac2_done_advances_the_release_version_to_the_nearest_newer_listed_version() {
    let f = fixture(None);
    for v in ["v1.8.0", "v1.10.0", "v1.9.0", "v1.7.0"] {
        f.store.add_version(&f.project, v, at(5)).unwrap();
    }
    f.store
        .set_release_version(&f.project, Some("v1.8.0"), at(6))
        .unwrap();
    let id = f.card("v1.8.0");
    f.done(id);
    assert_eq!(f.release().as_deref(), Some("v1.9.0"));
    assert!(
        f.reasons(id)
            .iter()
            .any(|r| r == "Release version moved from v1.8.0 to v1.9.0.")
    );
}

#[test]
fn ac2_with_no_newer_listed_version_the_release_version_stays_and_history_says_so() {
    let f = fixture(None);
    f.store.add_version(&f.project, "v1.8.0", at(5)).unwrap();
    f.store
        .set_release_version(&f.project, Some("v1.8.0"), at(6))
        .unwrap();
    let id = f.card("v1.8.0");
    f.done(id);
    assert_eq!(f.release().as_deref(), Some("v1.8.0"));
    assert!(
        f.reasons(id)
            .iter()
            .any(|r| r == "Release version stays at v1.8.0: no newer version is listed.")
    );
}

#[test]
fn ac2_the_history_follows_the_board_language() {
    let f = fixture(Some(HistoryLanguage::Japanese));
    for v in ["v1.8.0", "v1.9.0"] {
        f.store.add_version(&f.project, v, at(5)).unwrap();
    }
    f.store
        .set_release_version(&f.project, Some("v1.8.0"), at(6))
        .unwrap();
    let id = f.card("v1.8.0");
    f.done(id);
    assert_eq!(f.release().as_deref(), Some("v1.9.0"));
    assert!(
        f.reasons(id)
            .iter()
            .any(|r| r == "リリース版を v1.8.0 から v1.9.0 に進めました。")
    );
}

#[test]
fn ac3_nothing_changes_for_another_release_version_another_lane_or_a_second_done() {
    let f = fixture(None);
    for v in ["v1.8.0", "v1.9.0", "v1.10.0"] {
        f.store.add_version(&f.project, v, at(5)).unwrap();
    }
    let id = f.card("v1.8.0");
    // The release version is another version.
    f.store
        .set_release_version(&f.project, Some("v1.9.0"), at(6))
        .unwrap();
    f.done(id);
    assert_eq!(f.release().as_deref(), Some("v1.9.0"));
    assert!(!f.reasons(id).iter().any(|r| r.contains("Release version")));
    // The card moves to a lane other than Done.
    let other = f.card("v1.9.0");
    f.store
        .move_work_task_for_user(&f.project, other, Column::Todo, human("Send."), at(7))
        .unwrap();
    f.store
        .move_work_task_for_user(&f.project, other, Column::Doing, human("Start."), at(7))
        .unwrap();
    assert_eq!(f.release().as_deref(), Some("v1.9.0"));
    // Done advances once; a second Done changes nothing.
    f.done(other);
    assert_eq!(f.release().as_deref(), Some("v1.10.0"));
    let moved = f
        .store
        .move_work_task_for_user(&f.project, other, Column::Done, human("Again."), at(9))
        .unwrap();
    assert!(!moved.changed);
    assert_eq!(f.release().as_deref(), Some("v1.10.0"));
}

fn project_id(home: &std::path::Path) -> String {
    let projects: PathBuf = home.join("shared/projects");
    fs::read_dir(projects)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .file_name()
        .to_string_lossy()
        .into_owned()
}

fn cli(home: &std::path::Path, args: &[&str]) -> serde_json::Value {
    let mut all = vec!["--project", "Demo"];
    all.extend(args);
    all.push("--json");
    json_ok(home, &all)
}

#[test]
fn ac2_the_cli_advances_the_release_version_when_the_card_is_done() {
    let home = tempfile::tempdir().unwrap();
    let h = home.path();
    let (code, _, err) = agpj(h, &["project", "create", "Demo"]);
    assert_eq!(code, 0, "{err}");
    let added = cli(h, &["version", "add", "v1.8.0"]);
    assert_eq!(added["versions"][0], "v1.8.0");
    cli(h, &["version", "add", "v1.9.0"]);
    cli(h, &["version", "release", "v1.8.0"]);
    let moved = cli(
        h,
        &[
            "mv",
            "1",
            "done",
            "--actor",
            "human",
            "--reason",
            "Released.",
        ],
    );
    assert_eq!(moved["task"]["taskStatus"], "done");
    let shown = cli(h, &["version", "release"]);
    assert_eq!(shown["releaseVersion"], "v1.9.0");
    // v1.9.0 is the newest: finishing its card leaves the release version.
    cli(
        h,
        &[
            "mv",
            "2",
            "done",
            "--actor",
            "human",
            "--reason",
            "Released.",
        ],
    );
    assert_eq!(cli(h, &["version", "release"])["releaseVersion"], "v1.9.0");
}

#[test]
fn ac2_the_web_move_advances_the_release_version_when_the_card_is_done() {
    let home = tempfile::tempdir().unwrap();
    let h = home.path();
    let (code, _, err) = agpj(h, &["project", "create", "Demo"]);
    assert_eq!(code, 0, "{err}");
    cli(h, &["version", "add", "v1.8.0"]);
    cli(h, &["version", "add", "v1.9.0"]);
    cli(h, &["version", "release", "v1.8.0"]);
    let pid = project_id(h);
    let server = Running::start(h);
    let moved = server.write(
        "POST",
        &format!("/api/projects/{pid}/tasks/1/move"),
        Some(r#"{"taskStatus":"done","reason":"Released."}"#),
    );
    assert_eq!(moved.status, 200, "{}", moved.body);
    let versions = server.get(&format!("/api/projects/{pid}/versions"));
    assert_eq!(
        versions.json()["releaseVersion"],
        "v1.9.0",
        "{}",
        versions.body
    );
    server.stop();
}
