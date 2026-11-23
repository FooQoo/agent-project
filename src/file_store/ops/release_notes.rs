//! The release-work card that creating a version issues, and the release-version advance when
//! that card reaches Done.

use chrono::{DateTime, Utc};

use crate::domain::{Actor, Assignee, Column, NewTask, ProjectId, compare_versions};
use crate::error::AgpjError;
use crate::file_store::FileStore;
use crate::file_store::history_language::HistoryLanguage;
use crate::file_store::layout;
use crate::file_store::model::{HistoryRecord, TaskFile, TaskMetadata};
use crate::file_store::reader;
use crate::file_store::releases::{effective_release, listed_versions};

use super::{reason_or_default, save_project, stamp};

impl FileStore {
    /// Issue the Backlog release-work card of `name`, unless a card carrying
    /// that version as its `releaseNotesVersion` exists (archived ones count). Returns whether a
    /// card was created. The caller holds the project's write lock.
    pub(super) fn ensure_release_notes_task(
        &self,
        project_dir: &std::path::Path,
        project: &ProjectId,
        name: &str,
        now: DateTime<Utc>,
    ) -> Result<bool, AgpjError> {
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
                &format!("Release {name}"),
                &format!("{name} をリリースする"),
            )
            .to_owned();
        let description = language
            .text(
                &format!(
                    "Do the release work for {name}.\n\n- [ ] Write the release notes in `docs/release/{name}.md` from the template `docs/release/TEMPLATE.md`.\n- [ ] Do the release work: publish the release and confirm that it reached users.\n\nThis card is a plan: start the work only after a person sends it to Todo.\nWhen this card is Done and {name} is the release version, the release version moves to the next listed version.\n"
                ),
                &format!(
                    "{name} のリリース作業をする。\n\n- [ ] `docs/release/{name}.md` のリリースノートを、テンプレート `docs/release/TEMPLATE.md` をもとに書く。\n- [ ] リリース作業をする。リリースを公開し、利用者に届いたことを確かめる。\n\nこのカードは計画です。人が Todo に送ってから着手してください。\nこのカードが Done になり、{name} がリリース版のときは、リリース版が次に登録された版へ進みます。\n"
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

    /// Called under the project's write lock just before `task` is saved as Done. When the task
    /// is a release-work card and its version is the project's release version, move the
    /// release version to the nearest newer listed version and record that in the task's
    /// history. When no newer version is listed, the release version stays and the history
    /// says so. Any other task is left alone.
    pub(super) fn advance_release_for_done(
        &self,
        project: &ProjectId,
        project_dir: &std::path::Path,
        task: &mut TaskFile,
        now: DateTime<Utc>,
    ) -> Result<(), AgpjError> {
        let Some(version) = task.release_notes_version.clone() else {
            return Ok(());
        };
        let shared = self.shared_dir();
        let mut file = reader::read_project_file(shared, project)?;
        let listed = listed_versions(project_dir);
        if effective_release(&listed, file.release_version.as_deref()).as_deref()
            != Some(version.as_str())
        {
            return Ok(());
        }
        let next = listed
            .iter()
            .filter(|v| compare_versions(v, &version).is_gt())
            .min_by(|a, b| compare_versions(a, b))
            .cloned();
        let language = self.history_language(project, task)?;
        let (english, japanese) = match &next {
            Some(next) => (
                format!("Release version moved from {version} to {next}."),
                format!("リリース版を {version} から {next} に進めました。"),
            ),
            None => (
                format!("Release version stays at {version}: no newer version is listed."),
                format!(
                    "リリース版は {version} のままです。これより新しい版は登録されていません。"
                ),
            ),
        };
        if let Some(next) = next {
            file.release_version = Some(next);
            save_project(shared, project_dir, &file)?;
        }
        let reason = reason_or_default(None, language.text(&english, &japanese).to_owned())?;
        task.history.push(HistoryRecord {
            at: stamp(now),
            reason,
        });
        Ok(())
    }
}
