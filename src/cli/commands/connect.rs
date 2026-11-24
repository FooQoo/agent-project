//! `connect` and `agent ls`: joining this session to a registered agent, and reading the agents.
//! Registering, renaming, and switching agents are a person's actions in the web UI.

use serde_json::json;

use crate::agent::connection_store;
use crate::agent::session_identity::SessionIdentity;
use crate::cli::args::{AgentArgs, AgentCommand, ConnectArgs};
use crate::cli::{Context, Outcome};
use crate::error::AgpjError;
use crate::file_store::ConnectionOrigin;
use crate::server::sessions::Origin;

pub fn connect(ctx: &mut Context, args: &ConnectArgs) -> Result<Outcome, AgpjError> {
    let project = ctx.project_id()?.clone();
    // The environment secret wins over the stored one, so a new connection would be ignored.
    if std::env::var(crate::agent::connection_store::SECRET_ENV).is_ok_and(|v| !v.trim().is_empty())
    {
        return Err(AgpjError::InvalidArgument {
            message: "AGPJ_CONNECTION_SECRET is set, so this session already has its connection; unset it to connect with a code.".into(),
            command: None,
        });
    }
    let data_dir = ctx
        .files
        .shared_dir()
        .parent()
        .map(std::path::Path::to_path_buf)
        .ok_or_else(|| AgpjError::BoardUnavailable {
            reason: "The data folder cannot be found.".into(),
        })?;
    // Check, exchange, and save run under one lock, so two connects of a session cannot both pass.
    let _session = connection_store::lock_session(&data_dir, &project)?;
    // One session holds one agent: a second connection would replace the stored secret of the
    // first and leave it unreleasable.
    let held = match connection_store::try_load_secret(&data_dir, &project)? {
        Some(held) => active_agent(
            ctx.files
                .verify_connection(&project, &held, ctx.clock.now()),
        )?,
        None => None,
    };
    if let Some(agent_id) = held {
        return Err(AgpjError::InvalidArgument {
            message: format!(
                "This session is already connected to agent {agent_id}; run agpj disconnect first."
            ),
            command: None,
        });
    }
    let identity = SessionIdentity::from_env();
    let origin = match identity.origin {
        Origin::Board => ConnectionOrigin::Board,
        Origin::External => ConnectionOrigin::External,
    };
    // The secret is stored before the connection becomes active; a failed store changes nothing.
    let connected = ctx.files.exchange_connect_code_storing(
        &project,
        &args.code,
        origin,
        identity.tool.as_str(),
        &identity.key,
        ctx.clock.now(),
        |secret| connection_store::save_secret(&data_dir, &project, secret),
    )?;
    Ok(Outcome {
        changed: true,
        human: format!(
            "Connected to agent {} ({}): {}\n",
            connected.agent.id, connected.agent.name, connected.agent.role
        ),
        json: json!({"agent": connected.agent, "connection": connected.connection}),
    })
}

/// The agent a stored secret still connects, if any. Only an unknown, revoked, or replaced secret
/// means the session holds no connection; any other failure (a busy project, a read error) is
/// returned, so that a connect never overwrites the secret of a connection that may be active.
fn active_agent(
    verified: Result<crate::file_store::Verified, AgpjError>,
) -> Result<Option<String>, AgpjError> {
    match verified {
        Ok(verified) => Ok(Some(verified.agent_id)),
        Err(AgpjError::NotConnected | AgpjError::ConnectionReplaced) => Ok(None),
        Err(other) => Err(other),
    }
}

pub fn disconnect(ctx: &mut Context) -> Result<Outcome, AgpjError> {
    let project = ctx.project_id()?.clone();
    let data_dir = ctx
        .files
        .shared_dir()
        .parent()
        .map(std::path::Path::to_path_buf)
        .ok_or_else(|| AgpjError::BoardUnavailable {
            reason: "The data folder cannot be found.".into(),
        })?;
    // The same lock as `connect`: a disconnect never deletes a secret a connect just saved.
    let _session = connection_store::lock_session(&data_dir, &project)?;
    let secret =
        connection_store::try_load_secret(&data_dir, &project)?.ok_or(AgpjError::NotConnected)?;
    let paused = ctx
        .files
        .disconnect_session(&project, &secret, ctx.clock.now())?;
    connection_store::delete_secret_if(&data_dir, &project, &secret);
    Ok(Outcome {
        changed: true,
        human: "Disconnected from the agent; its running tasks are waiting.\n".to_string(),
        json: json!({"paused": paused}),
    })
}

pub fn agent(ctx: &mut Context, args: &AgentArgs) -> Result<Outcome, AgpjError> {
    match args.command {
        AgentCommand::Ls => {
            let agents = ctx.files.list_agents(ctx.project_id()?)?;
            let mut human = String::new();
            for agent in &agents {
                let state = match (&agent.archived_at, &agent.connection) {
                    (Some(_), _) => "deleted",
                    (None, Some(_)) => "connected",
                    (None, None) => "not connected",
                };
                human.push_str(&format!(
                    "{}  {}  {}  [{}]\n",
                    agent.id, agent.name, agent.role, state
                ));
            }
            if human.is_empty() {
                human.push_str("No agents are registered.\n");
            }
            Ok(Outcome {
                changed: false,
                human,
                json: json!({"agents": agents}),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_an_unknown_or_replaced_secret_means_no_connection() {
        let verified = crate::file_store::Verified {
            connection_id: "C-1".into(),
            agent_id: "A-1".into(),
        };
        assert_eq!(active_agent(Ok(verified)).unwrap(), Some("A-1".into()));
        assert_eq!(active_agent(Err(AgpjError::NotConnected)).unwrap(), None);
        assert_eq!(
            active_agent(Err(AgpjError::ConnectionReplaced)).unwrap(),
            None
        );
        assert!(matches!(
            active_agent(Err(AgpjError::DatabaseBusy)),
            Err(AgpjError::DatabaseBusy)
        ));
    }
}
