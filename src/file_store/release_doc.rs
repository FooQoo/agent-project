//! The generated part of a release document: grouping a project's tasks by one version, writing
//! the Markdown block, and replacing that block inside a file without touching the rest.
//!
//! Everything here is pure. `ops/release_doc.rs` reads the project, calls these functions, and
//! saves the file.

pub use self::render::render_block;

mod render;

use chrono::{DateTime, Utc};

use super::model::{HistoryRecord, TaskView};

/// The line that opens the generated block. Text outside the block is never touched.
pub const START_MARKER: &str = "<!-- agboard:generated:start -->";
/// The line that closes the generated block.
pub const END_MARKER: &str = "<!-- agboard:generated:end -->";

/// A planned-version change read from a task's history.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanChange {
    pub at: DateTime<Utc>,
    pub from: Option<String>,
    pub to: Option<String>,
}

/// Read a history entry written by a planned-version change, in either history language.
pub fn plan_change_of(record: &HistoryRecord) -> Option<PlanChange> {
    let reason = record.reason.trim();
    let (from, to) = if let Some(rest) = reason
        .strip_prefix("Changed planned version from ")
        .and_then(|r| r.strip_suffix('.'))
    {
        let (from, to) = rest.split_once(" to ")?;
        (from, to)
    } else if let Some(rest) = reason
        .strip_prefix("リリース予定の版を")
        .and_then(|r| r.strip_suffix("に変更しました。"))
    {
        rest.split_once("から")?
    } else {
        return None;
    };
    let name = |s: &str| match s {
        "none" | "なし" => None,
        other => Some(other.to_owned()),
    };
    Some(PlanChange {
        at: record.at,
        from: name(from),
        to: name(to),
    })
}

/// One task as the document lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item<'a> {
    pub task: &'a TaskView,
    /// Where the task went, for a task moved out of the version.
    pub moved_to: Option<Option<String>>,
    pub moved_at: Option<DateTime<Utc>>,
}

/// A task is verified when it has acceptance conditions and every one is done.
pub fn is_verified(task: &TaskView) -> bool {
    !task.acceptance_error && !task.acceptance.is_empty() && task.acceptance.iter().all(|a| a.done)
}

/// The groups of one version.
#[derive(Debug, Default)]
pub struct Groups<'a> {
    /// Every task planned for the version now, archived ones included.
    pub planned: Vec<&'a TaskView>,
    pub verified: Vec<&'a TaskView>,
    pub remaining: Vec<&'a TaskView>,
    /// Tasks whose history says they left the version, with where they went.
    pub moved: Vec<Item<'a>>,
}

/// Group `tasks` for `version`. A task counts as planned while its `plannedVersion` is this
/// version. A task counts as moved when its plan is now another one (or none) and its history
/// holds a change away from this version; the last such change decides the destination.
pub fn group<'a>(tasks: &'a [TaskView], version: &str) -> Groups<'a> {
    let mut groups = Groups::default();
    for task in tasks {
        if task.planned_version.as_deref() == Some(version) {
            groups.planned.push(task);
            if is_verified(task) {
                groups.verified.push(task);
            } else {
                groups.remaining.push(task);
            }
            continue;
        }
        let away = task
            .history
            .iter()
            .filter_map(plan_change_of)
            .rfind(|change| change.from.as_deref() == Some(version));
        if let Some(change) = away {
            groups.moved.push(Item {
                task,
                moved_to: Some(change.to),
                moved_at: Some(change.at),
            });
        }
    }
    let by_id = |a: &&TaskView, b: &&TaskView| a.id.cmp(&b.id);
    groups.planned.sort_by(by_id);
    groups.verified.sort_by(by_id);
    groups.remaining.sort_by(by_id);
    groups.moved.sort_by_key(|item| item.task.id);
    groups
}

/// What went wrong when splicing a block into a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpliceError {
    /// The markers are not one start line followed by one end line.
    BadMarkers(String),
}

fn is_marker(line: &str, marker: &str) -> bool {
    line.trim() == marker
}

/// Put `block` into `existing`. With one start and one end marker (in that order) the lines
/// between them, markers included, are replaced. With none, the block is appended after the
/// text, which is kept as it is. Any other arrangement is refused.
pub fn splice(existing: &str, block: &str) -> Result<String, SpliceError> {
    let lines: Vec<&str> = existing.split_inclusive('\n').collect();
    let starts: Vec<usize> = (0..lines.len())
        .filter(|&i| is_marker(lines[i], START_MARKER))
        .collect();
    let ends: Vec<usize> = (0..lines.len())
        .filter(|&i| is_marker(lines[i], END_MARKER))
        .collect();
    match (starts.as_slice(), ends.as_slice()) {
        ([], []) => {
            let mut out = existing.to_owned();
            if !out.is_empty() {
                if !out.ends_with('\n') {
                    out.push('\n');
                }
                out.push('\n');
            }
            out.push_str(block);
            Ok(out)
        }
        ([start], [end]) if start < end => {
            let mut out: String = lines[..*start].concat();
            out.push_str(block);
            out.push_str(&lines[end + 1..].concat());
            Ok(out)
        }
        ([_], [_]) => Err(SpliceError::BadMarkers(
            "The end marker comes before the start marker.".into(),
        )),
        _ => Err(SpliceError::BadMarkers(format!(
            "Found {} start and {} end markers; expected exactly one of each, or none.",
            starts.len(),
            ends.len()
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(reason: &str) -> HistoryRecord {
        HistoryRecord {
            at: "2026-11-01T00:00:00Z".parse().unwrap(),
            reason: reason.into(),
        }
    }

    #[test]
    fn plan_changes_are_read_in_both_languages() {
        let en = plan_change_of(&record("Changed planned version from v1 to v2.")).unwrap();
        assert_eq!(
            (en.from.as_deref(), en.to.as_deref()),
            (Some("v1"), Some("v2"))
        );
        let ja = plan_change_of(&record("リリース予定の版をv1からなしに変更しました。")).unwrap();
        assert_eq!((ja.from.as_deref(), ja.to), (Some("v1"), None));
        assert!(plan_change_of(&record("Something else.")).is_none());
    }

    #[test]
    fn splice_replaces_only_the_block() {
        let text = format!("intro\n{START_MARKER}\nold\n{END_MARKER}\noutro\n");
        let block = format!("{START_MARKER}\nnew\n{END_MARKER}\n");
        assert_eq!(
            splice(&text, &block).unwrap(),
            format!("intro\n{block}outro\n")
        );
    }

    #[test]
    fn splice_appends_when_there_are_no_markers_and_refuses_broken_ones() {
        let block = format!("{START_MARKER}\nnew\n{END_MARKER}\n");
        assert_eq!(splice("mine", &block).unwrap(), format!("mine\n\n{block}"));
        assert!(splice(&format!("{START_MARKER}\nx\n"), &block).is_err());
        assert!(splice(&format!("{END_MARKER}\n{START_MARKER}\n"), &block).is_err());
        assert!(splice(&format!("{block}{block}"), &block).is_err());
    }
}
