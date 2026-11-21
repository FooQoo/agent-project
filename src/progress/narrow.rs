//! Narrowing the progress table to the tasks of one version.

use std::collections::HashSet;

use super::Progress;
use crate::domain::VersionFilter;
use crate::file_store::reader::ProjectSnapshot;

/// Hide the tasks that `filter` does not keep from every task list of the table.
pub(super) fn narrow(table: &mut Progress, snapshot: &ProjectSnapshot, filter: &VersionFilter) {
    let kept: HashSet<i64> = snapshot
        .tasks
        .iter()
        .filter(|task| filter.keeps(task.planned_version.as_deref()))
        .map(|task| task.id)
        .collect();
    for epic in &mut table.epics {
        epic.tasks.retain(|task| kept.contains(&task.id));
    }
    table.epics.retain(|epic| !epic.tasks.is_empty());
    table.unassigned.retain(|task| kept.contains(&task.id));
    table.next_tasks.retain(|task| kept.contains(&task.id));
    table
        .final_result_missing
        .retain(|task| kept.contains(&task.id));
    for section in [&mut table.pending, &mut table.done].into_iter().flatten() {
        section.retain(|task| kept.contains(&task.id));
    }
}
