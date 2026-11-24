//! Saving a task's `task.json` and `TASK.md` as one pair.

use std::path::Path;

use crate::error::AgpjError;
use crate::file_store::frontmatter::{self};
use crate::file_store::io;
use crate::file_store::layout;
use crate::file_store::model::TaskFile;

pub(super) fn save_task(
    shared: &Path,
    project_dir: &Path,
    task: &TaskFile,
) -> Result<(), AgpjError> {
    save_task_with_markdown(shared, project_dir, task, None)
}

/// [`save_task`] for planning and bookkeeping that belongs to the project, not to the agent that
/// works the card: the session write guard does not apply.
pub(super) fn save_task_planning(
    shared: &Path,
    project_dir: &Path,
    task: &TaskFile,
) -> Result<(), AgpjError> {
    let _exempt = super::assignment::exempt();
    save_task(shared, project_dir, task)
}

pub(super) fn save_task_with_markdown(
    shared: &Path,
    project_dir: &Path,
    task: &TaskFile,
    replacement: Option<&str>,
) -> Result<(), AgpjError> {
    save_task_text(shared, project_dir, task, replacement).map(drop)
}

pub(super) fn confirm_external_edit(
    shared: &Path,
    project_dir: &Path,
    task: &TaskFile,
) -> Result<(), AgpjError> {
    use crate::file_store::transaction::save_pair_locked;
    let (targets, old, mut new) = task_pair(shared, project_dir, task, None)?;
    let mut metadata: crate::file_store::model::TaskMetadata =
        serde_json::from_slice(&new.metadata)
            .map_err(|error| io::write_error("task.json", error))?;
    metadata.external_edit = false;
    new.metadata = serde_json::to_vec_pretty(&metadata)
        .map_err(|error| io::write_error("task.json", error))?;
    new.metadata.push(b'\n');
    save_pair_locked(project_dir, &targets, &old, &new)
}

/// Like `save_task_with_markdown`, and returns the TASK.md text that was committed,
/// with `task.history` composed in, so a caller needs no read after the commit.
pub(super) fn save_task_text(
    shared: &Path,
    project_dir: &Path,
    task: &TaskFile,
    replacement: Option<&str>,
) -> Result<String, AgpjError> {
    use crate::file_store::transaction::save_pair_locked;
    let (targets, old, new) = task_pair(shared, project_dir, task, replacement)?;
    save_pair_locked(project_dir, &targets, &old, &new)?;
    Ok(String::from_utf8(new.markdown).expect("the composed TASK.md is UTF-8"))
}

/// The two files of a task as one change of a batch.
pub(super) fn task_pair_changes(
    shared: &Path,
    project_dir: &Path,
    task: &TaskFile,
    replacement: Option<&str>,
) -> Result<Vec<crate::file_store::transaction::FileChange>, AgpjError> {
    use crate::file_store::transaction::FileChange;
    let (targets, old, new) = task_pair(shared, project_dir, task, replacement)?;
    Ok(vec![
        FileChange {
            target: targets.metadata,
            old: Some(old.metadata),
            new: Some(new.metadata),
        },
        FileChange {
            target: targets.markdown,
            old: Some(old.markdown),
            new: Some(new.markdown),
        },
    ])
}

/// The targets of a task's pair, their current bytes, and the bytes that hold `task`.
fn task_pair(
    shared: &Path,
    project_dir: &Path,
    task: &TaskFile,
    replacement: Option<&str>,
) -> Result<
    (
        crate::file_store::transaction::PairTargets,
        crate::file_store::transaction::PairBytes,
        crate::file_store::transaction::PairBytes,
    ),
    AgpjError,
