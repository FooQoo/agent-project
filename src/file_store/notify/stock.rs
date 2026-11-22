//! Keeping the list of periodic summaries to its newest ten.

use super::{
    MAX_LISTED, NOTIFY_DIR, Summary, files_in, io, layout, project_dir_checked, read_lines,
    write_lines,
};
use crate::domain::ProjectId;
use crate::error::AgpjError;
use crate::file_store::FileStore;
use crate::file_store::lock::WriteLock;

impl FileStore {
    /// The summaries the list shows: not dismissed, newest first, at most [`MAX_LISTED`], even
    /// when hand-written lines or older data hold more.
    pub fn listed_summaries(&self, project: &ProjectId) -> Result<Vec<Summary>, AgpjError> {
        let mut found = self.summaries(project)?;
        found.retain(|s| !s.dismissed);
        found.truncate(MAX_LISTED);
        Ok(found)
    }

    /// Take stock: dismiss every summary beyond the newest [`MAX_LISTED`] that are not dismissed.
    /// Like [`Self::dismiss_summary`], it only sets `dismissed` on those lines, under the
    /// project's write lock, so no line is removed and lines that cannot be read stay as they
    /// are. Returns how many summaries it dismissed.
    pub fn take_stock_of_summaries(&self, project: &ProjectId) -> Result<usize, AgpjError> {
        let project_dir = project_dir_checked(&self.shared, project)?;
        let _lock = WriteLock::acquire(&layout::write_lock(&project_dir))?;
        let files = files_in(&project_dir.join(NOTIFY_DIR));
        let mut live: Vec<Summary> = Vec::new();
        for path in &files {
            let Ok(lines) = read_lines(path, &self.shared) else {
                continue;
            };
            for line in lines {
                if let Ok(s) = serde_json::from_str::<Summary>(&line)
                    && !s.dismissed
                    && !live.iter().any(|l| l.id == s.id)
                {
                    live.push(s);
                }
            }
        }
        live.sort_by(|a, b| (b.to, &b.id).cmp(&(a.to, &a.id)));
        let excess: Vec<&str> = live
            .iter()
            .skip(MAX_LISTED)
            .map(|s| s.id.as_str())
            .collect();
        if excess.is_empty() {
            return Ok(0);
        }
        for path in &files {
            let Ok(mut lines) = read_lines(path, &self.shared) else {
                continue;
            };
            let label = layout::shared_label(&self.shared, path);
            let mut changed = false;
            for line in &mut lines {
                let Ok(mut value) = serde_json::from_str::<serde_json::Value>(line) else {
                    continue;
                };
                let Ok(s) = serde_json::from_value::<Summary>(value.clone()) else {
                    continue;
                };
                if !s.dismissed && excess.contains(&s.id.as_str()) {
                    value["dismissed"] = serde_json::Value::Bool(true);
                    *line =
                        serde_json::to_string(&value).map_err(|e| io::write_error(&label, e))?;
                    changed = true;
                }
            }
            if changed {
                write_lines(path, &lines, &self.shared)?;
            }
        }
        Ok(excess.len())
    }
}
