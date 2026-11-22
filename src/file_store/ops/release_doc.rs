//! Generating a version's release document: `docs/release/<version>.md`.
//!
//! The generated part sits between two marker lines; everything else in the file is the
//! person's and is kept. Generating never publishes anything outside the shared folder.

use chrono::{DateTime, Utc};

use super::lock_project;
use super::version::VersionChoice;
use crate::domain::{ProjectId, validate_version_name};
use crate::error::AgpjError;
use crate::file_store::history_language::HistoryLanguage;
use crate::file_store::release_doc::{self, SpliceError};
use crate::file_store::releases::{create_release_file, effective_current, listed_versions};
use crate::file_store::{FileStore, io, layout, reader};

/// The outcome of generating a release document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseDoc {
    pub version: String,
    /// The path inside the project's Docs, such as `docs/release/v1.8.0.md`.
    pub path: String,
    /// The file did not exist and was created.
    pub created: bool,
    /// The file's content changed. Generating again from the same board changes nothing.
    pub changed: bool,
    pub planned: usize,
    pub verified: usize,
    pub done_unchecked: usize,
    pub remaining: usize,
    pub moved: usize,
    /// Planned tasks with a linked publication record.
    pub published: usize,
}

impl FileStore {
    /// Generate the release document of a version. `Current` is the project's current version
    /// and fails with `no_current_version` when there is none; nothing is written then. A named
    /// version that is not listed yet is listed first, like `version add` does.
    ///
    /// Only the block between the markers is written. A file without markers keeps its text and
    /// gets the block appended; a file with damaged markers, or one that changes while it is
    /// generated, is refused and left as it was. The save is atomic.
    pub fn generate_release_doc(
        &self,
        project: &ProjectId,
        choice: &VersionChoice,
        now: DateTime<Utc>,
    ) -> Result<ReleaseDoc, AgpjError> {
        let shared = self.shared_dir();
        let (project_dir, _lock) = lock_project(shared, project)?;
        let listed = listed_versions(&project_dir);
        let version = match choice {
            VersionChoice::Current => {
                let file = reader::read_project_file(shared, project)?;
                effective_current(&listed, file.current_version.as_deref())
                    .ok_or(AgpjError::NoCurrentVersion)?
            }
            VersionChoice::Named(name) => validate_version_name(name)?,
            VersionChoice::Unset => {
                return Err(AgpjError::InvalidArgument {
                    message: "Name a version or use the current version.".into(),
                    command: None,
                });
            }
        };
        let path = layout::release_dir(&project_dir).join(format!("{version}.md"));
        let label = layout::shared_label(shared, &path);
        let project_label = layout::project_label(&project_dir, &path);
        let refuse_linked = || -> Result<(), AgpjError> {
            if layout::has_symlink_below(&project_dir, &path)
                .map_err(|e| io::read_error(&label, e))?
            {
                return Err(AgpjError::ReleaseDocRefused {
                    file: project_label.clone(),
                    reason: "The path passes through a symbolic link.".into(),
                });
            }
            Ok(())
        };
        refuse_linked()?;
        let mut created = false;
        if !listed.contains(&version) {
            if create_release_file(shared, &project_dir, &version)? {
                self.ensure_release_notes_task(&project_dir, project, &version, now)?;
            }
            created = true;
        }
        refuse_linked()?;
        let meta = std::fs::symlink_metadata(&path).map_err(|e| io::read_error(&label, e))?;
        if !meta.is_file() {
            return Err(AgpjError::ReleaseDocRefused {
                file: project_label,
                reason: "The path is not a regular file.".into(),
            });
        }
        let before = io::read_text(&path, &label)?;
        let snapshot = reader::read_snapshot_locked(shared, project)?;
        let language = self.history_language.unwrap_or(HistoryLanguage::English);
        let block = release_doc::render_block(&snapshot.tasks, &version, language);
        let after = release_doc::splice(&before, &block).map_err(|SpliceError::BadMarkers(why)| {
            AgpjError::ReleaseDocRefused {
                file: project_label.clone(),
                reason: format!(
                    "{why} Fix the marker lines {} and {} by hand, or remove both to have the block appended.",
                    release_doc::START_MARKER,
                    release_doc::END_MARKER
                ),
            }
        })?;
        let changed = after != before;
        if changed {
            // A person can edit the file in the Docs without this project's lock: write only
            // if it is still what the new text was built from.
            if io::read_text(&path, &label)? != before {
                return Err(AgpjError::ReleaseDocRefused {
                    file: project_label,
                    reason: "The file changed while it was generated. Run the command again."
                        .into(),
                });
            }
            io::write_atomic(&path, after.as_bytes(), &label)?;
        }
        let groups = release_doc::group(&snapshot.tasks, &version);
        Ok(ReleaseDoc {
            path: project_label,
            created,
            changed,
            planned: groups.planned.len(),
            verified: groups.verified.len(),
            done_unchecked: groups.done_unchecked.len(),
            remaining: groups.remaining.len(),
            moved: groups.moved.len(),
            published: groups.published.len(),
            version,
        })
    }
}