> {
    use crate::file_store::history::{ParseMode, compose_task, parse_task};
    use crate::file_store::model::TaskMetadata;
    use crate::file_store::transaction::{PairBytes, PairTargets};

    let dir = layout::task_dir(project_dir, task.id);
    let json_path = layout::task_json(&dir);
    let md_path = layout::task_md(&dir);
    let json_label = layout::shared_label(shared, &json_path);
    let md_label = layout::shared_label(shared, &md_path);
    let old = PairBytes {
        metadata: io::read_bytes(&json_path, &json_label)?,
        markdown: io::read_bytes(&md_path, &md_label)?,
    };
    let text = std::str::from_utf8(&old.markdown).map_err(|_| {
        crate::error::AgpjError::FileUnreadable {
            file: md_label.clone(),
            reason: "the file is not valid UTF-8".into(),
        }
    })?;
    let old_meta: Option<crate::file_store::model::TaskMetadata> =
        serde_json::from_slice(&old.metadata).ok();
    super::assignment::check_session_write(shared, project_dir, old_meta.as_ref(), task)?;
    let parsed = parse_task(text, ParseMode::Stored);
    if let Some(defect) = parsed.history_defects.first() {
        return Err(AgpjError::FileInvalid {
            file: md_label,
            field: defect.field.clone(),
            reason: defect.reason.clone(),
        });
    }
    if task.history.len() < parsed.history.len()
        || task.history[..parsed.history.len()] != parsed.history
    {
        return Err(io::write_error(
            &json_label,
            "The task history changed before the operation could be saved.",
        ));
    }
    // A direct edit may arrive before the watcher. Record it during this save as well.
    let direct_edit = old_meta.as_ref().is_some_and(|meta| {
        meta.content_digest
            .as_ref()
            .is_some_and(|digest| *digest != content_digest(&old.markdown))
            && matches!(
                meta.task_status,
                crate::domain::Column::Doing | crate::domain::Column::Done
            )
    });
    // A direct edit that only ticked or cleared acceptance checkboxes is a legitimate way to
    // report progress: the checkboxes drive automatic completion. Any other change, including a
    // body change made in the same edit, fails the shape comparison and stays a direct edit.
    let checkbox_only = direct_edit
        && old_meta.as_ref().is_some_and(|meta| {
            is_checkbox_only_edit(
                &old.markdown,
                meta.content_digest.as_deref(),
                meta.shape_digest.as_deref(),
            )
        });
    let direct_edit = direct_edit && !checkbox_only;
    let mut history = task.history.clone();
    if direct_edit && !old_meta.as_ref().is_some_and(|m| m.external_edit) {
        history.push(crate::file_store::model::HistoryRecord {
            at: crate::clock::now(),
            reason: "The task file was updated directly; review is required.".into(),
        });
    }
    let markdown = if let Some(replacement) = replacement {
        let parts = frontmatter::parse(replacement);
        compose_task(replacement, &parts.body, None, &history)
            .map_err(|defect| AgpjError::FileInvalid {
                file: md_label.clone(),
                field: defect.field,
                reason: defect.reason,
            })?
            .into_bytes()
    } else if history == parsed.history {
        old.markdown.clone()
    } else {
        compose_task(text, &parsed.body, None, &history)
            .map_err(|defect| AgpjError::FileInvalid {
                file: md_label.clone(),
                field: defect.field,
                reason: defect.reason,
            })?
            .into_bytes()
    };
    let markdown = sync_planned_version(text, markdown, task, &md_label)?;
    let mut metadata = TaskMetadata::from(task);
    metadata.content_digest = Some(content_digest(&markdown));
    metadata.shape_digest = Some(shape_digest(&markdown));
    metadata.external_edit =
        direct_edit || task.external_edit || old_meta.as_ref().is_some_and(|m| m.external_edit);
    let mut json = serde_json::to_vec_pretty(&metadata)
        .map_err(|error| io::write_error(&json_label, error))?;
    json.push(b'\n');
    Ok((
        PairTargets {
            metadata: std::path::PathBuf::from(format!("tasks/{}/task.json", task.id)),
            markdown: std::path::PathBuf::from(format!("tasks/{}/TASK.md", task.id)),
        },
        old,
        PairBytes {
            metadata: json,
            markdown,
        },
    ))
}

/// Make the `plannedVersion` of the composed TASK.md match `task`. The value the task was read
/// with is the one in the old text, so an operation that did not touch it changes nothing, a
/// `plannedVersion` that cannot be read is left as written, and an operation that set or cleared
/// it writes the key.
fn sync_planned_version(
    old_text: &str,
    markdown: Vec<u8>,
    task: &TaskFile,
    label: &str,
) -> Result<Vec<u8>, AgpjError> {
    let composed = String::from_utf8(markdown).expect("the composed TASK.md is UTF-8");
    let old = frontmatter::planned_version_of(old_text);
    let now = frontmatter::planned_version_of(&composed);
    let write = match (&old, &now) {
        // Unreadable before and not set by the operation: keep what the person wrote.
        (Err(_), _) => task.planned_version.is_some(),
        (Ok(before), _) if *before != task.planned_version => true,
        // A replacement text that dropped the key keeps the task's plan.
        (Ok(_), Ok(after)) => after.is_none() && task.planned_version.is_some(),
        (Ok(_), Err(_)) => false,
    };
    if !write {
        return Ok(composed.into_bytes());
    }
    frontmatter::compose_planned_version(&composed, task.planned_version.as_deref())
        .map(String::into_bytes)
        .map_err(|defect| AgpjError::FileInvalid {
            file: label.to_owned(),
            field: defect.field,
            reason: defect.reason,
        })
}

/// Whether `markdown` differs from the accepted file (`digest`, `shape`) only in acceptance
/// checkboxes.
pub(super) fn is_checkbox_only_edit(
    markdown: &[u8],
    digest: Option<&str>,
    shape: Option<&str>,
) -> bool {
    digest.is_some_and(|digest| digest != content_digest(markdown))
        && shape == Some(shape_digest(markdown).as_str())
}

/// Digest the Markdown with every `done: true|false` of the acceptance block of the front matter
/// read as `done: false`, so two files that differ only in checkbox states have one shape.
pub(super) fn shape_digest(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes);
    let mut out = String::with_capacity(text.len());
    let mut fence = 0;
    let mut in_acceptance = false;
    for line in text.split_inclusive('\n') {
        let trimmed = line.trim_end();
        if fence < 2 && trimmed == "---" {
            fence += 1;
            in_acceptance = false;
        } else if fence == 1 {
            let top_level = !line.starts_with([' ', '\t', '-']) && !trimmed.is_empty();
            if top_level {
                in_acceptance = trimmed.starts_with("acceptance:");
            }
            if in_acceptance && let Some(at) = line.find("done:") {
                let after = &line[at + 5..];
                let value = after.trim_start();
                let token_len = value
                    .find(|c: char| !c.is_ascii_alphanumeric())
                    .unwrap_or(value.len());
                if matches!(&value[..token_len], "true" | "false") {
                    out.push_str(&line[..at]);
                    out.push_str("done: false");
                    out.push_str(&value[token_len..]);
                    continue;
                }
            }
        }
        out.push_str(line);
    }
    content_digest(out.as_bytes())
}

/// Digest the exact persisted Markdown bytes.
pub(super) fn content_digest(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(bytes))
}
