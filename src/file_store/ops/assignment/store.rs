//! Reading and saving the two files, and the checks every operation shares.

use super::*;

pub(crate) fn digest_of(text: &str) -> String {
    Sha256::digest(text.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub(super) fn random_bytes<const N: usize>() -> Result<[u8; N], AgpjError> {
    let mut bytes = [0u8; N];
    getrandom::fill(&mut bytes).map_err(|e| AgpjError::DatabaseError {
        reason: format!("No random bytes: {e}."),
    })?;
    Ok(bytes)
}

pub(super) fn new_code() -> Result<String, AgpjError> {
    let bytes = random_bytes::<CODE_LENGTH>()?;
    Ok(bytes
        .iter()
        .map(|b| CODE_ALPHABET[*b as usize % CODE_ALPHABET.len()] as char)
        .collect())
}

pub(super) fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Codes are typed by a person: case and a dash in the middle do not matter.
pub(super) fn normalize_code(raw: &str) -> String {
    raw.chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .collect::<String>()
        .to_ascii_uppercase()
}

pub(super) fn agents_path(project_dir: &std::path::Path) -> std::path::PathBuf {
    project_dir.join("agents.json")
}

pub(super) fn connections_path(project_dir: &std::path::Path) -> std::path::PathBuf {
    project_dir.join(".internal").join("connections.json")
}

pub(super) fn load<T: serde::de::DeserializeOwned + Default>(
    shared: &std::path::Path,
    path: &std::path::Path,
) -> Result<T, AgpjError> {
    let label = layout::shared_label(shared, path);
    // Only a confirmed missing file is an empty registry; an unreadable parent is an error.
    if !path.try_exists().map_err(|e| io::read_error(&label, e))? {
        return Ok(T::default());
    }
    let text = io::read_text(path, &label)?;
    let value: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| AgpjError::FileInvalid {
            file: label.clone(),
            field: String::new(),
            reason: format!("The JSON is malformed: {e}."),
        })?;
    crate::file_store::validate::check_format_version(&value, &label)?;
    serde_json::from_value(value).map_err(|e| {
        let message = e.to_string();
        AgpjError::FileInvalid {
            file: label,
            field: message.split('`').nth(1).unwrap_or("").to_string(),
            reason: message,
        }
    })
}

pub(super) fn save<T: Serialize>(
    shared: &std::path::Path,
    path: &std::path::Path,
    value: &T,
) -> Result<(), AgpjError> {
    io::write_json(path, value, &layout::shared_label(shared, path))
}

pub(super) fn check_text(field: &str, text: &str, max: usize) -> Result<String, AgpjError> {
    let trimmed = text.trim();
    if trimmed.is_empty() && field == "name" {
        return Err(AgpjError::InvalidValue {
            field: field.into(),
            reason: "The name cannot be empty.".into(),
        });
    }
    if trimmed.chars().count() > max {
        return Err(AgpjError::InvalidValue {
            field: field.into(),
            reason: format!("The {field} can have at most {max} characters."),
        });
    }
    if trimmed.chars().any(char::is_control) {
        return Err(AgpjError::InvalidValue {
            field: field.into(),
            reason: format!("The {field} must be one line of plain text."),
        });
    }
    Ok(trimmed.to_string())
}

pub(super) fn is_active(record: &ConnectionRecord) -> bool {
    record.revoked_at.is_none()
}

pub(super) fn view_of(agent: &AgentRecord, connections: &ConnectionsFile) -> AgentView {
    AgentView {
        id: agent.id.clone(),
        name: agent.name.clone(),
        role: agent.role.clone(),
        created_at: agent.created_at,
        archived_at: agent.archived_at,
        is_default: agent.is_default,
        connection: connections
            .connections
            .iter()
            .find(|c| c.agent_id == agent.id && is_active(c))
            .map(ConnectionView::from),
    }
}

pub(super) fn find_agent<'a>(file: &'a AgentsFile, id: &str) -> Result<&'a AgentRecord, AgpjError> {
    file.agents
        .iter()
        .find(|agent| agent.id == id)
        .ok_or_else(|| AgpjError::AgentNotFound { agent: id.into() })
}

pub(super) fn ensure_name_free(
    file: &AgentsFile,
    name: &str,
    except: Option<&str>,
) -> Result<(), AgpjError> {
    let taken = file
        .agents
        .iter()
        .any(|a| a.archived_at.is_none() && a.name == name && Some(a.id.as_str()) != except);
    if taken {
        Err(AgpjError::AgentNameTaken { name: name.into() })
    } else {
        Ok(())
    }
}

/// Look the secret up under the project lock, and record the activity when it is a while old.
pub(in crate::file_store::ops) fn verify_locked(
    shared: &std::path::Path,
    dir: &std::path::Path,
    secret: &str,
    now: DateTime<Utc>,
) -> Result<Verified, AgpjError> {
    let path = connections_path(dir);
    let mut connections: ConnectionsFile = load(shared, &path)?;
    let digest = digest_of(secret.trim());
    let Some(record) = connections
        .connections
        .iter_mut()
        .find(|c| c.secret_digest == digest)
    else {
        return Err(AgpjError::NotConnected);
    };
    if !is_active(record) {
        return Err(AgpjError::ConnectionReplaced);
    }
    let verified = Verified {
        connection_id: record.id.clone(),
        agent_id: record.agent_id.clone(),
    };
    if now - record.last_activity_at >= Duration::seconds(TOUCH_INTERVAL_SECONDS) {
        record.last_activity_at = now;
        save(shared, &path, &connections)?;
    }
    Ok(verified)
}
