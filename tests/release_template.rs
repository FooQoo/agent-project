//! Creating a version copies the release-note template into `docs/release/<version>.md` and
//! issues one Backlog card for writing the notes. These checks run on the shared store
//! operation that the CLI, the web API and the board all use.

use std::fs;
use std::path::PathBuf;

use agent_project::domain::{Assignee, Column, ProjectId};
use agent_project::file_store::FileStore;
use agent_project::file_store::history_language::HistoryLanguage;
use agent_project::file_store::releases::STANDARD_TEMPLATE as STANDARD_RELEASE_TEMPLATE;
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

fn fixture_with(language: Option<HistoryLanguage>) -> Fixture {
    let root = tempfile::tempdir().unwrap();
    let shared = root.path().join("shared");
    let store = FileStore::open(&shared).with_history_language(language);
    let project = store.create_project("Alpha", at(4)).unwrap().id;
    Fixture {
        _root: root,
        shared,
        store,
        project,
    }
}

fn fixture() -> Fixture {
    fixture_with(None)
}

impl Fixture {
    fn dir(&self) -> PathBuf {
        self.shared.join("projects").join(self.project.as_str())
    }

    fn release(&self) -> PathBuf {
        self.dir().join("docs/release")
    }

    fn tasks(&self) -> Vec<agent_project::file_store::model::TaskView> {
        let ids: Vec<i64> = match fs::read_dir(self.dir().join("tasks")) {
            Ok(entries) => entries
                .filter_map(Result::ok)
                .filter_map(|e| e.file_name().to_str()?.parse().ok())
                .collect(),
            Err(_) => Vec::new(),
        };
        let mut views: Vec<_> = ids
            .into_iter()
            .map(|id| self.store.task(&self.project, id).unwrap())
            .collect();
        views.sort_by_key(|v| v.id);
        views
    }

    fn task_json(&self, id: i64) -> Value {
        let path = self.dir().join(format!("tasks/{id}/task.json"));
        serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
    }

    fn project_json(&self) -> String {
        fs::read_to_string(self.dir().join("project.json")).unwrap()
    }
}

#[test]
fn ac1_a_missing_template_is_written_then_copied_and_never_listed() {
    let f = fixture();
    assert!(!f.release().join("TEMPLATE.md").exists());
    let added = f.store.add_version(&f.project, "v1.8.0", at(5)).unwrap();
    assert!(added.changed);
    assert_eq!(
        fs::read_to_string(f.release().join("TEMPLATE.md")).unwrap(),
        STANDARD_RELEASE_TEMPLATE
    );
    assert_eq!(
        fs::read_to_string(f.release().join("v1.8.0.md")).unwrap(),
        STANDARD_RELEASE_TEMPLATE
    );
    assert_eq!(added.value.versions, ["v1.8.0"]);
    // No temporary file is left behind.
    let names: Vec<String> = fs::read_dir(f.release())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names.len(), 2, "{names:?}");
}

#[test]
fn ac1_the_standard_template_has_the_release_note_shape() {
    let headings: Vec<&str> = STANDARD_RELEASE_TEMPLATE
        .lines()
        .filter(|l| l.starts_with("## "))
        .collect();
    assert_eq!(
        headings,
        [
            "## Changes",
            "## Fixes",
            "## Breaking changes",
            "## Required update steps"
        ]
    );
    assert!(!STANDARD_RELEASE_TEMPLATE.starts_with('#'));
}

#[test]
fn ac1_an_existing_template_is_copied_as_it_is_and_not_overwritten() {
    let f = fixture();
    fs::create_dir_all(f.release()).unwrap();
    fs::write(f.release().join("TEMPLATE.md"), "# Mine\n\nCustom.\n").unwrap();
    f.store.add_version(&f.project, "v1.8.0", at(5)).unwrap();
    f.store.add_version(&f.project, "v1.9.0", at(6)).unwrap();
    assert_eq!(
        fs::read_to_string(f.release().join("TEMPLATE.md")).unwrap(),
        "# Mine\n\nCustom.\n"
    );
    for v in ["v1.8.0", "v1.9.0"] {
        assert_eq!(
            fs::read_to_string(f.release().join(format!("{v}.md"))).unwrap(),
            "# Mine\n\nCustom.\n"
        );
    }
    assert_eq!(
        f.store.project_versions(&f.project).unwrap().versions,
        ["v1.9.0", "v1.8.0"]
    );
}

#[test]
fn ac1_an_existing_version_file_is_not_overwritten_by_add_or_current() {
    let f = fixture();
    fs::create_dir_all(f.release()).unwrap();
    fs::write(f.release().join("v1.8.0.md"), "# v1.8.0\n\nMine.\n").unwrap();
    let again = f.store.add_version(&f.project, "v1.8.0", at(5)).unwrap();
    assert_eq!(again.value.versions, ["v1.8.0"]);
    f.store
        .set_release_version(&f.project, Some("v1.8.0"), at(6))
        .unwrap();
    assert_eq!(
        fs::read_to_string(f.release().join("v1.8.0.md")).unwrap(),
        "# v1.8.0\n\nMine.\n"
    );
}

