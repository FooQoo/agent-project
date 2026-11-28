//! The write guard: who may change a task from Doing onward.
//!
//! A command-line process marks itself as a session once ([`enter_session_writes`]); the web server
//! never does, because a person acts there. While marked, every task save checks, under the project
//! lock and against the files as they are, that the connection of the calling session belongs to
//! the agent the task is assigned to. A Todo plan stays co-editable. Nothing is trusted from the
//! caller: not the task number, an agent name, a session ID, or the claimed actor.

use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};

use super::*;
use crate::file_store::model::TaskMetadata;

thread_local! {
    static SESSION: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
    static EXEMPT: Cell<u32> = const { Cell::new(0) };
}

/// Marks this thread as a session's command line until dropped.
pub struct SessionWrites(());

/// Check every task write of this thread against the session's connection. `data_dir` is where
/// the session's secret is kept.
pub fn enter_session_writes(data_dir: PathBuf) -> SessionWrites {
    SESSION.with(|s| *s.borrow_mut() = Some(data_dir));
    SessionWrites(())
}

impl Drop for SessionWrites {
    fn drop(&mut self) {
        SESSION.with(|s| *s.borrow_mut() = None);
    }
}

/// Planning and bookkeeping writes that belong to the project, not to the agent that works a
/// card (version plans, release notes, epic membership, pausing runs): not checked while held.
pub(in crate::file_store::ops) struct Exempt(());

pub(in crate::file_store::ops) fn exempt() -> Exempt {
    EXEMPT.with(|e| e.set(e.get() + 1));
    Exempt(())
}

impl Drop for Exempt {
    fn drop(&mut self) {
        EXEMPT.with(|e| e.set(e.get().saturating_sub(1)));
    }
}

fn guarded(assignee: Assignee, status: Column) -> bool {
    assignee == Assignee::Ai && matches!(status, Column::Doing | Column::Done)
}

/// Check a save of `task` against what is on disk (`old`). Refuses before anything is written.
pub(in crate::file_store::ops) fn check_session_write(
    shared: &Path,
    dir: &Path,
    old: Option<&TaskMetadata>,
    task: &TaskFile,
) -> Result<(), AgpjError> {
    check_session_write_inner(shared, dir, old, task, true)
}

/// Check a task write without refreshing connection activity. Multi-task operations use this
/// for their full authorization preflight, then perform the ordinary check once all targets pass.
pub(in crate::file_store::ops) fn check_session_write_without_touch(
    shared: &Path,
    dir: &Path,
    old: Option<&TaskMetadata>,
    task: &TaskFile,
) -> Result<(), AgpjError> {
    check_session_write_inner(shared, dir, old, task, false)
}

fn check_session_write_inner(
    shared: &Path,
    dir: &Path,
    old: Option<&TaskMetadata>,
    task: &TaskFile,
    touch: bool,
) -> Result<(), AgpjError> {
    let Some(data_dir) = SESSION.with(|s| s.borrow().clone()) else {
        return Ok(());
    };
    if EXEMPT.with(Cell::get) > 0 {
        return Ok(());
    }
    let old_guarded = old.is_some_and(|o| guarded(o.assignee, o.task_status));
    if !old_guarded && !guarded(task.assignee, task.task_status) {
        return Ok(());
    }
    let project = dir
        .file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| name.parse::<ProjectId>().ok())
        .ok_or(AgpjError::NotConnected)?;
    let old_agent = match old {
        Some(o) => o.agent_id.as_deref(),
        None => task.agent_id.as_deref(),
    };
    let owner = resolve_agent_id_locked(shared, dir, old_agent)?;
    let secret = crate::agent::connection_store::load_secret(&data_dir, &project)
        .ok_or(AgpjError::NotConnected)?;
    let authorize = |verified: &Verified| {
        let agent = verified.agent_id.as_str();
        // A session never hands a card to another agent; only a person does.
        let kept = task.agent_id.as_deref() == old_agent
            || (old_agent.is_none() && task.agent_id.as_deref() == Some(agent));
        if owner.as_deref() != Some(agent) || !kept {
            return Err(AgpjError::NotAssignee { id: task.id });
        }
        Ok(())
    };
    if touch {
        authorize_locked(shared, dir, &secret, stamp(chrono::Utc::now()), authorize)?;
    } else {
        super::authorize_locked_without_touch(shared, dir, &secret, authorize)?;
    }
    Ok(())
}

/// [`check_session_write`] for an operation that writes side files before it saves the task:
/// run it first, so a refused write leaves nothing behind.
pub(in crate::file_store::ops) fn ensure_session_may_write(
    shared: &Path,
    dir: &Path,
    task: &TaskFile,
) -> Result<(), AgpjError> {
    check_session_write(shared, dir, None, task)
}

/// The default agent's ID. It is created with the first AI task, so every AI task has an agent.
pub(in crate::file_store::ops) fn default_agent_id_locked(
    shared: &Path,
    dir: &Path,
    language: crate::file_store::history_language::HistoryLanguage,
    now: DateTime<Utc>,
) -> Result<String, AgpjError> {
    let mut file: AgentsFile = load(shared, &agents_path(dir))?;
    if let Some(agent) = file
        .agents
        .iter()
        .find(|a| a.is_default && a.archived_at.is_none())
    {
        return Ok(agent.id.clone());
    }
    let base = match language {
        crate::file_store::history_language::HistoryLanguage::Japanese => "エージェント",
        crate::file_store::history_language::HistoryLanguage::English => "Agent",
    };
    let mut name = base.to_string();
    let mut n = 1;
    while ensure_name_free(&file, &name, None).is_err() {
        n += 1;
        name = format!("{base} {n}");
    }
    let id = format!("A-{}", file.next_agent_id);
    file.next_agent_id += 1;
    file.agents.push(AgentRecord {
        id: id.clone(),
        name,
        role: String::new(),
        tool: None,
        created_at: stamp(now),
        archived_at: None,
        is_default: true,
    });
    save(shared, &agents_path(dir), &file)?;
    Ok(id)
}
