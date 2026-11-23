//! What a brainstorm is about: one epic or one task of the project.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::domain::ProjectId;
use crate::error::AgpjError;
use crate::file_store::validate::EpicId;
use crate::file_store::{layout, reader};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TargetKind {
    Epic,
    Task,
}

/// The epic or task a brainstorm deepens: `{"type": "epic" | "task", "id": "<string>"}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Target {
    #[serde(rename = "type")]
    pub kind: TargetKind,
    pub id: String,
}

fn invalid(reason: &str) -> AgpjError {
    AgpjError::InvalidValue {
        field: "target".into(),
        reason: reason.into(),
    }
}

impl Target {
    /// A task number (all ASCII digits) or an epic ID. Nothing is checked here.
    pub fn parse(text: &str) -> Target {
        let text = text.trim();
        let kind = if !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit()) {
            TargetKind::Task
        } else {
            TargetKind::Epic
        };
        Target {
            kind,
            id: text.to_string(),
        }
        .normalized()
    }

    /// The canonical form: an epic ID in uppercase, a task number without leading zeros.
    pub fn normalized(self) -> Target {
        let id = match self.kind {
            TargetKind::Epic => self.id.to_ascii_uppercase(),
            TargetKind::Task => self.id.parse::<i64>().map_or(self.id, |n| n.to_string()),
        };
        Target { id, ..self }
    }

    /// How a sentence names it: `epic X` or `task #N`.
    pub fn label(&self) -> String {
        match self.kind {
            TargetKind::Epic => format!("epic {}", self.id),
            TargetKind::Task => format!("task #{}", self.id),
        }
    }
}

/// Check that the target exists in the project: `task_not_found`, `task_archived`, or
/// `epic_not_found` otherwise.
pub fn check(shared: &Path, project: &ProjectId, target: &Target) -> Result<(), AgpjError> {
    match target.kind {
        TargetKind::Task => {
            let id: i64 = target
                .id
                .parse()
                .map_err(|_| invalid("A task target is a task number."))?;
            let task = reader::read_task_strict(shared, project, id)?;
            if task.archived_at.is_some() {
                return Err(AgpjError::TaskArchived { id });
            }
            Ok(())
        }
        TargetKind::Epic => {
            let not_found = || AgpjError::EpicNotFound {
                id: target.id.clone(),
            };
            let epic = EpicId::normalize(&target.id).map_err(|_| not_found())?;
            let project_dir = reader::project_dir_checked(shared, project)?;
            if layout::epic_dir(&project_dir, &epic).is_dir() {
                Ok(())
            } else {
                Err(not_found())
            }
        }
    }
}