#[test]
fn ac2_one_backlog_card_is_issued_for_a_new_version() {
    let f = fixture();
    f.store.add_version(&f.project, "v1.8.0", at(5)).unwrap();
    let tasks = f.tasks();
    assert_eq!(tasks.len(), 1);
    let task = &tasks[0];
    assert_eq!(task.title, "Write the release notes for v1.8.0");
    assert_eq!(task.assignee, Assignee::Ai);
    assert_eq!(task.task_status, Column::Backlog);
    assert_eq!(task.planned_version.as_deref(), Some("v1.8.0"));
    assert!(task.body.contains("docs/release/v1.8.0.md"));
    assert!(task.body.contains("TEMPLATE.md"));
    assert!(task.body.contains("sends it to Todo"));
    assert_eq!(f.task_json(task.id)["releaseNotesVersion"], "v1.8.0");
}

#[test]
fn ac2_the_card_follows_the_board_language() {
    let f = fixture_with(Some(HistoryLanguage::Japanese));
    f.store.add_version(&f.project, "v1.8.0", at(5)).unwrap();
    assert_eq!(f.tasks()[0].title, "v1.8.0 のリリースノートを作成する");
}

#[test]
fn ac2_adding_again_issues_no_second_card() {
    let f = fixture();
    f.store.add_version(&f.project, "v1.8.0", at(5)).unwrap();
    let again = f.store.add_version(&f.project, "v1.8.0", at(6)).unwrap();
    assert!(!again.changed);
    assert_eq!(f.tasks().len(), 1);
    // A second version gets its own card.
    f.store.add_version(&f.project, "v1.9.0", at(7)).unwrap();
    assert_eq!(f.tasks().len(), 2);
}

#[test]
fn ac2_the_marker_is_not_the_title() {
    let f = fixture();
    f.store.add_version(&f.project, "v1.8.0", at(5)).unwrap();
    let id = f.tasks()[0].id;
    // Renaming the card does not matter: the marker is not the title. Archiving does not either.
    f.store
        .edit_task(&f.project, id, Some("Notes"), None, at(6))
        .unwrap();
    f.store.add_version(&f.project, "v1.8.0", at(7)).unwrap();
    assert_eq!(f.tasks().len(), 1);
    f.store.archive_task(&f.project, id, at(8)).unwrap();
    f.store.add_version(&f.project, "v1.8.0", at(9)).unwrap();
    assert_eq!(f.tasks().len(), 1);
}

#[test]
fn ac2_a_retry_after_the_file_exists_but_the_card_is_missing_issues_the_card() {
    let f = fixture();
    // A failed first call: the file is there, the card was never created.
    fs::create_dir_all(f.release()).unwrap();
    fs::write(f.release().join("v1.8.0.md"), STANDARD_RELEASE_TEMPLATE).unwrap();
    assert!(f.tasks().is_empty());
    let retry = f.store.add_version(&f.project, "v1.8.0", at(5)).unwrap();
    assert!(retry.changed);
    assert_eq!(f.tasks().len(), 1);
    assert!(
        !f.store
            .add_version(&f.project, "v1.8.0", at(6))
            .unwrap()
            .changed
    );
    assert_eq!(f.tasks().len(), 1);
}

#[test]
fn ac2_a_task_in_the_way_does_not_block_the_add() {
    let f = fixture();
    f.store.add_version(&f.project, "v1.8.0", at(5)).unwrap();
    let id = f.tasks()[0].id;
    // An unreadable neighbour card must not stop another version from being added.
    f.store.add_version(&f.project, "v1.9.0", at(6)).unwrap();
    let other = f.tasks().into_iter().find(|t| t.id != id).unwrap();
    fs::write(f.dir().join(format!("tasks/{}/task.json", other.id)), "{").unwrap();
    f.store.add_version(&f.project, "v2.0.0", at(7)).unwrap();
    assert!(f.release().join("v2.0.0.md").exists());
}

#[test]
fn ac3_setting_a_new_release_version_issues_one_card_and_adding_keeps_release_unset() {
    let f = fixture();
    f.store.add_version(&f.project, "v1.8.0", at(5)).unwrap();
    assert_eq!(f.store.project_versions(&f.project).unwrap().release, None);
    assert!(!f.project_json().contains("releaseVersion"));

    f.store
        .set_release_version(&f.project, Some("v1.9.0"), at(6))
        .unwrap();
    assert_eq!(f.tasks().len(), 2);
    f.store
        .set_release_version(&f.project, Some("v1.9.0"), at(7))
        .unwrap();
    assert_eq!(f.tasks().len(), 2);
    // Adding a newer version leaves the release version alone.
    f.store.add_version(&f.project, "v2.0.0", at(8)).unwrap();
    let versions = f.store.project_versions(&f.project).unwrap();
    assert_eq!(versions.release.as_deref(), Some("v1.9.0"));
    assert_eq!(versions.versions, ["v2.0.0", "v1.9.0", "v1.8.0"]);
}

#[test]
fn ac3_setting_the_release_version_keeps_the_task_counter() {
    let f = fixture();
    f.store
        .set_release_version(&f.project, Some("v1.8.0"), at(5))
        .unwrap();
    let next = f
        .store
        .create_task(
            &f.project,
            agent_project::domain::NewTask {
                title: "Next".into(),
                description: String::new(),
                assignee: Assignee::Ai,
                actor: agent_project::domain::Actor::Human,
                draft: None,
            },
            at(6),
        )
        .unwrap();
    assert_eq!(next.id, 2);
    assert!(f.project_json().contains("v1.8.0"));
}
