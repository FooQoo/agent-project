//! Checks and derived values of the text a result carries: the cleaned fields, the content key of
//! a delivery, and the history excerpt.

use sha2::{Digest, Sha256};

use std::fs;
use std::path::Path;

use super::super::super::frontmatter::{self, ResultPlan};
use super::super::super::layout;
use super::super::super::model::TaskFile;
use super::super::super::reader;
use super::result::{MAX_RESULT_KEY_CHARS, MAX_RESULT_TEXT_CHARS};
use crate::domain::{AttachmentInput, MAX_RESULT_TITLE_CHARS, ProjectId, ResultKind, TaskResult};
use crate::error::AgpjError;

pub(super) fn invalid(field: &str, reason: &str) -> AgpjError {
    AgpjError::InvalidValue {
        field: field.into(),
        reason: reason.into(),
    }
}

pub(super) fn clean_text(field: &str, value: &str) -> Result<String, AgpjError> {
    let text = value.replace("\r\n", "\n").replace('\r', "\n");
    let text = text.trim();
    let count = text.chars().count();
    if count == 0 || count > MAX_RESULT_TEXT_CHARS {
        return Err(invalid(
            field,
            &format!("A {field} must be 1 to {MAX_RESULT_TEXT_CHARS} characters."),
        ));
    }
    Ok(text.to_owned())
}

pub(super) fn clean_title(value: &str) -> Result<String, AgpjError> {
    let title = value.trim();
    let count = title.chars().count();
    if count == 0 || count > MAX_RESULT_TITLE_CHARS {
        return Err(invalid(
            "title",
            &format!("A title must be 1 to {MAX_RESULT_TITLE_CHARS} characters."),
        ));
    }
    if title.chars().any(char::is_control) {
        return Err(invalid("title", "A title must be on one line."));
    }
    Ok(title.to_owned())
}

pub(super) fn clean_link(value: &str) -> Result<String, AgpjError> {
    let link = clean_text("link", value)?;
    if link.chars().any(char::is_control) {
        return Err(invalid("link", "A link must be on one line."));
    }
    Ok(link)
}

pub(super) fn clean_key(value: &str) -> Result<String, AgpjError> {
    let key = value.trim();
    let valid = !key.is_empty()
        && key.chars().count() <= MAX_RESULT_KEY_CHARS
        && key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | ':' | '-'));
    if !valid {
        return Err(invalid(
            "key",
            "A key must be 1 to 64 characters of letters, digits, '.', '_', ':' and '-'.",
        ));
    }
    Ok(key.to_owned())
}

/// 32 lowercase hex digits taken from the content of a delivery.
pub(super) fn content_key(
    kind: ResultKind,
    summary: &str,
    link: Option<&str>,
    images: &[AttachmentInput],
    plan: Option<&str>,
) -> String {
    let mut hash = Sha256::new();
    hash.update(kind.as_str().as_bytes());
    for part in [Some(summary), link] {
        hash.update([0]);
        hash.update(part.unwrap_or("").as_bytes());
    }
    // The separator before the image part is always written, so a result without
    // images and a result with one image keep the keys they had before several
    // images were allowed.
    hash.update([0]);
    for (index, image) in images.iter().enumerate() {
        if index > 0 {
            hash.update([0]);
        }
        hash.update(Sha256::digest(&image.bytes));
    }
    // A delivery against a plan differs from the same content without one. Nothing is written
    // for no plan, so the keys of results recorded without a plan are unchanged.
    if let Some(plan) = plan {
        hash.update([0]);
        hash.update(b"plan:");
        hash.update(plan.as_bytes());
    }
    hash.finalize()[..16]
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// The first line of the summary, cut to a length a history entry can carry.
pub(super) fn excerpt(summary: &str) -> String {
    let line = summary.lines().next().unwrap_or("").trim();
    if line.chars().count() > 200 {
        let cut: String = line.chars().take(199).collect();
        format!("{cut}…")
    } else {
        line.to_owned()
    }
}

/// The plan `result add --plan` names, read from the task's `TASK.md` (the caller holds the
/// project lock). An unknown ID, a plan of another task included, is `result_plan_not_found`;
/// `--final` is refused for an intermediate plan, and a publication record cannot deliver a plan.
pub(super) fn resolve_plan(
    shared: &Path,
    project: &ProjectId,
    id: i64,
    plan_id: &str,
    kind: ResultKind,
    publication: bool,
) -> Result<ResultPlan, AgpjError> {
    if publication {
        return Err(invalid(
            "plan",
            "A publication record cannot deliver a result plan.",
        ));
    }
    let plan_id = plan_id.trim();
    let text = reader::read_body(shared, project, id)?;
    let (plans, _) = frontmatter::result_plans_of(&text, true);
    let plan = plans.into_iter().find(|p| p.id == plan_id).ok_or_else(|| {
        AgpjError::ResultPlanNotFound {
            id,
            plan: plan_id.to_owned(),
        }
    })?;
    if kind == ResultKind::Final && !plan.is_final {
        return Err(invalid(
            "final",
            &format!(
                "{} is an intermediate plan; --final marks only the final plan's result.",
                plan.id
            ),
        ));
    }
    Ok(plan)
}

/// Whether the images stored for `existing` are, byte for byte, the images of a new delivery.
pub(super) fn same_images(
    task: &TaskFile,
    project_dir: &Path,
    id: i64,
    existing: &TaskResult,
    images: &[AttachmentInput],
) -> bool {
    let stored = existing.image_ids();
    stored.len() == images.len()
        && stored.iter().zip(images).all(|(attachment_id, image)| {
            task.attachments
                .iter()
                .find(|a| a.id == *attachment_id)
                .and_then(|a| {
                    let ext = layout::asset_extension(&a.media_type)?;
                    let path = layout::task_asset_path(project_dir, id, &a.id, ext).ok()?;
                    fs::read(path).ok()
                })
                .is_some_and(|bytes| Sha256::digest(&bytes) == Sha256::digest(&image.bytes))
        })
}
