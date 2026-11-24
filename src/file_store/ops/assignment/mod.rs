//! Agents and their connections.
//!
//! An agent is a registered role (`agents.json`); a connection is the parent session that runs
//! it right now (`.internal/connections.json`). A person registers agents and issues one-time
//! connection codes in the web UI; a session exchanges a code for a secret with `agpj connect`.
//! Every write that depends on a connection checks the secret under the project lock before it
//! changes anything. The secret's digest is stored; the secret itself never is.

mod assign;
mod connect;
mod guard;
mod model;
mod registry;
mod store;

use chrono::{DateTime, Duration, Utc};
use serde::Serialize;
use sha2::{Digest, Sha256};

use super::super::FileStore;
use super::super::io;
use super::super::layout;
use super::super::model::TaskFile;
use super::super::reader;
use super::{HistoryKind, Transition, lock_project, save_task, stamp, transition_event};
use crate::domain::{Assignee, Column, ProjectId, RunState};
use crate::error::AgpjError;

pub(in crate::file_store::ops) use connect::fresh_connection_locked;
pub use guard::{SessionWrites, enter_session_writes};
pub(in crate::file_store::ops) use guard::{
    check_session_write, default_agent_id_locked, ensure_session_may_write, exempt,
};
pub use model::*;
use model::{ConnectCode, ConnectionRecord, ConnectionsFile};
pub(in crate::file_store::ops) use registry::{
    ensure_agent_live_locked, has_other_live_agents_locked,
};
pub(crate) use registry::{registered_agents, registered_default_agent_id};
pub(crate) use store::digest_of;
pub(in crate::file_store::ops) use store::verify_locked;
use store::*;
