//! The stored records and the views of agents and connections.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// The `formatVersion` of `agents.json` and `.internal/connections.json`.
pub const REGISTRY_FORMAT_VERSION: u32 = 1;
/// The longest agent name, in characters.
pub const MAX_AGENT_NAME_CHARS: usize = 50;
/// The longest one-line role, in characters.
pub const MAX_AGENT_ROLE_CHARS: usize = 100;
/// How long a connection code works.
pub const CONNECT_CODE_MINUTES: i64 = 10;
/// A running task whose connection shows no activity for this long counts as waiting. A session
/// that is killed or crashes sends no end notice, so this is how its task returns to waiting.
pub const STALE_RUN_SECONDS: i64 = 30 * 60;
/// A connection's `lastActivityAt` is rewritten at most this often.
pub(super) const TOUCH_INTERVAL_SECONDS: i64 = 30;
pub(super) const CODE_ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
pub(super) const CODE_LENGTH: usize = 8;
pub(super) const SECRET_PREFIX: &str = "agpjc_";

/// One registered agent. The ID never changes, so cards keep pointing at it through renames.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct AgentRecord {
    pub id: String,
    pub name: String,
    pub role: String,
    /// The AI tool that runs this agent's tasks: `codex` or `claude-code`. Unset means none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool: Option<String>,
    #[serde(with = "crate::clock::serde_utc")]
    pub created_at: DateTime<Utc>,
    #[serde(with = "crate::clock::serde_opt_utc")]
    pub archived_at: Option<DateTime<Utc>>,
    /// The agent every AI task starts with. Created with the first AI task; it cannot be deleted.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub is_default: bool,
}

/// `agents.json`: the project's registered agents.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct AgentsFile {
    pub format_version: u32,
    /// The number the next agent gets; numbers are never reused.
    pub next_agent_id: u32,
    pub agents: Vec<AgentRecord>,
}

impl Default for AgentsFile {
    fn default() -> Self {
        Self {
            format_version: REGISTRY_FORMAT_VERSION,
            next_agent_id: 1,
            agents: Vec::new(),
        }
    }
}

/// Who started a connection's session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConnectionOrigin {
    /// A session the user started in a terminal or an app.
    External,
    /// A session the board launched.
    Board,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct ConnectionRecord {
    pub(super) id: String,
    pub(super) agent_id: String,
    pub(super) secret_digest: String,
    pub(super) origin: ConnectionOrigin,
    pub(super) tool: String,
    pub(super) session_label: String,
    #[serde(with = "crate::clock::serde_utc")]
    pub(super) issued_at: DateTime<Utc>,
    #[serde(with = "crate::clock::serde_utc")]
    pub(super) last_activity_at: DateTime<Utc>,
    #[serde(with = "crate::clock::serde_opt_utc")]
    pub(super) revoked_at: Option<DateTime<Utc>>,
    pub(super) replaced_by: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct ConnectCode {
    pub(super) digest: String,
    pub(super) agent_id: String,
    /// The code was issued by "switch": the exchange replaces the agent's active connection.
    pub(super) switch: bool,
    #[serde(with = "crate::clock::serde_utc")]
    pub(super) expires_at: DateTime<Utc>,
}

/// `.internal/connections.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct ConnectionsFile {
    pub(super) format_version: u32,
    pub(super) codes: Vec<ConnectCode>,
    pub(super) connections: Vec<ConnectionRecord>,
}

impl Default for ConnectionsFile {
    fn default() -> Self {
        Self {
            format_version: REGISTRY_FORMAT_VERSION,
            codes: Vec::new(),
            connections: Vec::new(),
        }
    }
}

/// What callers see of a connection: never the digest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionView {
    pub id: String,
    pub agent_id: String,
    pub origin: ConnectionOrigin,
    pub tool: String,
    pub session_label: String,
    #[serde(with = "crate::clock::serde_utc")]
    pub issued_at: DateTime<Utc>,
    #[serde(with = "crate::clock::serde_utc")]
    pub last_activity_at: DateTime<Utc>,
}

impl From<&ConnectionRecord> for ConnectionView {
    fn from(record: &ConnectionRecord) -> Self {
        Self {
            id: record.id.clone(),
            agent_id: record.agent_id.clone(),
            origin: record.origin,
            tool: record.tool.clone(),
            session_label: record.session_label.clone(),
            issued_at: record.issued_at,
            last_activity_at: record.last_activity_at,
        }
    }
}

/// An agent with its active connection, if any.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentView {
    pub id: String,
    pub name: String,
    pub role: String,
    pub tool: Option<String>,
    #[serde(with = "crate::clock::serde_utc")]
    pub created_at: DateTime<Utc>,
    #[serde(with = "crate::clock::serde_opt_utc")]
    pub archived_at: Option<DateTime<Utc>>,
    pub is_default: bool,
    pub connection: Option<ConnectionView>,
}

/// A connection code as shown to the person once.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IssuedCode {
    pub agent_id: String,
    pub code: String,
    #[serde(with = "crate::clock::serde_utc")]
    pub expires_at: DateTime<Utc>,
}

/// The result of exchanging a code: the secret is returned once and not stored.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Connected {
    pub agent: AgentView,
    pub connection: ConnectionView,
    /// The secret of the new connection. The session keeps it; the board keeps only its digest.
    pub secret: String,
}

/// The connection a verified secret belongs to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verified {
    pub connection_id: String,
    pub agent_id: String,
}
