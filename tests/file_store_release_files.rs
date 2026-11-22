//! The version list is the `*.md` files of `docs/release/`, and a task's planned version is the
//! `plannedVersion` key of its TASK.md. Both can be edited by hand.

use std::fs;
use std::path::PathBuf;

use agent_board::domain::{Actor, Assignee, NewTask, ProjectId};
use agent_board::file_store::{FileStore, VersionChoice};
use chrono::{DateTime, TimeZone, Utc};

fn at(day: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, day, 3, 0, 0).unwrap()
}

const BODY: &str = "---\nacceptance:\n  - id: AC-1\n    text: The thing works.\n    done: false\nextra: keep me\n---\n\nDo the work.\n";

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
    store
        .create_task(
            &project,
            NewTask {
                title: "Plan".into(),
                description: BODY.into(),
                assignee: Assignee::Ai,
                actor: Actor::Human,
                draft: None,
            },
            at(4),
        )
        .unwrap();
    Fixture {
        _root: root,
        shared,
        store,
        project,
    }
}

impl Fixture {
    fn dir(&self) -> PathBuf {
        self.shared.join("projects").join(self.project.as_str())
    }

    fn release(&self) -> PathBuf {
        self.dir().join("docs/release")
    }

    fn md_path(&self) -> PathBuf {
        self.dir().join("tasks/1/TASK.md")
    }

    fn md(&self) -> String {
        fs::read_to_string(self.md_path()).unwrap()
    }

    fn versions(&self) -> Vec<String> {
        self.store.project_versions(&self.project).unwrap().versions
    }

    fn planned(&self) -> Option<String> {
        self.store.task(&self.project, 1).unwrap().planned_version
    }

    fn set(&self, version: &str, day: u32) {
        self.store
            .set_task_planned_version(
                &self.project,
                1,
                &VersionChoice::Named(version.into()),
                Actor::Human,
                at(day),
            )
            .unwrap();
    }
}

#[test]
fn the_list_is_the_md_stems_in_natural_order_and_ignores_everything_else() {
    let f = fixture();
    assert!(f.versions().is_empty(), "no release folder yet");
    fs::create_dir_all(f.release().join("v1.12.0.md")).unwrap();
    fs::create_dir_all(f.release().join("sub")).unwrap();
    for (name, text) in [
        ("v1.10.0.md", ""),
        ("v1.9.0.md", "# notes\n"),
        ("v1.8.0.md", ""),
        ("next.md", ""),
        ("第2版.md", ""),
        ("v1.8.0.txt", ""),
        ("v1.7.0", ""),
        (".hidden.md", ""),
        ("has space.md", ""),
        ("sub/v9.md", ""),
    ] {
        fs::write(f.release().join(name), text).unwrap();
    }
    // The folder named like a file, the subfolder, other extensions, hidden and invalid names
    // are not versions. Natural order: v1.9.0 comes before v1.10.0.
    assert_eq!(
        f.versions(),
        ["next", "v1.8.0", "v1.9.0", "v1.10.0", "第2版"]
    );
    // A file elsewhere in the Docs is not a version.
    fs::write(f.dir().join("docs/v5.md"), "").unwrap();
    assert!(!f.versions().contains(&"v5".to_string()));
}

#[test]
fn adding_creates_an_empty_file_and_never_overwrites_one() {
    let f = fixture();
    let added = f.store.add_version(&f.project, "v1.8.0").unwrap();
    assert!(added.changed);
    assert_eq!(added.value.versions, ["v1.8.0"]);
    let file = f.release().join("v1.8.0.md");
    assert_eq!(fs::read(&file).unwrap(), b"");

    // Someone writes the release document; listing the version again keeps it.
    fs::write(&file, "# v1.8.0\n\nNotes.\n").unwrap();
    let again = f.store.add_version(&f.project, "v1.8.0").unwrap();
    assert!(!again.changed);
    assert_eq!(fs::read_to_string(&file).unwrap(), "# v1.8.0\n\nNotes.\n");

    // A hand-made file is already listed.
    fs::write(f.release().join("v2.0.0.md"), "x").unwrap();
    assert!(!f.store.add_version(&f.project, "v2.0.0").unwrap().changed);
    assert_eq!(
        fs::read_to_string(f.release().join("v2.0.0.md")).unwrap(),
        "x"
    );

    // Names that cannot be file names are refused and nothing is written.
    for bad in ["a/b", "..", ".x", "a\\b", "v1:2"] {
        assert!(f.store.add_version(&f.project, bad).is_err(), "{bad}");
    }
    assert_eq!(f.versions(), ["v1.8.0", "v2.0.0"]);
    // No temporary files are left behind.
    let names: Vec<_> = fs::read_dir(f.release())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert!(names.iter().all(|n| n.ends_with(".md")), "{names:?}");
}

