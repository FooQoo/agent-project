//! The Backlog card that creating a version issues for writing its release notes.

use chrono::{DateTime, Utc};

use crate::domain::{Actor, Assignee, Column, NewTask, ProjectId};
use crate::error::AgboardError;
use crate::file_store::FileStore;
use crate::file_store::history_language::HistoryLanguage;
use crate::file_store::layout;
use crate::file_store::model::TaskMetadata;
use crate::file_store::reader;

impl FileStore {
    /// Issue the Backlog card for writing the release notes of `name`, unless a card carrying
    /// that version as its `releaseNotesVersion` exists (archived ones count). Returns whether a
    /// card was created. The caller holds the project's write lock.
    pub(super) fn ensure_release_notes_task(
        &self,
        project_dir: &std::path::Path,
        project: &ProjectId,
        name: &str,
        now: DateTime<Utc>,
    ) -> Result<bool, AgboardError> {
        for id in reader::task_ids(project_dir)? {
            let path = layout::task_json(&layout::task_dir(project_dir, id));
            // A card that cannot be read cannot be the marked one; it must not block the add.
            let Ok(bytes) = std::fs::read(&path) else {
                continue;
            };
            let Ok(meta) = serde_json::from_slice::<TaskMetadata>(&bytes) else {
                continue;
            };
            if meta.release_notes_version.as_deref() == Some(name) {
                return Ok(false);
            }
        }
        let language = self.history_language.unwrap_or(HistoryLanguage::English);
        let title = language
            .text(
                &format!("Write the release notes for {name}"),
                &format!("{name} のリリースノートを作成する"),
            )
            .to_owned();
        let description = language
            .text(
                &format!(
                    "Write the release notes in `docs/release/{name}.md` from the template `docs/release/TEMPLATE.md`.\n\nThis card is a plan: start the work only after a person sends it to Todo.\n"
                ),
                &format!(
                    "`docs/release/{name}.md` のリリースノートを、テンプレート `docs/release/TEMPLATE.md` をもとに書く。\n\nこのカードは計画です。人が Todo に送ってから着手してください。\n"
                ),
            )
            .to_owned();
        self.create_task_locked(
            project_dir,
            project,
            NewTask {
                title,
                description,
                assignee: Assignee::Ai,
                actor: Actor::Ai,
                draft: None,
            },
            Column::Backlog,
            Some(name.to_owned()),
            now,
        )?;
        Ok(true)
    }
}
