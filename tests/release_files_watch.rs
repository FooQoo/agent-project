//! A version file or a `plannedVersion` written by hand reaches the board: the watcher reloads
//! the project, announces a `changed`, and the summary version moves.

use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use agent_project::domain::{Actor, Assignee, NewTask, ProjectId};
use agent_project::file_store::FileStore;
use agent_project::file_store::index::{ProjectEntry, ProjectIndex};
use agent_project::file_store::projection::ChangeSet;
use agent_project::file_store::watch::Watch;
use chrono::{TimeZone, Utc};
use tokio::sync::broadcast;

struct Fixture {
    _root: tempfile::TempDir,
    shared: PathBuf,
    project: ProjectId,
    index: Arc<RwLock<ProjectIndex>>,
    changed: broadcast::Receiver<ChangeSet>,
    _watch: Watch,
}

impl Fixture {
    async fn start() -> Fixture {
        let now = Utc.with_ymd_and_hms(2026, 10, 10, 0, 0, 0).unwrap();
        let root = tempfile::tempdir().unwrap();
        let shared = root.path().join("shared");
        let store = FileStore::open(&shared);
        let project = store.create_project("Alpha", now).unwrap().id;
        store
            .create_task(
                &project,
                NewTask {
                    title: "First".into(),
                    description: String::new(),
                    assignee: Assignee::Ai,
                    actor: Actor::Human,
                    draft: None,
                },
                now,
            )
            .unwrap();
        let index = Arc::new(RwLock::new(ProjectIndex::default()));
        let (sender, changed) = broadcast::channel(16);
        let (watch, _reloader) = Watch::start(&shared, index.clone(), sender).unwrap();
        Fixture {
            _root: root,
            shared,
            project,
            index,
            changed,
            _watch: watch,
        }
    }

    fn dir(&self) -> PathBuf {
        self.shared.join("projects").join(self.project.as_str())
    }

    async fn changed(&mut self) -> bool {
        matches!(
            tokio::time::timeout(Duration::from_secs(3), self.changed.recv()).await,
            Ok(Ok(_))
        )
    }

    fn summary_version(&self) -> String {
        let index = self.index.read().unwrap();
        index.entry(&self.project).unwrap().version().to_string()
    }

    fn versions(&self) -> Vec<String> {
        let index = self.index.read().unwrap();
        match index.entry(&self.project).unwrap() {
            ProjectEntry::Ready { projection, .. } => projection.versions.clone(),
            other => panic!("expected a ready project, got {other:?}"),
        }
    }

    fn planned(&self) -> Option<String> {
        let index = self.index.read().unwrap();
        match index.entry(&self.project).unwrap() {
            ProjectEntry::Ready { view, .. } => view.tasks[0].planned_version.clone(),
            other => panic!("expected a ready project, got {other:?}"),
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_version_file_made_or_removed_by_hand_changes_the_list_and_announces_it() {
    let mut fx = Fixture::start().await;
    let before = fx.summary_version();
    assert!(fx.versions().is_empty());

    let release = fx.dir().join("docs/release");
    fs::create_dir_all(&release).unwrap();
    fs::write(release.join("v1.8.0.md"), "").unwrap();
    assert!(
        fx.changed().await,
        "no changed event for a new version file"
    );
    assert_eq!(fx.versions(), ["v1.8.0"]);
    let listed = fx.summary_version();
    assert_ne!(listed, before);

    // Other Docs files are not versions and do not move the board.
    fs::write(fx.dir().join("docs/notes.md"), "x").unwrap();
    fs::write(release.join("v1.8.0.txt"), "x").unwrap();
    tokio::time::sleep(Duration::from_millis(800)).await;
    assert_eq!(fx.summary_version(), listed);

    fs::remove_file(release.join("v1.8.0.md")).unwrap();
    while fx.changed().await && !fx.versions().is_empty() {}
    assert!(fx.versions().is_empty());
    assert_ne!(fx.summary_version(), listed);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_hand_edit_of_planned_version_in_task_md_announces_a_change() {
    let mut fx = Fixture::start().await;
    assert_eq!(fx.planned(), None);
    let before = fx.summary_version();

    let path = fx.dir().join("tasks/1/TASK.md");
    let text = fs::read_to_string(&path).unwrap();
    fs::write(
        &path,
        text.replacen("---\n", "---\nplannedVersion: v1.8.0\n", 1),
    )
    .unwrap();
    assert!(fx.changed().await, "no changed event for the edit");
    assert_eq!(fx.planned().as_deref(), Some("v1.8.0"));
    assert_ne!(fx.summary_version(), before);
}