#[test]
fn project_json_keeps_no_version_list_and_setting_current_lists_the_version() {
    let f = fixture();
    let current = f
        .store
        .set_current_version(&f.project, Some("v1.8.0"))
        .unwrap();
    assert!(current.changed);
    assert_eq!(current.value.versions, ["v1.8.0"]);
    assert!(f.release().join("v1.8.0.md").is_file());
    let text = fs::read_to_string(f.dir().join("project.json")).unwrap();
    assert!(text.contains("\"currentVersion\": \"v1.8.0\""), "{text}");
    assert!(!text.contains("\"versions\""), "{text}");

    // A leftover `versions` field of an earlier build does not break loading and is dropped.
    let legacy = text.replace(
        "\"currentVersion\"",
        "\"versions\": [\"old\"],\n  \"currentVersion\"",
    );
    fs::write(f.dir().join("project.json"), legacy).unwrap();
    assert_eq!(f.versions(), ["v1.8.0"]);
    f.store.set_current_version(&f.project, None).unwrap();
    let text = fs::read_to_string(f.dir().join("project.json")).unwrap();
    assert!(!text.contains("versions") && !text.contains("currentVersion"));
}

#[test]
fn a_current_version_whose_file_is_gone_reads_as_none_and_comes_back_with_the_file() {
    let f = fixture();
    f.store
        .set_current_version(&f.project, Some("v1.8.0"))
        .unwrap();
    fs::remove_file(f.release().join("v1.8.0.md")).unwrap();

    let read = f.store.project_versions(&f.project).unwrap();
    assert!(read.versions.is_empty());
    assert_eq!(read.current, None);
    assert_eq!(f.store.snapshot(&f.project).unwrap().current_version, None);
    assert!(
        f.store
            .set_task_planned_version(&f.project, 1, &VersionChoice::Current, Actor::Human, at(5))
            .is_err(),
        "the current choice needs a listed current version"
    );
    // project.json still names it: nothing was cleared.
    let text = fs::read_to_string(f.dir().join("project.json")).unwrap();
    assert!(text.contains("v1.8.0"));

    fs::write(f.release().join("v1.8.0.md"), "").unwrap();
    assert_eq!(
        f.store
            .project_versions(&f.project)
            .unwrap()
            .current
            .as_deref(),
        Some("v1.8.0")
    );
}

#[test]
fn a_hand_edit_of_planned_version_is_read_and_a_write_keeps_everything_else() {
    let f = fixture();
    assert_eq!(f.planned(), None);
    // A person adds the key by hand, with a version the project does not list.
    let edited = f.md().replacen("---\n", "---\nplannedVersion: v9.9.9\n", 1);
    fs::write(f.md_path(), edited).unwrap();
    assert_eq!(f.planned().as_deref(), Some("v9.9.9"));
    let snapshot = f.store.snapshot(&f.project).unwrap();
    assert_eq!(snapshot.tasks[0].planned_version.as_deref(), Some("v9.9.9"));
    assert!(snapshot.problems.is_empty());

    // A write through the store checks the name, then records the move in the history.
    assert!(
        f.store
            .set_task_planned_version(
                &f.project,
                1,
                &VersionChoice::Named("v9.9.9".into()),
                Actor::Human,
                at(5)
            )
            .is_err(),
        "an unlisted version is refused"
    );
    f.store.add_version(&f.project, "v1.9.0").unwrap();
    f.set("v1.9.0", 6);
    let after = f.md();
    assert!(after.contains("plannedVersion: v1.9.0"), "{after}");
    assert!(after.contains("extra: keep me"), "{after}");
    assert!(after.contains("Do the work.\n"), "{after}");
    assert!(after.contains("AC-1") && after.contains("The thing works."));
    let view = f.store.task(&f.project, 1).unwrap();
    let last = &view.history.last().unwrap().reason;
    assert_eq!(last, "Changed planned version from v9.9.9 to v1.9.0.");
    assert_eq!(view.history.last().unwrap().at, at(6));
    assert_eq!(view.acceptance.len(), 1);

    // Clearing removes the key and nothing else; other saves keep it in place.
    f.store
        .add_history(&f.project, 1, Some("note"), at(7))
        .unwrap();
    assert_eq!(f.planned().as_deref(), Some("v1.9.0"));
    f.store
        .set_task_planned_version(&f.project, 1, &VersionChoice::Unset, Actor::Human, at(8))
        .unwrap();
    let cleared = f.md();
    assert!(!cleared.contains("plannedVersion"), "{cleared}");
    assert!(cleared.contains("extra: keep me"));
    // A hand edit that removes the key means unplanned.
    f.set("v1.9.0", 9);
    fs::write(f.md_path(), f.md().replace("plannedVersion: v1.9.0\n", "")).unwrap();
    assert_eq!(f.planned(), None);
}

#[test]
fn a_hand_written_value_of_the_wrong_type_is_a_problem_and_loads_as_unplanned() {
    let f = fixture();
    for bad in ["1.8", "[v1, v2]", "{a: b}"] {
        let edited = f
            .md()
            .replacen("---\n", &format!("---\nplannedVersion: {bad}\n"), 1);
        fs::write(f.md_path(), edited).unwrap();
        let snapshot = f.store.snapshot(&f.project).unwrap();
        assert_eq!(snapshot.tasks[0].planned_version, None, "{bad}");
        assert!(
            snapshot
                .problems
                .iter()
                .any(|p| p.field == "plannedVersion"),
            "{bad}: {:?}",
            snapshot.problems
        );
        // The body and the acceptance conditions still load.
        assert_eq!(snapshot.tasks[0].acceptance.len(), 1);
        f.store.task(&f.project, 1).expect("the task still reads");
        fs::write(
            f.md_path(),
            f.md().replacen(&format!("plannedVersion: {bad}\n"), "", 1),
        )
        .unwrap();
    }
    assert!(f.store.snapshot(&f.project).unwrap().problems.is_empty());
}
